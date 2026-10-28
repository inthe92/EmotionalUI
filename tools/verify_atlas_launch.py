#!/usr/bin/env python3
"""Offline Atlas identity -> recommendation -> real supervisor with own fixture."""
import argparse,json,sqlite3,subprocess,sys,tempfile
from pathlib import Path
from verify_player_flow import press,replay

def run(binary,atlas,output):
 output.mkdir(parents=True,exist_ok=False);root=Path(tempfile.mkdtemp(prefix='eui-atlas-launch-')).resolve();system=root/'roms'/'snes';system.mkdir(parents=True)
 (system/'own.fixture').write_text('Synthetic own data, not a game.\n');xml=system/'gamelist.xml';xml.write_text('<gameList><game><path>own.fixture</path><name>Super Metroid</name></game></gameList>')
 db=root/'library.sqlite'
 def execute(name,args):
  p=subprocess.run([str(binary),*args],capture_output=True,text=True,timeout=45);(output/(name+'.log')).write_text(p.stdout+'\n'+p.stderr)
  if p.returncode:raise RuntimeError(name+' failed; see '+str(output/(name+'.log')))
  return p.stdout
 execute('import',['--db',str(db),'--import-gamelist',str(xml),'--rom-root',str(system)])
 with sqlite3.connect(db) as c:
  rows=c.execute('select id from imported_games').fetchall();assert len(rows)==1;local=rows[0][0]
 execute('link',['--db',str(db),'--library-root',str(root/'roms'),'--atlas',str(atlas),'--atlas-link',local+'=atlas:snes:000005'])
 return_replay=root/'return.toml';steps=[dict(at_ms=700,hero_title='Super Metroid')];press(steps,1000,100)
 for i in range(8):press(steps,1200+i*100,101)
 steps.append(dict(at_ms=2100,menu=True,menu_index=8));steps.append(dict(at_ms=2300,kind=1,code=102,value=1));replay(return_replay,steps)
 game=root/'fake_game.py';game.write_text('from pathlib import Path\nimport time\nPath(__file__).with_name("launched").write_text("ok")\ntime.sleep(2.2)\n')
 profile=root/'profile.toml';profile.write_text('handoff="process-exit"\nclock_confirmed=true\n[[launchers]]\nid="synthetic"\n'+''.join(k+'='+json.dumps(v)+'\n' for k,v in {'root':str(system),'executable':sys.executable,'working_directory':str(root),'arguments':[str(game),'{rom}'],'extensions':['.fixture'],'waits_for_exit':True}.items()))
 first=[dict(at_ms=700,hero_title='Super Metroid',resume=False)];press(first,1100,102);first.append(dict(at_ms=1800));replay(root/'start.toml',first)
 text=execute('flow',['--functional','--launcher-config',str(profile),'--library-root',str(root/'roms'),'--db',str(db),'--atlas',str(atlas),'--atlas-experimental','--view','emotional','--reduced-motion','--audio-mode','silence','--at','2026-01-15T14:00','--sim-replay',str(root/'start.toml'),'--sim-return-replay',str(return_replay)])
 assert (root/'launched').read_text()=='ok' and 'linked=1' in text
 with sqlite3.connect(db) as c:
  links=c.execute('select game_id,atlas_id from atlas_links').fetchall();logs=c.execute('select game_id,outcome from launch_log').fetchall();activity=c.execute('select game_id,recommended from atlas_activity').fetchall()
 assert links==[(local,'atlas:snes:000005')] and logs==[(local,'exited-success')] and activity==[(local,1)],(links,logs,activity)
 result={'explicit_atlas_link':True,'recommendation_title_displayed':True,'same_local_id_launched':True,'recommended_activity_recorded':True,'process':'synthetic','physical_console':False}
 (output/'report.json').write_text(json.dumps(result,indent=2)+'\n');return result
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',type=Path,required=True);p.add_argument('--atlas',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();print(json.dumps(run(a.binary.resolve(),a.atlas.resolve(),a.output.resolve()),indent=2))
