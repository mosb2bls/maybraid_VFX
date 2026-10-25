"""Portable engine-independent reference: rest 3D dissolve, bounded invertible warp, age defaults."""
import math
MASK=0xffffffff

def hash3(x,y,z,seed):
 h=((x&MASK)*0x8da6b343)^((y&MASK)*0xd8163841)^((z&MASK)*0xcb1ab31f)^(seed&MASK);h&=MASK
 h^=h>>16;h=(h*0x7feb352d)&MASK;h^=h>>15;h=(h*0x846ca68b)&MASK;h^=h>>16
 return (h&0xffffff)/16777215.0

def value_noise(p,seed):
 cell=[math.floor(v) for v in p];f=[p[i]-cell[i] for i in range(3)];f=[v*v*v*(v*(v*6-15)+10) for v in f];out=0.
 for z in range(2):
  for y in range(2):
   for x in range(2):out+=hash3(cell[0]+x,cell[1]+y,cell[2]+z,seed)*(f[0] if x else 1-f[0])*(f[1] if y else 1-f[1])*(f[2] if z else 1-f[2])
 return out

def dissolve_noise(rest_p,seed):return .7*value_noise([x*2.2 for x in rest_p],seed)+.3*value_noise([rest_p[i]*4.4+[19.3,-7.1,11.7][i] for i in range(3)],seed^0x9e3779b9)
def S(v):v=min(1,max(0,v));return v*v*(3-2*v)
def age_defaults(a):
 a=min(1,max(0,a));return dict(scale=.06+.84*S(a/.16)+.28*S((a-.16)/.64),outward_m=.08*S(a/.20)+.70*S((a-.05)/.80),upward_m=.04*S(a/.15)+.55*S((a-.10)/.85),dissolve_threshold=-.02+1.04*S((a-.65)/.33),remove=a>=1.)
def warp(p,kappa=.05,phase=math.pi/3,H=1.):
 x,y,z=p;u=math.pi*y/H;return (x+kappa*math.sin(u),y,z+.6*kappa*(math.sin(u+phase)-math.sin(phase)))
def inverse_warp(p,kappa=.05,phase=math.pi/3,H=1.):
 x,y,z=p;u=math.pi*y/H;return (x-kappa*math.sin(u),y,z-.6*kappa*(math.sin(u+phase)-math.sin(phase)))
def warp_normal(p,n,kappa=.05,phase=math.pi/3,H=1.):
 u=math.pi*p[1]/H;a=kappa*math.pi/H*math.cos(u);b=.6*kappa*math.pi/H*math.cos(u+phase);v=(n[0],n[1]-a*n[0]-b*n[2],n[2]);s=math.sqrt(sum(x*x for x in v));return tuple(x/s for x in v)
