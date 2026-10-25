#ifndef MAYBRAID_LANDING_NOISE
#define MAYBRAID_LANDING_NOISE
float landing_hash(int x,y,z,seed)
{
 int h=(x*-1918454973)^(y*-669632447)^(z*-887442657)^seed;
 h^=shrz(h,16);h*=2146121005;
 h^=shrz(h,15);h*=-2073254261;
 h^=shrz(h,16);
 return float(h&16777215)/16777215.0;
}
float landing_value(vector p;int seed)
{
 int ix=int(floor(p.x)),iy=int(floor(p.y)),iz=int(floor(p.z));
 vector f=p-floor(p);f=f*f*f*(f*(f*6-15)+10);
 float v=0;
 for(int z=0;z<2;z++)for(int y=0;y<2;y++)for(int x=0;x<2;x++)
   v+=landing_hash(ix+x,iy+y,iz+z,seed)*(x?f.x:1-f.x)*(y?f.y:1-f.y)*(z?f.z:1-f.z);
 return v;
}
float landing_noise(vector p;int seed)
{
 return .7*landing_value(p*2.2,seed)+.3*landing_value(p*4.4+set(19.3,-7.1,11.7),seed^-1640531527);
}
#endif
