"""Build package-relative Houdini projects using the included assets and rules.
Run with hython. Existing projects are preserved unless --replace is supplied.
"""
from pathlib import Path
import subprocess,sys,json
import hou,numpy as np
ROOT=Path(__file__).resolve().parents[1]
BOOT='''import sys,hou,importlib
from pathlib import Path
source=Path(hou.text.expandString('$HIP/../Source')).resolve()
modules=['portable_io','local_deformation','wave_motion','landing_motion','body_density','offline_emission','dynamic_dust','particle_fields','houdini_geometry','houdini_volume_io','project_geometry']
if any(k in sys.modules and Path(sys.modules[k].__file__).resolve().parent!=source for k in modules):
    for k in modules:sys.modules.pop(k,None)
sys.path.insert(0,str(source)) if str(source) not in sys.path else None
import project_geometry as geometry
g=hou.pwd().geometry();g.clear();t=(hou.frame()-1)/30
'''
def shaders():
    folder=ROOT/'Source/Shaders';vcc=str(Path(hou.text.expandString('$HFS'))/'bin/vcc')
    for source,label in [('CartoonSurface','CartoonSurface'),('LandingSurface','LandingSurface'),('DustVolume','DustVolume'),('Backdrop','Backdrop'),('GrainSurface','GrainSurface')]:
        path=folder/(source+'.hda')
        if not path.exists():subprocess.run([vcc,'-l',path.name,source+'.vfl'],cwd=folder,check=True,capture_output=True)
        hou.hda.installFile(str(path));d=hou.hda.definitionsInFile(str(path))[0];name=d.nodeTypeName();d.copyToHDAFile('Embedded');hou.hda.installFile('Embedded');hou.hda.uninstallFile(str(path));hou.node('/shop').createNode(name,label)
def python_object(name,code,material=None):
    obj=hou.node('/obj').createNode('geo',name);n=obj.createNode('python','Evaluate');n.parm('python').set(BOOT+code);n.setDisplayFlag(True);n.setRenderFlag(True)
    if material:obj.parm('shop_materialpath').set('/shop/'+material)
    return obj
def camera(name='MainCamera',width=5.4,res=(640,480),center=(.88,.6,0)):
    from houdini_geometry import MAIN
    cam=hou.node('/obj').createNode('cam',name);pos=MAIN[2]*10+center
    cam.setWorldTransform(hou.Matrix4(tuple(MAIN[0])+(0.,)+tuple(MAIN[1])+(0.,)+tuple(MAIN[2])+(0.,)+tuple(pos)+(1.,)));cam.parm('projection').set('ortho');cam.parm('orthowidth').set(width);cam.parmTuple('res').set(res);return cam
def rop(objects,cam):
    r=hou.node('/out').createNode('ifd','Render');r.parm('camera').set(cam.path());r.parm('vobject').set('');r.parm('forceobject').set(' '.join(objects));r.parm('alights').set('');r.parm('soho_autoheadlight').set(False);r.parmTuple('vm_samples').set((3,3));r.parm('vm_renderengine').set('raytrace');r.parm('vm_volumesteprate').set(.5);r.parm('vm_picture').set('$HIP/../Previews/Render_$F4.exr');return r
def save(name,frame=46,end=121):
    hou.setFps(30);hou.playbar.setFrameRange(1,end);hou.playbar.setPlaybackRange(1,end);hou.setFrame(frame)
    path=ROOT/'Houdini'/name
    if path.exists():
        if '--replace' not in sys.argv:raise FileExistsError(path)
        path.unlink()
    hou.hipFile.setName(str(path));hou.hipFile.save()
    from project_archive import make_relative
    make_relative(path);print('SAVED',path.name,flush=True)
def cartoon(effect):
    hou.hipFile.clear(suppress_save_prompt=True);shaders();name='RollingWave' if effect=='wave' else 'Landing'
    python_object(name,f'g.merge(geometry.cartoon(t,"{effect}"))','CartoonSurface' if effect=='wave' else 'LandingSurface')
    python_object('Backdrop','g.merge(geometry.backdrop())');cam=camera(center=(.88,.6,0) if effect=='wave' else (0,.30,0),width=4.8)
    rop([name,'Backdrop'],cam);save(name+'.hip',46 if effect=='wave' else 13,121 if effect=='wave' else 37)
def native_pyro():
    import offline_emission as emission
    root=hou.node('/obj').createNode('geo','FineDustSimulation');pt=root.parmTemplateGroup()
    for key,value in emission.asdict(emission.Settings()).items():
        if key in ['seed']:pt.append(hou.IntParmTemplate(key,key.replace('_',' ').title(),1,default_value=(value,)))
        else:pt.append(hou.FloatParmTemplate(key,key.replace('_',' ').title(),1,default_value=(value,)))
    root.setParmTemplateGroup(pt)
    src=root.createNode('python','MovingEmission');src.parm('python').set(BOOT+'''import offline_emission as model
p=hou.pwd().parent();settings=model.Settings(**{k:p.evalParm(k) for k in model.asdict(model.Settings())})
f=model.source_fields(float(t),settings);geometry.emission_volumes(g,f)
''')
    p=root.createNode('pyrosolver','SimulateFineDust');p.setInput(0,src)
    cfg=dict(divsize=.04,enable_buoyancy=False,enable_turbulence=True,turbulence_swirlsize=.4,turbulence_levels=1,turbulence_usecontrol=False,turbulence_influencefield='density',enable_disturbance=True,disturbance=.015,disturbance_blocksize=.16,disturbance_rough=.2,closeystatus='below',closeyat=0.,source_activate2=False,source_activate3=False,source_activate4=False,addflamefield=False,cachemaxsize=1600,resize_padding=.48,minimumsubsteps=2,substeps=3,dissipation_clampbelow=.00005)
    for key,value in cfg.items():p.parm(key).set(value)
    for key in ['turbulence','dissipation']:p.parm(key).setExpression('ch("../'+key+'")')
    p.allowEditingOfContents();forces=p.node('dopnet1/FORCES');w=forces.createNode('gasfieldwrangle','BirthMomentumAndLocalGuide');w.parm('snippet').set((ROOT/'Source/Shaders/LocalCoupling.vfl').read_text());pt=w.parmTemplateGroup();pt.append(hou.FloatParmTemplate('strength','Following rate per second',1,default_value=(5.,)));w.setParmTemplateGroup(pt);w.parm('strength').setExpression('ch("/obj/FineDustSimulation/guide_strength")');forces.node('sources_output').setInput(0,w)
    for i in [2,3,5]:p.parm('export_enable'+str(i)).set(False)
    portable=root.createNode('python','ReadPortableFields');portable.parm('python').set(BOOT+'g.merge(geometry.portable_fields(t))');portable.setDisplayFlag(True);portable.setRenderFlag(True);root.setDisplayFlag(False);root.layoutChildren()
    root.setComment('Default rendering reads portable reference fields. SimulateFineDust recooks the secondary simulation from frame1; body motion receives no feedback. Export through export_offline.py to a new destination.')
def combined():
    hou.hipFile.clear(suppress_save_prompt=True);shaders();native_pyro()
    python_object('DustVolumes','g.merge(geometry.offline_volumes(t))','DustVolume');python_object('VisibleGrains','g.merge(geometry.offline_grains(t))','GrainSurface');python_object('Backdrop','g.merge(geometry.backdrop())')
    cam=camera(width=4.8);rop(['DustVolumes','VisibleGrains','Backdrop'],cam);save('CombinedDust.hip')
def dynamic():
    hou.hipFile.clear(suppress_save_prompt=True);shaders()
    controls=hou.node('/obj').createNode('null','ParticleControls');pt=controls.parmTemplateGroup()
    for key,value in [('cloud_rate_per_region_s',23.),('grain_rate_per_region_s',16.),('guide_rate_per_s',4.),('wind_scale',1.),('concentration',.62)]:pt.append(hou.FloatParmTemplate(key,key.replace('_',' ').title(),1,default_value=(value,)))
    controls.setParmTemplateGroup(pt)
    for i,scenario in enumerate(['normal','wind_turn']):
        python_object('CloudVolumes'+str(i),f'g.merge(geometry.dynamic_volumes(t,"{scenario}",{i}))','DustVolume')
    python_object('GrainsAndBackdrop','g.merge(geometry.dynamic_context(t))')
    cam=hou.node('/obj').createNode('cam','ComparisonCamera');cam.parmTuple('t').set((0,0,30));cam.parm('projection').set('ortho');cam.parm('orthowidth').set(10.8);cam.parmTuple('res').set((1280,480))
    rop(['CloudVolumes0','CloudVolumes1','GrainsAndBackdrop'],cam);save('DynamicDust.hip')
def cloud_library():
    hou.hipFile.clear(suppress_save_prompt=True);shaders();python_object('SmallCloudVariants','g.merge(geometry.cloud_library())','DustVolume');cam=camera(width=3.6,center=(0,.3,0),res=(960,640));rop(['SmallCloudVariants'],cam);save('SmallCloudLibrary.hip',1)
if __name__=='__main__':
    if ROOT.name=='Cartoon':cartoon('wave');cartoon('landing')
    else:combined();dynamic();cloud_library()
