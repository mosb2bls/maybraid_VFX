"""Recook the selected secondary Pyro authoring setup to portable fields.
Run with hython --output NEW_FOLDER. Native sampled-cache files are not needed.
"""
from pathlib import Path
import argparse,json,sys
import hou,numpy as np
from houdini_volume_io import export_field
ROOT=Path(__file__).resolve().parents[1]
def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);a=p.parse_args();folder=a.output.resolve();folder.mkdir(parents=True,exist_ok=False)
    hou.hipFile.load(str(ROOT/'Houdini/CombinedDust.hip'),suppress_save_prompt=True);solver=hou.node('/obj/FineDustSimulation/SimulateFineDust');solver.parm('resimulate').pressButton();frames=[]
    for i in range(121):
        hou.setFrame(i+1);solver.cook(force=True)
        if solver.errors():raise RuntimeError(solver.errors())
        native={v.attribValue('name'):v for v in solver.geometry().prims()};fields={}
        for name,key in [('density','density'),('velocity_x','vel.x'),('velocity_y','vel.y'),('velocity_z','vel.z')]:
            m=export_field(native[key],folder/f'Frame_{i:03}_{name}.f32.gz');m['units']='dimensionless density' if name=='density' else 'world metres per second';fields[name]=m
        frames.append(dict(index=i,time_s=i/30,fields=fields));print('EXPORTED',i,flush=True)
    template=json.loads((ROOT/'Assets/Reference/Pyro/Sequence.json').read_text());template['frames']=frames;template['bytes']=dict(raw=sum(m['uncompressed_bytes'] for f in frames for m in f['fields'].values()),gzip=sum(m['bytes'] for f in frames for m in f['fields'].values()));template.pop('native_comparison',None)
    (folder/'Sequence.json').write_text(json.dumps(template,indent=2)+'\n')
if __name__=='__main__':main()
