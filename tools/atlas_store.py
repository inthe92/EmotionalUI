#!/usr/bin/env python3
"""Development-only Atlas identity store, resumable jobs and compact export.

No ROM scanning, UI imports, model invocation or network calls in this module.
"""
import argparse
import copy
import json
import re
import secrets
import sqlite3
import time
from collections import Counter
from contextlib import contextmanager
from pathlib import Path

import atlas
from atlas_research import digest

PLATFORM = re.compile(r'[a-z][a-z0-9_]{0,23}\Z')
MAX_CONSOLE = 32 * 1024 * 1024


def encode(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


class Store:
    def __init__(self, path, kind='real'):
        if kind not in ('real', 'synthetic'):
            raise ValueError('Explicit dataset kind required')
        existed = str(path)!=':memory:' and Path(path).exists()
        self.db = sqlite3.connect(path, timeout=10)
        if existed and (self.db.execute('PRAGMA application_id').fetchone()[0]!=1096045139 or self.db.execute('PRAGMA user_version').fetchone()[0]!=1):
            self.db.close()
            raise ValueError('Not an Atlas research store; personal/foreign databases are never migrated')
        self.db.execute('PRAGMA application_id=1096045139')
        self.db.execute('PRAGMA user_version=1')
        self.db.row_factory = sqlite3.Row
        self.db.execute('PRAGMA foreign_keys=ON')
        self.db.executescript('''
        CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS providers(id TEXT PRIMARY KEY,policy TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS documents(hash TEXT PRIMARY KEY,provider TEXT NOT NULL,external_id TEXT NOT NULL,revision TEXT NOT NULL,body TEXT NOT NULL,UNIQUE(provider,external_id,revision));
        CREATE TABLE IF NOT EXISTS works(id TEXT PRIMARY KEY,title TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS implementations(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id),platform TEXT NOT NULL,title TEXT NOT NULL,document TEXT REFERENCES documents(hash),legacy TEXT,stable TEXT NOT NULL DEFAULT 'version=1\n',experimental TEXT,explanation TEXT);
        CREATE INDEX IF NOT EXISTS impl_platform_title ON implementations(platform,title);
        CREATE TABLE IF NOT EXISTS editions(id TEXT PRIMARY KEY,implementation_id TEXT NOT NULL REFERENCES implementations(id),region TEXT NOT NULL,revision TEXT,translation TEXT,source TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS external_ids(provider TEXT NOT NULL,external_id TEXT NOT NULL,platform TEXT NOT NULL,implementation_id TEXT NOT NULL REFERENCES implementations(id),qualifiers TEXT NOT NULL,PRIMARY KEY(provider,external_id,platform));
        CREATE INDEX IF NOT EXISTS external_by_impl ON external_ids(implementation_id);
        CREATE INDEX IF NOT EXISTS editions_by_impl ON editions(implementation_id);
        CREATE TABLE IF NOT EXISTS identity_candidates(implementation_id TEXT NOT NULL,candidate_id TEXT NOT NULL,reason TEXT NOT NULL,PRIMARY KEY(implementation_id,candidate_id));
        CREATE TABLE IF NOT EXISTS identity_reconciliations(alias_id TEXT PRIMARY KEY,canonical_id TEXT NOT NULL REFERENCES implementations(id),legacy_document TEXT NOT NULL,external_document TEXT NOT NULL,receipt TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS sequences(platform TEXT PRIMARY KEY,value INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS import_errors(hash TEXT PRIMARY KEY,provider TEXT NOT NULL,code TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS imports(hash TEXT PRIMARY KEY,provider TEXT NOT NULL,input_version TEXT NOT NULL,count INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS jobs(key TEXT PRIMARY KEY,implementation_id TEXT NOT NULL REFERENCES implementations(id),input_hash TEXT NOT NULL,processor TEXT NOT NULL,state TEXT NOT NULL CHECK(state IN ('pending','running','done','retry','blocked')),attempts INTEGER NOT NULL DEFAULT 0,max_attempts INTEGER NOT NULL DEFAULT 3,lease_until REAL,token TEXT,not_before REAL NOT NULL DEFAULT 0,error TEXT,request TEXT NOT NULL,result TEXT,result_hash TEXT);
        CREATE INDEX IF NOT EXISTS jobs_ready ON jobs(state,not_before,key);
        CREATE INDEX IF NOT EXISTS jobs_lease ON jobs(state,lease_until);
        CREATE TABLE IF NOT EXISTS events(sequence INTEGER PRIMARY KEY,job TEXT,state TEXT NOT NULL,code TEXT NOT NULL);
        ''')
        old = self.db.execute("SELECT value FROM meta WHERE key='dataset_kind'").fetchone()
        if old and old[0] != kind:
            self.db.close()
            raise ValueError('Cannot mix real and synthetic stores')
        self.db.execute("INSERT OR IGNORE INTO meta VALUES('dataset_kind',?)", (kind,))
        self.db.execute("INSERT OR IGNORE INTO meta VALUES('schema','1')")
        if self.db.execute("SELECT value FROM meta WHERE key='schema'").fetchone()[0] != '1':
            raise ValueError('Unsupported research store version')
        self.db.commit()
        self.kind = kind

    def close(self):
        self.db.close()

    @contextmanager
    def transaction(self):
        self.db.execute('BEGIN IMMEDIATE')
        try:
            yield
        except BaseException:
            self.db.rollback()
            raise
        else:
            self.db.commit()

    def policy(self, provider, policy):
        fields = {'acquire', 'store', 'extract', 'redistribute', 'basis'}
        if set(policy) != fields or not policy['basis'] or any(type(policy[k]) is not bool for k in fields - {'basis'}):
            raise ValueError('Explicit per-use provider policy required')
        old = self.db.execute('SELECT policy FROM providers WHERE id=?', (provider,)).fetchone()
        if old and old[0] != encode(policy):
            raise ValueError('Provider policy conflict: review before changing permissions')
        self.db.execute('INSERT OR IGNORE INTO providers VALUES(?,?)', (provider, encode(policy)))

    def allocate(self, platform):
        row = self.db.execute('INSERT INTO sequences VALUES(?,1) ON CONFLICT(platform) DO UPDATE SET value=value+1 RETURNING value', (platform,)).fetchone()
        return f'atlas:{platform}:{row[0]:06}'

    def import_atlas(self, data):
        """Preserve existing IDs/profiles exactly; never touch personal identity links."""
        atlas.validate(data)
        if self.kind != 'real':
            raise ValueError('Legacy real Atlas cannot seed a synthetic store')
        fingerprint = digest(data)
        with self.transaction():
            if self.db.execute('SELECT 1 FROM imports WHERE hash=?', (fingerprint,)).fetchone():
                return 0
            provider = 'local-atlas'
            # Only original Atlas notes/proposals; not permission to store full reviews.
            self.policy(provider, dict(acquire=True, store=True, extract=False, redistribute=True,
                                      basis='Existing authored Atlas data only; source-site rights remain separate. No external processor permission inferred.'))
            self.db.execute('INSERT INTO documents VALUES(?,?,?,?,?)', (fingerprint, provider, data['version'], fingerprint, encode(data)))
            selected = set(data.get('experiment', {}).get('selected_ids', []))
            for g in data['games']:
                gid, wid, platform = g['id'], g['identity']['work_id'], atlas.platform_of(g)
                if self.db.execute('SELECT 1 FROM implementations WHERE id=?', (gid,)).fetchone():
                    raise ValueError('Legacy identity already exists with another input version; explicit reconciliation required')
                self.db.execute('INSERT OR IGNORE INTO works VALUES(?,?)', (wid, g['objective']['title']['value']))
                self.db.execute('INSERT INTO implementations VALUES(?,?,?,?,?,?,?,?,?)',
                                (gid, wid, platform, g['objective']['title']['value'], fingerprint, encode(g), atlas.profile(g)[0],
                                 atlas.profile(g, True)[0] if gid in selected else None,
                                 data.get('experiment', {}).get('explanations', {}).get(gid)))
                identity = g['identity']
                self.db.execute('INSERT INTO editions VALUES(?,?,?,?,?,?)',
                                (gid + ':edition:legacy', gid, identity['region'], identity.get('revision'), identity.get('translation'), fingerprint))
                self.db.execute('INSERT INTO sequences VALUES(?,?) ON CONFLICT(platform) DO UPDATE SET value=max(value,excluded.value)', (platform, int(gid.rsplit(':', 1)[1])))
            self.db.execute('INSERT INTO imports VALUES(?,?,?,?)', (fingerprint, provider, data['version'], len(data['games'])))
        return len(data['games'])

    def import_records(self, envelope):
        try:
            return self._import_records(envelope)
        except (ValueError,KeyError,TypeError):
            with self.db:
                self.db.execute('INSERT OR IGNORE INTO import_errors VALUES(?,?,?)',(digest(envelope),str(envelope.get('provider','unknown'))[:40],'invalid-or-unpermitted-input'))
            raise

    def _import_records(self, envelope):
        if envelope.get('schema') != 1 or envelope.get('kind') != self.kind or not envelope.get('input_version'):
            raise ValueError('Import schema, version or dataset kind mismatch')
        records = envelope['records']
        if not isinstance(records, list) or len(records) > (20000 if self.kind == 'synthetic' else 300):
            raise ValueError('Bounded pilot import required')
        provider = envelope['provider']
        if not re.fullmatch(r'[a-z][a-z0-9-]{0,30}', provider):
            raise ValueError('Invalid provider')
        fingerprint, imported = digest(envelope), 0
        with self.transaction():
            self.policy(provider, envelope['policy'])
            if not envelope['policy']['store']:
                raise ValueError('Storage permission unresolved/disabled')
            if self.db.execute('SELECT 1 FROM imports WHERE hash=?', (fingerprint,)).fetchone():
                return 0
            seen = set()
            for r in records:
                ext = r['external_id']
                if not isinstance(ext, str) or not re.fullmatch(r'[A-Za-z0-9_.-]{1,100}', ext) or ext in seen:
                    raise ValueError('Invalid/duplicate external identity')
                seen.add(ext)
                if not r.get('title') or not isinstance(r['title'], str) or len(r['title']) > 512 or not r.get('revision'):
                    raise ValueError('Title and input revision required')
                if not r.get('platforms') or len(r['platforms']) > 24 or len(set(r['platforms'])) != len(r['platforms']):
                    raise ValueError('Explicit unique platforms required')
                if any(not PLATFORM.fullmatch(p) or p == 'multi' for p in r['platforms']):
                    raise ValueError('Invalid platform')
                content = digest(dict(provider=provider,record=r))
                old = self.db.execute('SELECT hash FROM documents WHERE provider=? AND external_id=? AND revision=?', (provider, ext, str(r['revision']))).fetchone()
                if old and old[0] != content:
                    raise ValueError('Same source revision has conflicting content')
                self.db.execute('INSERT OR IGNORE INTO documents VALUES(?,?,?,?,?)', (content, provider, ext, str(r['revision']), encode(r)))
                # Sharing this work key is the provider's assertion, never a title match.
                wid = 'atlas:work:' + provider + ':' + ext
                self.db.execute('INSERT OR IGNORE INTO works VALUES(?,?)', (wid, r['title']))
                for platform in r['platforms']:
                    old = self.db.execute('SELECT implementation_id FROM external_ids WHERE provider=? AND external_id=? AND platform=?', (provider, ext, platform)).fetchone()
                    if old:
                        self.db.execute('UPDATE implementations SET document=?,title=? WHERE id=?', (content, r['title'], old[0]))
                        self.db.execute('UPDATE external_ids SET qualifiers=? WHERE provider=? AND external_id=? AND platform=?',(encode(r.get('external_ids',[])),provider,ext,platform))
                        continue
                    gid = self.allocate(platform)
                    candidates = self.db.execute('SELECT id FROM implementations WHERE platform=? AND title=?', (platform, r['title'])).fetchall()
                    self.db.execute('INSERT INTO implementations(id,work_id,platform,title,document) VALUES(?,?,?,?,?)', (gid, wid, platform, r['title'], content))
                    self.db.execute('INSERT INTO external_ids VALUES(?,?,?,?,?)', (provider, ext, platform, gid, encode(r.get('external_ids', []))))
                    self.db.execute('INSERT INTO editions VALUES(?,?,?,?,?,?)', (gid + ':edition:unspecified', gid, 'unspecified', None, None, content))
                    for candidate in candidates:
                        self.db.execute('INSERT INTO identity_candidates VALUES(?,?,?)', (gid, candidate[0], 'same-title/platform; unresolved, never auto-merged'))
                    imported += 1
            self.db.execute('INSERT INTO imports VALUES(?,?,?,?)', (fingerprint, provider, envelope['input_version'], imported))
        return imported

    def reconcile(self, alias, canonical, receipt):
        """Explicit research mapping only; never follows personal links or title matches."""
        required = {'basis', 'sources', 'platform', 'edition_scope', 'human_review'}
        if set(receipt) != required or receipt['human_review'] is not None or not receipt['basis'] or not receipt['sources'] or not receipt['edition_scope']:
            raise ValueError('Traceable proposed mapping required; no fabricated human approval')
        from atlas_research import canonical_url
        for url in receipt['sources']:canonical_url(url)
        with self.transaction():
            old=self.db.execute('SELECT * FROM identity_reconciliations WHERE alias_id=?',(alias,)).fetchone()
            if old:
                if old['canonical_id']!=canonical or old['receipt']!=encode(receipt):raise ValueError('Conflicting identity reconciliation')
                return False
            src=self.db.execute('SELECT * FROM implementations WHERE id=?',(alias,)).fetchone()
            dst=self.db.execute('SELECT * FROM implementations WHERE id=?',(canonical,)).fetchone()
            if not src or not dst or alias==canonical or src['platform']!=dst['platform'] or dst['platform']!=receipt['platform']:
                raise ValueError('Explicit same-platform implementations required')
            if src['legacy'] or not dst['legacy'] or src['experimental'] or self.db.execute('SELECT 1 FROM jobs WHERE implementation_id IN (?,?)',(alias,canonical)).fetchone():
                raise ValueError('Reconciliation limited to unprocessed metadata candidate and preserved legacy target')
            self.db.execute('INSERT INTO identity_reconciliations VALUES(?,?,?,?,?)',(alias,canonical,dst['document'],src['document'],encode(receipt)))
            self.db.execute('UPDATE external_ids SET implementation_id=? WHERE implementation_id=?',(canonical,alias))
            self.db.execute('UPDATE implementations SET document=? WHERE id=?',(src['document'],canonical))
            self.db.execute('DELETE FROM identity_candidates WHERE implementation_id=? OR candidate_id=?',(alias,alias))
            self.db.execute('DELETE FROM editions WHERE implementation_id=?',(alias,))
            self.db.execute('DELETE FROM implementations WHERE id=?',(alias,))
            # The retired provider work/document remain available for the audit trail.
        return True

    def enqueue(self, gid, request, processor='fixture-v1'):
        from atlas_extraction import validate_request
        validate_request(request)
        if self.kind=='real' and any(s['kind']=='owned-fixture' for s in request['evidence']):
            raise ValueError('Owned synthetic evidence cannot enter a real catalog')
        if request['game_id'] != gid:
            raise ValueError('Request identity mismatch')
        row = self.db.execute('SELECT d.provider,i.document FROM implementations i JOIN documents d ON i.document=d.hash WHERE i.id=?', (gid,)).fetchone()
        if not row:
            raise ValueError('Unknown implementation')
        if request['identity_hash'] != row['document']:
            raise ValueError('Request identity revision is stale')
        policy = json.loads(self.db.execute('SELECT policy FROM providers WHERE id=?', (row[0],)).fetchone()[0])
        fingerprint = digest(request)
        key = digest(dict(game_id=gid, input_hash=fingerprint, processor=processor,
                          schema=request['schema'], prompt=request['prompt_version'], model=request['model']))
        state, error = ('pending', None) if policy['extract'] and all(s['permissions']['extract'] for s in request['evidence']) else ('blocked', 'extraction-not-permitted')
        self.db.execute('INSERT OR IGNORE INTO jobs(key,implementation_id,input_hash,processor,state,error,request) VALUES(?,?,?,?,?,?,?)', (key, gid, fingerprint, processor, state, error, encode(request)))
        return key

    def claim(self, now=None, limit=1, lease=120):
        now = time.time() if now is None else now
        if not 1 <= limit <= 500 or not 1 <= lease <= 3600:
            raise ValueError('Bounded lease/batch required')
        with self.transaction():
            expired = self.db.execute("SELECT key,attempts,max_attempts FROM jobs WHERE state='running' AND lease_until<=?", (now,)).fetchall()
            for r in expired:
                state = 'blocked' if r['attempts'] >= r['max_attempts'] else 'retry'
                self.db.execute('UPDATE jobs SET state=?,token=NULL,lease_until=NULL,error=? WHERE key=?', (state, 'lease-expired', r['key']))
                self.db.execute('INSERT INTO events(job,state,code) VALUES(?,?,?)', (r['key'], state, 'lease-expired'))
            rows = self.db.execute("SELECT key,request FROM jobs WHERE state IN ('pending','retry') AND not_before<=? AND attempts<max_attempts ORDER BY key LIMIT ?", (now, limit)).fetchall()
            claimed = []
            for r in rows:
                token = secrets.token_hex(16)
                self.db.execute("UPDATE jobs SET state='running',attempts=attempts+1,lease_until=?,token=?,error=NULL WHERE key=?", (now + lease, token, r['key']))
                self.db.execute("INSERT INTO events(job,state,code) VALUES(?,'running','claimed')", (r['key'],))
                claimed.append(dict(key=r['key'], token=token, request=json.loads(r['request'])))
            return claimed

    def complete(self, rows, now=None):
        from atlas_extraction import validate_response
        now = time.time() if now is None else now
        with self.transaction():
            for key, token, response in rows:
                job = self.db.execute('SELECT * FROM jobs WHERE key=?', (key,)).fetchone()
                if not job:
                    raise ValueError('Unknown job')
                if job['state'] == 'done' and job['result_hash'] == digest(response):
                    continue
                if job['state'] != 'running' or job['token'] != token or job['lease_until'] <= now:
                    raise ValueError('Stale/foreign lease')
                validate_response(json.loads(job['request']), response)
                self.db.execute("UPDATE jobs SET state='done',result=?,result_hash=?,token=NULL,lease_until=NULL WHERE key=?", (encode(response), digest(response), key))
                self.db.execute("INSERT INTO events(job,state,code) VALUES(?,'done','validated-proposal')", (key,))

    def fail(self, key, token, code, now=None, retry_after=1):
        now = time.time() if now is None else now
        if not re.fullmatch('[a-z0-9-]{1,60}', code) or not 0 <= retry_after <= 86400:
            raise ValueError('Bounded error code/backoff required')
        with self.transaction():
            r = self.db.execute('SELECT * FROM jobs WHERE key=?', (key,)).fetchone()
            if not r or r['state'] != 'running' or r['token'] != token or r['lease_until'] <= now:
                raise ValueError('Stale/foreign lease')
            state = 'blocked' if r['attempts'] >= r['max_attempts'] else 'retry'
            self.db.execute('UPDATE jobs SET state=?,error=?,not_before=?,token=NULL,lease_until=NULL WHERE key=?', (state, code, now + retry_after, key))
            self.db.execute('INSERT INTO events(job,state,code) VALUES(?,?,?)', (key, state, code))

    def stats(self):
        platforms = dict(self.db.execute('SELECT platform,count(*) FROM implementations GROUP BY platform'))
        return dict(kind=self.kind, implementations=sum(platforms.values()), platforms=platforms,
                    works=self.db.execute('SELECT count(DISTINCT work_id) FROM implementations').fetchone()[0],
                    editions=self.db.execute('SELECT count(*) FROM editions').fetchone()[0],
                    unresolved_candidates=self.db.execute('SELECT count(*) FROM identity_candidates').fetchone()[0],
                    jobs=dict(self.db.execute('SELECT state,count(*) FROM jobs GROUP BY state')))

    def export(self, path, version='0.3.0-experimental.1'):
        """No raw documents/jobs in the console file. Existing reader schema stays 1."""
        from atlas_extraction import profile_result
        final = Path(path)
        path = final.with_name(final.name+'.building')
        if final.exists() or path.exists():
            raise ValueError('Refusing to overwrite export or retained interrupted build')
        rows = list(self.db.execute('SELECT i.*,p.policy FROM implementations i JOIN documents d ON d.hash=i.document JOIN providers p ON p.id=d.provider ORDER BY i.id'))
        # Different completed revisions are not resolved by wall-clock order.
        done = {}
        current_documents={r['id']:r['document'] for r in rows}
        for job in self.db.execute("SELECT * FROM jobs WHERE state='done' ORDER BY implementation_id,key"):
            if json.loads(job['request'])['identity_hash']==current_documents.get(job['implementation_id']):
                done.setdefault(job['implementation_id'], []).append(job)
        legacy_documents={}
        legacy_sources={}
        identities = [(r['id'], r['work_id'], r['platform'], r['title'], r['document']) for r in rows]
        source_hash = digest(dict(identities=identities, results={k:[x['result_hash'] for x in v] for k,v in done.items()}))
        with sqlite3.connect(path) as db:
            db.executescript('''PRAGMA application_id=1161909324;PRAGMA user_version=1;
            CREATE TABLE meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
            CREATE TABLE games(id TEXT PRIMARY KEY,title TEXT NOT NULL,data TEXT NOT NULL);
            CREATE TABLE profiles(game_id TEXT PRIMARY KEY,profile TEXT NOT NULL);
            CREATE TABLE profiles_experimental(game_id TEXT PRIMARY KEY,profile TEXT NOT NULL,explanation TEXT NOT NULL);
            CREATE TABLE identity_refs(game_id TEXT PRIMARY KEY,work_id TEXT NOT NULL,platform TEXT NOT NULL);
            CREATE TABLE editions(id TEXT PRIMARY KEY,implementation_id TEXT NOT NULL,region TEXT NOT NULL,revision TEXT,translation TEXT,source TEXT NOT NULL);
            CREATE TABLE external_refs(provider TEXT NOT NULL,external_id TEXT NOT NULL,platform TEXT NOT NULL,game_id TEXT NOT NULL,PRIMARY KEY(provider,external_id,platform));
            CREATE TABLE source_refs(game_id TEXT NOT NULL,source_id TEXT NOT NULL,url TEXT NOT NULL,locator TEXT NOT NULL,PRIMARY KEY(game_id,source_id));''')
            db.execute('CREATE TABLE identity_aliases(alias_id TEXT PRIMARY KEY,game_id TEXT NOT NULL,receipt TEXT NOT NULL)')
            for rec in self.db.execute('SELECT alias_id,canonical_id,receipt FROM identity_reconciliations ORDER BY alias_id'):
                db.execute('INSERT INTO identity_aliases VALUES(?,?,?)',tuple(rec))
            for k,v in dict(schema='1',version=version,source_sha256=source_hash,experimental_channel='opt-in-only',dataset_kind=self.kind,format='compact-v1').items():
                db.execute('INSERT INTO meta VALUES(?,?)', (k,v))
            for r in rows:
                if not json.loads(r['policy'])['redistribute']:
                    raise ValueError('Redistribution permission unresolved/disabled')
                gid = r['id']
                candidates = done.get(gid, [])
                if len(candidates) > 1:
                    raise ValueError('Multiple completed revisions: select an explicit snapshot before export')
                text, explanation, refs = r['experimental'], r['explanation'], []
                if candidates:
                    job = candidates[0]
                    request, response = json.loads(job['request']), json.loads(job['result'])
                    text, explanation, refs = profile_result(request,response)
                elif r['legacy'] and text:
                    # Preserve references to the original lot; full evidence stays in research.
                    legacy = json.loads(r['legacy'])
                    rec=self.db.execute('SELECT legacy_document FROM identity_reconciliations WHERE canonical_id=? ORDER BY alias_id LIMIT 1',(gid,)).fetchone()
                    legacy_document=rec[0] if rec else r['document']
                    if legacy_document not in legacy_documents:
                        source=json.loads(self.db.execute('SELECT body FROM documents WHERE hash=?',(legacy_document,)).fetchone()[0])
                        legacy_documents[legacy_document]=source
                        grouped={}
                        for c in source.get('research',{}).get('claims',{}).values():grouped.setdefault(c['game_id'],set()).update(c['sources'])
                        legacy_sources[legacy_document]=grouped
                    source=legacy_documents[legacy_document]
                    source_ids={a['source'] for a in legacy['editorial'].values()}|legacy_sources[legacy_document].get(gid,set())
                    for sid in sorted(source_ids):
                        s = source['sources'][sid]
                        refs.append(dict(id=sid,url=s.get('url',''),locator=s.get('locator','original Atlas assertion')))
                identity=json.loads(r['legacy'])['identity'] if r['legacy'] else dict(work_id=r['work_id'],implementation_id=gid,region='unspecified',revision=None,translation=None)
                db.execute('INSERT INTO games VALUES(?,?,?)',(gid,r['title'],encode(dict(identity=identity,platform=r['platform']))))
                for edition in self.db.execute('SELECT * FROM editions WHERE implementation_id=? ORDER BY id',(gid,)):
                    db.execute('INSERT INTO editions VALUES(?,?,?,?,?,?)',tuple(edition))
                for ext in self.db.execute('SELECT provider,external_id,platform,implementation_id FROM external_ids WHERE implementation_id=? ORDER BY provider,external_id,platform',(gid,)):
                    db.execute('INSERT INTO external_refs VALUES(?,?,?,?)',tuple(ext))
                db.execute('INSERT INTO identity_refs VALUES(?,?,?)',(gid,r['work_id'],r['platform']))
                db.execute('INSERT INTO profiles VALUES(?,?)',(gid,r['stable']))
                if text and text != 'version=1\n':
                    db.execute('INSERT INTO profiles_experimental VALUES(?,?,?)',(gid,text,explanation))
                for ref in refs:
                    db.execute('INSERT OR IGNORE INTO source_refs VALUES(?,?,?,?)',(gid,ref['id'],ref['url'],ref['locator']))
            db.commit();db.execute('VACUUM')
        if path.stat().st_size > MAX_CONSOLE:
            raise ValueError('Export exceeds existing 32 MiB console limit; retained for inspection, not usable as console candidate')
        path.chmod(0o444)
        path.rename(final)
        path=final
        return dict(bytes=path.stat().st_size, source_sha256=source_hash, implementations=len(rows))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('command',choices=('import-atlas','import-file','stats','export'))
    p.add_argument('--store',type=Path,required=True);p.add_argument('--input',type=Path);p.add_argument('--output',type=Path)
    p.add_argument('--kind',choices=('real','synthetic'),default='real');a=p.parse_args()
    store=Store(a.store,a.kind)
    try:
        if a.command=='import-atlas':result=store.import_atlas(atlas.load(a.input))
        elif a.command=='import-file':result=store.import_records(atlas.load(a.input))
        elif a.command=='export':result=store.export(a.output)
        else:result=store.stats()
        print(json.dumps(result,ensure_ascii=False,indent=2))
    finally:store.close()


if __name__=='__main__':main()
