#!/usr/bin/env python3
import os,sys
zig=os.environ['EMOTIONAL_ZIG']
os.execv(zig,[zig,'ar',*sys.argv[1:]])
