#!/usr/bin/env python3
"""Build the public preview from a reviewed, hash-pinned allowlist. Never publish.

Refuses pending artwork/notices and binaries without a matching build receipt.
No private-package fallback, directory-wide source copy, or external system library.
"""
import argparse
import hashlib
import json
import struct
import tarfile
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parent.parent
VERSION = '0.1.0-preview.1'
ENTRIES = {'EmotionalUI — Modo prueba.sh': 'trial',
           'EmotionalUI — GPU experimental.sh': 'gpu',
           'EmotionalUI — Restaurar.sh': 'restore',
           'EmotionalUI — Desinstalar.sh': 'uninstall'}
PAYLOAD = '.emotional-ui-preview-package'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def checked(root, name, digest):
    rel = PurePosixPath(name)
    if not name or rel.is_absolute() or '..' in rel.parts or str(rel) != name or '\\' in name:
        raise ValueError('Unsafe allowlist path')
    p = root/name
    if any(q.is_symlink() for q in (p, *p.parents)) or not p.is_file() or sha(p) != digest:
        raise ValueError('Changed or missing allowlisted file: '+name)
    return p


def review(plan, root=ROOT):
    if plan.get('version') != VERSION or not plan.get('source_files'):
        raise ValueError('Missing version/source allowlist')
    problems = [name for name, value in plan.get('clearance', {}).items() if value != 'cleared']
    if set(plan.get('clearance', {})) != {'room_redistribution', 'dependency_notices', 'public_source_privacy'}:
        problems.append('incomplete clearance record')
    if problems:
        raise ValueError('Public ZIP blocked: '+', '.join(problems))
    if not all(plan.get('clearance_evidence', {}).get(name) for name in plan['clearance']):
        raise ValueError('Clearance requires recorded evidence, not status flags alone')
    for name, digest in plan['source_files'].items():
        checked(root, name, digest)
    return hashlib.sha256(json.dumps(plan['source_files'], sort_keys=True).encode()).hexdigest()


def build(plan, receipt, software, gpu, output, root=ROOT):
    snapshot = review(plan, root)
    if receipt.get('source_snapshot_sha256') != snapshot:
        raise ValueError('Build receipt does not match the reviewed source snapshot')
    for renderer, binary in [('software', software), ('femtovg', gpu)]:
        data = binary.read_bytes()
        if len(data) < 64 or data[:6] != b'\x7fELF\x02\x01' or struct.unpack_from('<H', data, 18)[0] != 183:
            raise ValueError('Requires ELF64 AArch64: '+renderer)
        if sha(binary) != receipt.get('binaries', {}).get(renderer):
            raise ValueError('Binary checksum does not match build receipt: '+renderer)
    if output.exists() or output.is_symlink():
        raise ValueError('Refusing to overwrite an existing ZIP')
    with tempfile.TemporaryDirectory(prefix='emotional-preview-') as tmp:
        base = Path(tmp)
        package = base/PAYLOAD
        package.mkdir()
        copies = {'emotional-ui': software, 'emotional-ui-gpu': gpu}
        for name in ('manage.py', 'menu.py'):
            copies[name] = checked(root, 'platform/distribution/'+name, plan['source_files']['platform/distribution/'+name])
        copies['phase12_preview.py'] = checked(root, 'platform/darkos/phase12_preview.py', plan['source_files']['platform/darkos/phase12_preview.py'])
        copies['input.toml'] = checked(root, 'platform/distribution/rg353v-input.toml', plan['source_files']['platform/distribution/rg353v-input.toml'])
        for name in ('LICENSE', 'THIRD_PARTY_LICENSES.md', 'AUDIO_ASSET_PROVENANCE.md',
                     'assets/fonts/DejaVu-COPYRIGHT.txt', 'docs/dependency-notices.txt'):
            copies['licenses/'+Path(name).name] = checked(root, name, plan['source_files'][name])
        for name, source in copies.items():
            target = package/name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.read_bytes())
            target.chmod(0o755 if name in ('emotional-ui', 'emotional-ui-gpu') else 0o644)
        (package/'release-plan.json').write_text(json.dumps(plan, indent=2, sort_keys=True)+'\n')
        (package/'build-receipt.json').write_text(json.dumps(
            dict(source_snapshot_sha256=snapshot,
                 binaries={key: receipt['binaries'][key] for key in ('software', 'femtovg')}),
            indent=2, sort_keys=True)+'\n')
        with tarfile.open(package/'corresponding-source.tar.gz', 'w:gz') as tar:
            for name, digest in sorted(plan['source_files'].items()):
                p = checked(root, name, digest)
                info = tar.gettarinfo(str(p), 'emotional-ui/'+name)
                info.uid = info.gid = 0
                info.uname = info.gname = ''
                info.mtime = 0
                with p.open('rb') as f:
                    tar.addfile(info, f)
        files = {p.relative_to(package).as_posix(): sha(p) for p in package.rglob('*') if p.is_file()}
        manifest = dict(schema=1, kind='emotional-ui-trial', version=VERSION,
                        architecture='aarch64', main_install_enabled=False, public_release=True,
                        default_renderer='software', experimental_renderer='femtovg',
                        executable_files=['emotional-ui', 'emotional-ui-gpu'],
                        source_snapshot_sha256=snapshot, files=files)
        (package/'package.json').write_text(json.dumps(manifest, indent=2, sort_keys=True)+'\n')
        (base/'READ-ME-FIRST.md').write_bytes(checked(root, 'PREVIEW_INSTALL.md', plan['source_files']['PREVIEW_INSTALL.md']).read_bytes())
        for name, action in ENTRIES.items():
            script = base/name
            script.write_text('#!/bin/sh\nset -eu\nexec python3 "$(dirname -- "$0")/'+PAYLOAD+'/menu.py" '+action+'\n')
            script.chmod(0o755)
        # Exclusive creation; a failed build never replaces an existing archive.
        try:
            with zipfile.ZipFile(output, 'x', compression=zipfile.ZIP_DEFLATED) as z:
                for p in sorted(base.rglob('*')):
                    if p.is_file():
                        z.write(p, p.relative_to(base).as_posix())
        except Exception:
            # Preserve a partial result for inspection; never label it successful.
            raise
    return dict(zip=output.name, sha256=sha(output), bytes=output.stat().st_size)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--plan', type=Path, default=ROOT/'platform/distribution/preview-files.json')
    p.add_argument('--receipt', type=Path)
    p.add_argument('--software', type=Path)
    p.add_argument('--gpu', type=Path)
    p.add_argument('--output', type=Path)
    a = p.parse_args()
    try:
        plan = json.loads(a.plan.read_text())
        review(plan)
        if not all((a.receipt, a.software, a.gpu, a.output)):
            raise ValueError('Cleared source also requires both binaries, a matching build receipt and a new output path')
        print(json.dumps(build(plan, json.loads(a.receipt.read_text()), a.software, a.gpu, a.output)))
    except (ValueError, KeyError, OSError) as e:
        p.exit(1, str(e)+'\n')


if __name__ == '__main__':
    main()
