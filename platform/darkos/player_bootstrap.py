#!/usr/bin/env python3
"""Install the exact requested private functional package, then use its menu entry."""
import os
from pathlib import Path
import subprocess
import sys
import time
import manage
import phase12_preview as recovery


def main():
    source = Path(__file__).resolve().parent
    private = recovery.verify_bundle(source)
    if not recovery.functional_bundle(private):
        raise ValueError('Expected a functional EmotionalUI package')
    package = manage.manifest(source)
    manage.compatible(source, package)
    namespace = 'emotional-ui-beta-' if private.get('beta') else 'emotional-ui-player-'
    destination = Path.home()/'.local/opt'/(namespace+private['binary_sha256'][:12])
    destination.parent.mkdir(parents=True,exist_ok=True)
    manage.install(source,destination,confirmed=True)
    os.execv('/usr/bin/python3',['/usr/bin/python3','-B',str(destination/'player_menu.py')])


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print('EmotionalUI: '+str(error),file=sys.stderr,flush=True)
        time.sleep(5)
        sys.exit(1)
