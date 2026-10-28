#!/usr/bin/env python3
"""Bounded serial CC0 metadata connector. No review scraping or accounts.

Public Action API / Query Service only, full claim qualifiers retained. Cached
responses are verified by SHA-256. A failed access/backoff stops this batch.
"""
import argparse
import hashlib
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

import atlas
from atlas_research import digest
from atlas_store import encode

UA='EmotionalUI-Atlas/0.1 (https://github.com/inthe92/EmotionalUI; bounded metadata pilot)'
POLICY=dict(acquire=True,store=True,extract=True,redistribute=True,
            basis='Wikidata structured entity data CC0; not article text, images, third-party linked sites or emotional evidence. https://www.wikidata.org/wiki/Wikidata:Licensing')


class AccessError(ValueError):
    pass


class Client:
    def __init__(self,cache,offline=False,opener=None,clock=time.time,sleep=time.sleep):
        self.cache=Path(cache);self.cache.mkdir(parents=True,exist_ok=True)
        self.offline=offline;self.open=opener or urllib.request.urlopen
        self.clock=clock;self.sleep=sleep;self.last=0;self.requests=0

    def get(self,url,ttl=86400):
        p=urllib.parse.urlsplit(url)
        if p.scheme!='https' or p.hostname not in ('www.wikidata.org','query.wikidata.org') or p.username or p.password:
            raise ValueError('Only public Wikidata endpoints permitted')
        key=hashlib.sha256(url.encode()).hexdigest();path=self.cache/(key+'.json')
        cached=atlas.load(path) if path.exists() else None
        if cached and digest(cached['body'])!=cached['sha256']:raise AccessError('cache-integrity-error')
        if cached and (self.offline or self.clock()-cached['fetched_at']<ttl):return cached['body']
        if self.offline:raise AccessError('offline-cache-miss')
        retry=self.cache/'provider.retry.json'
        if retry.exists() and atlas.load(retry)['not_before']>self.clock():raise AccessError('provider-backoff-active')
        self.sleep(max(0,1-(self.clock()-self.last)))
        request=urllib.request.Request(url,headers={'User-Agent':UA,'Accept':'application/json'})
        self.requests+=1
        try:
            with self.open(request,timeout=15) as response:
                raw=response.read(4*1024*1024+1)
                if len(raw)>4*1024*1024:raise AccessError('response-too-large')
                body=json.loads(raw)
        except urllib.error.HTTPError as error:
            delay=error.headers.get('Retry-After','60')
            try:delay=max(1,min(86400,int(delay)))
            except ValueError:delay=60
            retry.write_text(encode(dict(not_before=self.clock()+delay,code='http-'+str(error.code))))
            raise AccessError('http-'+str(error.code)+'; batch stopped, Retry-After respected') from None
        except (OSError,ValueError) as error:
            raise AccessError('unavailable-or-invalid-json; batch stopped') from None
        finally:self.last=self.clock()
        if isinstance(body,dict) and body.get('error'):
            code=str(body['error'].get('code','api-error'))[:60]
            if not re.fullmatch('[a-zA-Z0-9_-]+',code):code='api-error'
            retry.write_text(encode(dict(not_before=self.clock()+60,code=code)))
            raise AccessError(code+'; batch stopped, maxlag/limits not retried in a loop')
        envelope=dict(url=url,fetched_at=self.clock(),sha256=digest(body),body=body)
        tmp=path.with_suffix('.pending');tmp.write_text(encode(envelope));tmp.replace(path)
        return body

    def entities(self,ids):
        if not 1<=len(ids)<=20 or len(ids)!=len(set(ids)) or any(not re.fullmatch('Q[1-9][0-9]*',i) for i in ids):
            raise ValueError('At most twenty unique explicit Wikidata Q IDs per request')
        query=dict(action='wbgetentities',ids='|'.join(ids),props='labels|aliases|claims',languages='en|es',format='json',maxlag='5')
        result=self.get('https://www.wikidata.org/w/api.php?'+urllib.parse.urlencode(query))
        entities=result.get('entities',{})
        if not isinstance(entities,dict) or any(i not in ids for i in entities):raise AccessError('unexpected-entity-response')
        return entities

    def discover(self,platform_id,limit=80):
        if not re.fullmatch('Q[1-9][0-9]*',platform_id) or not 1<=limit<=100:
            raise ValueError('Explicit platform and bounded discovery required')
        query=f'SELECT DISTINCT ?game WHERE {{ ?game wdt:P31/wdt:P279* wd:Q7889; wdt:P400 wd:{platform_id} . }} ORDER BY ?game LIMIT {limit}'
        body=self.get('https://query.wikidata.org/sparql?'+urllib.parse.urlencode(dict(query=query,format='json')))
        bindings=body.get('results',{}).get('bindings',[])
        if len(bindings)>limit:raise AccessError('discovery-limit-exceeded')
        ids=[]
        for b in bindings:
            uri=b['game']['value']
            if not re.fullmatch(r'http://www.wikidata.org/entity/Q[1-9][0-9]*',uri):raise AccessError('invalid-discovery-identity')
            ids.append(uri.rsplit('/',1)[1])
        return ids


def normalize(entity,platforms):
    qid=entity.get('id','')
    if not re.fullmatch('Q[1-9][0-9]*',qid) or entity.get('type')!='item' or 'missing' in entity or not entity.get('lastrevid'):
        raise ValueError('Concrete versioned Wikidata entity required')
    label=entity.get('labels',{}).get('en',entity.get('labels',{}).get('es',{})).get('value')
    if not label:raise ValueError('No usable title label')
    claims=entity.get('claims',{})
    detected=set()
    for c in claims.get('P400',[]):
        if c.get('rank')=='deprecated':continue
        value=c.get('mainsnak',{}).get('datavalue',{}).get('value',{})
        if isinstance(value,dict) and value.get('id') in platforms:detected.add(platforms[value['id']])
    if not detected:raise ValueError('No supported platform statement; no inference from title')
    keep=('P31','P400','P577','P178','P123','P136','P11688','P5794','P9043','P144','P629')
    observations={p:claims[p] for p in keep if p in claims}
    # All qualifiers/references stay attached to their original statement, including
    # P9043 nested under P5794. Dates are NOT flattened to a universal release year.
    external=[]
    for prop,rows in observations.items():
        for row in rows:
            main=row.get('mainsnak',{})
            if main.get('datatype')=='external-id':
                external.append(dict(property=prop,value=main.get('datavalue',{}).get('value'),qualifiers=row.get('qualifiers',{}),statement=row.get('id')))
    return dict(external_id=qid,title=label,revision=str(entity['lastrevid']),platforms=sorted(detected),
                url='https://www.wikidata.org/wiki/'+qid,labels=entity.get('labels',{}),aliases=entity.get('aliases',{}),
                external_ids=external,observations=observations,identity_scope='Provider work assertion with separate platform implementations. Edition/revision/translation unknown; P144/P629 relations need review, never title merging.')


def acquire(client,platforms,max_titles=240):
    if not 1<=len(platforms)<=3 or not 1<=max_titles<=300:raise ValueError('Pilot bounded to three platforms and 300 identities')
    ids=[]
    for qid in platforms:
        ids+=client.discover(qid,min(100,max_titles//len(platforms)))
    ids=sorted(set(ids))[:max_titles]
    rows=[];errors=[]
    for start in range(0,len(ids),20):
        entities=client.entities(ids[start:start+20])
        for qid in ids[start:start+20]:
            try:rows.append(normalize(entities[qid],platforms))
            except (ValueError,KeyError) as error:errors.append(dict(id=qid,code='missing-label-or-platform',detail=str(error)[:120]))
    return dict(schema=1,kind='real',provider='wikidata',input_version='entity-normalizer-1',policy=POLICY,records=rows),errors


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--cache',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--platforms',type=Path,required=True,help='Explicit verified mapping of Q ID to local platform key')
    p.add_argument('--offline',action='store_true');p.add_argument('--limit',type=int,default=240);a=p.parse_args()
    if a.output.exists():raise ValueError('Refusing to overwrite input snapshot')
    client=Client(a.cache,a.offline)
    try:
        envelope,errors=acquire(client,atlas.load(a.platforms),a.limit)
        atlas.save(a.output,envelope);atlas.save(a.output.with_suffix('.coverage.json'),dict(requests=client.requests,records=len(envelope['records']),errors=errors,emotional_profiles=0))
        print(encode(dict(records=len(envelope['records']),requests=client.requests,emotional_profiles=0)))
    except AccessError as error:
        atlas.save(a.output.with_suffix('.error.json'),dict(code=str(error),requests=client.requests,records_imported=0))
        raise


if __name__=='__main__':main()
