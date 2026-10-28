#!/usr/bin/env python3
"""Offline editorial pipeline. Never downloads ROMs or calls a model provider."""
import argparse,copy,csv,hashlib,json,re,sqlite3
from pathlib import Path
from contextlib import closing
SCHEMA=1
ID_PATTERN=r'atlas:[a-z][a-z0-9_]{0,23}:[0-9]{6}'
def platform_of(g):return g['objective']['platform']['value']
ATTRS={'calm','intensity','nostalgia','exploration','mystery','mental_demand','tension','pace','resume_ease','short_sessions','session_minutes'}
CONTEXTS=['spring','summer','autumn','winter','clear','cloud','rain','morning','afternoon','night','late']
def load(p):
 def unique(pairs):
  result={}
  for key,value in pairs:
   if key in result:raise ValueError('Duplicate JSON key: '+key)
   result[key]=value
  return result
 return json.loads(Path(p).read_text(),object_pairs_hook=unique)
def save(p,data):
 with Path(p).open('x') as f:json.dump(data,f,ensure_ascii=False,indent=2);f.write('\n')
def validate(d):
 if d.get('schema')!=SCHEMA or not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.-]+)?',d.get('version','')):raise ValueError('Unsupported schema/version')
 if not re.fullmatch(r'[a-z][a-z0-9_]{0,23}',d.get('platform','')) or not isinstance(d.get('games'),list) or len(d['games'])>100000:raise ValueError('Expected bounded platform catalog')
 registry=d.get('identity_registry',{});works=registry.get('works',{})
 if registry and registry.get('schema')!=1:raise ValueError('Unsupported identity registry')
 sources=d.get('sources',{});ids=set();aliases=set();identities=set()
 for g in d['games']:
  if set(g)-{'id','objective','editorial','aliases','identity'}:raise ValueError('Unknown entry field')
  if not {'title','platform','year','developer','publisher','genre','mechanics','duration'}<=g.get('objective',{}).keys():raise ValueError('Missing explicit objective field; use null for unknown')
  i=g['id']
  if not re.fullmatch(ID_PATTERN,i) or i in ids:raise ValueError('Invalid/duplicate stable ID')
  ids.add(i);title=g['objective']['title']
  ident=g.get('identity',{})
  if not (re.fullmatch(ID_PATTERN,ident.get('work_id','')) or ident.get('work_id') in works) or not ident.get('region') or any(ident.get(k) is not None and not isinstance(ident[k],str) for k in ('revision','translation')):raise ValueError('Explicit edition identity required')
  platform=platform_of(g)
  if not isinstance(platform,str) or not re.fullmatch(r'[a-z][a-z0-9_]{0,23}',platform) or platform=='multi' or i.split(':')[1]!=platform or d['platform'] not in ('multi',platform):raise ValueError('Platform/identity contradiction')
  if ident.get('implementation_id',i)!=i:raise ValueError('Implementation ID mismatch')
  key=(title.get('value','').casefold(),platform,ident.get('work_id'),ident.get('region'),ident.get('revision'),ident.get('translation'))
  if key in identities:raise ValueError('Duplicate title/edition identity; resolve explicitly')
  identities.add(key)
  if not isinstance(title.get('value'),str) or not title['value'].strip():raise ValueError('Missing title')
  for k,a in list(g['objective'].items())+list(g.get('editorial',{}).items()):
   if a is None:continue
   if set(a)-{'value','confidence','source','status','origin','reviewer','note','evidence','rubric'}:raise ValueError('Unknown assertion field')
   if not a.get('origin') or a.get('source') not in sources or a.get('status') not in ('unknown','proposed','source_checked','approved','rejected'):raise ValueError('Missing provenance or invalid status')
   if type(a.get('confidence')) is not int or not 0<=a['confidence']<=100:raise ValueError('Invalid confidence')
   if a['value'] is None and a['status']!='unknown':raise ValueError('Unknown value must remain unknown')
   if a['status']=='approved' and not a.get('reviewer'):raise ValueError('Approval needs an accountable reviewer')
   if k in g['objective'] and a['value'] is not None:
    v=a['value']
    if k not in ('title','platform','year','developer','publisher','genre','mechanics','duration'):raise ValueError('Unknown objective field')
    if k=='platform' and v!=platform:raise ValueError('Platform contradiction')
    if k=='year' and (type(v) is not int or not 1950<=v<=2100):raise ValueError('Invalid release year')
    if k in ('developer','publisher') and not isinstance(v,str):raise ValueError('Expected organization string')
    if k in ('genre','mechanics') and (not isinstance(v,list) or not all(isinstance(x,str) and x for x in v)):raise ValueError('Expected documented categories')
    if k=='duration' and (type(v) is not int or v<=0):raise ValueError('Duration requires a positive documented minute count')
   if k in g.get('editorial',{}):
    if k not in ATTRS and not (k.startswith('season.') and k[7:] in CONTEXTS[:4]) and not (k.startswith('context.') and k[8:] in CONTEXTS) and not k.startswith('celebration.'):raise ValueError('Unknown editorial attribute')
    v=a['value'];limit=240 if k=='session_minutes' else 100
    if v is not None and (type(v) is not int or not 0<=v<=limit or k=='session_minutes' and v==0):raise ValueError('Attribute out of range')
  for a in g.get('aliases',[]):
   if a.get('source') not in sources or not a.get('title') or not a.get('region'):raise ValueError('Alias needs provenance/region')
   key=(i,a['title'],a['region'],a.get('revision'),a.get('translation'))
   if key in aliases:raise ValueError('Duplicate alias')
   aliases.add(key)
 seen_relations=set()
 for r in d.get('relations',[]):
  key=(r['from'],r['to'],r['kind'])
  if key in seen_relations:raise ValueError('Duplicate relation')
  seen_relations.add(key)
  if r['from'] not in ids or r['to'] not in ids or r['from']==r['to']:raise ValueError('Invalid relation target')
  if r['kind'] not in ('exploration','pace','atmosphere','adventure_structure','brief_sessions','discovery','documented_history'):raise ValueError('Unknown relation kind')
  if type(r.get('confidence')) is not int or r.get('source') not in sources or r.get('status') not in ('proposed','approved','rejected') or not 0<=r.get('confidence',-1)<=100:raise ValueError('Invalid relation provenance')
  if r['status']=='approved' and not r.get('reviewer'):raise ValueError('Relation approval needs reviewer')
 experiment=d.get('experiment')
 if experiment is not None:
  if set(experiment)!={'channel','selected_ids','human_review','author','explanations'} or experiment['channel']!='experimental' or experiment['human_review'] is not None or not experiment['author']:raise ValueError('Experimental selection is not human approval')
  selected=experiment['selected_ids']
  if not isinstance(selected,list) or len(selected)!=len(set(selected)) or not set(selected)<=ids:raise ValueError('Invalid experimental IDs')
  selected_set=set(selected)
  if set(experiment['explanations'])!=selected_set:raise ValueError('Explanation coverage mismatch')
  for g in d['games']:
   if g['id'] not in selected_set:continue
   if not isinstance(experiment['explanations'][g['id']],str) or not 1<=len(experiment['explanations'][g['id']])<=180:raise ValueError('Bounded original explanation required')
   if not g['objective'].get('mechanics') or g['objective']['mechanics']['status']!='source_checked':raise ValueError('Research gameplay evidence before activation')
   for a in g['editorial'].values():
    src=sources[a['source']]
    if not src.get('url','').startswith('https://') or not src.get('facts') or not a.get('note'):raise ValueError('Experimental provenance and reasoning required')
 for g in d['games']:
  if g['identity']['work_id'] not in ids and g['identity']['work_id'] not in works:raise ValueError('Unresolved work identity')
 from atlas_research import validate_research
 validate_research(d)
 return d
def profile(g,experimental=False):
 values={k:a['value'] for k,a in g.get('editorial',{}).items() if a['value'] is not None and (a['status']=='approved' and a['confidence']>=70 or experimental and a['status']=='proposed' and a['confidence']>=60)};lines=['version=1'];tables={}
 for k,v in sorted(values.items()):
  if '.' in k:
   group,key=k.split('.',1);table={'season':'seasons','context':'extra','celebration':'celebrations'}[group];tables.setdefault(table,{})[key]=v
  elif k=='short_sessions':tables.setdefault('extra',{})[k]=v
  else:lines.append(f'{k}={v}')
 for group,rows in sorted(tables.items()):
  lines+=['['+group+']']+[json.dumps(k)+'='+str(v) for k,v in sorted(rows.items())]
 return '\n'.join(lines)+'\n',values
def build(d,path):
 validate(d);p=Path(path)
 if any(g['objective']['title']['status'] not in ('source_checked','approved') for g in d['games']):raise ValueError('Unverified title cannot be published')
 if p.exists():raise ValueError('Refusing to overwrite catalog')
 with closing(sqlite3.connect(p)) as db:
  db.executescript('PRAGMA application_id=1161909324;PRAGMA user_version=1;CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);CREATE TABLE games(id TEXT PRIMARY KEY,title TEXT NOT NULL,data TEXT NOT NULL);CREATE TABLE profiles(game_id TEXT PRIMARY KEY,profile TEXT NOT NULL);CREATE TABLE candidates(context TEXT NOT NULL,game_id TEXT NOT NULL,weight INTEGER NOT NULL,PRIMARY KEY(context,game_id));CREATE TABLE relations(source_id TEXT,target_id TEXT,data TEXT);CREATE TABLE profiles_experimental(game_id TEXT PRIMARY KEY,profile TEXT NOT NULL,explanation TEXT NOT NULL);')
  for k,v in [('schema','1'),('version',d['version']),('source_sha256',hashlib.sha256(json.dumps(d,sort_keys=True,ensure_ascii=False).encode()).hexdigest())]:db.execute('INSERT INTO meta VALUES(?,?)',(k,v))
  exp=d.get('experiment',{});selected=set(exp.get('selected_ids',[]))
  db.execute('INSERT INTO meta VALUES(?,?)',('experimental_channel','opt-in-only'))
  for g in sorted(d['games'],key=lambda g:g['id']):
   db.execute('INSERT INTO games VALUES(?,?,?)',(g['id'],g['objective']['title']['value'],json.dumps(g,ensure_ascii=False,sort_keys=True)))
   text,values=profile(g);db.execute('INSERT INTO profiles VALUES(?,?)',(g['id'],text))
   if g['id'] in selected:
    experimental_text,_=profile(g,experimental=True)
    db.execute('INSERT INTO profiles_experimental VALUES(?,?,?)',(g['id'],experimental_text,exp['explanations'][g['id']]))
   for context in CONTEXTS:db.execute('INSERT INTO candidates VALUES(?,?,?)',(context,g['id'],values.get('season.'+context,values.get('context.'+context,0))))
  for r in sorted(d.get('relations',[]),key=lambda r:(r['from'],r['to'],r['kind'])):db.execute('INSERT INTO relations VALUES(?,?,?)',(r['from'],r['to'],json.dumps(r,sort_keys=True)))
  if 'research' in d:
   # Additive provenance tables: existing read-only runtime/profile contract stays intact.
   db.executescript('CREATE TABLE research_sources(id TEXT PRIMARY KEY,data TEXT NOT NULL);CREATE TABLE research_claims(id TEXT PRIMARY KEY,game_id TEXT NOT NULL,data TEXT NOT NULL);CREATE TABLE research_manifest(key TEXT PRIMARY KEY,data TEXT NOT NULL);')
   for key in sorted(d['research']['source_ids']):db.execute('INSERT INTO research_sources VALUES(?,?)',(key,json.dumps(d['sources'][key],ensure_ascii=False,sort_keys=True)))
   for key,c in sorted(d['research']['claims'].items()):db.execute('INSERT INTO research_claims VALUES(?,?,?)',(key,c['game_id'],json.dumps(c,ensure_ascii=False,sort_keys=True)))
   for key,value in sorted(d['research'].items()):
    if key not in ('source_ids','claims'):db.execute('INSERT INTO research_manifest VALUES(?,?)',(key,json.dumps(value,ensure_ascii=False,sort_keys=True)))
  db.commit();db.execute('VACUUM')
 p.chmod(0o444)
def requests(d):
 return [{'id':g['id'],'title':g['objective']['title']['value'],'objective':g['objective'],'instruction':'Research primary sources; return {id,editorial}. Each value must include confidence, source and note; unknown stays null. Output is a proposal, never approval.'} for g in validate(d)['games']]
def import_results(d,rows):
 d=copy.deepcopy(validate(d));d.pop('experiment',None);games={g['id']:g for g in d['games']};seen=set()
 for row in rows:
  if row['id'] not in games or row['id'] in seen:raise ValueError('Unknown/duplicate result ID')
  seen.add(row['id'])
  for k,a in row['editorial'].items():
   if k in games[row['id']].get('editorial',{}) and games[row['id']]['editorial'][k].get('status')=='approved':raise ValueError('Cannot overwrite reviewed attribute')
   a=copy.deepcopy(a);a.update(status='unknown' if a.get('value') is None else 'proposed',origin='external_ai_import');a.pop('reviewer',None);games[row['id']].setdefault('editorial',{})[k]=a
 return validate(d)
def resolve(d,title):
 return [{'id':g['id'],'title':g['objective']['title']['value'],'aliases':g.get('aliases',[])} for g in validate(d)['games'] if title.casefold() in [g['objective']['title']['value'].casefold(),*[a['title'].casefold() for a in g.get('aliases',[])]]]
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('command',choices=['validate','build','requests','import-results','resolve','import-list']);p.add_argument('source',type=Path);p.add_argument('--output',type=Path);p.add_argument('--results',type=Path);p.add_argument('--title');a=p.parse_args();d=load(a.source)
 if a.command=='validate':validate(d);print('valid',len(d['games']))
 elif a.command=='resolve':print(json.dumps(resolve(d,a.title),ensure_ascii=False,indent=2))
 elif a.command=='build':build(d,a.output)
 elif a.command=='requests':save(a.output,requests(d))
 elif a.command=='import-results':save(a.output,import_results(d,load(a.results)))
 else:
  validate(d);nextid=max([int(g['id'].split(':')[-1]) for g in d['games']]+[0])+1
  with a.results.open() as f:
   for row in csv.DictReader(f):
    if resolve(d,row['title']):raise ValueError('Title already present: resolve explicitly')
    i=f'atlas:snes:{nextid:06}';objective={k:None for k in ('title','platform','year','developer','publisher','genre','mechanics','duration')}
    objective['title']={'value':row['title'],'confidence':0,'source':row['source'],'status':'proposed','origin':'list_import'};objective['platform']={'value':'snes','confidence':0,'source':row['source'],'status':'proposed','origin':'list_import'}
    d['games'].append({'id':i,'identity':{'work_id':i,'region':row.get('region') or 'unspecified','revision':None,'translation':None},'objective':objective,'editorial':{},'aliases':[]});nextid+=1
  save(a.output,validate(d))
if __name__=='__main__':main()
