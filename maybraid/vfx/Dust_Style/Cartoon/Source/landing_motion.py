"""Portable one-shot landing emitter. Standard Python only; no Houdini/solver.

RH, +Y up, metres/seconds. All random values are stateless uint32 hashes.
Original GLB rest positions/normals/densities and body dissolve are reused.
"""
from dataclasses import dataclass, asdict
import math

MASK = 0xffffffff
TAU = 2*math.pi
SEEDS = (13, 37, 71)

@dataclass(frozen=True)
class Settings:
    seed: int = 13
    spread_radius: float = 1.25
    duration: float = 1.05
    puff_count: int = 16
    size_variation: float = .22
    timing_variation: float = 1.
    motion_strength: float = 1.

def mix32(x):
    x &= MASK;x ^= x>>16;x=(x*0x7feb352d)&MASK
    x ^= x>>15;x=(x*0x846ca68b)&MASK;x ^= x>>16
    return x

def random01(seed, identity, channel):
    return mix32((seed&MASK)^(((identity+1)*0x9e3779b9)&MASK)^(((channel+1)*0x85ebca6b)&MASK))/4294967296.

def S(x):
    x=max(0.,min(1.,x));return x*x*(3-2*x)

def spawn_table(settings=Settings()):
    c=settings
    if not 12<=c.puff_count<=16:raise ValueError('puff_count must be12..16, including two central and two curl puffs')
    if c.duration<=0 or c.spread_radius<=0:raise ValueError('Positive duration and spread radius required')
    if not 0<=c.size_variation<=.4 or not 0<=c.timing_variation<=1.5 or not .65<=c.motion_strength<=1.25:
        raise ValueError('Outside supported landing controls')
    r=lambda i,k:random01(c.seed,i,k)
    n=c.puff_count-4
    weights=[.70+.60*r(i,0) for i in range(n)]
    rotation=TAU*r(91,1);angles=[];angle=rotation
    for w in weights:
        angles.append(angle);angle+=TAU*w/sum(weights)
    out=[]
    for i in range(2):
        birth=i*.018*c.duration
        out.append(dict(id=i,kind='central',seed=mix32(c.seed^((i+1)*0x27d4eb2d)),
            variant=(i+int(3*r(i,2)))%3,birth=birth,lifetime=c.duration*(.255+.055*r(i,3)),
            angle=TAU*r(i,4),yaw=TAU*r(i,5),yaw_change=0.,angle_change=.03*(2*r(i,6)-1),
            start_radius=.025*r(i,7),end_radius=.075+.025*r(i,8),
            base_scale=[.095,.071,.092],dissolve_start=.31+.05*r(i,9),curl=0.,parent=None))
    for j,theta in enumerate(angles):
        i=2+j
        birth=c.duration*(.027+.108*(.5+c.timing_variation*(r(i,1)-.5)))
        life=min(c.duration*(.735+.175*r(i,2)),c.duration-birth)
        size=1+c.size_variation*(2*r(i,3)-1)
        out.append(dict(id=i,kind='ring',seed=mix32(c.seed^((i+1)*0x27d4eb2d)),
            variant=(j+int(2*r(i,4)))%3,birth=birth,lifetime=life,
            angle=theta,yaw=theta+math.pi*(.38+.22*r(i,5)),
            yaw_change=.075*(2*r(i,6)-1),angle_change=.055*(2*r(i,7)-1),
            start_radius=.105+.065*r(i,8),end_radius=c.spread_radius*(.62+.11*r(i,9)),
            base_scale=[.292*size*c.spread_radius/1.25,.146*(.86+.24*r(i,10)),.275*size*c.spread_radius/1.25],
            dissolve_start=.57+.09*r(i,11),curl=.008+.008*r(i,12),parent=None))
    # Two sparse, slightly later overlapping shoulders, attached to two widely
    # separated sectors rather than an evenly timed second ring or recursion.
    for k,j in enumerate([int(r(80,0)*n), (int(r(80,0)*n)+n//2+1)%n]):
        parent=out[j+2];i=n+2+k
        birth=c.duration*(.143+.036*r(i,1))
        out.append(dict(id=i,kind='curl',seed=mix32(c.seed^((i+1)*0x27d4eb2d)),
            variant=(parent['variant']+1)%3,birth=birth,lifetime=c.duration*(.64+.08*r(i,2)),
            angle=parent['angle']+.075*(2*r(i,3)-1),yaw=parent['yaw']+.20,
            yaw_change=.095*(2*r(i,4)-1),angle_change=.06*(2*r(i,5)-1),
            start_radius=.36+.03*r(i,6),end_radius=parent['end_radius']+.025,
            base_scale=[x*.72 for x in parent['base_scale']],
            dissolve_start=.56+.055*r(i,7),curl=.025+.010*r(i,8),parent=parent['id']))
    return sorted(out,key=lambda p:(p['birth'],p['id']))

def puff_state(p,time,settings=Settings()):
    age=time-p['birth']
    if age<0 or age>=p['lifetime']:return None
    a=age/p['lifetime'];u=max(0.,min(1.,a/.84))
    travel=1-(1-u)**3
    radius=p['start_radius']+(p['end_radius']-p['start_radius'])*travel*settings.motion_strength
    theta=p['angle']+p['angle_change']*S(a/.85)
    growth=.035+.965*S(a/(.29 if p['kind']=='central' else .245))
    # One-way nonuniform growth: gentle upward billowing, no rigid wheel tilt.
    scale=[x*growth for x in p['base_scale']]
    scale[1]*=.88+.12*S((a-.08)/.52)
    yaw=p['yaw']+p['yaw_change']*S(a/.75)
    # Rounded bottoms stay4mm above ground; larger curl is SHAPE height,
    # not a floating root. A small one-way y-scale increment does that work.
    scale[1]*=1+p['curl']*settings.motion_strength*S((a-.12)/.55)/.15
    threshold=-.02+1.04*S((a-p['dissolve_start'])/(.96-p['dissolve_start']))
    return dict(id=p['id'],kind=p['kind'],seed=p['seed'],variant=p['variant'],
        birth=p['birth'],lifetime=p['lifetime'],age_seconds=age,normalized_age=a,
        position=[math.cos(theta)*radius,.004,math.sin(theta)*radius],
        scale=scale,yaw=yaw,radius=radius,angle=theta,
        dissolve_threshold=threshold,deformation=0.,growth=growth,parent=p['parent'])

def states(time,settings=Settings()):
    return [s for p in spawn_table(settings) if (s:=puff_state(p,time,settings)) is not None]

def world(rest,state):
    x,y,z=[rest[k]*state['scale'][k] for k in range(3)];c=math.cos(state['yaw']);s=math.sin(state['yaw'])
    return [c*x-s*z+state['position'][0],y+state['position'][1],s*x+c*z+state['position'][2]]

def inverse_world(point,state):
    x,y,z=[point[k]-state['position'][k] for k in range(3)];c=math.cos(state['yaw']);s=math.sin(state['yaw'])
    return [(c*x+s*z)/state['scale'][0],y/state['scale'][1],(-s*x+c*z)/state['scale'][2]]

def spec_defaults():return asdict(Settings())

if __name__=='__main__':
    import argparse,json
    parser=argparse.ArgumentParser(description='Deterministic landing puff states; no Houdini dependency')
    parser.add_argument('--seed',type=int,default=13)
    parser.add_argument('--time',type=float,default=.4)
    parser.add_argument('--radius',type=float,default=1.25)
    parser.add_argument('--duration',type=float,default=1.05)
    parser.add_argument('--count',type=int,default=16)
    args=parser.parse_args()
    config=Settings(seed=args.seed,spread_radius=args.radius,duration=args.duration,puff_count=args.count)
    print(json.dumps(dict(settings=asdict(config),event_time=args.time,states=states(args.time,config)),indent=2))
