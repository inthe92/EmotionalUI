#!/usr/bin/env python3
"""Generate argv data from explicit existing paths, not ES shell templates. Does not launch."""
import argparse,json
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--root',type=Path,required=True);p.add_argument('--executable',type=Path,required=True);p.add_argument('--core',type=Path,required=True);p.add_argument('--working-directory',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
root=a.root.resolve(strict=True);exe=a.executable.resolve(strict=True);core=a.core.resolve(strict=True);cwd=a.working_directory.resolve(strict=True)
if not(root.is_dir() and cwd.is_dir() and exe.is_file() and core.is_file()):raise SystemExit('Expected an installed executable/core and existing directories')
text='handoff="process-exit"\nclock_confirmed=false\n\n[[launchers]]\nid="retroarch-explicit"\n'
for k,v in [('root',root),('executable',exe),('working_directory',cwd)]:text+=k+'='+json.dumps(str(v))+'\n'
text+='arguments='+json.dumps(['-L',str(core),'{rom}'])+'\nwaits_for_exit=true\n'
with a.output.open('x') as f:f.write(text)
print(a.output)
