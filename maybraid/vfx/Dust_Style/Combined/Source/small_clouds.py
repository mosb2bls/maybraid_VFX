"""Generate four static spatial dust elements. Python + NumPy, no Houdini."""
from pathlib import Path
import json,hashlib
import numpy as np
def generate(folder):
    folder=Path(folder);folder.mkdir(parents=True,exist_ok=True);n=64;h=2/n
    z,y,x=np.meshgrid((np.arange(n)+.5)*h-1,(np.arange(n)+.5)*h-1,(np.arange(n)+.5)*h-1,indexing='ij')
    def blob(c,r):return np.exp(-2.4*(((x-c[0])/r[0])**2+((y-c[1])/r[1])**2+((z-c[2])/r[2])**2))
    billow=blob((-.22,-.05,.12),(.65,.50,.54))+.8*blob((.30,.17,-.15),(.46,.57,.45))+.5*blob((.48,-.27,.25),(.34,.27,.33))
    curl=np.zeros_like(x)
    for j,theta in enumerate(np.linspace(-.65,3.7,10)):
        curl+=(.65+.25*np.cos(j*.7))*blob((.45*np.cos(theta),.38*np.sin(theta),.19*np.sin(theta*1.4)),(.24,.22,.29))
    trail=np.zeros_like(x)
    for j,s in enumerate(np.linspace(-.65,.63,7)):
        trail+=(.5+.45*j/6)*blob((s,.22*np.sin(s*3),.23*np.cos(s*3+.4)),(.29,.23,.27))
    cluster=.9*blob((-.38,.22,-.28),(.40,.37,.40))+.75*blob((.24,-.16,.29),(.42,.38,.39))+.8*blob((.33,.36,-.20),(.33,.29,.35))+.3*blob((-.10,-.38,-.22),(.48,.20,.25))
    border=np.minimum.reduce([1-np.abs(x),1-np.abs(y),1-np.abs(z)]);fade=np.clip((border-.08)/.14,0,1);fade=fade*fade*(3-2*fade)
    assets=[];errors=[]
    for i,(name,rho) in enumerate(zip(['Billow','Curl','Trail','Cluster'],[billow,curl,trail,cluster])):
        texture=.80+.11*np.sin(8*x+2*y+i)*np.sin(5*z-3*y)+.08*np.cos(9*z+4*x-i)
        a=(np.maximum(rho,0)*fade*texture);a/=a.max();a=a.astype('f4');half=a.astype('<f2');path=folder/(name+'.f16');path.write_bytes(half.tobytes())
        m=dict(id=i,name=name,file=path.name,dimensions_xyz=[n,n,n],scalar_type='float16',byte_order='little',channel_order='X fastest, then Y, then Z',index_to_world_m=[[h,0,0,-1+h/2],[0,h,0,-1+h/2],[0,0,h,-1+h/2]],coordinate='local element coordinates; particle scale in metres, quaternion local-to-world',outside=0.,range=[0.,1.],sha256=hashlib.sha256(path.read_bytes()).hexdigest(),bytes=path.stat().st_size)
        assets.append(m);errors.append(dict(id=i,max_quantization_error=float(np.max(np.abs(a-half.astype('f4')))),relative_integral_error=float(abs(a.sum()-half.astype('f4').sum())/a.sum()),occupied_voxels=int(np.count_nonzero(half>.02))))
    doc=dict(schema='maybraid.small-cloud-library.v1',representation='Four distinct static 3D density elements, not cards or body-shape copies',assets=assets,defaults=dict(extinction_per_m=3.,color_linear_rgb=[.65,.44,.25],radius_range_m=[.10,.19],lifetime_range_s=[1.0,1.8],base_concentration=.62),sampling='Trilinear linear scalar; outside0. Apply inverse particle quaternion and positive XYZ scale. Sum with other densities before shared cap/integration.',quantization=errors)
    (folder/'CloudLibrary.json').write_text(json.dumps(doc,indent=2)+'\n');return doc
if __name__=='__main__':generate(Path(__file__).resolve().parents[1]/'Assets/DynamicDust/SmallClouds')
