#!/usr/bin/env python3
"""Collect locally available Cargo license texts; report gaps instead of inventing them.
Input: locked/offline Cargo metadata filtered to ARM64 with linuxkms-gpu enabled.
The resolution includes build and optional crates, not proof of linkage into an ELF.
"""
import argparse
import hashlib
import json
from pathlib import Path


def collect(metadata, output, recovered=None):
    ids = {node['id'] for node in metadata['resolve']['nodes']}
    packages = sorted((p for p in metadata['packages'] if p['id'] in ids),
                      key=lambda p: (p['name'], p['version']))
    inventory = ['name\tversion\tlicense\trepository']
    notices = ['# ARM64 preview dependency notices — incomplete until gaps are resolved\n',
               'Slint 1.13.1 is used under GPL-3.0-only. Other upstream alternatives\n'
               'are retained verbatim as upstream notices, not selected for EmotionalUI.\n'
               'Cargo resolution includes build/optional crates; it is not a binary linkage report.\n']
    missing = []
    supplements = []
    recovered_records = {}
    if recovered:
        recovery_data = json.loads((recovered/'manifest.json').read_text())
        entries = recovery_data['entries'] if isinstance(recovery_data, dict) else recovery_data
        recovered_records = {(p['name'], p['version']): p for p in entries}
    for p in packages:
        inventory.append('\t'.join(str(p.get(k) or '-').replace('\n', ' ').replace('\t', ' ')
                                   for k in ('name', 'version', 'license', 'repository')))
        if p['name'].startswith('emotional-'):
            continue
        root = Path(p['manifest_path']).parent
        files = {f for f in root.iterdir() if f.is_file() and
                 f.name.lower().startswith(('license', 'copying', 'copyright', 'notice', 'authors'))}
        if p.get('license_file') and (root/p['license_file']).is_file():
            files.add(root/p['license_file'])
        if (root/'LICENSES').is_dir():
            files.update(f for f in (root/'LICENSES').iterdir() if f.is_file())
        notices.append('\n'+'='*72+'\n'+p['name']+' '+p['version']+'\n'+
                       str(p.get('license') or 'UNDECLARED')+'\n'+str(p.get('repository') or '')+'\n')
        upstream = recovered_records.get((p['name'], p['version']))
        recovered_texts = []
        if not files and upstream:
            for item in upstream.get('files', []):
                source = recovered/item['file']
                if source.is_symlink() or hashlib.sha256(source.read_bytes()).hexdigest() != item['sha256']:
                    raise ValueError('Upstream notice checksum mismatch')
                recovered_texts.append('\n--- '+item['url']+' ---\n'+source.read_text()+'\n')
        if not files and not recovered_texts and (p['name'], p['version']) in {('drm-fourcc', '2.2.0'), ('simd_helpers', '0.1.0')} and p.get('license') == 'MIT':
            # These exact published crates declare SPDX MIT but supply no license
            # file/copyright line. Preserve what was actually received; never
            # manufacture a copyright year or infer a rights holder from authors.
            original = root/'Cargo.toml.orig'
            if not original.is_file():
                raise ValueError('Missing original upstream MIT declaration')
            import tomllib
            declared = tomllib.loads(original.read_text())['package']
            if declared.get('license') != 'MIT':
                raise ValueError('Unexpected upstream license declaration')
            standard = Path(__file__).resolve().parents[1]/'docs/licenses/MIT-standard.txt'
            supplements.append(dict(name=p['name'], version=p['version'],
                                    scope='build-time proc-macro' if p['name']=='simd_helpers' else 'runtime',
                                    authors_metadata=declared.get('authors', []),
                                    upstream_declaration_sha256=hashlib.sha256(original.read_bytes()).hexdigest(),
                                    standard_terms='https://spdx.org/licenses/MIT.html',
                                    note='Upstream supplies no separate copyright notice. Standard terms below are a supplement, not an upstream-specific grant or invented copyright. Placeholders deliberately unchanged.'))
            recovered_texts.append('\nRECEIVED UPSTREAM SPDX DECLARATION (original manifest, verbatim):\n'+original.read_text())
            for f in sorted(root.glob('README*')):
                if f.is_file():
                    recovered_texts.append('\nRECEIVED UPSTREAM '+f.name+':\n'+f.read_text())
            recovered_texts.append('\nSTANDARD MIT TERMS SUPPLEMENT — https://spdx.org/licenses/MIT.html\n'
                                   'Not supplied as a separate license file by this crate. No upstream copyright year/holder is invented; template placeholders are unmodified.\n'+standard.read_text())
        if not files and not recovered_texts:
            missing.append({'name': p['name'], 'version': p['version'], 'license': p.get('license'),
                            'repository': p.get('repository')})
            notices.append('PENDING: upstream license text not present in the local crate package.\n')
        for f in sorted(files):
            notices.append('\n--- '+f.relative_to(root).as_posix()+' ---\n'+f.read_text(errors='strict')+'\n')
        notices.extend(recovered_texts)
    output.mkdir(parents=True, exist_ok=True)
    (output/'dependency-licenses.tsv').write_text('\n'.join(inventory)+'\n')
    (output/'dependency-notices.txt').write_text(''.join(notices))
    (output/'dependency-notices-pending.json').write_text(json.dumps(missing, indent=2)+'\n')
    (output/'dependency-notices-supplements.json').write_text(json.dumps(supplements, indent=2)+'\n')
    if not missing:
        notices[0] = '# ARM64 preview dependency notices — received notices and identified standard supplements\n'
        (output/'dependency-notices.txt').write_text(''.join(notices))
    return {'packages': len(packages), 'missing_license_texts': len(missing)}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('metadata', type=Path)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--recovered', type=Path)
    a = p.parse_args()
    print(json.dumps(collect(json.loads(a.metadata.read_text()), a.output, a.recovered)))
