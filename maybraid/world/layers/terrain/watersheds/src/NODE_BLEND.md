A depression should not independently contribute a carve, rim, and apron as three terrain-height modifiers. It should contribute a hydraulic interior, a water-surface constraint, and a zone of influence. The rim and apron should then be derived from the boundary of the combined hydraulic interior.

That prevents most of the nonsensical rims by construction.

1. Union the wet geometry first; derive rims afterward

The current failure is probably that a stream thalweg still thinks it has two exposed banks when it meets a lake. Its bank/rim field is being evaluated locally, without knowing that one side—or both sides—has become interior to another water feature.

Instead, give every component fields such as:

struct HydroPrimitive {
    // Negative inside the hydraulically occupied region.
    footprint: DistanceField,

    // Desired bed elevation at a point.
    bed: ElevationField,

    // Desired water-surface elevation.
    surface: ElevationField,

    // Distance over which this primitive may influence terrain.
    buffer: DistanceField,
}

Then construct the complex in this order:

Union all hydraulic footprints.
Resolve the bed and water surface over that union.
Compute distance from the boundary of the union.
Generate banks, rims, and aprons only from boundary segments that remain exposed.

Conceptually:

combined_interior = union(lake, stream_a, stream_b, bog, ...)
combined_boundary = boundary(combined_interior)
rim = function(distance_to(combined_boundary))

At a stream–lake junction, the end of the stream footprint lies inside the union, so it produces no terminal rim. At a stream confluence, the internal banks disappear naturally.

This is safer than trying to teach each primitive a growing list of adjacency exceptions.

2. Use a signed “hydraulic occupancy” field

Your buffer-query idea is good. I would make the query return more than a Boolean union:

struct HydroSample {
    occupancy: f32,       // signed distance or coverage
    target_bed: f32,
    water_surface: f32,
    flow_priority: f32,
    bank_class: BankClass,
}

The occupancy field answers:

Is this point wet/interior?
How far is it from the hydraulic interior?
Which banks are exterior rather than internal?
How broad should the transition be?

For thalwegs, the footprint can be generated from distance to the centreline with a varying half-width:

ϕ
s
	​

(x)=d(x,centreline)−w(s)

For a lake:

ϕ
l
	​

(x)=d
lake footprint
	​

(x)

Then use either a hard union:

ϕ(x)=
i
min
	​

ϕ
i
	​

(x)

or a controlled smooth minimum around junctions.

A hard minimum is often desirable for topology. You can smooth the resulting terrain heights separately, without letting smoothing accidentally disconnect the water geometry.

3. “Take the minimum bed” is safe, but only after separating bed and surface

For carve depth, taking the minimum target bed is a good conservative rule:

z
bed
	​

(x)=
i
min
	​

z
bed,i
	​

(x)

It guarantees that no primitive’s required channel is filled in by another primitive.

However, I would not use the same unconstrained minimum rule for water surfaces everywhere.

There are three different situations:

Lake plus inflowing stream

Near the lake, the stream water surface should converge to the lake level:

z
s
	​

(s
mouth
	​

)=z
lake
	​


Upstream, it should rise monotonically. The stream bed can snap downward to the lake bed or form a delta/shoal transition, depending on the feature.

So:

Water surface: constrained blend toward lake level.
Bed: minimum or controlled blend.
Banks: derived from the union, so no mouth rim.
Stream confluence

The child/downstream water surface should control the junction. Each incoming stream must approach it from an equal or higher surface elevation.

A useful junction rule is:

z
junction
	​

=z
downstream
	​

(0)

and then reparameterize the final segment of each tributary so its surface descends into that value.

Adjacent lake-like bodies

Taking the minimum water level would effectively declare that the lower basin drains the higher one. That is correct only if the connecting saddle is below the higher surface.

For bodies of water, the important quantity is the spill elevation:

z
spill
	​

=
γ
min
	​

x∈γ
max
	​

z
terrain
	​

(x)

You do not need to solve this continuously for pocket waters. You can define or approximate a saddle height between bodies. Then:

If both proposed surfaces are below the saddle, they remain separate.
If either rises above it, they merge or spill.
Once connected, the joined water surface should generally be common, unless you explicitly model flow through a constriction.

So minimum is a good bed-union operator, but water surfaces need connectivity and spill semantics.

4. Make junctions explicit objects

Higher-degree graphs will become much easier if nodes are not just overlapping stream segments.

Introduce junction primitives:

enum HydroNode {
    Lake(Lake),
    Bog(Bog),
    Reach(StreamReach),
    Confluence(Confluence),
    Mouth(Mouth),
    Spring(Spring),
    Spillway(Spillway),
}

A Mouth or Confluence owns a short transition region. It can:

widen the incoming thalweg;
flatten or reshape the final bed profile;
converge the stream surface to the receiver surface;
suppress internal banks;
optionally deposit a fan, delta, gravel bar, or plunge pool.

That is not merely a visual patch. Hydrological junctions genuinely have different morphology from ordinary reaches.

You can still generate these automatically from graph topology. Any vertex with degree other than two becomes a junction object.

5. Use longitudinal and transverse coordinates for streams

A stream should not be defined purely as a radial carve around a path. Give each reach local coordinates:

s: distance along the centreline;
n: signed lateral distance from it.

Then define:

z
surface
	​

(s)
z
bed
	​

(s,n)=z
surface
	​

(s)−D(s)P(
w(s)
n
	​

)

where P is the cross-section profile.

This makes several guarantees easy:

water surface is monotone downstream;
bed remains below water surface;
width changes gradually;
depth changes gradually;
banks disappear where the footprint becomes interior to another feature.

Near junctions, you can blend the stream’s (s,n)-based bed into a junction-specific 2D field.

6. For path quality, constrain first and optimize second

A cost function will help, but the biggest improvement will come from making invalid or implausible paths unavailable.

I would use a two-stage system.

Stage A: build a hydrologically safe routing surface

Create a smoothed “routing terrain” distinct from the rendered terrain:

h
r
	​

=G
σ
	​

∗h

or use a multiscale terrain representation.

The stream path is selected against h
r
	​

, while the final carve is applied to the rough original terrain.

This prevents small rocks and local roughness from sending the stream into bizarre paths while preserving visual detail afterward.

You can also construct a lower envelope or valley-biased routing field:

h
r
	​

(x)=h
smooth
	​

(x)−λV(x)

where V rewards valley-like locations.

Stage B: enforce hard reach constraints

For each candidate edge or path segment, reject it if it violates limits such as:

excessive longitudinal slope;
excessive cross-slope;
too much required excavation;
too much curvature;
insufficient separation from a cliff edge;
immediate slope reversals;
excessive stream-power change.

Then optimize among what remains.

This is much safer than assigning every terrible path a large but finite cost.

7. Useful hard constraints
Monotone hydraulic grade

Require:

z
surface
	​

(s+Δs)≤z
surface
	​

(s)

with a minimum and maximum allowed slope:

g
min
	​

≤−
ds
dz
surface
	​

	​

≤g
max
	​


A nearly flat reach can become a bog, pool, or lake. A reach above the maximum slope becomes a cascade or waterfall feature, rather than an ordinary stream.

That gives you an explicit morphology switch instead of accidental waterfalls.

Excavation budget

For a candidate route:

C
cut
	​

=∫max(0,z
required bed
	​

−z
available valley floor
	​

)
2
ds

Reject paths exceeding a threshold. Streams can carve, but they should not casually tunnel through ridges or cling to cliffs unless that is a deliberately selected archetype.

Cross-slope safety

Sample terrain on both sides of the centreline. Penalize or reject cases where the ambient terrain drops sharply away from the stream:

C
edge
	​

=∫max(0,S
lateral
	​

−S
max
	​

)
2
ds

This specifically catches streams running along precarious cliff edges.

You can preserve that as a rare “hanging stream” or “ledge stream” archetype rather than allowing it accidentally.

Curvature continuity

Bound:

∣κ(s)∣≤κ
max
	​


and preferably also limit rapid changes in curvature. This removes awkward zigzags from cell-scale pathfinding.

8. A very effective alternative: route a corridor, not a line

Instead of asking pathfinding to find the exact thalweg, ask it to find a broad feasible corridor.

Then generate the final stream inside the corridor using smooth curves and hydrological constraints.

For example:

Find a coarse path across a low-resolution routing grid.
Expand it into a corridor of acceptable cells.
Fit a spline through the corridor.
Optimize spline control points against slope, excavation, curvature, and cliff distance.
Generate the channel cross-section around the spline.

A grid path is usually too literal to become final geometry. Treating it as a topological guide avoids many ugly results.

9. Separate ordinary reaches from waterfalls

Waterfalls should be graph nodes or reach classes, not just places where the stream slope happened to explode.

Given required elevation loss Δh over available horizontal distance L:

if Δh/L is within stream bounds, create an ordinary reach;
if somewhat above, create a cascade reach;
if far above, insert a waterfall node;
if the geometry cannot support any of these, reroute.

A waterfall node can own:

lip;
vertical or stepped drop;
plunge pool;
widened downstream channel;
spray/wet-rock region.

This lets waterfalls remain common and fun without making the ordinary stream generator unstable.

10. Suggested hierarchy for your system

I would organize the pocket-water generator like this:

Hydrology graph
    ↓
Resolve node surfaces and directed flow
    ↓
Generate coarse feasible corridors
    ↓
Fit reaches and create junction primitives
    ↓
Union hydraulic footprints
    ↓
Resolve bed field
    ↓
Derive exposed hydraulic boundary
    ↓
Generate banks/rims/aprons from exposed boundary
    ↓
Apply local roughness and visual modulation

The crucial inversion is:

Do not union finished terrain modifications. Union the semantic water geometry, then produce the terrain modification for the union.

That is the same basic advantage as constructive geometry: topology is resolved before ornamentation.

11. Minimal change from your present design

You probably do not need to rewrite everything immediately. The smallest useful change would be:

Add a hydraulic_footprint_distance query to every depression node.
Form the minimum/union across the whole complex.
Multiply each primitive’s rim contribution by an exposed-boundary mask.

Something like:

m
i
	​

(x)=bankBand(ϕ
i
	​

(x))⋅outsideAllOthers(x)

where:

outsideAllOthers(x)=f(
j

=i
min
	​

ϕ
j
	​

(x))

This will suppress rims inside lakes and confluences.

The stronger eventual version is to stop treating the rim as belonging to primitive i at all, and derive it from the combined field:

m
rim
	​

(x)=R(ϕ
union
	​

(x))

with local attributes—height, width, material—selected from the nearby contributing primitives.

Bottom line

Your three ideas are directionally right, with two refinements:

Yes: every node should expose a queryable footprint/buffer field that can be unioned.
Yes: minimum is a good safe operator for the required bed elevation.
Refinement: water surface should be solved from graph direction, junction constraints, and spill elevations, not just pointwise minimum.
Major architectural improvement: derive rims and aprons from the boundary of the combined hydraulic footprint rather than blending independently generated rims.
For pathfinding: use a smoothed routing terrain, hard feasibility constraints, and explicit cascade/waterfall reach types before applying a softer cost function.

That should make the system much more “safe by construction”: ordinary streams cannot run uphill, cannot accidentally become waterfalls, cannot retain banks inside lakes, and cannot merge without a junction that reconciles their surfaces and beds.

Yes. The simplest answer is to put the footprints into a **spatial index**, then only evaluate the small set whose bounds overlap the sample point.

For a cellular terrain system, a uniform grid is probably the best fit.

```rust
struct FootprintIndex {
    // Spatial bucket -> candidate footprint IDs.
    buckets: HashMap<CellId, SmallVec<[FootprintId; 8]>>,
}
```

When registering a footprint:

1. Compute its conservative AABB, including its influence/buffer radius.
2. Find every index cell touched by that AABB.
3. Insert the footprint ID into those cells.

When sampling (x):

```rust
fn sample(&self, x: Vec2) -> HydroSample {
    let cell = self.cell_for_point(x);

    self.buckets
        .get(&cell)
        .into_iter()
        .flatten()
        .map(|id| self.footprints[*id].sample(x))
        .fold(HydroSample::empty(), HydroSample::union)
}
```

The lookup becomes approximately:

[
O(1 + k)
]

where (k) is the number of nearby candidate footprints, rather than (O(n)) over the whole complex.

## For your hierarchy, reuse the generation cells

You may not even need a separate index. Each generated cell can store the footprint IDs whose compact support intersects it:

```rust
struct TerrainCell {
    hydro_footprints: SmallVec<[FootprintId; 8]>,
}
```

A footprint belongs to every cell intersecting its **bounded influence region**, not merely its wet interior. Then terrain sampling inside that cell loads only that subset.

This aligns neatly with your earlier modulation model:

[
H(x)
====

F_{i_1,x}
\circ F_{i_2,x}
\circ \cdots
\circ F_{i_k,x}(h_0(x))
]

where (i_1,\dots,i_k) are obtained from the cell’s local footprint list.

Because every modulation has compact support, broad-phase bounds can safely be conservative. A false positive only causes one unnecessary footprint evaluation; a false negative would cause a seam or missing carve.

## Grid versus BVH

A few reasonable choices:

* **Uniform grid / generation cells:** best when footprints are distributed through terrain and sampling happens repeatedly.
* **BVH or R-tree:** better when footprint sizes vary enormously or the geometry is sparse.
* **Quadtree:** useful if your world hierarchy already uses one, though usually more complicated than needed.
* **Per-complex BVH:** good when each water complex is independently generated and contains many curved reaches.

For a pocket-water complex with perhaps tens of primitives, I would use two levels:

```text
world/generation cell
    -> candidate water complexes
        -> candidate primitives within that complex
```

The outer index prevents testing unrelated complexes. The inner index avoids checking every reach in a large branching complex.

## Rasterized ownership is another option

If terrain is ultimately sampled on a known grid, you can precompute a small local raster:

```rust
enum Ownership {
    None,
    One(FootprintId),
    Several(Range<FootprintId>),
}
```

Each raster sample or tile stores the candidate footprint IDs. Sampling is then extremely cheap.

However, I would store **candidate membership**, not a single winning owner. At confluences and lake mouths, overlap is semantically important; selecting one owner too early would lose the information needed for unioning beds and surfaces.

## Avoid indexing the exact curved footprint

Index only a cheap conservative bound:

* lake: expanded AABB;
* stream reach: capsule or AABB around each polyline segment;
* junction: circle or AABB;
* apron/buffer: expand by maximum support radius.

Then perform the exact signed-distance or profile evaluation only after the broad-phase lookup.

For streams, indexing each reach segment or small group of segments is better than indexing an entire long stream by one huge AABB:

```text
stream
  ├─ segment group 0 -> local AABB
  ├─ segment group 1 -> local AABB
  └─ segment group 2 -> local AABB
```

Otherwise a winding stream’s AABB may cover most of the complex and provide little pruning.

## Likely best implementation here

I would attach a compact list of `HydroPrimitiveId`s to each terrain-generation cell, populated from conservative support bounds. For unusually large complexes, each complex can additionally maintain a tiny BVH over its reaches.

That gives you:

[
O(1 + c + k)
]

where:

* (c) is the small number of complexes touching the terrain cell;
* (k) is the small number of local primitives whose bounds contain the sample.

In ordinary cases, both should remain nearly constant regardless of the total number of water features in the world.
