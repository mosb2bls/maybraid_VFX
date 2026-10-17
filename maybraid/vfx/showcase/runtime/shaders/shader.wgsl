#import bevy_pbr::{mesh_view_bindings::view, view_transformations::{position_ndc_to_world,frag_coord_to_ndc}, prepass_utils::prepass_depth}
#import bevy_core_pipeline::tonemapping::tone_mapping
struct Playback {
 time:vec4<f32>, settings:vec4<f32>, influence:vec4<f32>, velocity:vec4<f32>,
 origins:array<vec4<f32>,10>, right:vec4<f32>, up:vec4<f32>, centre:vec4<f32>, experiment:vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> p:Playback;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var atlas:texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var atlas_sampler:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var density:texture_3d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var density_sampler:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var multiview:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var multiview_sampler:sampler;
struct Out {@builtin(position) position:vec4<f32>,}
@vertex fn vertex(@location(0) position:vec3<f32>)->Out {
 var out:Out;out.position=vec4(position.xy,0.5,1.0);return out;
}
fn atlas_frame(uv:vec2<f32>,frame:u32)->vec4<f32> {
 let tile=vec2<f32>(f32(frame%7u),f32(frame/7u));
 return textureSampleLevel(atlas,atlas_sampler,(tile*264.0+vec2(4.5)+clamp(uv,vec2(0.0),vec2(1.0))*255.0)/1848.0,0.0);
}
// Each sample is premultiplied in linear RGB before time or angular interpolation.
fn multiview_frame(uv:vec2<f32>,frame:u32,angle:u32)->vec4<f32> {
 let tile=vec2<f32>(f32(frame%7u),f32(frame/7u));
 let q=(tile*136.0+vec2(4.5)+clamp(uv,vec2(0.0),vec2(1.0))*127.0)/952.0;
 let a=textureSampleLevel(multiview,multiview_sampler,q,i32(angle),0.0);
 return vec4(a.rgb*a.a,a.a);
}
fn box_sdf(world:vec3<f32>,centre:vec3<f32>,half_size:vec3<f32>)->f32 {
 let q=abs(world-centre)-half_size;
 return length(max(q,vec3(0.0)))+min(max(q.x,max(q.y,q.z)),0.0);
}
fn boundary_mask(world:vec3<f32>)->f32 {
 // True zero-density exclusion at/inside the two fixed test boxes. This is NOT flow or a contact solver.
 if p.experiment.z<0.5 {return 1.0;}
 let wall=box_sdf(world,vec3(0.65,0.7,-0.3),vec3(0.15,0.7,1.25));
 let box=box_sdf(world,vec3(-1.0,0.4,-0.7),vec3(0.4));
 return smoothstep(0.0,0.08,min(wall,box))*smoothstep(0.0,0.025,world.y);
}
fn optical_coverage(alpha:f32)->f32 {
 return 1.0-pow(max(1.0-alpha,0.000001),p.experiment.y);
}
fn density_frame(uvw:vec3<f32>,frame:u32)->f32 {
 // Prevent filtering into the adjacent *animation* cell. Explicitly zero outside spatial bounds.
 if any(uvw<vec3(0.0)) || any(uvw>vec3(1.0)) {return 0.0;}
 let q=clamp(uvw,vec3(0.5/64.0,0.5/32.0,0.5/64.0),vec3(1.0-0.5/64.0,1.0-0.5/32.0,1.0-0.5/64.0));
 let tile=vec2<f32>(f32(frame%7u),f32(frame/7u));
 return textureSampleLevel(density,density_sampler,vec3((tile+q.xy)/7.0,q.z),0.0).r;
}
fn disturb(world:vec3<f32>)->vec4<f32> {
 let d=(world-p.influence.xyz)/p.velocity.w;
 let weight=exp(-dot(d,d)*2.5)*p.influence.w;
 // Smooth, stateless local clearing + <= 0.16m coordinate shift. No pileup or wake.
 return vec4(p.velocity.xyz*weight*0.055,1.0-0.58*weight);
}
fn rho(world:vec3<f32>)->f32 {
 let field=disturb(world);
 var sum=0.0;
 for(var g=0u;g<u32(p.settings.x);g+=1u) {
  let local=world-p.origins[g].xyz;
  let uvw=(local-field.xyz-vec3(-3.2,0.0,-3.2))/vec3(6.4,3.2,6.4);
  let a=density_frame(uvw,u32(p.time.x));let b=density_frame(uvw,u32(p.time.y));
  // Cap perturbed density by unperturbed density: distortion cannot create a density increase.
  let original_uvw=(local-vec3(-3.2,0.0,-3.2))/vec3(6.4,3.2,6.4);
  var value=mix(a,b,p.time.z);
  if p.influence.w>0.001 {value=min(value,mix(density_frame(original_uvw,u32(p.time.x)),density_frame(original_uvw,u32(p.time.y)),p.time.z));}
  sum+=value*field.w;
 }
 return sum*boundary_mask(world)*p.experiment.y;
}
fn hit_box(ro:vec3<f32>,rd:vec3<f32>,origin:vec3<f32>)->vec2<f32> {
 let inv=1.0/(select(vec3(0.000001),rd,abs(rd)>vec3(0.000001)));
 let a=(origin+vec3(-3.2,0.0,-3.2)-ro)*inv;
 let b=(origin+vec3(3.2,3.2,3.2)-ro)*inv;
 let lo=min(a,b);let hi=max(a,b);
 return vec2(max(max(lo.x,lo.y),lo.z),min(min(hi.x,hi.y),hi.z));
}
fn baked_view(ro:vec3<f32>,rd:vec3<f32>,limit:f32,origin:vec3<f32>,angle:u32)->vec4<f32> {
 // Reconstruct each authored orthographic plane in ITS OWN world basis, not the old B plane.
 let theta=f32(angle)*0.7853981633974483;
 let elevation=0.3490658503988659;
 let right=vec3(cos(theta),0.0,-sin(theta));
 let up=vec3(-sin(elevation)*sin(theta),cos(elevation),-sin(elevation)*cos(theta));
 let normal=cross(right,up);
 let centre=vec3(0.0,0.55,0.0)+origin;
 let denom=dot(rd,normal);
 if abs(denom)<0.00001 {return vec4(0.0);}
 let t=dot(centre-ro,normal)/denom;
 if t<=0.0 || t>=limit {return vec4(0.0);}
 let world=ro+rd*t;let field=disturb(world);
 let delta=world-centre;
 let uv=vec2(dot(delta,right)/7.8+0.5,0.5-dot(delta,up)/7.8);
 if any(uv<vec2(0.0)) || any(uv>vec2(1.0)) {return vec4(0.0);}
 let shift=vec2(dot(field.xyz,right),-dot(field.xyz,up))/7.8;
 let a=multiview_frame(uv-shift,u32(p.time.x),angle);
 let b=multiview_frame(uv-shift,u32(p.time.y),angle);
 let sample=mix(a,b,p.time.z);
 var coverage=sample.a;
 if p.influence.w>0.001 {
  let original=mix(multiview_frame(uv,u32(p.time.x),angle),multiview_frame(uv,u32(p.time.y),angle),p.time.z);
  coverage=min(coverage,original.a);
 }
 let alpha=optical_coverage(coverage)*smoothstep(0.0,0.18,limit-t)*field.w*boundary_mask(world);
 return vec4(sample.rgb/max(sample.a,0.000001)*alpha,alpha);
}
@fragment fn fragment(input:Out)->@location(0) vec4<f32> {
 if p.settings.w<0.5 {return vec4(0.0);}
 let ndc=frag_coord_to_ndc(input.position);
 let ro=view.world_position.xyz;
 let rd=normalize(position_ndc_to_world(vec3(ndc.xy,1.0))-ro);
 let opaque=prepass_depth(input.position,0u);
 var limit=1000.0;
 if opaque>0.0 {limit=max(0.0,dot(position_ndc_to_world(vec3(ndc.xy,opaque))-ro,rd));}
 var rgb=vec3(0.0);var trans=1.0;
 if p.time.w<0.5 {
  // A fixed capture plane per group, solved analytically. It remains a 2D representation.
  let normal=normalize(cross(p.right.xyz,p.up.xyz));
  let denom=dot(rd,normal);
  if abs(denom)<0.00001 {return vec4(0.0);}
  for(var g=0u;g<u32(p.settings.x);g+=1u) {
   let centre=p.centre.xyz+p.origins[g].xyz;
   let t=dot(centre-ro,normal)/denom;
   if t<=0.0 || t>=limit {continue;}
   let world=ro+rd*t;let delta=world-centre;
   let uv=vec2(dot(delta,p.right.xyz)/7.8+0.5,0.5-dot(delta,p.up.xyz)/7.8);
   if any(uv<vec2(0.0)) || any(uv>vec2(1.0)) {continue;}
   let a=atlas_frame(uv,u32(p.time.x));let b=atlas_frame(uv,u32(p.time.y));
   // Premultiplied interpolation then straight-alpha output: no color fringes at frame fades.
   let coverage=mix(a.a,b.a,p.time.z);
   let color=mix(a.rgb*a.a,b.rgb*b.a,p.time.z)/max(coverage,0.000001);
   let alpha=optical_coverage(coverage)*boundary_mask(world)*smoothstep(0.0,0.18,limit-t)*disturb(world).w;
   rgb+=trans*alpha*color;trans*=1.0-alpha;
  }
 } else if p.time.w>1.5 {
  for(var g=0u;g<u32(p.settings.x);g+=1u) {
   let offset=ro-(vec3(0.0,0.55,0.0)+p.origins[g].xyz);
   let theta=atan2(offset.x,offset.z);
   let angle=(theta/6.283185307179586-floor(theta/6.283185307179586))*8.0;
   var i=u32(floor(angle))%8u;var j=(i+1u)%8u;var blend=fract(angle);
   blend=blend*blend*(3.0-2.0*blend);
   if p.experiment.x>0.5 {i=u32(floor(angle+0.5))%8u;j=i;blend=0.0;}
   let a=baked_view(ro,rd,limit,p.origins[g].xyz,i);
   var sample=a;
   if j!=i {sample=mix(a,baked_view(ro,rd,limit,p.origins[g].xyz,j),blend);}
   rgb+=trans*sample.rgb;trans*=1.0-sample.a;
  }
 } else {
  var start=1000.0;var end=0.0;
  for(var g=0u;g<u32(p.settings.x);g+=1u) {let hit=hit_box(ro,rd,p.origins[g].xyz);if hit.y>max(hit.x,0.0) {start=min(start,max(hit.x,0.0));end=max(end,hit.y);}}
  end=min(end,limit);
  if end<=start {return vec4(0.0);}
  let step=(end-start)/p.settings.y;
  // Uniform bounded midpoint quadrature; sum all overlapping groups *before* Beer-Lambert.
  for(var s=0u;s<u32(p.settings.y);s+=1u) {
   let world=ro+rd*(start+(f32(s)+0.5)*step);
   let d=rho(world);let alpha=1.0-exp(-p.settings.z*d*step);
   // Reference approximation: constant unlit earth source color, no self-shadowing or phase term.
   rgb+=trans*alpha*vec3(0.381326,0.246201,0.135633);trans*=1.0-alpha;
   if trans<0.003 {break;}
  }
 }
 let alpha=1.0-trans;
 let mapped=tone_mapping(vec4(rgb/max(alpha,0.000001),1.0),view.color_grading);
 return vec4(mapped.rgb,alpha);
}
