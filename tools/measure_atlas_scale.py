#!/usr/bin/env python3
"""Local synthetic throughput/size measurement; never real coverage or hardware FPS."""
import argparse
import copy
import json
import resource
import sqlite3
import time
from pathlib import Path

import atlas
from atlas_extraction import fixture_request,fixture_response,PROMPT_VERSION
from atlas_research import RUBRIC,coverage
from atlas_store import Store,encode


def coverage_fixture(count):
    sources={};claims={};records={};games=[]
    for i in range(count):
        platform=('snes','gba','nes','psx')[i%4];gid=f'atlas:{platform}:{i//4+1:06}';sid=f'fixture-{i}';cid=f'claim-{i}'
        sources[sid]=dict(url=f'https://example.org/owned-synthetic/{i}',title='Synthetic only',medium='Owned fixture',author='fixture',published=None,accessed='2026-10-24',kind='publisher',language='en',game_id=gid,platform=platform,edition=dict(reviewed='owned synthetic example',scope='not a real game'),independence_key='owned-fixture',license='original owned fixture',facts='synthetic planning rounds',locator='owned fixture',access_policy='no network')
        claims[cid]=dict(game_id=gid,kind='fact',summary='Synthetic fixture, not researched game knowledge.',sources=[sid],supports=['mechanics','calm'],status='source_checked',rationale='owned test',uncertainty='all real-game knowledge unknown')
        objective={k:None for k in ('title','platform','year','developer','publisher','genre','mechanics','duration')}
        for k,v in [('title',f'Synthetic game {i}'),('platform',platform),('mechanics',['synthetic planning'])]:
            objective[k]=dict(value=v,confidence=75,source=sid,status='source_checked',origin='owned fixture')
        objective['mechanics']['evidence']=[cid]
        games.append(dict(id=gid,identity=dict(work_id=gid,region='unspecified',revision=None,translation=None),objective=objective,editorial={'calm':dict(value=80,confidence=60,source=sid,status='proposed',origin='owned fixture',evidence=[cid],rubric=RUBRIC['id'],note='Synthetic inference only')},aliases=[]))
        records[gid]=dict(gaps=['all real coverage'],unknowns=['real experience'])
    return dict(schema=1,version='0.3.0-experimental.1',platform='multi',sources=sources,games=games,relations=[],research=dict(schema=1,rubric=RUBRIC,source_ids=list(sources),claims=claims,records=records,batches={}))


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--count',type=int,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args()
    if not 1<=a.count<=20000:raise ValueError('Bounded local fixture count')
    a.out.mkdir(parents=True,exist_ok=False);start=time.perf_counter();cpu=time.process_time();times={}
    records=[dict(external_id=f'SYN{i:06}',revision='1',title=f'Synthetic game {i:06}',platforms=[('snes','gba','nes','psx')[i%4]],observations=[dict(field=f'fixture-{k}',text='SYNTHETIC TEST, NOT REAL GAME EVIDENCE. '+'x'*480) for k in range(4)]) for i in range(a.count)]
    store=Store(a.out/'research.db','synthetic');t=time.perf_counter()
    store.import_records(dict(schema=1,kind='synthetic',provider='owned-fixture',input_version='1',policy=dict(acquire=True,store=True,extract=True,redistribute=True,basis='Original owned synthetic fixtures only'),records=records));times['import_s']=time.perf_counter()-t
    t=time.perf_counter()
    with store.transaction():
        for r in store.db.execute('SELECT id,document FROM implementations ORDER BY id').fetchall():store.enqueue(r['id'],fixture_request(r['id'],r['document']))
    times['enqueue_s']=time.perf_counter()-t;t=time.perf_counter();processed=0
    while True:
        jobs=store.claim(limit=500,lease=3600)
        if not jobs:break
        store.complete([(j['key'],j['token'],fixture_response(j['request'])) for j in jobs]);processed+=len(jobs)
    times['validate_store_fixture_results_s']=time.perf_counter()-t
    t=time.perf_counter();export_error=None
    try:export=store.export(a.out/'console.db')
    except ValueError as e:export_error=str(e);export=dict(bytes=(a.out/'console.db.building').stat().st_size)
    times['compact_export_s']=time.perf_counter()-t
    t=time.perf_counter();data=coverage_fixture(a.count);times['construct_coverage_fixture_s']=time.perf_counter()-t
    t=time.perf_counter();report=coverage(data);times['validated_coverage_s']=time.perf_counter()-t
    assert report['enriched']==a.count
    peak=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    import sys
    peak_mib=peak/(1024*1024 if sys.platform=='darwin' else 1024)
    result=dict(processing_contract=PROMPT_VERSION,kind='synthetic-only',count=a.count,platforms=store.stats()['platforms'],fixture_results=processed,real_games_researched=0,paid_inference_calls=0,times=times,wall_s=time.perf_counter()-start,cpu_process_s=time.process_time()-cpu,peak_process_rss_mib=peak_mib,research_bytes=(a.out/'research.db').stat().st_size,compact_bytes=export['bytes'],export_error=export_error,console_limit_bytes=32*1024*1024,hardware_performance=None,notes='One local process, owned synthetic text/claims/responses. Cache/result reuse tested separately. Wall/CPU/RSS include Python, SQLite, fixture generation and validation, not model inference, network or physical console.')
    store.close();(a.out/'measurement.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))


if __name__=='__main__':main()
