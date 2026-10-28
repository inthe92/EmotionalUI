"""Provider-neutral extraction contract. Validates saved responses; calls no model.

Structural validation proves citation resolution/shape, never semantic truth or
human approval. Only supplied, permitted evidence may enter a request.
"""
import re
from atlas_research import RUBRIC, SCORED, canonical_url, digest

PROMPT_VERSION = 'atlas-evidence-2'


def text(value, maximum):
    if not isinstance(value,str) or not value.strip() or len(value)>maximum:
        raise ValueError('Bounded non-empty text required')


def validate_request(request):
    if set(request)!={'schema','game_id','identity_hash','prompt_version','model','rubric','evidence'} or type(request['schema']) is not int or request['schema']!=1 or request['rubric']!=RUBRIC['id']:
        raise ValueError('Extraction request schema/rubric mismatch')
    if not re.fullmatch(r'atlas:[a-z][a-z0-9_]{0,23}:\d{6}',request['game_id']):
        raise ValueError('Explicit implementation identity required')
    if request['prompt_version']!=PROMPT_VERSION or not re.fullmatch('[a-f0-9]{64}',request['identity_hash']):
        raise ValueError('Processing version and identity snapshot required')
    text(request['model'],80)
    evidence=request['evidence']
    if not isinstance(evidence,list) or not 1<=len(evidence)<=12:
        raise ValueError('Bounded evidence set required; metadata alone is not a profile')
    ids=set();passages=set();total=0
    for source in evidence:
        required={'id','url','locator','kind','independence_key','permissions','passages','sha256','game_id','edition_scope'}
        if set(source)!=required or source['id'] in ids:
            raise ValueError('Unique evidence and complete provenance required')
        ids.add(source['id']);canonical_url(source['url']);text(source['locator'],512)
        if source['game_id']!=request['game_id']:raise ValueError('Evidence implementation mismatch; no port inheritance')
        text(source['edition_scope'],500)
        text(source['independence_key'],120)
        if source['kind'] not in ('publisher','review','retrospective','player-opinion','owned-fixture'):
            raise ValueError('Unsupported evidence kind')
        permissions=source['permissions']
        if set(permissions)!={'store','extract','redistribute','basis'} or any(type(permissions[k]) is not bool for k in ('store','extract','redistribute')) or not permissions['basis']:
            raise ValueError('Per-document use permissions required')
        if not permissions['store']:
            raise ValueError('Document retention not permitted')
        if digest(source['passages'])!=source['sha256'] or not 1<=len(source['passages'])<=8:
            raise ValueError('Evidence hash or passage count mismatch')
        for passage in source['passages']:
            if set(passage)!={'id','text'} or passage['id'] in passages:
                raise ValueError('Unique passage identifiers required')
            passages.add(passage['id']);text(passage['text'],8000);total+=len(passage['text'])
    if total>64000:raise ValueError('Extraction payload exceeds bounded text budget')


def validate_response(request,response):
    validate_request(request)
    required={'schema','game_id','state','human_review','claims','attributes','mechanics','explanation','unknowns','uncertainty'}
    if set(response)!=required or type(response['schema']) is not int or response['schema']!=1 or response['game_id']!=request['game_id'] or response['state']!='proposal' or response['human_review'] is not None:
        raise ValueError('Extraction output is a proposal for the exact implementation, never approval')
    text(response['explanation'],180)
    for key in ('unknowns','uncertainty'):
        if not isinstance(response[key],list) or not response[key] or len(response[key])>32:
            raise ValueError('Unknowns/uncertainty required')
        for item in response[key]:text(item,500)
    sources={s['id']:s for s in request['evidence']}
    passages={p['id']:s['id'] for s in request['evidence'] for p in s['passages']}
    claims={}
    if not isinstance(response['claims'],list) or len(response['claims'])>64:
        raise ValueError('Bounded claims required')
    for c in response['claims']:
        if set(c)!={'id','kind','summary','sources','passages','supports','reasoning'} or c['id'] in claims or c['kind'] not in ('fact','interpretation','player-opinion'):
            raise ValueError('Unique typed claims required')
        text(c['summary'],500);text(c['reasoning'],500)
        if not c['sources'] or len(c['sources'])!=len(set(c['sources'])) or not set(c['sources'])<=sources.keys() or not c['passages'] or not set(c['passages'])<=passages.keys():
            raise ValueError('Unresolvable source/passage citation')
        if {passages[p] for p in c['passages']}!=set(c['sources']):
            raise ValueError('Cited passages must belong to the cited sources')
        if c['kind']=='fact' and any(sources[s]['kind']=='player-opinion' for s in c['sources']):
            raise ValueError('A player account cannot establish mechanics')
        if not isinstance(c['supports'],list) or not set(c['supports'])<=SCORED|{'mechanics'}:
            raise ValueError('Unsupported claim attributes')
        claims[c['id']]=c
    mechanics=response['mechanics']
    if set(mechanics)!={'value','evidence'} or not isinstance(mechanics['value'],list) or not mechanics['value'] or len(mechanics['value'])>20:
        raise ValueError('Explicitly sourced mechanics needed before experimental scoring')
    for mechanic in mechanics['value']:text(mechanic,120)
    if not mechanics['evidence'] or any(cid not in claims or claims[cid]['kind']!='fact' or 'mechanics' not in claims[cid]['supports'] for cid in mechanics['evidence']):
        raise ValueError('Mechanics require factual citations')
    if not isinstance(response['attributes'],dict) or not set(response['attributes'])<=SCORED:
        raise ValueError('No nostalgia/duration or unsupported score')
    for field,a in response['attributes'].items():
        if set(a)!={'value','confidence','evidence','reasoning'}:
            raise ValueError('Attributable score contract required')
        text(a['reasoning'],500)
        if type(a['confidence']) is not int:raise ValueError('Integer confidence required')
        if a['value'] is None:
            if a['confidence']!=0 or a['evidence']:raise ValueError('Unknown must remain unknown')
            continue
        if type(a['value']) is not int or a['value'] not in RUBRIC['values'] or a['confidence'] not in (60,65,70) or not a['evidence']:
            raise ValueError('Ordinal score and documented confidence required')
        used=[]
        for cid in a['evidence']:
            if cid not in claims or field not in claims[cid]['supports']:
                raise ValueError('Irrelevant claim for score')
            used+=claims[cid]['sources']
        if a['confidence']==70 and len({sources[s]['independence_key'] for s in used})<2:
            raise ValueError('Independent corroboration required')


def profile_result(request,response):
    import atlas
    validate_response(request,response)
    used={sid for c in response['claims'] for sid in c['sources']}
    if any(not s['permissions']['redistribute'] for s in request['evidence'] if s['id'] in used):
        raise ValueError('Evidence-derived output redistribution not permitted')
    editorial={k:dict(value=a['value'],confidence=a['confidence'],status='proposed') for k,a in response['attributes'].items()}
    profile=atlas.profile(dict(editorial=editorial),True)[0]
    refs=[{k:s[k] for k in ('id','url','locator')} for s in request['evidence'] if s['id'] in used]
    return profile,response['explanation'],refs


def fixture_request(gid,identity_hash):
    """Owned test prose. Never a factual description of a real game."""
    passages=[dict(id='fixture-passage',text='SYNTHETIC TEST ONLY: each round is a separate target-planning puzzle without a time limit. This is not a real game description.')]
    source=dict(id='owned-fixture',game_id=gid,edition_scope='Owned synthetic test for this exact implementation only; not real-game evidence.',url='https://example.org/owned-atlas-fixture',locator='owned synthetic test paragraph',kind='owned-fixture',independence_key='test-fixture',permissions=dict(store=True,extract=True,redistribute=True,basis='Original authored test fixture, not an external review'),passages=passages,sha256=digest(passages))
    return dict(schema=1,game_id=gid,identity_hash=identity_hash,prompt_version=PROMPT_VERSION,model='fixture-no-inference',rubric=RUBRIC['id'],evidence=[source])


def fixture_response(request):
    return dict(schema=1,game_id=request['game_id'],state='proposal',human_review=None,
                claims=[dict(id='test-mechanics',kind='fact',summary='Owned fixture describes separate untimed planning rounds.',sources=['owned-fixture'],passages=['fixture-passage'],supports=['mechanics','calm','mental_demand','short_sessions'],reasoning='Direct statement in owned synthetic prose, not a researched real game.')],
                mechanics=dict(value=['synthetic round planning'],evidence=['test-mechanics']),
                attributes={k:dict(value=v,confidence=60,evidence=['test-mechanics'],reasoning='Synthetic inference for throughput tests, not emotional coverage.') for k,v in [('calm',80),('mental_demand',60),('short_sessions',80)]},
                explanation='SYNTHETIC FIXTURE · rounds for a pipeline test, not a real game recommendation.',unknowns=['All real-game experience'],uncertainty=['Owned synthetic response; no model or human review.'])
