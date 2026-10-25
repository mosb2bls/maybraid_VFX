"""Indexed glTF2 GLB, same repository convention: metres, RH +Y, identity node."""
import numpy as np,json,struct

def arrays(geo):
 p=np.array([tuple(x.position()) for x in geo.points()],dtype='<f4');n=np.array([x.attribValue('N') for x in geo.points()],dtype='<f4');n/=np.linalg.norm(n,axis=1)[:,None];tri=[]
 for prim in geo.prims():
  ids=[v.point().number() for v in prim.vertices()]
  for j in range(1,len(ids)-1):tri.append([ids[0],ids[j],ids[j+1]])
 t=np.array(tri,dtype='<u4');cross=np.cross(p[t[:,1]]-p[t[:,0]],p[t[:,2]]-p[t[:,0]])
 # Native Houdini polygon winding is clockwise; glTF requires CCW exterior.
 if np.median(np.sum(cross*n[t].mean(1),axis=1))<0:t=t[:,[0,2,1]]
 return p,n,t

def write_glb(path,p,n,tri):
 indices=tri.astype('<u2') if len(p)<65536 else tri.astype('<u4');blob=bytearray();views=[];access=[]
 for a,ct,ty,target in [(p,5126,'VEC3',34962),(n,5126,'VEC3',34962),(indices.reshape(-1),5123 if indices.dtype.itemsize==2 else 5125,'SCALAR',34963)]:
  blob+=b'\0'*((-len(blob))%4);views.append(dict(buffer=0,byteOffset=len(blob),byteLength=a.nbytes,target=target));blob+=a.tobytes();ac=dict(bufferView=len(views)-1,componentType=ct,count=len(a),type=ty)
  if not access:ac.update(min=p.min(0).tolist(),max=p.max(0).tolist())
  access.append(ac)
 doc=dict(asset=dict(version='2.0',generator='Maybraid Houdini reusable lifecycle puffs'),scene=0,scenes=[dict(nodes=[0])],nodes=[dict(mesh=0,name=path.stem,extras=dict(units='metres',anchor=[0,0,0],rest_coordinates='Original POSITION before deformation, instance TRS and age growth'))],meshes=[dict(primitives=[dict(attributes=dict(POSITION=0,NORMAL=1),indices=2,material=0,mode=4)])],materials=[dict(name='Bevy_owned_neutral_placeholder',doubleSided=False,alphaMode='OPAQUE',pbrMetallicRoughness=dict(baseColorFactor=[.6,.6,.6,1],metallicFactor=0,roughnessFactor=1))],bufferViews=views,accessors=access,buffers=[dict(byteLength=len(blob))])
 js=json.dumps(doc,separators=(',',':')).encode();js+=b' '*((-len(js))%4);blob+=b'\0'*((-len(blob))%4);path.write_bytes(struct.pack('<III',0x46546c67,2,12+8+len(js)+8+len(blob))+struct.pack('<II',len(js),0x4e4f534a)+js+struct.pack('<II',len(blob),0x004e4942)+blob)
