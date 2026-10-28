#!/usr/bin/env python3
"""Summarize real ranking outputs; never generate human quality scores."""
import argparse,collections,csv,json
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent

def summarize(source,output):
 d=json.loads(source.read_text())
 if 'before_version' in d:return evidence_summary(d,output)
 catalog=json.loads((ROOT/'assets/atlas/source/snes-pilot.json').read_text());names={g['id']:g['objective']['title']['value'] for g in catalog['games']}
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
def evidence_summary(d,output):
 """Describe controlled production-policy outputs; never grade their quality."""
 catalog=json.loads((ROOT/'assets/atlas/source/snes-evidence-experimental.json').read_text())
 if catalog['version']!=d['after_version']:raise ValueError('Evaluation/catalog version mismatch')
 names={g['id']:g['objective']['title']['value'] for g in catalog['games']}
 output.mkdir(exist_ok=False)
 counters={k:collections.Counter(i for s in d['scenarios'] for i in s[k]) for k in ('before','after')}
 if not all(i in names for c in counters.values() for i in c):raise ValueError('Unknown ranking identity')
 report=dict(scope='Measured ranking output only, not improved quality, popularity, calibrated affinity or human approval. Same synthetic installed games, fixed legacy signals and usage per scenario; not the personal library. Unknown duration keeps the identical legacy 30-minute control for both variants.',
             before_version=d['before_version'],after_version=d['after_version'],human_review=None,
             changed_scenarios=[s['scenario'] for s in d['scenarios'] if s['before']!=s['after']],
             unique_top_two={k:len(c) for k,c in counters.items()},counts={k:dict(c) for k,c in counters.items()},
             repeated_slots={k:sum(c.values())-len(c) for k,c in counters.items()},
             scenarios=d['scenarios'],explanations_kind='Proposed editorial context; not a full causal decomposition of the ranking policy',
             after_editorial_context={i:catalog['experiment']['explanations'][i] for i in sorted(counters['after'])})
 (output/'summary.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
 lines=['# Evidence-lot ranking comparison','','Same production policy and synthetic installed collection/usage. No human quality rating. Fixed slot 17; eight scenarios differ only in the recorded inputs, available subset, or fixed usage fixtures. Legacy emotion controls are 50 and legacy session length is 30 minutes for both versions; neither is research evidence. No personal library is read.','','| Scenario | Before · experimental.1 | After · experimental.2 |','|---|---|---|']
 for s in d['scenarios']:lines.append('|'+s['scenario']+'|'+' / '.join(names[i] for i in s['before'])+'|'+' / '.join(names[i] for i in s['after'])+'|')
 lines+=['',f"Changed scenarios: {len(report['changed_scenarios'])}/{len(d['scenarios'])}. Distinct top-two games: {report['unique_top_two']}. Repeated slots: {report['repeated_slots']}.",'','## Proposed editorial context · not a full ranking explanation','']
 lines += ['- '+names[i]+': '+text for i,text in report['after_editorial_context'].items()]
 lines+=['','The same candidate still recurs frequently. Sparse attributes and a fixed variety seed affect the result. Numeric changes do not establish better recommendations; editorial/human assessment remains pending.']
 (output/'comparison.md').write_text('\n'.join(lines)+'\n')
 with (output/'human-review.csv').open('x') as f:
  w=csv.writer(f);w.writerow(['scenario','before','after','contextual_fit','variety','explanation_supported','reviewer','notes'])
  for s in d['scenarios']:w.writerow([s['scenario'],' / '.join(names[i] for i in s['before']),' / '.join(names[i] for i in s['after']),'','','','',''])
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('source',type=Path);p.add_argument('output',type=Path);a=p.parse_args();summarize(a.source,a.output)
