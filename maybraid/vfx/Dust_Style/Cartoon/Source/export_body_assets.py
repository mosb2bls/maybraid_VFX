"""Regenerate body meshes/rest densities from the editable BodyShapes project.
Run with hython --output NEW_FOLDER; delivered assets are never overwritten.
"""
from pathlib import Path
import argparse,json,hashlib
import hou,numpy as np
from mesh_export import arrays,write_glb
ROOT=Path(__file__).resolve().parents[1]
def main():
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);a=p.parse_args();folder=a.output.resolve();folder.mkdir(parents=True,exist_ok=False)
    hou.hipFile.load(str(ROOT/'Houdini/BodyShapes.hip'),suppress_save_prompt=True);records=[]
    for i,name in enumerate(['Shoulder','Column','Swept']):
        g=hou.node('/obj/'+name+'Shape');mesh=g.node('ExportMesh').geometry();P,N,T=arrays(mesh);write_glb(folder/(name+'.glb'),P,N,T)
        v=g.node('ExportDensity').geometry().prims()[0];rho=np.frombuffer(v.allVoxelsAsString(),np.float32).astype('<f2');rho.tofile(folder/(name+'Density.f16'))
        bb=v.boundingBox();dims=v.resolution();records.append(dict(id=i,name=name,mesh=name+'.glb',density=name+'Density.f16',grid=dict(dimensions_xyz=list(dims),bounds_min_m=list(bb.minvec()),bounds_max_m=list(bb.maxvec()),spacing_xyz_m=((np.array(bb.maxvec())-np.array(bb.minvec()))/dims).tolist())))
    (folder/'GeneratedAssets.json').write_text(json.dumps(dict(schema='maybraid.regenerated-body.v1',assets=records),indent=2)+'\n')
if __name__=='__main__':main()
