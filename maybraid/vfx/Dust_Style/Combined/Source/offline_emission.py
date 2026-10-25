"""Offline secondary dust emission and grain reference; metres, +Y, seconds."""
from dataclasses import dataclass,asdict
from functools import lru_cache
import os,re
import numpy as np
import body_density as body
motion=body.motion


@dataclass(frozen=True)
class Settings:
    emission:float=.65
    guide_strength:float=5.0
    release:float=.50
    turbulence:float=.10
    dissipation:float=.035
    grain_rate:float=90.
    grain_size:float=1.
    seed:int=271
    voxel:float=.04

# parent, anchor kind, start/end parent age, amplitude. Unequal, separated regions.
REGIONS=[(0,1,.18,1.10,1.),(0,0,1.65,2.55,.65),(3,0,.26,1.35,1.2),
         (3,2,.38,1.55,.80),(6,1,.24,1.50,1.1),(6,0,.50,1.55,.60),
         (9,0,.26,1.50,1.0),(9,2,.42,1.65,.70),(11,1,.30,1.60,.85),
         (13,0,.16,.55,.6)]
SOURCE_LO=np.array([-2.4,0.,-2.8]);SOURCE_HI=np.array([4.4,2.4,2.8])

@lru_cache(2048)
def state_table(t):return {s['id']:s for s in body.states(float(t))}
def material_velocity(rest,t,pid,dt=.0005):
    s=state_table(float(t))[pid];birth=s['birth'];lo=max(birth,t-dt);hi=t+dt
    return (motion.world(rest,state_table(float(hi))[pid])-motion.world(rest,state_table(float(lo))[pid]))/(hi-lo)
def pulse(age,start,end):
    if not start<age<end:return 0.
    u=(age-start)/(end-start)
    return float(np.sin(np.pi*u)**1.25)
def regions(t,settings=Settings()):
    out=[]
    for rid,(pid,kind,start,end,amplitude) in enumerate(REGIONS):
        s=state_table(float(t)).get(pid)
        if s is None:continue
        gate=pulse(s['age'],start,end)
        if gate==0 or t>2.82:continue
        a=body.rest_data(s['variant'])['anchors'][kind]
        # A small outward center offset with overlapping source radii keeps
        # birth connected while allowing peripheral release to remain visible.
        center=a['point']+a['normal']*.16
        radii=np.array([[.34,.25,.34],[.40,.21,.31],[.39,.18,.37]][kind])
        out.append(dict(id=rid,parent=pid,kind=kind,state=s,center=center,radii=radii,
                        anchor=a['point'],normal=a['normal'],gate=gate*amplitude))
    return out
def relative_release(rest,r,settings=Settings()):
    s=r['state'];N=motion.world_normals(r['center'][None,:],r['normal'][None,:],s)[0]
    # Restrained outward lift and broad roll around the parent shoulder.
    delta=motion.world(rest,s)-motion.world(r['center'],s)
    axis=s['rotation']@np.array([0.,0.,1.])
    rolling=np.cross(np.broadcast_to(axis,delta.shape),delta)*.65
    trail=s['rotation']@np.array([-.38,.22,.0])
    return settings.release*(N*.55+trail)+rolling
def emit_velocity(rest,r,t,settings=Settings()):
    return material_velocity(rest,t,r['parent'])+relative_release(rest,r,settings)

def source_fields(t,settings=Settings()):
    h=settings.voxel;lo=SOURCE_LO;dims=np.rint((SOURCE_HI-lo)/h).astype(int)
    rho=np.zeros(tuple(dims[::-1]));weight=np.zeros_like(rho);moment=np.zeros(rho.shape+(3,));birth_moment=np.zeros_like(moment)
    active=regions(float(t),settings)
    for r in active:
        s=r['state'];center=motion.world(r['center'],s)
        # Guide support larger than density source, smooth/finite. Numerical
        # bounds use conservative deformation derivative amplification2.5.
        extent=float(np.max(r['radii']*s['scale']))*2.5*2.1
        first=np.maximum(0,np.floor((center-extent-lo)/h).astype(int));last=np.minimum(dims,np.ceil((center+extent-lo)/h).astype(int))
        z,y,x=np.meshgrid(np.arange(first[2],last[2]),np.arange(first[1],last[1]),np.arange(first[0],last[0]),indexing='ij')
        W=lo+(np.stack([x,y,z],axis=-1).reshape(-1,3)+.5)*h
        rest=motion.rest_from_world(W,s);q=(rest-r['center'])/r['radii'];r2=np.sum(q*q,axis=1)
        k=np.maximum(0,1-r2)**2
        g=np.maximum(0,1-r2/2.1**2)**2*r['gate']
        # Broad rest variation only; Pyro, not time-noise, evolves released dust.
        noise=.75+.25*body.random_value(rest*2.1,settings.seed+r['id']*101)
        d=settings.emission*r['gate']*k*noise
        vel=emit_velocity(rest,r,t,settings)
        sl=(slice(first[2],last[2]),slice(first[1],last[1]),slice(first[0],last[0]))
        rho[sl]+=d.reshape(x.shape);weight[sl]+=g.reshape(x.shape)
        moment[sl]+=vel.reshape(x.shape+(3,))*g.reshape(x.shape+(1,))
        birth_moment[sl]+=vel.reshape(x.shape+(3,))*d.reshape(x.shape+(1,))
    velocity=moment/np.maximum(weight[...,None],1e-20)
    return dict(lo=lo.copy(),hi=SOURCE_HI.copy(),dims=dims,density=rho.astype('f4'),
                guide=(1-np.exp(-weight)).astype('f4'),velocity=velocity.astype('f4'),
                birth_velocity=(birth_moment/np.maximum(rho[...,None],1e-20)).astype('f4'),
                birth_momentum=birth_moment.astype('f4'),regions=active)

def grain_births(settings=Settings()):
    """Counter-based fixed event table, independent of frame partition/order."""
    out=[];rng=np.random.default_rng(settings.seed)
    # Candidate times over actual source activity. Acceptance tracks pulses.
    for rid,(pid,kind,start,end,amplitude) in enumerate(REGIONS):
        birth=state_table(3.)[pid]['birth'];count=int(np.ceil((end-start)*settings.grain_rate/len(REGIONS)*2.4))
        for j in range(count):
            t=birth+start+(j+.5)/count*(end-start)
            if t>2.82 or rng.random()>pulse(t-birth,start,end)*amplitude/1.2:continue
            rs={r['id']:r for r in regions(float(t),settings)};r=rs.get(rid)
            if r is None:continue
            offset=rng.normal(size=3);offset/=max(np.linalg.norm(offset),1e-12)
            offset*=rng.random()**(1/3)*.75*r['radii']
            rest=r['center']+offset;P=motion.world(rest,r['state']);V=emit_velocity(rest[None,:],r,t,settings)[0]
            out.append(dict(id=rid*10000+j,region=rid,parent=pid,birth=float(t),life=float(rng.uniform(.65,1.45)),
                            rest=rest.tolist(),P=P.tolist(),v=V.tolist(),radius=float(rng.uniform(.0021,.0044)*settings.grain_size),
                            drag=float(rng.uniform(2.,5.)),gravity=float(rng.uniform(.16,.36)),shade=float(rng.uniform(.36,.64))))
    return sorted(out,key=lambda x:(x['birth'],x['id']))
