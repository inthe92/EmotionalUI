#!/usr/bin/env python3
"""Build the reversible ARM64 beta ZIP; no network or device access."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_entrypoint_imports(bundle):
    code = """import importlib,sys
from pathlib import Path
root=Path(sys.argv[1]).resolve()
sys.path.insert(0,str(root))
for name in ['manage','phase12_preview','functional_preview','menu','functional_menu','player_menu','player_bootstrap','darkos_launcher_profiles','rom_drop']:
    module=importlib.import_module(name)
    assert Path(module.__file__).resolve().parent==root,name
"""
    result = subprocess.run([sys.executable,'-I','-B','-c',code,str(bundle.resolve())],
                            capture_output=True,text=True,timeout=15)
    if result.returncode:
        raise RuntimeError('Incomplete player package: '+result.stderr[-1500:])


def build(binary, output):
    version = (ROOT/'app/VERSION').read_text().strip()
    digest = sha(binary)
    output.mkdir(exist_ok=False)
    prefix = 'EmotionalUI-Beta-'+version
    bundle = output/prefix
    bundle.mkdir()
    sources = {
        'emotional-ui': binary,
        'input.toml': ROOT/'platform/distribution/rg353v-input.toml',
        'LICENSE': ROOT/'LICENSE',
        'LICENSE-FONTS.txt': ROOT/'assets/fonts/DejaVu-COPYRIGHT.txt',
        'THIRD_PARTY_LICENSES.md': ROOT/'THIRD_PARTY_LICENSES.md',
        'DEPENDENCY-NOTICES.txt': ROOT/'docs/dependency-notices.txt',
        'LICENSE-ATLAS.txt': ROOT/'assets/atlas/BETA-NOTICE.md',
        'emotional-atlas.db': ROOT/'assets/atlas/emotional-atlas.db',
        'emotional-atlas.json': ROOT/'assets/atlas/emotional-atlas.json',
        'manage.py': ROOT/'platform/distribution/manage.py',
        'menu.py': ROOT/'platform/distribution/menu.py',
        'phase12_preview.py': ROOT/'platform/darkos/phase12_preview.py',
        'functional_preview.py': ROOT/'platform/darkos/functional_preview.py',
        'functional_menu.py': ROOT/'platform/darkos/functional_menu.py',
        'player_menu.py': ROOT/'platform/darkos/player_menu.py',
        'player_bootstrap.py': ROOT/'platform/darkos/player_bootstrap.py',
        'rom_drop.py': ROOT/'platform/darkos/rom_drop.py',
        'darkos_launcher_profiles.py': ROOT/'tools/darkos_launcher_profiles.py',
    }
    for name, source in sources.items():
        shutil.copyfile(source, bundle/name)
    verify_entrypoint_imports(bundle)
    (bundle/'emotional-ui').chmod(0o755)
    (bundle/'README.txt').write_text(
        'EmotionalUI '+version+' · RG353V / dArkOS ARM64\n'
        'Copia esta carpeta y EmotionalUI - Beta.sh al mismo directorio Ports.\n'
        'Abre EmotionalUI - Beta desde el menu de Ports.\n'
        'Conserva EmulationStation y la version anterior. No cambia el arranque.\n'
        'Datos y progreso privados: ~/.local/share/emotional-ui-player\n'
        'Salir: Ajustes / Volver a la consola. No borres esta carpeta durante una partida.\n'
        'Atlas experimental: identidades y perfiles no equivalen a ROMs instaladas ni revision humana.\n'
        'Los juegos sin perfil siguen disponibles en Biblioteca.\n'
        'Emotional Drop: abre la capsula, elige Añadir juegos y escanea el QR en la misma red.\n'
        'La transferencia es temporal; los archivos existentes no se reemplazan.\n'
        'Requiere emuladores/cores ya instalados y compatibles; no incluye ROMs ni BIOS.\n'
        'La compilacion y las pruebas locales no certifican el hardware de esta beta.\n')
    beta = dict(kind='emotional-ui-beta', beta=True, version=version,
                private_demo=False, public_release=True, functional_preview=True,
                binary_sha256=digest,
                files={p.name:sha(p) for p in sorted(bundle.iterdir())})
    (bundle/'phase12-manifest.json').write_text(json.dumps(beta,indent=2)+'\n')
    package = dict(schema=1, kind='emotional-ui-trial', architecture='aarch64',
                   main_install_enabled=False, beta=True, version=version,
                   public_release=True,
                   files={p.name:sha(p) for p in sorted(bundle.iterdir())},
                   executable_files=['emotional-ui'])
    (bundle/'package.json').write_text(json.dumps(package,indent=2)+'\n')
    launcher = output/'EmotionalUI - Beta.sh'
    launcher.write_text('#!/bin/sh\nset -eu\neui_beta_dir=$(CDPATH= cd -- "$(dirname -- "$0")/'+prefix+'" && pwd)\nexec /usr/bin/python3 -B "$eui_beta_dir/player_bootstrap.py"\n')
    launcher.chmod(0o755)
    archive = output/(prefix+'-RG353V.zip')
    with zipfile.ZipFile(archive,'x',compression=zipfile.ZIP_DEFLATED,compresslevel=9) as target:
        target.write(launcher,launcher.name)
        for path in sorted(bundle.iterdir()):
            target.write(path,prefix+'/'+path.name)
    result = dict(version=version,binary_sha256=digest,zip_sha256=sha(archive),
                  bundle=str(bundle),zip=str(archive),launcher=str(launcher),
                  bytes=archive.stat().st_size,physical_validated=False,
                  boot_modified=False,roms_or_bios_included=False)
    (output/'delivery.json').write_text(json.dumps(result,indent=2)+'\n')
    (output/'SHA256SUMS').write_text(sha(archive)+'  '+archive.name+'\n')
    return result


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    a=p.parse_args()
    print(json.dumps(build(a.binary.resolve(),a.output.resolve()),indent=2))
