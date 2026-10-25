"""Runtime-oriented small clouds and visible grains; NumPy, no Houdini/Pyro.

Fixed120Hz tick clock. Randomness is indexed by seed/region/tick/channel.
Retire, emit, integrate velocity ONCE, integrate position ONCE, update appearance.
Released world positions are never transformed when the body turns.
"""
from dataclasses import dataclass,asdict
from functools import lru_cache
import math
import numpy as np
import offline_emission as emit
import wave_motion as wave
S=emit.body.S
MASK=0xffffffff
def mix32(x):
    x&=MASK;x^=x>>16;x=(x*0x7feb352d)&MASK;x^=x>>15;x=(x*0x846ca68b)&MASK;x^=x>>16;return x
def random01(seed,region,tick,channel):
    return mix32(seed^mix32(region+0x9e3779b9)^mix32(tick+0x85ebca6b)^mix32(channel+0x27d4eb2d))/4294967296.
def qmul(a,b):
    x,y,z,w=a;X,Y,Z,W=b
    return np.array([w*X+x*W+y*Z-z*Y,w*Y-x*Z+y*W+z*X,w*Z+x*Y-y*X+z*W,w*W-x*X-y*Y-z*Z])
def qaxis(axis,angle):
    axis=np.array(axis,float);axis/=np.linalg.norm(axis);return np.r_[axis*np.sin(angle/2),np.cos(angle/2)]
def qmatrix(q):
    x,y,z,w=q
    return np.array([[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],[2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],[2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]])
@dataclass(frozen=True)
class Config:
    dt:float=1/120
    seed:int=9271
    cloud_rate_per_region_s:float=23.
    grain_rate_per_region_s:float=16.
    max_clouds:int=96
    max_grains:int=96
    guide_rate_per_s:float=4.
    guide_age_decay_s:float=.45
    guide_reach_m:float=.48
    concentration:float=.62
    wind_scale:float=1.
@lru_cache(2048)
def body_states(t,scenario='normal'):
    states=wave.instances(float(t),True,True)
    if scenario=='normal':return states
    if scenario!='wind_turn':raise ValueError(scenario)
    # Rigid yaw of the complete reference body, preserving all local clocks,
    # growth, supportY, births and late-roll rules. Beauty input is unaffected.
    angle=.65*float(S((t-1.15)/1.3));G=wave.yaw_matrix(angle);out=[]
    for s in states:
        out.append(dict(s,rotation=G@s['rotation'],position=G@(s['position']+s['pivot'])-s['pivot']))
    return out
def state(t,pid,scenario):return next(s for s in body_states(float(t),scenario) if s['id']==pid)
def material_velocity(rest,t,pid,scenario):
    birth=state(t,pid,scenario)['birth'];lo=max(birth,t-.0005);hi=t+.0005
    return (wave.world(rest,state(hi,pid,scenario))-wave.world(rest,state(lo,pid,scenario)))/(hi-lo)
def regions(t,scenario):
    if t>2.82:return []
    out=[]
    for r in emit.regions(float(t)):
        s=state(t,r['parent'],scenario);r=dict(r,state=s);r['P']=wave.world(r['center'],s)
        r['material_v']=material_velocity(r['center'],t,r['parent'],scenario);out.append(r)
    return out
def ambient(P,t,scenario,wind_scale=1.):
    P=np.asarray(P);wind=np.array([.18,0.,.02])
    if scenario=='wind_turn':wind=wind*(1-float(S((t-1.4)/.65)))+np.array([-.15,.03,.75])*float(S((t-1.4)/.65))
    # Coherent finite world-space rolling cells, shared by both particle kinds.
    out=np.broadcast_to(wind*wind_scale,P.shape).copy()
    for center,axis,radius,strength in [([.45,.45,.0],[0,0,-1],1.10,.70),([1.35,.65,-.35],[.15,1,.1],.85,.42)]:
        c=np.array(center)+[.08*np.sin(t*.8),0,.06*np.sin(t*.6)];d=P-c;w=np.maximum(0,1-np.sum(d*d,axis=-1)/radius**2)**2
        ax=np.array(axis,float);ax/=np.linalg.norm(ax);out+=np.cross(np.broadcast_to(ax,P.shape),d)*strength*w[...,None]
    return out
class Simulation:
    def __init__(self,scenario='normal',config=Config()):self.scenario=scenario;self.c=config;self.reset()
    def reset(self):self.tick=0;self.accumulator=0.;self.particles=[];self.dropped=[0,0];self.births=[]
    @property
    def time(self):return self.tick*self.c.dt
    def _spawn(self,r,kind,t):
        c=self.c;rand=lambda ch:random01(c.seed,r['id'],self.tick,ch+kind*40)
        identity=kind*(1<<30)+r['id']*100000+self.tick
        direction=np.array([2*rand(1)-1,2*rand(2)-1,2*rand(3)-1]);direction/=max(np.linalg.norm(direction),1e-12)
        rest=r['center']+direction*rand(4)**(1/3)*r['radii']*.7
        P=wave.world(rest,r['state']);inherited=material_velocity(rest,t,r['parent'],self.scenario)
        N=wave.world_normals(rest[None,:],r['normal'][None,:],r['state'])[0]
        release=.19*N+r['state']['rotation']@np.array([-.055,.075,0.])
        radius=(.10+.09*rand(5)) if kind==0 else (.0021+.0023*rand(5))
        life=(1.+.8*rand(6)) if kind==0 else (.65+.8*rand(6))
        q=qmul(qaxis([0,1,0],2*np.pi*rand(7)),qaxis([1,0,0],.7*(rand(8)-.5)))
        p=dict(id=identity,parent=r['parent'],region=r['id'],kind=kind,variant=int(rand(9)*4) if kind==0 else 0,birth=t,life=life,
               P=P,v=inherited+release,radius=radius,q=q,drag=(1.5+1.3*rand(10)) if kind==0 else (2.+3*rand(10)),gravity=0. if kind==0 else .16+.20*rand(11),
               base_concentration=c.concentration*(.8+.4*rand(12)),spin=.18+.35*rand(13),scale=np.repeat(radius,3),concentration=0.,age=0.,color=np.array([.65,.44,.25])*(1. if kind==0 else .36+.28*rand(14)),flags=0)
        self.births.append(dict(id=identity,parent=r['parent'],region=r['id'],kind=kind,birth=t,life=life,rest=rest.tolist(),position=P.tolist(),inherited_velocity=inherited.tolist(),release_velocity=release.tolist()))
        return p
    def step(self):
        c=self.c;t=self.time;dt=c.dt
        self.particles=[p for p in self.particles if t<p['birth']+p['life']]
        active=regions(t,self.scenario)
        for r in active:
            for kind,rate,cap in [(0,c.cloud_rate_per_region_s,c.max_clouds),(1,c.grain_rate_per_region_s,c.max_grains)]:
                if random01(c.seed,r['id'],self.tick,kind*40)>=rate*dt*r['gate']:continue
                if sum(p['kind']==kind for p in self.particles)>=cap:self.dropped[kind]+=1;continue
                self.particles.append(self._spawn(r,kind,t))
        if self.particles:
            P=np.array([p['P'] for p in self.particles]);V=np.array([p['v'] for p in self.particles]);flow=ambient(P,t+dt/2,self.scenario,c.wind_scale)
            weights=np.zeros(len(P));moment=np.zeros_like(P)
            for r in active:
                distance=np.linalg.norm(P-r['P'],axis=1);w=r['gate']*np.maximum(0,1-(distance/c.guide_reach_m)**2)**2
                weights+=w;moment+=w[:,None]*r['material_v']
            target_body=moment/np.maximum(weights[:,None],1e-12)
            for j,p in enumerate(self.particles):
                age=t-p['birth'];following=(1-np.exp(-weights[j]))*np.exp(-age/c.guide_age_decay_s)
                kbody=c.guide_rate_per_s*following;K=p['drag']+kbody
                target=(p['drag']*flow[j]+kbody*target_body[j]+np.array([0,-p['gravity'],0]))/K
                vnew=target+(V[j]-target)*np.exp(-K*dt);p['P']=P[j]+.5*(V[j]+vnew)*dt;p['v']=vnew
                age=t+dt-p['birth'];a=age/p['life'];p['age']=age
                if p['kind']==0:
                    grow=(.3+.7*float(S(age/.18)))*(1+.4*a);p['scale']=p['radius']*grow*np.array([1.12,.80,1.])
                    p['concentration']=p['base_concentration']*float(S(age/.12))*np.exp(-.55*age)*(1-float(S((a-.65)/.35)))
                    p['q']=qmul(qaxis([0,0,-1],p['spin']*dt),p['q']);p['q']/=np.linalg.norm(p['q'])
                    floor=p['scale'][1]
                else:p['scale']=p['radius']*np.array([1.,.66,1.23]);p['concentration']=1.;floor=p['radius']
                if p['P'][1]<floor:p['P'][1]=floor;p['v'][1]=max(0,-p['v'][1]*.12);p['v'][[0,2]]*=.8
        self.tick+=1
    def advance(self,seconds,paused=False):
        if paused:return
        if seconds<0:raise ValueError('Reset explicitly; negative time is not integration')
        self.accumulator+=seconds
        while self.accumulator+1e-12>=self.c.dt:self.step();self.accumulator-=self.c.dt
    def snapshot(self):
        keys=['id','parent','kind','variant','birth','life','P','v','scale','concentration','q','age','color','flags']
        return [{k:(p[k].tolist() if isinstance(p[k],np.ndarray) else p[k]) for k in keys} for p in self.particles if p['age']<p['life']]
def simulate(scenario='normal',duration=4.,config=Config()):
    sim=Simulation(scenario,config);frames=[(0.,[])];n=round(duration/config.dt)
    for i in range(n):sim.step();frames.append((sim.time,sim.snapshot()))
    return frames,sim
def rules():
    return dict(schema='maybraid.dynamic-dust.v1',config=asdict(Config()),space='metres, RH,+Y,world positions after release',order='retire; deterministic tick-based emission; evaluate shared analytic ambient flow and compact body guide; exact frozen-target velocity relaxation; trapezoidal position update once; age/size/concentration; ground',random='uint32 wrapping mix32; uniform=hash/2^32. Inputs seed,region,tick,channel; no frame/stream randomness',pool='Separate capacities; reject newest birth when full; never move or evict live particles. Stable ID=kind*2^30+region*100000+tick; reset on new event, combine with external event ID for overlapping events.',pause_reset='pause performs no steps and accrues no time; reset clears clock, particles, births and accumulator. Fixed1/120s steps; advance accumulates wall time without dropping steps. Caller owns catch-up budgeting.',scenarios=dict(normal='unchanged selected body; wind(.18,0,.02)m/s',wind_turn='diagnostic rigid whole-body yaw .65*S((t-1.15)/1.3); same births/local clocks/support/late rolls; wind smoothsteps at1.4..2.05s toward(-.15,.03,.75)m/s. Previously released particle positions are never transformed.'),following='guide kernels around current moving source anchors, blended parent material velocities, exp(-particleAge/.45) attenuation; zero outside .48m and after emission. Never reparent particles.',runtime='Python+NumPy reference, not Rust/WGSL. Analytic ambient/curl is a practical approximation, never samples the exported Pyro reference. Client supplies terrain, event IDs, wind/local influence fields, GPU rendering and scene-depth/volume composition.')
