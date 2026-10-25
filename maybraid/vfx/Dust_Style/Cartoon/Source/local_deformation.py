"""Shared volumetric map and reference organization; no engine dependencies."""
import numpy as np
DURATION=3.
# Four coefficients: front/rear vertical redistribution, rolling shear, depth swell, side shear.
KEYS=[[(0,0,0,0,0),(.35,.28,-.12,.04,-.03),(1.1,.18,.42,.18,.10),(2.,-.12,.31,-.08,-.06),(3.,.04,.22,.06,.02)],[(0,0,0,0,0),(.5,-.10,-.08,.12,.06),(1.45,.30,.38,-.05,-.10),(2.3,.12,.45,.16,.04),(3.,.03,.27,.06,.01)],[(0,0,0,0,0),(.25,.18,-.08,-.08,.04),(.85,.32,.32,.16,-.08),(1.8,-.08,.46,.08,.10),(3.,.02,.24,.04,.01)]]
def controls(t,clip=0):
 k=np.array(KEYS[clip]);t=np.clip(t,0,3);i=min(np.searchsorted(k[:,0],t,side='right')-1,len(k)-2);u=(t-k[i,0])/(k[i+1,0]-k[i,0]);s=u*u*(3-2*u);return k[i,1:]*(1-s)+k[i+1,1:]*s

def forward(p,c):
 p=np.asarray(p,dtype='f8');x,y,z=np.moveaxis(p,-1,0);a,b,g,d=c
 f=np.exp(-((x-.40)/.65)**2-(z/.85)**2)-.6*np.exp(-((x+.55)/.60)**2-(z/.85)**2)
 Y=y*np.exp(a*f);h=Y*np.exp(-.5*(Y/1.25)**2);q=np.exp(-((Y-.75)/.65)**2)
 return np.stack([x+b*h,Y,z*np.exp(g*q)+d*h],axis=-1)
def inverse(p,c):
 p=np.asarray(p,dtype='f8');X,Y,Z=np.moveaxis(p,-1,0);a,b,g,d=c;h=Y*np.exp(-.5*(Y/1.25)**2);q=np.exp(-((Y-.75)/.65)**2);x=X-b*h;z=(Z-d*h)/np.exp(g*q);f=np.exp(-((x-.40)/.65)**2-(z/.85)**2)-.6*np.exp(-((x+.55)/.60)**2-(z/.85)**2)
 return np.stack([x,Y/np.exp(a*f),z],axis=-1)
def jacobian(p,c,eps=.0005):
 return np.stack([(forward(p+np.eye(3)[j]*eps,c)-forward(p-np.eye(3)[j]*eps,c))/(2*eps) for j in range(3)],axis=-1)
def normals(p,n,c):
 J=jacobian(p,c);out=np.linalg.solve(np.swapaxes(J,-1,-2),np.asarray(n)[...,None])[...,0];return out/np.linalg.norm(out,axis=-1,keepdims=True)
def S(x):x=np.clip(x,0,1);return x*x*(3-2*x)
def pose(t,i,scenario=0,speed=1.):
 group=i//4;lane=i%4;birth=group*.40+lane*.055;tau=t-birth
 if tau<0:return None
 angle=(lane-1.5)*.28;direction=np.array([np.cos(angle),0,np.sin(angle)]);r0=.22+group*.12
 # A common decelerating front; later groups have greater catch-up velocity.
 distance=speed*2.25*(1-np.exp(-tau/(.52+group*.06)))+group*.14*S(tau/1.2)
 lateral=(lane-1.5)*.46+.13*np.sin(tau*1.3+group*.65)*S(tau/.4)
 position=np.array([r0+distance-1.4,.025+.24*np.sin(np.pi*np.clip(tau/2.3,0,1))**2,lateral])
 # Neighboring lanes share rolling phase; front/back order changes as cohorts catch up.
 position[0]+=.24*np.sin(1.9*tau+group*.85)*S(tau/.35);position[1]+=.14*(.5+.5*np.sin(1.9*tau+group*.85))*S(tau/.5)
 yaw=angle
 if scenario:
  theta=.65*S((t-1.1)/1.6);R=np.array([[np.cos(theta),0,-np.sin(theta)],[0,1,0],[np.sin(theta),0,np.cos(theta)]]);position=position@R.T;yaw+=theta
  weight=np.exp(-((position[0]-.45)**2+(position[2]-.1)**2)/1.2**2);position[2]+=.65*weight*S((t-.9)/1.2);position[0]+=.18*weight*S((t-.9)/1.2)
 scale=np.array([.64,.54,.63])*(.78+.12*np.cos(i*.73))*(.12+.88*S(tau/.36));phase=np.clip(tau*(.73+.08*lane)+group*.16,0,3)
 return dict(position=position,yaw=yaw,scale=scale,phase=phase,clip=(lane+group)%3,variant=i%3,birth=birth)
