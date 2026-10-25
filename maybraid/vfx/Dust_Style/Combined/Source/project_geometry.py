"""Native project callbacks; all resources resolve within this category."""
from pathlib import Path
import json,hou,numpy as np
import houdini_geometry as geo
ROOT=Path(__file__).resolve().parents[1]
def cartoon(t,effect):return geo.surface(t,effect)
def backdrop():return geo.stage()
def grid_geometry(field):
    g=hou.Geometry();g.addAttrib(hou.attribType.Prim,'name','')
    for name in ['density','lightfield']:
        v=g.createVolume(*map(int,field['dims']),hou.BoundingBox(*field['lo'],*field['hi']));v.setAttribValue('name',name);v.setAllVoxelsFromString(field[name].astype('f4').tobytes())
    return g
def emission_volumes(g,f):
    g.addAttrib(hou.attribType.Prim,'name','')
    for name,a in [('density',f['density']),('guide',f['guide'])]+[(f'v.{k}',f['velocity'][...,j]) for j,k in enumerate('xyz')]+[(f'birthm.{k}',f['birth_momentum'][...,j]) for j,k in enumerate('xyz')]:
        v=g.createVolume(*map(int,f['dims']),hou.BoundingBox(*f['lo'],*f['hi']));v.setAttribValue('name',name);v.setAllVoxelsFromString(np.asarray(a,dtype='f4').tobytes())
def portable_fields(t):
    import particle_fields as fields
    from houdini_volume_io import native_volume
    out=hou.Geometry()
    for name,(a,meta) in fields.offline_raw(int(np.clip(round(t*30),0,120))).items():out.merge(native_volume(a,meta,{'density':'density','velocity_x':'vel.x','velocity_y':'vel.y','velocity_z':'vel.z'}[name]))
    return out
def offline_volumes(t):
    import particle_fields
    return grid_geometry(particle_fields.offline_field(float(t)))
def offline_grains(t):
    import particle_fields
    return geo.grains(particle_fields.rows(t,'offline'))
def config():
    import dynamic_dust
    p=hou.node('/obj/ParticleControls');names=['cloud_rate_per_region_s','grain_rate_per_region_s','guide_rate_per_s','wind_scale','concentration']
    return dynamic_dust.Config(**{n:p.evalParm(n) for n in names})
def transform(geometry,column):
    view=geo.MAIN;center=np.array([.88,.60,0]);shift=np.array([(column-.5)*5.4,0.,0.]);t=-center@view.T+shift
    geometry.transform(hou.Matrix4(tuple(view.T[0])+(0.,)+tuple(view.T[1])+(0.,)+tuple(view.T[2])+(0.,)+tuple(t)+(1.,)));return geometry
def dynamic_volumes(t,scenario,column):
    import particle_fields
    return transform(grid_geometry(particle_fields.dynamic_field(float(t),scenario,config=config())),column)
def dynamic_context(t):
    import particle_fields
    result=hou.Geometry();result.addAttrib(hou.attribType.Prim,'shop_materialpath','')
    for col,scenario in enumerate(['normal','wind_turn']):
        g=geo.grains(particle_fields.dynamic_field(float(t),scenario,config=config())['particles']);g.addAttrib(hou.attribType.Prim,'shop_materialpath','')
        for p in g.prims():p.setAttribValue('shop_materialpath','/shop/GrainSurface')
        result.merge(transform(g,col));result.merge(transform(geo.stage(width=5.4),col))
    return result
def cloud_library():
    import particle_fields,body_density
    result=hou.Geometry();result.addAttrib(hou.attribType.Prim,'name','')
    for i,(a,meta) in enumerate(particle_fields.library()):
        g=hou.Geometry();g.addAttrib(hou.attribType.Prim,'name','')
        for name,array in [('density',a*.62),('lightfield',body_density.transport_light(a*.62,2/64*.3))]:
            v=g.createVolume(64,64,64,hou.BoundingBox(-1,-1,-1,1,1,1));v.setAttribValue('name',name);v.setAllVoxelsFromString(array.astype('f4').tobytes())
        matrix=hou.hmath.buildScale((.30,.30,.30))*hou.hmath.buildTranslate(((i-1.5)*.72,.30,0));g.transform(matrix);result.merge(g)
    return result
