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
    for source,label in [('CartoonSurface','CartoonSurface'),('LandingSurface','LandingSurface'),('Backdrop','Backdrop')]:
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

if __name__=='__main__':cartoon('wave');cartoon('landing')
