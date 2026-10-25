"""Public binary readers. No Houdini; volume/particle decoding uses stdlib only.
NumPy is imported only by read_glb and the optional array adapter.
"""
from pathlib import Path
import array,gzip,json,struct,sys,math
PARTICLE=struct.Struct('<IIHH2f3f3f3ff4ff3fI')
PARTICLE_LAYOUT=dict(stride_bytes=96,byte_order='little',header_bytes=0,
 fields=[dict(name=n,type=t,offset_bytes=o,count=c) for n,t,o,c in [
 ('id','uint32',0,1),('parent','uint32',4,1),('kind','uint16',8,1),('variant','uint16',10,1),
 ('birth_s','float32',12,1),('lifetime_s','float32',16,1),('position_m','float32',20,3),('velocity_m_s','float32',32,3),
 ('scale_m','float32',44,3),('concentration','float32',56,1),('quaternion_xyzw','float32',60,4),('age_s','float32',76,1),('color_linear_rgb','float32',80,3),('flags','uint32',92,1)]],
 kinds={'0':'small cloud','1':'visible grain'},quaternion='unit xyzw; active local-to-world rotation; scale before rotation',
 frames='Index JSON records byte_offset,count,time_s. No padding between records. Rows sorted by stable ID. Outside the recorded time interval is an error.')
def read_bytes(path):
    path=Path(path)
    return gzip.decompress(path.read_bytes()) if path.suffix=='.gz' else path.read_bytes()
def decode_scalars(data,encoding):
    if encoding=='float32':
        a=array.array('f');a.frombytes(data)
        if sys.byteorder!='little':a.byteswap()
        return a
    if encoding=='float16':return [v[0] for v in struct.iter_unpack('<e',data)]
    raise ValueError(encoding)
def inverse3(a):
    x,y,z=a;det=x[0]*(y[1]*z[2]-y[2]*z[1])-x[1]*(y[0]*z[2]-y[2]*z[0])+x[2]*(y[0]*z[1]-y[1]*z[0])
    if abs(det)<1e-20:raise ValueError('Singular sampling transform')
    return [[(y[1]*z[2]-y[2]*z[1])/det,(x[2]*z[1]-x[1]*z[2])/det,(x[1]*y[2]-x[2]*y[1])/det],
            [(y[2]*z[0]-y[0]*z[2])/det,(x[0]*z[2]-x[2]*z[0])/det,(x[2]*y[0]-x[0]*y[2])/det],
            [(y[0]*z[1]-y[1]*z[0])/det,(x[1]*z[0]-x[0]*z[1])/det,(x[0]*y[1]-x[1]*y[0])/det]]
class Volume:
    def __init__(self,metadata,base):
        self.meta=metadata;self.dims=metadata['dimensions_xyz'];self.data=decode_scalars(read_bytes(Path(base)/metadata['file']),metadata['scalar_type'])
        assert len(self.data)==math.prod(self.dims)
        m=metadata['index_to_world_m'];self.origin=[r[3] for r in m];self.inv=inverse3([r[:3] for r in m])
    def sample(self,point):
        p=[point[i]-self.origin[i] for i in range(3)];q=[sum(r[j]*p[j] for j in range(3)) for r in self.inv]
        start=[math.floor(x) for x in q];f=[q[i]-start[i] for i in range(3)];nx,ny,nz=self.dims;out=0.
        for z in (0,1):
            for y in (0,1):
                for x in (0,1):
                    ix,iy,iz=[start[i]+v for i,v in enumerate((x,y,z))]
                    if 0<=ix<nx and 0<=iy<ny and 0<=iz<nz:
                        out+=self.data[ix+nx*(iy+ny*iz)]*(f[0] if x else 1-f[0])*(f[1] if y else 1-f[1])*(f[2] if z else 1-f[2])
        return out
def volume_array(metadata,base):
    import numpy as np
    b=read_bytes(Path(base)/metadata['file']);dtype='<f2' if metadata['scalar_type']=='float16' else '<f4'
    return np.frombuffer(b,dtype=dtype).astype('f4').reshape(tuple(metadata['dimensions_xyz'][::-1]))
def sample_array(points,a,meta):
    import numpy as np
    m=np.asarray(meta['index_to_world_m']);q=(np.asarray(points)-m[:,3])@np.linalg.inv(m[:,:3]).T
    i=np.floor(q).astype('i8');f=q-i;dims=np.array(meta['dimensions_xyz']);out=np.zeros(len(q))
    for z in (0,1):
        for y in (0,1):
            for x in (0,1):
                j=i+[x,y,z];v=np.all((j>=0)&(j<dims),axis=1)
                out[v]+=a[j[v,2],j[v,1],j[v,0]]*(f[v,0] if x else 1-f[v,0])*(f[v,1] if y else 1-f[v,1])*(f[v,2] if z else 1-f[v,2])
    return out
def read_glb(path):
    import numpy as np
    data=Path(path).read_bytes();magic,version,total=struct.unpack_from('<III',data);assert magic==0x46546c67 and version==2 and total==len(data)
    n,kind=struct.unpack_from('<II',data,12);doc=json.loads(data[20:20+n]);offset=20+n;size,kind=struct.unpack_from('<II',data,offset);blob=data[offset+8:offset+8+size]
    primitive=doc['meshes'][0]['primitives'][0]
    def accessor(index):
        a=doc['accessors'][index];v=doc['bufferViews'][a['bufferView']];dtype={5126:'<f4',5125:'<u4',5123:'<u2'}[a['componentType']];count={'SCALAR':1,'VEC3':3}[a['type']]
        return np.frombuffer(blob,dtype=dtype,count=a['count']*count,offset=v.get('byteOffset',0)+a.get('byteOffset',0)).copy().reshape(a['count'],count)
    return accessor(primitive['attributes']['POSITION']),accessor(primitive['attributes']['NORMAL']),accessor(primitive['indices']).reshape(-1,3)
def quaternion_matrix(q):
    import numpy as np
    x,y,z,w=q
    return np.array([[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],[2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],[2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]])
def particle_tuple(p):
    return (p['id'],p['parent'],p['kind'],p['variant'],p['birth'],p['life'],*p['P'],*p['v'],*p['scale'],p['concentration'],*p['q'],p['age'],*p['color'],p.get('flags',0))
def unpack_particle(b,offset=0):
    v=PARTICLE.unpack_from(b,offset)
    return dict(id=v[0],parent=v[1],kind=v[2],variant=v[3],birth=v[4],life=v[5],P=list(v[6:9]),v=list(v[9:12]),scale=list(v[12:15]),concentration=v[15],q=list(v[16:20]),age=v[20],color=list(v[21:24]),flags=v[24])
def write_particles(path,frames,extra=None):
    path=Path(path);index=[];offset=0
    with path.open('wb') as out:
        for time,particles in frames:
            rows=sorted(particles,key=lambda p:p['id']);index.append(dict(time_s=float(time),byte_offset=offset,count=len(rows)))
            for p in rows:out.write(PARTICLE.pack(*particle_tuple(p)));offset+=PARTICLE.size
    meta=dict(schema='maybraid.particles.v1',file=path.name,layout=PARTICLE_LAYOUT,frames=index,**(extra or {}))
    path.with_suffix('.json').write_text(json.dumps(meta,indent=2)+'\n');return meta
def read_particle_frame(path,index,frame):
    path=Path(path);f=index['frames'][frame]
    with path.open('rb') as stream:stream.seek(f['byte_offset']);b=stream.read(f['count']*PARTICLE.size)
    assert len(b)==f['count']*PARTICLE.size
    return [unpack_particle(b,i*PARTICLE.size) for i in range(f['count'])]
