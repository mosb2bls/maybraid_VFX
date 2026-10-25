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
def verify_pyro():
 folder=ROOT/'Assets/Reference/Pyro';doc=json.loads((folder/'Sequence.json').read_text());count=0
 assert len(doc['frames'])==121
 for i,f in enumerate(doc['frames']):
  assert f['time_s']==i/30
  for m in f['fields'].values():
   p=folder/m['file'];raw=gzip.decompress(p.read_bytes());assert sha(p)==m['sha256'] and hashlib.sha256(raw).hexdigest()==m['uncompressed_sha256']
   # Independent NumPy decode, separately from the stdlib runtime reader.
   a=np.frombuffer(raw,'<f4');assert len(a)==math.prod(m['dimensions_xyz']) and np.isfinite(a).all();assert float(a.min())==m['minimum'] and float(a.max())==m['maximum']
   M=np.array(m['index_to_world_m']);assert abs(np.linalg.det(M[:,:3]))>1e-12;count+=1
 checks=json.loads((ROOT/'Assets/Reference/NativeSampleChecks.json').read_text());error=0.
 for p in checks['probes']:
  m=doc['frames'][p['frame']]['fields'][p['field']];v=io.Volume(m,folder);error=max(error,max(abs(v.sample(w)-n) for w,n in zip(p['world_positions_m'],p['native_samples'])))
 assert error<2e-6
 from pyro_reference import Sequence
 seq=Sequence(folder/'Sequence.json');p=[1.,.4,0.];t=1.517
 for field in ['density','velocity_x','velocity_y','velocity_z']:
  x=seq.sample(p,t,field);q=t*30;i=math.floor(q);u=q-i;expected=(1-u)*io.Volume(doc['frames'][i]['fields'][field],folder).sample(p)+u*io.Volume(doc['frames'][i+1]['fields'][field],folder).sample(p);assert abs(x-expected)<1e-10
 try:seq.sample(p,4.1,'density');raise AssertionError('Out-of-range accepted')
 except ValueError:pass
 return dict(fields_decoded=count,native_sample_max_error=error,lossless_float32=True)
def verify_particles(path):
 doc=json.loads(path.with_suffix('.json').read_text());raw=path.read_bytes();last_end=0;identities={};max_live=[0,0]
 for f in doc['frames']:
  assert f['byte_offset']==last_end;last_end+=f['count']*96;ids=[];live=[0,0]
  for j in range(f['count']):
   v=struct.unpack_from('<IIHH2f3f3f3ff4ff3fI',raw,f['byte_offset']+j*96);assert all(math.isfinite(x) for x in v);identity=v[0];ids.append(identity);live[v[2]]+=1
   key=(v[1],v[2],v[3],v[4],v[5]);assert identity not in identities or identities[identity]==key;identities[identity]=key
   assert abs(sum(x*x for x in v[16:20])-1)<2e-6 and min(v[12:15])>0 and v[15]>=0
   assert -1e-5<=v[20]<v[5]+1e-5 and abs(v[20]-(f['time_s']-v[4]))<1e-5
  assert ids==sorted(set(ids));max_live=[max(a,b) for a,b in zip(max_live,live)]
 assert last_end==len(raw)
 return dict(frames=len(doc['frames']),stable_ids=len(identities),max_live_clouds_grains=max_live)
def verify_dynamic():
 import dynamic_dust as d,particle_fields as fields
 lib=fields.library();assert len(lib)==4
 for a,m in lib:
  assert sha(ROOT/'Assets/DynamicDust/SmallClouds'/m['file'])==m['sha256'];assert np.isfinite(a).all() and a.max()>.9 and min(a.min(),0)==0
  assert max(abs(a[0]).max(),abs(a[-1]).max(),abs(a[:,0]).max(),abs(a[:,-1]).max(),abs(a[:,:,0]).max(),abs(a[:,:,-1]).max())==0
 assert len({hashlib.sha256(a.tobytes()).hexdigest() for a,m in lib})==4
 outputs=[];birth_probes=[];max_fd=0.;lag=0.
 for scenario,name in [('normal','NormalMotion'),('wind_turn','WindAndTurn')]:
  frames,sim=d.simulate(scenario);path=ROOT/'Assets/DynamicDust'/(name+'.bin');idx=json.loads(path.with_suffix('.json').read_text())
  for k,(t,rows) in enumerate(frames):
   expected=b''.join(io.PARTICLE.pack(*io.particle_tuple(p)) for p in sorted(rows,key=lambda p:p['id']));f=idx['frames'][k]
   with path.open('rb') as stream:stream.seek(f['byte_offset']);assert stream.read(len(expected))==expected
  for b in sim.births[::31]:
   t=b['birth'];P=np.array(b['rest']);pid=b['parent'];h=.0001
   if t-d.state(t,pid,scenario)['birth']<2*h:continue
   W=lambda time:wave.world(P,d.state(time,pid,scenario));v=(-W(t+2*h)+8*W(t+h)-8*W(t-h)+W(t-2*h))/(12*h);error=float(abs(v-b['inherited_velocity']).max());max_fd=max(max_fd,error)
   birth_probes.append(dict(scenario=scenario,**b,independent_velocity_5point=v.tolist()))
  births={b['id']:b for b in sim.births}
  for p in frames[300][1]:
   if p['kind']==0 and p['age']>.5:
    b=births[p['id']];attached=wave.world(np.array(b['rest']),d.state(2.5,p['parent'],scenario));lag=max(lag,float(np.linalg.norm(np.array(p['P'])-attached)))
  sim.advance(1.);assert not sim.snapshot()
  outputs.append(verify_particles(path))
 a=d.Simulation();b=d.Simulation()
 for i in range(90):a.advance(1/30)
 for i in range(180):b.advance(1/60)
 assert a.snapshot()==b.snapshot() and a.tick==b.tick==360
 before=a.snapshot();a.advance(5.,paused=True);assert a.snapshot()==before and a.tick==360
 a.reset();assert a.tick==0 and not a.particles and a.accumulator==0
 # Prove dynamic composition remains valid when offline flow access is forbidden.
 fields.offline_raw=lambda *args:(_ for _ in ()).throw(AssertionError('Pyro sampled'))
 f=fields.dynamic_field(.5,'wind_turn',h=.06);assert np.isfinite(f['density']).all() and f['density'].max()>0
 assert max_fd<1e-4 and lag>.1
 return dict(max_birth_velocity_error_m_s=max_fd,released_cloud_max_separation_from_current_parent_m=lag,frame_partition_equal=True,pause_reset=True,no_pyro_access=True,scenarios=outputs,hash_probes=[dict(seed=9271,region=r,tick=k,channel=c,value=d.random01(9271,r,k,c)) for r,k,c in [(0,0,0),(3,120,5),(9,301,40)]],birth_probes=birth_probes,snapshot_probes=[dict(time_s=t,scenario='wind_turn',particles=rows[:3]) for t,rows in d.simulate('wind_turn')[0][::120]])
def main():
 p=argparse.ArgumentParser();p.add_argument('--output',type=Path);a=p.parse_args();result=dict(schema='maybraid.verification.v1',body=verify_body(),pyro=verify_pyro(),offline_particles=verify_particles(ROOT/'Assets/Reference/VisibleGrains.bin'),dynamic=verify_dynamic(),passed=True)
 if a.output:a.output.write_text(json.dumps(result,indent=2)+'\n')
 print(json.dumps({k:v for k,v in result.items() if k not in ['body','dynamic']},indent=2))
if __name__=='__main__':main()
