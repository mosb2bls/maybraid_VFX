"""Filled dust body appearance, independent of Houdini and motion.

The canonical runtime reference evaluates reused rest densities after the exact
unchanged inverse deformation/TRS. Dense world grids below are live, disposable
offline authoring samples, not an exported animation or runtime requirement.
"""
from dataclasses import dataclass,asdict
from functools import lru_cache
from pathlib import Path
import json,math
import numpy as np
import wave_motion as motion
import local_deformation as deform
import landing_motion as landing
A=Path(__file__).resolve().parents[1];B=A/'Assets/Body'

@dataclass(frozen=True)
class Style:
    thickness:float=3.0
    variation:float=.32
    light_contrast:float=.9
    ramp_end_density:float=.18
    overlap_cap:float=2.2
    voxel_size:float=.0175

KEY=np.array([-3.8,5.5,3.]);KEY/=np.linalg.norm(KEY)

def S(x):x=np.clip(x,0,1);return x*x*(3-2*x)
def _hash(x,y,z,seed):
    h=(x.astype('u4')*np.uint32(0x8da6b343))^(y.astype('u4')*np.uint32(0xd8163841))^(z.astype('u4')*np.uint32(0xcb1ab31f))^np.uint32(seed)
    h^=h>>np.uint32(16);h*=np.uint32(0x7feb352d);h^=h>>np.uint32(15);h*=np.uint32(0x846ca68b);h^=h>>np.uint32(16)
    return (h&np.uint32(0xffffff)).astype(float)/16777215.
def _value(p,seed):
    cell=np.floor(p).astype('i8');f=p-cell;f=f*f*f*(f*(f*6-15)+10);out=np.zeros(len(p))
    for z in [0,1]:
        for y in [0,1]:
            for x in [0,1]:out+=_hash(cell[:,0]+x,cell[:,1]+y,cell[:,2]+z,seed)*(f[:,0] if x else 1-f[:,0])*(f[:,1] if y else 1-f[:,1])*(f[:,2] if z else 1-f[:,2])
    return out
def dissolve_array(rest,seed):
    rest=np.asarray(rest,dtype=float)
    return .7*_value(rest*2.2,seed)+.3*_value(rest*4.4+[19.3,-7.1,11.7],seed^0x9e3779b9)
def random_value(p,seed):
    p=np.asarray(p,dtype='f8');phase=(seed&0xffff)*.000219
    # Bounded broad-to-medium rest-coordinate concentration; no temporal phase.
    return (.5+.22*np.sin(p@np.array([2.1,1.7,.9])+phase)
            +.16*np.sin(p@np.array([-1.4,3.2,2.6])+phase*1.71)
            +.10*np.sin(p@np.array([4.7,-.8,3.1])+phase*.63))

@lru_cache(None)
def rest_data(v):
    from portable_io import read_glb
    meta=json.loads((B/'BodyAssets.json').read_text())['assets'][v]
    grid=meta['grid'];lo=np.array(grid['bounds_min_m']);h=np.array(grid['spacing_xyz_m']);dims=np.array(grid['dimensions_xyz'])
    rho=np.fromfile(B/meta['density'],dtype='<f2').astype('f4').reshape(tuple(dims[::-1]))
    P,N,_=read_glb(B/meta['mesh']);nonzero=np.argwhere(rho>0)
    anchors=[]
    targets=[np.array([.15,.83*P[:,1].max(),.40]),np.array([-.85,.55,.18]),np.array([.45,.16,-.55])]
    for k,target in enumerate(targets):
        ids=np.where((P[:,1]>.12)&(N[:,1]>-.1))[0];i=ids[np.argmin(np.sum((P[ids]-target)**2,axis=1))]
        anchors.append(dict(point=P[i].astype(float),normal=N[i].astype(float),kind=k))
    return dict(rho=rho,lo=lo,h=h,shape=dims,P=P,N=N,anchors=anchors,support_lo=lo+(nonzero.min(0)[::-1]-.5)*h,support_hi=lo+(nonzero.max(0)[::-1]+1.5)*h)

def sample_rest(p,v):
    d=rest_data(v);q=(np.asarray(p)-d['lo'])/d['h']-.5
    i=np.floor(q).astype('i8');f=q-i;out=np.zeros(len(q))
    for z in [0,1]:
        for y in [0,1]:
            for x in [0,1]:
                j=i+[x,y,z];valid=np.all((j>=0)&(j<d['shape']),axis=1)
                out[valid]+=d['rho'][j[valid,2],j[valid,1],j[valid,0]]*(f[valid,0] if x else 1-f[valid,0])*(f[valid,1] if y else 1-f[valid,1])*(f[valid,2] if z else 1-f[valid,2])
    return out

def states(t,application='wave'):
    if application=='wave':return motion.instances(float(t),True,True)
    out=[]
    for s in landing.states(float(t),landing.Settings(seed=13)):
        out.append(dict(s,age=s['age_seconds'],rotation=motion.yaw_matrix(s['yaw']),
            pivot=np.zeros(3),coefficients=np.zeros(4),phase=0.,clip=0))
    return out

def appearance_seed(s):
    # Wave motion defines stable IDs but no active material seed (dissolve OFF).
    # These fixed appearance-only seeds never modify its organization.
    return int(s['seed']) if 'seed' in s else landing.mix32(13^((s['id']+1)*0x27d4eb2d))

def evaluate(rest,s,style=Style(),world_y=None):
    seed=appearance_seed(s);n=random_value(rest,seed)
    original=sample_rest(rest,s['variant'])
    main=S(np.maximum(original,0)/style.ramp_end_density)*(1+style.variation*(2*n-1))
    fine=np.zeros(len(rest))
    # Secondary landing application retains its ORIGINAL spatial dissolution.
    if 'kind' in s and s['dissolve_threshold']>0:
        keep=dissolve_array(rest,seed)>s['dissolve_threshold']
        main*=keep;fine*=keep
    return main,fine

def rest_support(s,style=Style()):
    d=rest_data(s['variant']);lo=d['support_lo'].copy();hi=d['support_hi'].copy()
    return lo,hi

def warped_bounds(lo,hi,c):
    a,b,g,d=c;hmax=1.25*np.exp(-.5)
    yl=min(lo[1]*np.exp(abs(a)),lo[1]*np.exp(-abs(a)))
    yh=max(hi[1]*np.exp(abs(a)),hi[1]*np.exp(-abs(a)))
    return np.array([lo[0]-abs(b)*hmax,yl,lo[2]*np.exp(abs(g))-abs(d)*hmax]),np.array([hi[0]+abs(b)*hmax,yh,hi[2]*np.exp(abs(g))+abs(d)*hmax])

def instance_bounds(s,style=Style()):
    lo,hi=warped_bounds(*rest_support(s,style),s['coefficients'])
    corners=np.array([[x,y,z] for x in [lo[0],hi[0]] for y in [lo[1],hi[1]] for z in [lo[2],hi[2]]])
    W=(corners*s['scale']-s['pivot'])@s['rotation'].T+s['pivot']+s['position']
    return W.min(0),W.max(0)

def transport_light(rho,h,style=Style()):
    # Deterministic upwind directional optical-depth transport, recomputed from
    # the CURRENT composed volume. This is an offline lighting approximation,
    # not physical multiple scattering or a Bevy implementation.
    nz,ny,nx=rho.shape;z,x=np.indices((nz,nx),dtype=float)
    qx=x+KEY[0]/KEY[1];qz=z+KEY[2]/KEY[1]
    ix=np.floor(qx).astype(int);iz=np.floor(qz).astype(int);fx=qx-ix;fz=qz-iz
    tau=np.zeros_like(rho);ds=h/KEY[1]
    for y in range(ny-2,-1,-1):
        prev=np.zeros((nz,nx))
        for dz in [0,1]:
            for dx in [0,1]:
                xx=ix+dx;zz=iz+dz;valid=(xx>=0)&(xx<nx)&(zz>=0)&(zz<nz)
                prev[valid]+=tau[zz[valid],y+1,xx[valid]]*(fx[valid] if dx else 1-fx[valid])*(fz[valid] if dz else 1-fz[valid])
        tau[:,y,:]=prev+.5*(rho[:,y,:]+rho[:,y+1,:])*ds*style.thickness
    dz,dy,dx=np.gradient(rho,h)
    mag=np.sqrt(dx*dx+dy*dy+dz*dz)
    facing=np.maximum(0,(dx*-KEY[0]+dy*-KEY[1]+dz*-KEY[2])/np.maximum(mag,1e-6))
    edge=S(mag/7.)
    form=(1-.30*edge)+.30*edge*(.3+.7*facing)
    return ((.28+.90*np.exp(-style.light_contrast*tau))*form).astype('f4')

@lru_cache(2)
def frame_fields(t,style=Style(),application='wave'):
    st=states(t,application);h=style.voxel_size
    if not st:
        lo=np.array([-.2,0,-.2]);dims=np.array([10,10,10]);hi=lo+dims*h
        empty=np.zeros(tuple(dims[::-1]),dtype='f4')
        return dict(lo=lo,hi=hi,dims=dims,main=empty,full=empty.copy(),fine=empty.copy(),main_light=empty.copy(),full_light=empty.copy(),states=st)
    boxes=[instance_bounds(s,style) for s in st]
    lo=np.floor((np.min([b[0] for b in boxes],axis=0)-.12)/h)*h
    hi=np.ceil((np.max([b[1] for b in boxes],axis=0)+.12)/h)*h
    dims=np.rint((hi-lo)/h).astype(int);shape=tuple(dims[::-1])
    M=np.zeros(shape);F=np.zeros(shape)
    for s,(mn,mx) in zip(st,boxes):
        first=np.maximum(0,np.floor((mn-lo)/h-.5).astype(int));last=np.minimum(dims,np.ceil((mx-lo)/h+.5).astype(int)+1)
        z,y,x=np.meshgrid(np.arange(first[2],last[2]),np.arange(first[1],last[1]),np.arange(first[0],last[0]),indexing='ij')
        W=lo+(np.stack([x,y,z],axis=-1).reshape(-1,3)+.5)*h
        rest=motion.rest_from_world(W,s)
        main,fine=evaluate(rest,s,style,W[:,1]);sl=(slice(first[2],last[2]),slice(first[1],last[1]),slice(first[0],last[0]))
        M[sl]+=main.reshape(x.shape);F[sl]+=fine.reshape(x.shape)
    cap=style.overlap_cap
    main=(cap*(-np.expm1(-M/cap))).astype('f4')
    full=(cap*(-np.expm1(-(M+F)/cap))).astype('f4')
    return dict(lo=lo,hi=hi,dims=dims,main=main,full=full,fine=F.astype('f4'),
                main_light=transport_light(main,h,style),full_light=transport_light(full,h,style),states=st)
