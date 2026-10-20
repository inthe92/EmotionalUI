#!/usr/bin/env python3
"""Rebuild the opt-in research subset from original assistant annotations, offline.
No model responses or human approvals are fabricated. No web scraping at build time.
"""
import copy,json,sys
from pathlib import Path
import atlas
ROOT=Path(__file__).resolve().parent.parent

def build(output):
 d=copy.deepcopy(atlas.load(ROOT/'assets/atlas/source/snes-pilot.json'))
 research=atlas.load(ROOT/'assets/atlas/research/snes-21.json')
 d['version']='0.2.0-experimental.1'
 ids=[]
 for row in research['games']:
  g=next(g for g in d['games'] if g['id']==row['id']);ids.append(g['id'])
  key='research-'+g['id'].split(':')[-1]
  d['sources'][key]={'url':row['url'],'accessed':'2026-10-18','kind':'primary-publisher-page','scope':'Work-level gameplay evidence; not ROM region, save support or original release metadata.','facts':row['facts'],'license':'Copyright holder retains page rights. Original factual paraphrases only; no art or page text redistributed.'}
  g['objective']['mechanics']={'value':row['mechanics'],'confidence':90,'source':key,'status':'source_checked','origin':'assistant_primary_source_analysis','note':'Paraphrased gameplay information. Human verification pending; source_checked is not human approval.'}
  # Replace only this experimental copy. The published pilot remains untouched.
  g['editorial']={k:{'value':v,'confidence':65,'source':key,'status':'proposed','origin':'assistant_editorial_interpretation','note':row['rationale']} for k,v in row['attributes'].items()}
  row['source']=key
 d['experiment']={'channel':'experimental','selected_ids':ids,'human_review':None,'author':'Codex assistant; source-assisted interpretation, not API research output','explanations':{r['id']:r['explanation'] for r in research['games']}}
 # Relations remain proposals, never evidence of historical influence.
 d['relations'] += [{'from':'atlas:snes:000026','to':'atlas:snes:000032','kind':'brief_sessions','confidence':65,'source':'research-000026','status':'proposed','note':'Assistant comparison of discrete puzzle structure; not a documented historical connection.'},{'from':'atlas:snes:000005','to':'atlas:snes:000013','kind':'exploration','confidence':65,'source':'research-000005','status':'proposed','note':'Assistant proposal: abilities open ways forward; not influence or equivalence.'}]
 atlas.save(output,atlas.validate(d))
if __name__=='__main__':build(Path(sys.argv[1]))
