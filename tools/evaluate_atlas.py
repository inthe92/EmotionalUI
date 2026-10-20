#!/usr/bin/env python3
"""Summarize real ranking outputs; never generate human quality scores."""
import argparse,collections,csv,json
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent

def summarize(source,output):
 d=json.loads(source.read_text());catalog=json.loads((ROOT/'assets/atlas/source/snes-pilot.json').read_text());names={g['id']:g['objective']['title']['value'] for g in catalog['games']}
 output.mkdir(exist_ok=False)
 counters={k:collections.Counter(i for s in d['scenarios'] for i in s[k]) for k in ('random_seeded','popularity_proxy','existing_rank','atlas_rank')}
 report={'scope':'Automatic descriptive counts, not quality or human preference validation. Popularity is a fixed proxy. Sparse 21/52 coverage introduces coverage bias.','changed_scenarios':[s['scenario'] for s in d['scenarios'] if not s['same_as_existing']],'unique_top_two':{k:len(v) for k,v in counters.items()},'counts':{k:dict(v) for k,v in counters.items()},'human_review':None}
 (output/'summary.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
 with (output/'human-review.csv').open('x') as f:
  w=csv.writer(f);w.writerow(['scenario','method','principal','alternative','contextual_fit_1_to_5','interest_1_to_5','discovery_1_to_5','explanation_supported','human_notes'])
  for s in d['scenarios']:
   for k in counters:w.writerow([s['scenario'],k,*[names[i] for i in s[k]],'','','','',''])
 lines=['# Comparación automática · sin puntuación humana','','| Escenario | Ranking anterior | Atlas experimental |','|---|---|---|']
 for s in d['scenarios']:lines.append('|'+s['scenario']+'|'+' / '.join(names[i] for i in s['existing_rank'])+'|'+' / '.join(names[i] for i in s['atlas_rank'])+'|')
 (output/'comparison.md').write_text('\n'.join(lines)+'\n')
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('output',type=Path);a=p.parse_args();summarize(a.source,a.output)
