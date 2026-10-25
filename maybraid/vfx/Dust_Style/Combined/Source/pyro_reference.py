"""Portable optional offline flow playback. Stdlib reader, no Houdini required."""
from pathlib import Path
from functools import lru_cache
import json,bisect
from portable_io import Volume
class Sequence:
    def __init__(self,manifest=None):
        self.path=Path(manifest) if manifest else Path(__file__).resolve().parents[1]/'Assets/Reference/Pyro/Sequence.json'
        self.meta=json.loads(self.path.read_text());self.times=[f['time_s'] for f in self.meta['frames']]
    @lru_cache(12)
    def volume(self,index,field):return Volume(self.meta['frames'][index]['fields'][field],self.path.parent)
    def sample(self,point,time,field='density'):
        if not self.times[0]<=time<=self.times[-1]:raise ValueError('Time outside recorded flow; choose an explicit tail policy')
        i=max(0,bisect.bisect_right(self.times,time)-1);j=min(len(self.times)-1,i+1)
        f=0 if i==j else (time-self.times[i])/(self.times[j]-self.times[i])
        a=self.volume(i,field).sample(point)
        return a if f==0 else a*(1-f)+self.volume(j,field).sample(point)*f
    def velocity(self,point,time):return [self.sample(point,time,'velocity_'+axis) for axis in 'xyz']
