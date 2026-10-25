"""Read-only portable verification. Python + NumPy, no Houdini.
Use --output FILE to save compact port probes and measured checks.
"""
from pathlib import Path
import argparse,hashlib,json,gzip,struct,math
import numpy as np
import portable_io as io,local_deformation as deform,wave_motion as wave
ROOT=Path(__file__).resolve().parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def verify_body():
 doc=json.loads((ROOT/'Assets/Body/BodyAssets.json').read_text());probes=[];maximum=0.;normal_error=0.
 for a in doc['assets']:
  folder=ROOT/'Assets/Body';assert sha(folder/a['mesh'])==a['mesh_sha256'];assert sha(folder/a['density'])==a['density_sha256']
  p,n,tris=io.read_glb(folder/a['mesh']);assert np.isfinite(p).all() and np.isfinite(n).all() and tris.max()<len(p)
  d=np.frombuffer((folder/a['density']).read_bytes(),'<f2');assert len(d)==math.prod(a['grid']['dimensions_xyz']) and np.isfinite(d).all() and d.min()>=0
  for t in [.37,1.47,2.85]:
   state=wave.parent_state(t+1.,a['id']*3);P=p[::max(1,len(p)//19)].astype(float);N=n[::max(1,len(n)//19)].astype(float)
   world=wave.world(P,state);back=wave.rest_from_world(world,state);maximum=max(maximum,float(abs(back-P).max()))
   h=.0001;J=np.stack([(-wave.world(P+2*h*e,state)+8*wave.world(P+h*e,state)-8*wave.world(P-h*e,state)+wave.world(P-2*h*e,state))/(12*h) for e in np.eye(3)],-1)
   expected=np.linalg.solve(J.transpose(0,2,1),N[...,None])[...,0];expected/=np.linalg.norm(expected,axis=1)[:,None];actual=wave.world_normals(P,N,state);normal_error=max(normal_error,float(abs(expected-actual).max()))
   probes.append(dict(time_s=t+1.,parent=a['id']*3,rest_position_m=P[0].tolist(),rest_normal=N[0].tolist(),world_position_m=world[0].tolist(),world_normal=actual[0].tolist(),inverse_rest_m=back[0].tolist()))
 stream=np.frombuffer((ROOT/'Assets/Body/LocalDeformation.f32').read_bytes(),'<f4').reshape(3,145,4)
 for clip in range(3):assert np.max(abs(stream[clip]-[deform.controls(i/48,clip) for i in range(145)]))<3e-8
 assert maximum<1e-12 and normal_error<1e-5
 return dict(inverse_max_error_m=maximum,normals_max_component_error=normal_error,probes=probes)
def main():
 import landing_motion as landing
 p=argparse.ArgumentParser();p.add_argument('--output',type=Path);a=p.parse_args();probes=[]
 for t in [.1,.4,.85]:
  for state in landing.states(t):
   P=[.2,.7,-.3];W=landing.world(P,state);back=landing.inverse_world(W,state);assert max(abs(x-y) for x,y in zip(P,back))<1e-12
   probes.append(dict(time_s=t,id=state['id'],rest_position_m=P,world_position_m=W,inverse_rest_m=back))
 assert not landing.states(1.1)
 result=dict(schema='maybraid.verification.v1',body=verify_body(),landing_probes=probes,passed=True)
 if a.output:a.output.write_text(json.dumps(result,indent=2)+'\n')
 print('Cartoon portable checks passed')
if __name__=='__main__':main()
