"""Regenerate offline visible grains from any compatible exported Pyro sequence.
Python + NumPy; no Houdini. Output is the public96-byte particle format.
"""
from pathlib import Path
import argparse,json,math
import numpy as np
import offline_emission as model
from pyro_reference import Sequence
from portable_io import write_particles
def generate(manifest,output):
    output=Path(output).resolve()
    if output.exists() or output.with_suffix('.json').exists():raise FileExistsError(output)
    flow=Sequence(manifest);births=model.grain_births();N=len(births);P=np.zeros((N,3));V=P.copy();frames=[]
    for step in range(481):
        t=step/120;rows=[]
        for j,b in enumerate(births):
            if not b['birth']<=t<b['birth']+b['life']:continue
            previous=(step-1)/120
            if step==0 or b['birth']>previous:P[j]=b['P'];V[j]=b['v'];dt=t-b['birth']
            else:dt=1/120
            if dt>0:
                u=np.array(flow.velocity(P[j],t-dt*.5));v0=V[j].copy();target=u+[0,-b['gravity']/b['drag'],0];V[j]=target+(V[j]-target)*np.exp(-b['drag']*dt);P[j]+=.5*(v0+V[j])*dt
                if P[j,1]<b['radius']:P[j,1]=b['radius'];V[j,1]=max(0,-V[j,1]*.12);V[j,[0,2]]*=.8
            theta=b['id']*.71;rows.append(dict(id=b['id'],parent=b['parent'],kind=1,variant=0,birth=b['birth'],life=b['life'],P=P[j].tolist(),v=V[j].tolist(),scale=(np.array([1.,.66,1.23])*b['radius']).tolist(),concentration=1.,q=[0,math.sin(theta/2),0,math.cos(theta/2)],age=t-b['birth'],color=(np.array([.65,.44,.25])*b['shade']).tolist(),flags=0))
        frames.append((t,rows))
    return write_particles(output,frames,dict(role='Offline grain reference regenerated from portable flow',source_rate_hz=120.))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--sequence',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();generate(a.sequence,a.output)
