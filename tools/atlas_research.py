#!/usr/bin/env python3
"""Small offline evidence batches for Atlas. No scraper, network or AI runtime.

Sources and original paraphrases are entered after reading a page. Applying the
same batch is idempotent; changing an already applied batch fails closed.
"""
import argparse
import copy
import hashlib
import json
import re
from collections import Counter
from datetime import date
from pathlib import Path
from urllib.parse import urlsplit, urlunsplit

SCORED = {'calm', 'intensity', 'exploration', 'mystery', 'mental_demand',
          'tension', 'pace', 'resume_ease', 'short_sessions'}
RUBRIC = {
    'id': 'experience-anchors-v1',
    'values': [0, 20, 40, 60, 80, 100],
    'meaning': 'Ordinal editorial anchors, not ratings, probabilities or player measurements.',
    'anchors': {
        'calm': '0 sustained pressure; 20 pressured; 40 mixed; 60 mostly unhurried; 80 quiet focus; 100 no described pressure.',
        'intensity': '0 no action; 20 sparse action; 40 intermittent; 60 continuous moderate action; 80 fast demanding action; 100 relentless action.',
        'exploration': '0 absent; 20 linear traversal; 40 occasional branches; 60 optional secrets; 80 revisiting and substantial discovery; 100 discovery governs progress.',
        'mystery': '0 explained throughout; 20 few unknowns; 40 intermittent questions; 60 recurring uncertainty; 80 enigmatic world; 100 uncertainty dominates.',
        'mental_demand': '0 little decision-making; 20 simple choices; 40 routine planning; 60 repeated planning; 80 multi-step reasoning; 100 sustained complex deduction.',
        'tension': '0 no described stakes; 20 low stakes; 40 intermittent threat; 60 frequent threat; 80 sustained threat; 100 constant severe threat.',
        'pace': '0 no advancement; 20 deliberate/menu-led; 40 stop-start; 60 continuous moderate; 80 fast; 100 very fast sustained.',
        'resume_ease': '0 extensive reconstruction; 20 substantial context; 40 remembering goals; 60 recognizable local goal; 80 self-contained tasks; 100 immediately legible task.',
        'short_sessions': '0 no natural stopping unit; 20 long connected tasks; 40 context-heavy units; 60 separable stages; 80 short tasks/matches; 100 immediately repeatable units. Never a minute estimate.',
    },
    'confidence': {
        '60': 'Inference from documented mechanics; limited experiential coverage.',
        '65': 'Explicit relevant account from one editorial voice; still a proposal.',
        '70': 'Relevant corroboration by at least two independent source groups; not calibrated certainty.',
    },
    'mechanics_confidence': {
        '75': 'Explicitly described mechanics in one consulted source group; direct play not performed.',
        '85': 'Specific mechanical description corroborated by two independent consulted groups; not a calibrated probability.',
    },
    'unknown': 'Omit unsupported attributes. Nostalgia is personal; never scored here. No session-duration estimates.',
}


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False,
                                     separators=(',', ':')).encode()).hexdigest()


def canonical_url(url):
    p = urlsplit(url)
    if p.scheme != 'https' or not p.hostname or p.username or p.password or p.query:
        raise ValueError('Evidence needs a public canonical HTTPS URL without credentials/query')
    return urlunsplit(('https', p.netloc.lower(), p.path.rstrip('/') or '/', '', ''))


def validate_research(d):
    research = d.get('research')
    if research is None:
        if any(a is not None and ('evidence' in a or 'rubric' in a)
               for g in d['games'] for a in [*g['objective'].values(), *g['editorial'].values()]):
            raise ValueError('Detailed provenance cannot exist without research registry')
        return
    if research.get('schema') != 1 or research.get('rubric') != RUBRIC:
        raise ValueError('Unsupported research schema/rubric')
    games = {g['id']: g for g in d['games']}
    sources, claims = d['sources'], research['claims']
    source_ids = set(research['source_ids'])
    if len(research['source_ids']) != len(set(research['source_ids'])):
        raise ValueError('Duplicate source registry ID')
    documents = {}
    for key in research['source_ids']:
        s = sources[key]
        required = {'url', 'title', 'medium', 'author', 'published', 'accessed',
                    'kind', 'language', 'game_id', 'platform', 'edition',
                    'independence_key', 'license', 'facts', 'locator', 'access_policy'}
        if not required <= s.keys() or any(not s[k] for k in required - {'author', 'published'}):
            raise ValueError('Incomplete source provenance: ' + key)
        canonical = canonical_url(s['url'])
        document = canonical, s['locator']
        if document in documents and documents[document] != key:
            raise ValueError('Duplicate source document; redirects/syndication are not new voices')
        documents[document] = key
        if s['game_id'] not in games or s['platform'] != games[s['game_id']]['objective']['platform']['value']:
            raise ValueError('Wrong work/platform in source')
        if not s['edition'].get('reviewed') or not s['edition'].get('scope'):
            raise ValueError('Explicit reviewed edition/scope required')
        if s['accessed'] is None:
            raise ValueError('Access date is required')
        for k in ('published', 'accessed'):
            if s[k] is not None:
                day = date.fromisoformat(s[k])
                if day > date.fromisoformat(s['accessed']):
                    raise ValueError('Publication/access chronology mismatch')
        if s['kind'] not in ('publisher', 'review', 'retrospective', 'player-opinion'):
            raise ValueError('Unknown evidence source kind')
        if s['kind'] == 'player-opinion' and not s.get('sample_limit'):
            raise ValueError('Community samples need explicit limitations')
    for key, c in claims.items():
        if c.get('game_id') not in games or c.get('kind') not in ('fact', 'interpretation', 'player-opinion'):
            raise ValueError('Invalid claim identity/kind')
        if c.get('status') not in ('source_checked', 'proposed', 'disputed') or not c.get('rationale') or not c.get('uncertainty'):
            raise ValueError('Claim needs reasoning, uncertainty and editorial state')
        if not isinstance(c.get('summary'), str) or not 1 <= len(c['summary']) <= 800:
            raise ValueError('Bounded original paraphrase required')
        if not c.get('sources') or len(set(c['sources'])) != len(c['sources']):
            raise ValueError('Specific unique sources required')
        for src in c['sources']:
            if src not in source_ids or sources[src]['game_id'] != c['game_id']:
                raise ValueError('Claim/source work mismatch')
        if c['kind'] == 'fact' and any(sources[s]['kind'] == 'player-opinion' for s in c['sources']):
            raise ValueError('A player opinion is not factual mechanics evidence')
        for other in c.get('contrasts', []):
            if other not in claims or claims[other]['game_id'] != c['game_id']:
                raise ValueError('Invalid contrasting claim')
    for g in games.values():
        for field, a in list(g['objective'].items()) + list(g['editorial'].items()):
            if g['id'] in research['records'] and (field in g['editorial'] or field == 'mechanics') and (a is None or not a.get('evidence')):
                raise ValueError('Enriched attributes cannot lose their specific evidence')
            if a is None or 'evidence' not in a:
                continue
            evidence = a['evidence']
            if not evidence or len(evidence) != len(set(evidence)):
                raise ValueError('Assertion needs unique evidence')
            relevant = []
            for cid in evidence:
                if cid not in claims or claims[cid]['game_id'] != g['id']:
                    raise ValueError('Assertion/claim work mismatch')
                c = claims[cid]
                if c['status'] == 'disputed' or field not in c.get('supports', []):
                    raise ValueError('Disputed/irrelevant evidence cannot drive a profile')
                relevant.extend(c['sources'])
            if a['source'] not in relevant:
                raise ValueError('Legacy source must be present in detailed evidence')
            if field == 'mechanics':
                if a['confidence'] not in (75, 85):
                    raise ValueError('Documentary confidence band required')
                if a['confidence'] == 85 and len({sources[s]['independence_key'] for s in relevant}) < 2:
                    raise ValueError('Mechanical corroboration needs independent sources')
            if field in g['editorial']:
                if field not in SCORED or a['value'] not in RUBRIC['values'] or a.get('rubric') != RUBRIC['id']:
                    raise ValueError('Unsupported experiential score/rubric')
                if a['confidence'] not in (60, 65, 70):
                    raise ValueError('Explainable confidence band required')
                if a['confidence'] == 70 and len({sources[s]['independence_key'] for s in relevant}) < 2:
                    raise ValueError('Corroboration cannot be manufactured by syndication')
    for gid, record in research['records'].items():
        if gid not in games or not record.get('gaps') or not record.get('unknowns'):
            raise ValueError('Coverage must preserve gaps and unknowns')
    for receipt in research['batches'].values():
        if len(receipt['sha256']) != 64:
            raise ValueError('Invalid batch receipt')


def apply_batch(source, batch):
    import atlas
    d = copy.deepcopy(atlas.validate(source))
    if batch.get('schema') != 1 or not batch.get('batch_id') or batch.get('rubric') != RUBRIC['id']:
        raise ValueError('Unsupported research batch')
    if not re.fullmatch(r'\d+\.\d+\.\d+-experimental\.\d+', batch.get('version', '')):
        raise ValueError('Research batch must target an experimental version')
    r = d.setdefault('research', dict(schema=1, rubric=copy.deepcopy(RUBRIC), source_ids=[], claims={}, records={}, batches={}))
    sha = digest(batch)
    if batch['batch_id'] in r['batches']:
        if r['batches'][batch['batch_id']]['sha256'] != sha:
            raise ValueError('Applied batch changed; create a new batch ID')
        return d
    for key, value in batch['sources'].items():
        if key in d['sources'] and d['sources'][key] != value:
            raise ValueError('Source conflict: ' + key)
        d['sources'][key] = copy.deepcopy(value)
        if key not in r['source_ids']:
            r['source_ids'].append(key)
    for key, value in batch['claims'].items():
        if key in r['claims'] and r['claims'][key] != value:
            raise ValueError('Claim conflict: ' + key)
        r['claims'][key] = copy.deepcopy(value)
    games = {g['id']: g for g in d['games']}
    exp = d.setdefault('experiment', dict(channel='experimental', selected_ids=[], human_review=None,
                                        author='Codex evidence synthesis; human review pending', explanations={}))
    seen = set()
    for row in batch['profiles']:
        gid = row['id']
        if gid in seen or gid not in games or gid in r['records']:
            raise ValueError('Duplicate/unknown/already enriched work')
        seen.add(gid)
        g = games[gid]
        if any(a and a['status'] == 'approved' for a in [g['objective']['mechanics'], *g['editorial'].values()]):
            raise ValueError('Cannot overwrite approved metadata')
        g['objective']['mechanics'] = copy.deepcopy(row['mechanics'])
        if not g['objective']['mechanics'].get('evidence'):
            raise ValueError('A research batch must cite mechanics evidence')
        g['objective']['mechanics'].update(status='source_checked', origin='evidence_batch_paraphrase')
        g['objective']['mechanics'].pop('reviewer', None)
        g['editorial'] = {}
        for field, a in row['attributes'].items():
            if field not in SCORED or not a.get('evidence') or not a.get('note'):
                raise ValueError('A scored proposal needs attributable evidence and reasoning')
            a = copy.deepcopy(a)
            a.update(status='proposed', origin='evidence_editorial_inference', rubric=RUBRIC['id'])
            a.pop('reviewer', None)
            g['editorial'][field] = a
        r['records'][gid] = {k: copy.deepcopy(row[k]) for k in ('gaps', 'unknowns')}
        exp['explanations'][gid] = row['explanation']
        if gid not in exp['selected_ids']:
            exp['selected_ids'].append(gid)
    r['batches'][batch['batch_id']] = dict(sha256=sha, game_ids=sorted(seen))
    r['source_ids'].sort()
    d['version'] = batch['version']
    return atlas.validate(d)


def coverage(d):
    import atlas
    atlas.validate(d)
    r = d.get('research', {})
    by_game = {}
    for claim in r.get('claims', {}).values():
        by_game.setdefault(claim['game_id'], set()).update(claim['sources'])
    rows = []
    for g in d['games']:
        gid = g['id']
        src = by_game.get(gid, set())
        groups = {d['sources'][s]['independence_key'] for s in src}
        kinds = Counter(d['sources'][s]['kind'] for s in src)
        editorial_groups = {d['sources'][s]['independence_key'] for s in src
                            if d['sources'][s]['kind'] in ('review', 'retrospective')}
        rows.append(dict(id=gid, title=g['objective']['title']['value'], enriched=gid in r.get('records', {}),
                         sources=sorted(src), independent_groups=len(groups), source_kinds=dict(kinds),
                         independent_editorial_groups=len(editorial_groups),
                         gaps=r.get('records', {}).get(gid, {}).get('gaps', ['Not investigated in this batch']),
                         unknowns=r.get('records', {}).get(gid, {}).get('unknowns', ['Experiential evidence unknown'])))
    return dict(version=d['version'], human_review=None, games=rows,
                total=len(rows), enriched=sum(x['enriched'] for x in rows),
                with_two_independent_groups=sum(x['independent_groups'] >= 2 for x in rows),
                with_two_independent_editorial_groups=sum(x['independent_editorial_groups'] >= 2 for x in rows))


def main():
    import atlas
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('command', choices=('apply', 'coverage'))
    p.add_argument('source', type=Path)
    p.add_argument('--batch', type=Path)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    d = atlas.load(a.source)
    atlas.save(a.output, apply_batch(d, atlas.load(a.batch)) if a.command == 'apply' else coverage(d))


if __name__ == '__main__':
    main()
