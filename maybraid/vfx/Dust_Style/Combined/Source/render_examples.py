"""Render the runtime-oriented particle examples with Houdini/Mantra.
Output goes to a caller-selected folder. No delivered beauty file is changed.
"""
from pathlib import Path
import argparse,subprocess,time
import hou
ROOT=Path(__file__).resolve().parents[1]
def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--scout',action='store_true');a=p.parse_args();a.output=a.output.resolve();a.output.mkdir(parents=True,exist_ok=True)
    hou.hipFile.load(str(ROOT/'Houdini/DynamicDust.hip'),suppress_save_prompt=True);rop=hou.node('/out/Render');rop.parmTuple('vm_samples').set((4,4));H=Path(hou.text.expandString('$HFS'))/'bin'
    for i in ([18] if a.scout else range(48)):
        hou.setFrame(1+i*2.5);path=a.output/f'Particles_{i:03}.exr'
        if path.exists():raise FileExistsError(path)
        rop.parm('vm_picture').set(str(path));start=time.time();rop.render()
        subprocess.run([str(H/'iconvert'),'--ocio','--iscolorspace','lin_rec709','--tocolorspace','srgb_texture',str(path),str(path.with_suffix('.png'))],check=True,capture_output=True)
        print('RENDERED',i,round(time.time()-start,2),flush=True)
if __name__=='__main__':main()
