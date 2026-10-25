# Maybraid Dust

Houdini-authored dust assets and portable reference rules for client integration. No Bevy implementation is included.

Watch `Cartoon/Previews/RollingWave.mp4`, `Landing.mp4`, and `Combined/Previews/CombinedDust.mp4` for the selected effects. These accepted Houdini renders are unchanged. `RuntimeClouds.mp4` and `WindAndTurn.mp4` demonstrate the new particle-driven approximation in Houdini. `FineCloudsAndGrains.png` isolates the selected offline fine density and grain geometry at close range.

## Runtime inputs

Each category has `Assets/Contents.json`, `Assets/Body/BodyAssets.json`, and `Assets/MotionRules.json`. The three GLB meshes, three rest-density grids and four-coefficient local deformation stream are reusable body assets. The analytic authoring curves reproduce the selected motion; the optional 48 Hz linear controller stream has a measured maximum displacement fit error of 1.643 mm.

For dynamic fine dust, load `Combined/Assets/DynamicDust/SmallClouds/CloudLibrary.json`: four distinct static 64³ density elements, 2 MiB total. Their grids use dimensionless local coordinates; each particle’s positive XYZ scale converts them to metres, followed by its quaternion and world position. Appearance and lifetime defaults are in this manifest and `DynamicRules.json`. `Combined/Source/dynamic_dust.py` is the engine-independent Python/NumPy reference. The normal and wind/turn binaries are verification samples, not required animation playback.

1. Evaluate body motion and local deformation, then scale, rotation about the pivot, support correction and translation. Transform surface normals by the inverse transpose. Invert the same complete map when sampling body rest density.
2. Map rest emission anchors through that complete body transform. Inherit full material-point velocity, including deformation and scale/rotation changes; add the separately specified release velocity.
3. Release clouds into world space. Advance at 120 Hz with deterministic tick hashes, limited nearby following, shared wind and coherent local curl, drag, growth and concentration decay. Separate bounded pools retire particles by lifetime. Turning the body never transforms released particles. Pause accrues no time; reset clears the event.
4. Sample each cloud’s 3D element through its inverse transform. Sum main-body and cloud optical concentrations before the shared soft cap and extinction integration. Grains use their own small geometry, drag, gravity and lifetime, with related emission and flow influences.

Exact equations, update order and defaults are in the source/manifests. `Assets/Verification.json` contains port probes, including hashes, mapped positions, normals, inherited velocities and particle snapshots. Overlapping effects need a client event ID in addition to the supplied per-event IDs.

## Optional offline reference

`Combined/Assets/Reference/Pyro/Sequence.json` describes the selected world-space secondary density and XYZ velocity: 121 samples at 30 Hz, 0–4 seconds. Each field retains its own native grid and voxel-centre transform. Headerless little-endian float32, X fastest then Y then Z, is losslessly gzip-compressed: approximately 452 MiB compressed / 1.07 GiB decoded. No resolution, timing or precision reduction was applied. Trilinear spatial sampling uses zero outside; interpolate adjacent timestamped fields linearly. The supplied reader rejects time outside the recording.

`VisibleGrains.json` indexes the selected offline particle reference at 120 Hz. Particle binaries use the explicit 96-byte little-endian layout in their index JSON, with stable IDs, births/lifetimes, position, velocity, scale, quaternion and appearance. `GrainBirths.json` retains precise birth parameters. `portable_io.py` and `pyro_reference.py` decode volumes/particles with standard Python only; NumPy is needed for motion, array adapters and dynamic simulation. Runtime loading requires no Houdini or original project paths.

## Authoring and regeneration

Open projects under either `Houdini/` folder. `BodyShapes.hip` contains editable procedural mesh/density generators; Cartoon includes wave and landing projects. Combined includes the selected effect, dynamic two-scenario demonstration and spatial cloud library. Projects use relative packaged sources and embedded shaders. Authoring was checked with Houdini 22.0.466 and Mantra; rebuilding projects/shaders requires its VEX compiler.

Run from the category directory, choosing new output destinations:

```sh
hython Source/export_body_assets.py --output GeneratedBody
python Source/verify_assets.py
```

Additional Combined commands:

```sh
python Source/small_clouds.py
python Source/export_dynamic.py --output GeneratedParticles
hython Source/export_offline.py --output GeneratedPyro
python Source/export_grains.py --sequence GeneratedPyro/Sequence.json --output GeneratedGrains.bin
hython Source/render_examples.py --output GeneratedRenders
```

The selected project defaults to portable-field playback. Its `FineDustSimulation/SimulateFineDust` node recooks from frame 1; native caches are intentionally excluded. `create_projects.py --replace` rebuilds scene setups after source changes; preserve edited scenes first. Body geometry edits require compatible rest grids, anchors and support recalculation.

The dynamic system approximates offline Pyro; it does not reproduce it frame for frame. The client must implement Bevy/Rust/GPU loading, simulation, density composition, lighting/shadows, scene-depth intersections, terrain collisions, external influence fields, event management and performance budgets. Reference collisions use a flat Y=0 ground. The selected body holds its terminal state; no seamless loop or complete body fade is supplied. Lighting is an offline directional-transport approximation. `Combined/Assets/Inventory.json` lists delivered files, sizes and SHA-256 hashes, excluding itself.
