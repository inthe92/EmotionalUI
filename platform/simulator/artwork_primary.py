from pathlib import Path
import subprocess,os,sys
sim=sys.argv[1];base=Path(sys.argv[2])
for view,name in [('resume','continuar-principal'),('emotional','descubre-principal')]:
 replay=base/(name+'.toml');replay.write_text('keyboard=true\n[mapping]\n[[steps]]\nat_ms=1300\ntext="up"\npressed=true\nselected=0\n[[steps]]\nat_ms=1308\ntext="up"\npressed=false\n')
 p=subprocess.run([sim,'--trial','--library-root',str(base/'collection'),'--db',str(base/'collection.sqlite'),'--view',view,'--at','2026-10-19T18:30','--audio-mode','silence','--reduced-motion','--smoke-seconds','3','--sim-replay',str(replay),'--snapshot',str(base/(name+'.png'))],capture_output=True,text=True,env=dict(os.environ,EMOTIONAL_ARTWORK_CACHE=str(base/'cache')),timeout=20)
 (base/(name+'.stdout')).write_text(p.stdout);(base/(name+'.stderr')).write_text(p.stderr);assert p.returncode==0,p.stderr
