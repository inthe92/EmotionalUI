#!/usr/bin/env python3
"""Offline sampled upright candidates. No network, sampler or conversions at runtime.
Requires numpy for production and an installed FFmpeg; MIDI generation is stdlib.
"""
import argparse,hashlib,json,math,struct,subprocess,wave,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
sys.path.insert(0,str(ROOT/'tools'))
from compose_audio import vlq

def midi(version):
    events=[(0,b'\xff\x51\x03\x0f\x42\x40'),(0,b'\xc0\x00')];motifs={m['id']:m for m in version['motifs']}
    for start,name in version['phrases']:
        for at,pitch,velocity,hold,cents in motifs[name]['notes']:
            events += [(round((start+at)*480),bytes([0x90,pitch,velocity])),(round((start+at+hold)*480),bytes([0x80,pitch,0]))]
    events.sort(key=lambda x:x[0]);last=0;track=bytearray()
    for tick,event in events:track+=vlq(tick-last)+event;last=tick
    track+=vlq(round(version['seconds']*480)-last)+b'\xff\x2f\x00'
    return b'MThd'+struct.pack('>IHHH',6,0,1,480)+b'MTrk'+struct.pack('>I',len(track))+track

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--sample-cache',type=Path,required=True);ap.add_argument('--output',type=Path,required=True);ap.add_argument('--ffmpeg',default='/opt/homebrew/opt/ffmpeg-full/bin/ffmpeg');a=ap.parse_args()
    import numpy as np
    score=json.loads((ROOT/'assets/audio/phase9/source/score.json').read_text());lock=json.loads((ROOT/'assets/audio/phase9/instrument/sources.lock.json').read_text());rate=score['sample_rate'];samples={};manifest=[]
    a.output.mkdir(parents=True,exist_ok=False)
    for entry in lock['files']:
        if not entry['path'].endswith('.wav'):continue
        path=a.sample_cache/entry['cache_file'];assert hashlib.sha256(path.read_bytes()).hexdigest()==entry['sha256']
        # Raw 24-bit stereo samples remain outside repo. Decode only during production.
        pcm=subprocess.check_output([a.ffmpeg,'-nostdin','-v','error','-i',str(path),'-t','14','-ac','1','-ar',str(rate),'-af','lowpass=f=4800','-f','f32le','-'])
        index=int(path.stem.split('_')[-1]);pitch=21+index*2
        samples[pitch]=np.frombuffer(pcm,dtype='<f4').copy()
    def save(path,signal):
        path.parent.mkdir(parents=True,exist_ok=True);peak=float(np.max(np.abs(signal)));assert peak<.94
        pcm=np.rint(signal*32767).astype('<i2').tobytes()
        with wave.open(str(path),'wb') as w:w.setparams((1,2,rate,0,'NONE','not compressed'));w.writeframes(pcm)
        manifest.append(dict(file=str(path.relative_to(a.output)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),seconds=len(signal)/rate,peak=peak,zero_percent=100*float(np.count_nonzero(np.frombuffer(pcm,dtype='<i2')==0))/len(signal)))
    for name,version in score['variants'].items():
        target=a.output/name;target.mkdir();(target/'source.mid').write_bytes(midi(version));clips={}
        for motif in version['motifs']:
            signal=np.zeros(round(motif['seconds']*rate),dtype=np.float64)
            for at,pitch,velocity,hold,cents in motif['notes']:
                key=min(samples,key=lambda k:abs(k-pitch));source=samples[key];ratio=2**((pitch-key+cents/100)/12)
                n=min(round((hold+3.4)*rate),int((len(source)-1)/ratio));t=np.arange(n)/rate
                note=np.interp(np.arange(n)*ratio,np.arange(len(source)),source)
                # Preserve recorded hammer/body. Gentle release follows the explicit performance.
                envelope=np.minimum(1,t/.006)*np.exp(-np.maximum(0,t-hold)/(.85+velocity/100))
                note*=envelope*(velocity/48)**1.35*version['gain']
                start=round(at*rate);n=min(n,len(signal)-start);signal[start:start+n]+=note[:n]
            # Low-pass + restrained finite room reflections, no sustained noise bed.
            frequencies=np.fft.rfftfreq(len(signal),1/rate);filtered=np.fft.irfft(np.fft.rfft(signal)/(1+(frequencies/version['brightness_hz'])**4),n=len(signal));signal=filtered
            dry=signal.copy()
            for delay,amount in [(0.043,.5),(.089,.27),(.151,.14),(.223,.07)]:
                d=round(delay*rate);signal[d:]+=dry[:-d]*version['room_wet']*amount
            fade=round(.4*rate);signal[-fade:]*=np.linspace(1,0,fade);signal[:round(.004*rate)]*=np.linspace(0,1,round(.004*rate))
            # Truly zero the sub-quantization residual outside note tails.
            signal[np.abs(signal)<.3/32767]=0;clips[motif['id']]=signal;save(target/'runtime'/(motif['id']+'.wav'),signal)
        for env in ['rain','air']:(target/'runtime'/(env+'.wav')).write_bytes((ROOT/'assets/audio/runtime'/(env+'.wav')).read_bytes())
        preview=np.zeros(round(version['seconds']*rate))
        for start,motif in version['phrases']:
            offset=round(start*rate);clip=clips[motif];preview[offset:offset+len(clip)]+=clip
        save(target/(name+'.wav'),preview)
        for ext,args in [('ogg',['-c:a','libvorbis','-q:a','5']),('m4a',['-c:a','aac','-b:a','96k'])]:
            subprocess.run([a.ffmpeg,'-nostdin','-n','-v','error','-i',str(target/(name+'.wav')),*args,str(target/(name+'.'+ext))],check=True)
    (a.output/'render-manifest.json').write_text(json.dumps({'sample_instrument':'VSCO 2 CE Upright Piano, Simon Dalzell / Ivy Audio; Versilian Studios','sample_license':'CC0-1.0','score_license':'GPL-3.0-only','sample_commit':lock['commit'],'rate':rate,'numpy_version':np.__version__,'python_version':sys.version.split()[0],'ffmpeg_version':subprocess.check_output([a.ffmpeg,'-version'],text=True).splitlines()[0],'score_sha256':hashlib.sha256((ROOT/'assets/audio/phase9/source/score.json').read_bytes()).hexdigest(),'resources':manifest},indent=2)+'\n')
if __name__=='__main__':main()
