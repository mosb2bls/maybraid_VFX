"""Authoring-only bridge between Houdini volumes and the public scalar format."""
from pathlib import Path
import gzip,hashlib,json
import hou,numpy as np
def export_field(volume,path):
    path=Path(path);dims=np.array(volume.resolution());linear=np.array(volume.transform().asTuple()).reshape(3,3)
    center=np.array(volume.points()[0].position());axes=2*linear.T/dims[None,:];origin=center+(-1+1/dims)@linear
    matrix=np.column_stack([axes,origin]);raw=np.frombuffer(volume.allVoxelsAsString(),dtype='f4').astype('<f4').tobytes()
    assert len(raw)==int(np.prod(dims))*4
    with path.open('wb') as stream:
        with gzip.GzipFile(fileobj=stream,filename='',mode='wb',compresslevel=6,mtime=0) as gz:gz.write(raw)
    a=np.frombuffer(raw,'<f4');assert np.isfinite(a).all()
    return dict(file=path.name,dimensions_xyz=dims.tolist(),scalar_type='float32',byte_order='little',compression='gzip',channel_order='X fastest, then Y, then Z',index_to_world_m=matrix.tolist(),native_primitive_linear=linear.tolist(),native_primitive_center_m=center.tolist(),outside=0.,bytes=path.stat().st_size,uncompressed_bytes=len(raw),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),uncompressed_sha256=hashlib.sha256(raw).hexdigest(),minimum=float(a.min()),maximum=float(a.max()))
def native_volume(array,meta,name):
    dims=meta['dimensions_xyz'];m=np.array(meta['index_to_world_m']);out=hou.Geometry();out.addAttrib(hou.attribType.Prim,'name','')
    v=out.createVolume(*dims,hou.BoundingBox(0,0,0,*dims));v.setAttribValue('name',name);v.setAllVoxelsFromString(np.asarray(array,dtype='f4').tobytes())
    linear=m[:,:3];offset=m[:,3]-.5*linear.sum(1)
    xf=hou.Matrix4(tuple(linear[:,0])+(0.,)+tuple(linear[:,1])+(0.,)+tuple(linear[:,2])+(0.,)+tuple(offset)+(1.,));out.transform(xf)
    return out
