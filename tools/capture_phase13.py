#!/usr/bin/env python3
"""Real 640x480 Slint review; owned fixtures, no hardware or artistic approval."""
import argparse
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
from PIL import Image, ImageDraw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--seed', type=Path, required=True)
    parser.add_argument('--rom-root', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--cases', nargs='+', help='Only regenerate affected captures')
    args = parser.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=False)
    cases = [('continuar', 'resume', []), ('tu-momento', 'emotional', []),
             ('biblioteca', 'resume', ['e', 'e']), ('ajustes', 'resume', ['e', 'e', 'e']),
             ('contextual', 'resume', ['d']), ('foco-pestanas', 'resume', ['up', 'right']),
             ('capsula', 'resume', ['up', 'up', 'z']),
             ('movimiento-reducido', 'emotional', []), ('titulo-largo', 'emotional', []),
             ('continuar-titulo-largo', 'resume', []),
             ('sin-arte', 'emotional', []), ('vacio', 'resume', [])]
    if args.cases:
        cases = [case for case in cases if case[0] in args.cases]
    rows = []
    for name, view, keys in cases:
        db = out / (name + '.sqlite')
        if name != 'vacio':
            shutil.copyfile(args.seed, db)
        if name in ('titulo-largo', 'continuar-titulo-largo', 'sin-arte'):
            with sqlite3.connect(db) as con:
                for ident, raw in con.execute('SELECT id,metadata FROM imported_games').fetchall():
                    lines = raw.splitlines()
                    if name.endswith('titulo-largo'):
                        lines = [('name = '+json.dumps('El pequeño viaje de regreso al último faro entre las montañas del norte'))
                                 if line.startswith('name = ') else line for line in lines]
                    else:
                        # Fixture-only removal of references; originals remain untouched.
                        fields = ('image', 'thumbnail', 'boxart', 'fanart', 'screenshot', 'marquee')
                        lines = [line for line in lines if line.split('=',1)[0].strip() not in fields]
                    con.execute('UPDATE imported_games SET metadata=? WHERE id=?', ('\n'.join(lines)+'\n', ident))
        steps = []
        for i, key in enumerate(keys):
            steps.extend([dict(at_ms=500+i*100, text=key, pressed=True),
                          dict(at_ms=508+i*100, text=key, pressed=False)])
        replay = out / (name + '.toml')
        steps.append(dict(at_ms=1500, capture=str(out / (name+'.png'))))
        replay.write_text('keyboard=true\n[mapping]\n'+''.join(
            '\n[[steps]]\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in step.items()) for step in steps))
        cmd = [str(args.binary.resolve()), '--db', str(db), '--view', view,
               '--at', '2026-10-19T18:30', '--audio-mode', 'silence', '--reduced-motion',
               '--smoke-test', '--smoke-seconds', '2', '--sim-replay', str(replay)]
        cmd += ['--demo-count', '0'] if name == 'vacio' else ['--library-root', str(args.rom_root.resolve())]
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=25,
                                env=dict(os.environ, SLINT_SCALE_FACTOR='1',
                                         EMOTIONAL_ARTWORK_CACHE=str(out.parent/'cache')))
        (out/(name+'.log')).write_text(result.stdout+'\n'+result.stderr)
        if result.returncode:
            raise RuntimeError(name+': '+result.stderr)
        with Image.open(out/(name+'.png')) as im:
            assert im.mode == 'RGB' and im.size == (640,480), (name, im.mode, im.size)
        rows.append(dict(case=name, real_slint=True, hardware=False))
    if all((out/(name+'.png')).exists() for name in ['continuar','tu-momento']):
        pair = Image.new('RGB',(1296,512),'#171b19')
        draw = ImageDraw.Draw(pair)
        for i, name in enumerate(['continuar','tu-momento']):
            draw.text((i*656+16,8),name,fill='#eee8d8')
            with Image.open(out/(name+'.png')) as im: pair.paste(im,(i*656,32))
        pair.save(out/'comparacion.png')
    (out/'captures.json').write_text(json.dumps(dict(captures=rows, human_approval=False),indent=2)+'\n')
    print(out)


if __name__ == '__main__':
    main()
