#!/usr/bin/env python3
"""Focused secondary-title captures using owned fixtures; no LCD or GPU claim."""
import argparse
import hashlib
import json
import re
import shutil
import sqlite3
from contextlib import closing
from pathlib import Path
from PIL import Image
from check_phase14_motion import key, run


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cases():
    sentence = 'La última luz del faro junto al río y el viaje de regreso a casa '
    return [
        ('short', 'Faro', False),
        ('medium', 'El viaje de vuelta entre montañas', False),
        ('sixty', (sentence * 2)[:60], False),
        ('hundred', (sentence * 2)[:100], False),
        ('long-word', 'ExtraordinariamenteIncomprensiblementeIninterrumpidamente', False),
        ('accented', 'Más allá del río: música, ilusión y pequeños sueños', False),
        ('special', '¿Volvemos? ¡Sí! — Edición #2 / verano & otoño (2026)', False),
        ('no-art', (sentence * 2)[:60], True),
        ('secondary-wrap', 'Entre montañas', False),
    ]


def prepare(seed, output, title, no_art):
    shutil.copyfile(seed, output)
    with closing(sqlite3.connect(output)) as connection, connection:
        rows = connection.execute('SELECT id, metadata FROM imported_games ORDER BY id').fetchall()
        assert len(rows) == 3, 'Expected the existing owned three-game fixture'
        for game_id, metadata in rows:
            metadata, replacements = re.subn(r'^name\s*=.*$', 'name = ' + json.dumps(title, ensure_ascii=False), metadata, flags=re.M)
            assert replacements == 1
            if no_art:
                metadata = re.sub(r'^(?:boxart|image|thumbnail|screenshot|fanart|titleshot|marquee)\s*=.*\n?', '', metadata, flags=re.M)
            connection.execute('UPDATE imported_games SET metadata=? WHERE id=?', (metadata, game_id))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('seed', 'fixtures', 'out'):
        parser.add_argument('--' + name, required=True, type=Path)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    original = digest(args.seed)
    records = []
    for name, title, no_art in cases():
        seed = args.out / (name + '.sqlite')
        prepare(args.seed, seed, title, no_art)
        record = dict(case=name, title=title, characters=len(title), no_art=no_art, physical=False, captures=[])
        if not args.prepare_only:
            if not args.binary or not args.baseline:
                parser.error('Capture mode requires --baseline and --binary')
            for label, binary in [('before', args.baseline), ('after', args.binary)]:
                folder = args.out / (name + '-' + label)
                steps = [dict(at_ms=700, capture=str(folder/'continuar.png'), resume_count=3)]
                steps += key(900, 'e')
                steps += [dict(at_ms=1400, capture=str(folder/'tu-momento.png'), active_section=1, hero_title=title.strip())]
                steps += key(1600, 'down') + key(1800, 'd')
                steps += [dict(at_ms=2150, capture=str(folder/'full-title-options.png'), game_options=True)]
                run(binary.resolve(), args.out, seed, args.fixtures, folder.name, 'resume', steps, seconds=3)
                for image in folder.glob('*.png'):
                    with Image.open(image) as captured:
                        assert captured.size == (640, 480)
                    record['captures'].append(str(image))
            for view in ('continuar', 'tu-momento', 'full-title-options'):
                with Image.open(args.out/(name+'-before')/(view+'.png')) as before, Image.open(args.out/(name+'-after')/(view+'.png')) as after:
                    pair = Image.new('RGB', (1280, 480))
                    pair.paste(before, (0, 0)); pair.paste(after, (640, 0))
                    pair.save(args.out/(name+'-'+view+'-comparison.png'))
        records.append(record)
    assert digest(args.seed) == original, 'The input seed must remain byte-identical'
    result = dict(source_seed_sha256=original, physical=False, cases=records,
                  approval='HUMAN_REVIEW_PENDING', scope='CompactGame overflow; fonts and geometry unchanged')
    for label in ('baseline', 'binary'):
        binary = getattr(args, label)
        if binary:
            result[label+'_sha256'] = digest(binary)
    (args.out/'report.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')


if __name__ == '__main__':
    main()
