"""World density for native Houdini demonstration of portable particle rules.
Only the offline function reads the optional Pyro reference. dynamic_field does
not import or sample that reference and evaluates the proposed small clouds.
"""
from pathlib import Path
from functools import lru_cache
import json,numpy as np
import body_density as body
import wave_motion as wave
import dynamic_dust as dynamic
from portable_io import volume_array,sample_array,read_particle_frame
ROOT=Path(__file__).resolve().parents[1]
@lru_cache(None)
def library():
    path=ROOT/'Assets/DynamicDust/SmallClouds/CloudLibrary.json';doc=json.loads(path.read_text())
    return [(volume_array(a,path.parent),a) for a in doc['assets']]
@lru_cache(4)
def particle_index(kind):
    path=ROOT/('Assets/Reference/VisibleGrains.json' if kind=='offline' else 'Assets/DynamicDust/'+('NormalMotion' if kind=='normal' else 'WindAndTurn')+'.json')
    return path,json.loads(path.read_text())
def rows(t,kind):
    path,idx=particle_index(kind);frame=min(len(idx['frames'])-1,max(0,round(t*120)))
    return read_particle_frame(path.with_suffix('.bin'),idx,frame)
_live={}
def live_rows(t,scenario,config):
    key=(scenario,config)
    if key not in _live:_live[key]=(dynamic.Simulation(scenario,config),{0:[]})
    sim,history=_live[key];tick=max(0,round(t/config.dt))
    while sim.tick<tick:sim.step();history[sim.tick]=sim.snapshot()
    return history[tick]
def bounds(states,particles,h):
    boxes=[body.instance_bounds(s,body.Style()) for s in states]
    for p in particles:
        if p['kind']!=0:continue
        R=dynamic.qmatrix(p['q']);extent=np.abs(R)@np.array(p['scale']);boxes.append((np.array(p['P'])-extent,np.array(p['P'])+extent))
    lo=np.floor((np.min([b[0] for b in boxes],0)-.12)/h)*h;hi=np.ceil((np.max([b[1] for b in boxes],0)+.12)/h)*h
    return lo,hi,np.rint((hi-lo)/h).astype(int)
def coordinates(lo,h,first,last):
    z,y,x=np.meshgrid(np.arange(first[2],last[2]),np.arange(first[1],last[1]),np.arange(first[0],last[0]),indexing='ij')
    return lo+(np.stack([x,y,z],axis=-1).reshape(-1,3)+.5)*h,x.shape,(slice(first[2],last[2]),slice(first[1],last[1]),slice(first[0],last[0]))
@lru_cache(4)
def dynamic_field(t,scenario='normal',h=.025,config=dynamic.Config()):
    particles=live_rows(t,scenario,config);states=dynamic.body_states(float(t),scenario);lo,hi,dims=bounds(states,particles,h);M=np.zeros(tuple(dims[::-1]));F=M.copy()
    for s in states:
        mn,mx=body.instance_bounds(s);first=np.maximum(0,np.floor((mn-lo)/h-.5).astype(int));last=np.minimum(dims,np.ceil((mx-lo)/h+.5).astype(int)+1)
        W,shape,sl=coordinates(lo,h,first,last);rest=wave.rest_from_world(W,s);main,_=body.evaluate(rest,s,world_y=W[:,1]);M[sl]+=main.reshape(shape)
    for p in particles:
        if p['kind']!=0 or p['concentration']<=0:continue
        R=dynamic.qmatrix(p['q']);extent=np.abs(R)@np.array(p['scale']);P=np.array(p['P']);first=np.maximum(0,np.floor((P-extent-lo)/h-.5).astype(int));last=np.minimum(dims,np.ceil((P+extent-lo)/h+.5).astype(int)+1)
        W,shape,sl=coordinates(lo,h,first,last);local=((W-P)@R)/p['scale'];a,meta=library()[p['variant']];rho=sample_array(local,a,meta)*p['concentration'];rho[W[:,1]<0]=0;F[sl]+=rho.reshape(shape)
    rho=(2.2*(-np.expm1(-(M+F)/2.2))).astype('f4');light=body.transport_light(rho,h)
    return dict(density=rho,lightfield=light,lo=lo,hi=hi,dims=dims,particles=particles)
@lru_cache(4)
def offline_raw(index):
    path=ROOT/'Assets/Reference/Pyro/Sequence.json';doc=json.loads(path.read_text());frame=doc['frames'][index]
    return {name:(volume_array(meta,path.parent),meta) for name,meta in frame['fields'].items()}
def sample_offline(W,t,field='density'):
    f=np.clip(t*30,0,120);i=int(np.floor(f));j=min(120,i+1);a=f-i
    value,meta=offline_raw(i)[field];out=sample_array(W,value,meta)
    if a>1e-8:
        value,meta=offline_raw(j)[field];out=out*(1-a)+sample_array(W,value,meta)*a
    return out
@lru_cache(2)
def offline_field(t,h=.0175):
    base=body.frame_fields(float(t));corners=[]
    for index in {int(np.floor(np.clip(t*30,0,120))),int(np.ceil(np.clip(t*30,0,120)))}:
        native=offline_raw(index)['density'][1];mat=np.array(native['index_to_world_m']);n=np.array(native['dimensions_xyz'])
        corners.extend(np.array([[x,y,z] for x in [-.5,n[0]-.5] for y in [-.5,n[1]-.5] for z in [-.5,n[2]-.5]])@mat[:,:3].T+mat[:,3])
    corners=np.array(corners)
    lo=np.floor((np.minimum(base['lo'],corners.min(0))-.12)/h)*h;hi=np.ceil((np.maximum(base['hi'],corners.max(0))+.12)/h)*h;dims=np.rint((hi-lo)/h).astype(int);M=np.zeros(tuple(dims[::-1]),dtype='f4');F=M.copy()
    start=np.rint((base['lo']-lo)/h).astype(int);end=start+base['dims'];sl=tuple(slice(start[k],end[k]) for k in [2,1,0]);M[sl]=base['main']
    for z in range(0,dims[2],8):
        W,shape,sl=coordinates(lo,h,[0,0,z],[dims[0],dims[1],min(z+8,dims[2])]);d=np.maximum(sample_offline(W,t),0);d=.75*d+.12*d**1.35;d[W[:,1]<0]=0;F[sl]=d.reshape(shape)
    rho=(2.2-(2.2-M)*np.exp(-F/2.2)).astype('f4')
    return dict(density=rho,lightfield=body.transport_light(rho,h),lo=lo,hi=hi,dims=dims,particles=rows(t,'offline'))
