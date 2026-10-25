"""Native rendering helpers. Houdini-only; runtime rules are in separate modules."""
from pathlib import Path
from functools import lru_cache
import json,hou,numpy as np
import wave_motion as wave
import landing_motion as landing
from portable_io import read_glb
ROOT=Path(__file__).resolve().parents[1]
def basis(eye,target):
    f=np.asarray(target,float)-eye;f/=np.linalg.norm(f);r=np.cross(f,[0,1,0]);r/=np.linalg.norm(r);return np.array([r,np.cross(r,f),-f])
MAIN=basis([4.8,3.3,6.3],[0,.7,0]);SIDE=basis([0,1.,7.],[0,.7,0])
@lru_cache(None)
def mesh(variant):
    P,N,T=read_glb(ROOT/'Assets/Body'/(['Shoulder','Column','Swept'][variant]+'.glb'));g=hou.Geometry()
    for name,value in [('N',(0.,1.,0.)),('previewN',(0.,1.,0.)),('restpos',(0.,0.,0.)),('toon',1.),('style',1.),('puffseed',0),('cutoff',-.02),('Cd',(.6,.6,.6))]:g.addAttrib(hou.attribType.Point,name,value)
    points=[]
    for p,n in zip(P,N):
        pt=g.createPoint();pt.setPosition(tuple(map(float,p)));pt.setAttribValue('N',tuple(map(float,n)));pt.setAttribValue('restpos',tuple(map(float,p)));points.append(pt)
    for ids in T:
        poly=g.createPolygon()
        for i in ids[::-1]:poly.addVertex(points[int(i)])
    return g,P,N
def surface(time,effect='wave',settings=None):
    result=hou.Geometry()
    states=wave.instances(time,True,True) if effect=='wave' else landing.states(time,settings or landing.Settings())
    for s in states:
        original,P,N=mesh(s['variant']);g=hou.Geometry(original)
        if effect=='wave':W=wave.world(P,s);NN=wave.world_normals(P,N,s)
        else:
            R=wave.yaw_matrix(s['yaw']);W=(P*np.array(s['scale']))@R.T+s['position'];NN=(N/np.array(s['scale']))@R.T;NN/=np.linalg.norm(NN,axis=1)[:,None]
            seed=s['seed'] if s['seed']<2**31 else s['seed']-2**32;g.setPointIntAttribValues('puffseed',[seed]*len(P));g.setPointFloatAttribValues('cutoff',[s['dissolve_threshold']]*len(P))
        g.setPointFloatAttribValues('P',W.ravel().tolist());g.setPointFloatAttribValues('N',NN.ravel().tolist());g.setPointFloatAttribValues('previewN',NN.ravel().tolist());result.merge(g)
    return result
def stage(center=(.88,.6,0),width=8.4):
    g=hou.Geometry();g.addAttrib(hou.attribType.Point,'Cd',(.2,.28,.33));g.addAttrib(hou.attribType.Prim,'shop_materialpath','/shop/Backdrop')
    for ix in range(round(width/.6)):
        for iy in range(12):
            x=-width/2+ix*.6;y=-3.6+iy*.6;poly=g.createPolygon();poly.setAttribValue('shop_materialpath','/shop/Backdrop');color=(.2,.28,.33) if (ix+iy)%2 else (.34,.4,.43)
            for xx,yy in [(x,y),(x+.6,y),(x+.6,y+.6),(x,y+.6)]:
                p=g.createPoint();p.setPosition(tuple(np.array([xx,yy,-5])@MAIN+np.array(center)));p.setAttribValue('Cd',color);poly.addVertex(p)
    return g
def grains(rows):
    from portable_io import quaternion_matrix as qmatrix
    result=hou.Geometry();result.addAttrib(hou.attribType.Point,'Cd',(.3,.2,.1));result.addAttrib(hou.attribType.Point,'previewN',(0.,1.,0.))
    unit=np.array([[1,0,0],[-1,0,0],[0,1,0],[0,-1,0],[0,0,1],[0,0,-1]],float)
    for row in rows:
        if row['kind']!=1:continue
        rel=(unit*np.array(row['scale']))@qmatrix(row['q']).T;points=[]
        for p,n in zip(rel+row['P'],rel):
            pt=result.createPoint();pt.setPosition(tuple(p));pt.setAttribValue('Cd',tuple(row['color']));pt.setAttribValue('previewN',tuple(n/np.linalg.norm(n)));points.append(pt)
        for a in [0,1]:
            for b in [2,3]:
                for c in [4,5]:
                    poly=result.createPolygon()
                    for i in [a,b,c]:poly.addVertex(points[i])
    return result
