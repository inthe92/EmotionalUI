#!/usr/bin/env python3
"""Original offline score -> MIDI + PCM studies. Python stdlib, deterministic seed.
No sample library, MIDI player, network or runtime synthesis dependency.
"""
from pathlib import Path
import array,hashlib,json,math,random,struct,sys,wave
ROOT=Path(__file__).resolve().parent.parent

def vlq(value):
    parts=[value&127];value>>=7
    while value:parts.append((value&127)|128);value>>=7
    return bytes(reversed(parts))

def midi(score):
    # 60 BPM, 480 ticks/second: seconds remain explicit, not quantized to a grid.
    events=[(0,b'\xff\x51\x03\x0f\x42\x40'),(0,b'\xc0\x00')]
    motifs={m['id']:m for m in score['motifs']}
    for start,name in score['preview']['phrases']:
        for onset,pitch,velocity,hold in motifs[name]['notes']:
            events.extend([(round((start+onset)*480),bytes([0x90,pitch,velocity])),(round((start+onset+hold)*480),bytes([0x80,pitch,0]))])
    events.sort(key=lambda e:e[0]);track=bytearray();last=0
    for tick,data in events:track+=vlq(tick-last)+data;last=tick
    track+=vlq(max(0,score['preview']['seconds']*480-last))+b'\xff\x2f\x00'
    return b'MThd'+struct.pack('>IHHH',6,0,1,480)+b'MTrk'+struct.pack('>I',len(track))+track

def render_phrase(motif,rate,seed):
    signal=[0.0]*round(motif['seconds']*rate);rng=random.Random(seed)
    for onset,pitch,velocity,hold in motif['notes']:
        hz=440*2**((pitch-69)/12);start=round(onset*rate);phase=rng.uniform(-.015,.015)
        length=min(len(signal)-start,round((hold+3.5)*rate))
        for i in range(length):
            t=i/rate;attack=1-math.exp(-t/0.025)
            # Quiet inharmonic partials and damped release, deliberately no bright hammer/click.
            tone=sum(amplitude*math.sin(math.tau*hz*multiple*t+phase)*math.exp(-t*(0.48+multiple*.16)) for multiple,amplitude in ((1,1),(2.001,.19),(3.006,.065),(4.012,.018)))
            release=math.exp(-max(0,t-hold)/.8)
            signal[start+i]+=tone*attack*release*(velocity/127)*.53
    # Restrained original room response; no external impulse-response sample.
    dry=signal[:]
    for seconds,gain in ((.047,.11),(.079,.08),(.131,.05),(.193,.025)):
        delay=round(seconds*rate)
        for i in range(delay,len(signal)):signal[i]+=dry[i-delay]*gain
    fade=round(.25*rate)
    for i in range(fade):signal[-i-1]*=i/fade
    return signal

def environment(kind,rate,seed,seconds):
    rng=random.Random(seed);out=[];slow=0.0;state=0.0;decay=.965 if kind=='rain' else .995
    for i in range(round(seconds*rate)):
        state=decay*state+(1-decay)*rng.uniform(-1,1);slow=.999*slow+.001*state
        t=i/rate;envelope=min(1,t/2,(seconds-t)/2)
        out.append((state if kind=='rain' else slow)*envelope*(.55 if kind=='rain' else .8)*(0.65+0.15*math.sin(t*.47)))
    return out

def wav(path,signal,rate,base=ROOT):
    samples=array.array('h',(round(max(-.95,min(.95,v))*32767) for v in signal))
    if sys.byteorder!='little':samples.byteswap()
    with wave.open(str(path),'wb') as f:f.setnchannels(1);f.setsampwidth(2);f.setframerate(rate);f.writeframes(samples.tobytes())
    return {'file':str(path.relative_to(base)),'samples':len(signal),'seconds':len(signal)/rate,'peak':max(map(abs,signal),default=0),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}

def main():
    import argparse
    parser=argparse.ArgumentParser();parser.add_argument('--output-root',type=Path,default=ROOT);args=parser.parse_args();destination=args.output_root.resolve()
    for folder in ('source','runtime','previews'):(destination/'assets/audio'/folder).mkdir(parents=True,exist_ok=True)
    score=json.loads((ROOT/'assets/audio/source/score.json').read_text());rate=score['sample_rate'];records=[];clips={}
    for i,m in enumerate(score['motifs']):
        clips[m['id']]=render_phrase(m,rate,score['seed']+i)
        records.append(wav(destination/f"assets/audio/runtime/{m['id']}.wav",clips[m['id']],rate,destination))
    for i,(name,seconds) in enumerate((('rain',18),('air',16))):records.append(wav(destination/f'assets/audio/runtime/{name}.wav',environment(name,rate,score['seed']+10+i,seconds),rate,destination))
    preview=[0.0]*round(score['preview']['seconds']*rate)
    for start,name in score['preview']['phrases']:
        at=round(start*rate)
        for i,value in enumerate(clips[name]):preview[at+i]+=value
    records.append(wav(destination/'assets/audio/previews/la-habitacion-permanece.wav',preview,rate,destination))
    (destination/'assets/audio/source/la-habitacion-permanece.mid').write_bytes(midi(score))
    (destination/'assets/audio/source/render-manifest.json').write_text(json.dumps({'generator':'tools/compose_audio.py','sample_rate':rate,'sample_libraries':[],'resources':records},indent=2)+'\n')
    print(json.dumps(records,indent=2))
if __name__=='__main__':main()
