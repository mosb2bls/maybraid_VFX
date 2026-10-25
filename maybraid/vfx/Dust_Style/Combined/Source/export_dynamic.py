"""Generate runtime-approximation verification samples; Python + NumPy only."""
from pathlib import Path
import argparse,json
import dynamic_dust as dynamic
from portable_io import write_particles
def generate(folder):
    folder=Path(folder).resolve();folder.mkdir(parents=True,exist_ok=True)
    (folder/'DynamicRules.json').write_text(json.dumps(dynamic.rules(),indent=2)+'\n')
    for scenario,name in [('normal','NormalMotion'),('wind_turn','WindAndTurn')]:
        frames,sim=dynamic.simulate(scenario)
        write_particles(folder/(name+'.bin'),frames,dict(role='Runtime particle approximation verification samples',scenario=scenario,config=dynamic.asdict(sim.c)))
        (folder/(name+'Births.json')).write_text(json.dumps(sim.births,indent=2)+'\n')
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);a=p.parse_args();generate(a.output)
