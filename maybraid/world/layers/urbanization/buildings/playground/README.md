# Richmond Buildings Playground

Interactive viewer for Richmond building components and authored buildings.

## Run

```bash
cargo run -p buildings-playground
# or with a startup command:
cargo run -p buildings-playground -- show linear
cargo run -p buildings-playground -- show arc-90
cargo run -p buildings-playground -- show wizards-tower --noise 0.4
cargo run -p buildings-playground -- show stacked-rings --floor-count 6 --floor-height 2.5 --radius 3
cargo run -p buildings-playground -- show bedroom
cargo run -p buildings-playground -- show bedroom --extent 6,2.8,4 --noise 0.2 --door
cargo run -p buildings-playground -- show bedroom --extent 8,3,8 --occupancy 0.8 --spaciousness 1.2
cargo run -p buildings-playground -- show residential-bathroom
cargo run -p buildings-playground -- show residential-half-bathroom
cargo run -p buildings-playground -- show residential-bathroom-examples
cargo run -p buildings-playground -- show kitchen-examples
cargo run -p buildings-playground -- show dining-room-examples
cargo run -p buildings-playground -- show living-room-examples
cargo run -p buildings-playground -- show sitting-room-examples
cargo run -p buildings-playground -- show study-examples
cargo run -p buildings-playground -- show commercial-stall
cargo run -p buildings-playground -- show commercial-stall-strip --extent 14,3.5,5 --seed 42
cargo run -p buildings-playground -- show les-halles-floor-plan
cargo run -p buildings-playground -- show les-halles-floor-plan --extent 48,4,36 --seed 42
cargo run -p buildings-playground -- show les-halles-floor-plan --ceiling
cargo run -p buildings-playground -- show les-halles-floor-plan-examples
cargo run -p buildings-playground -- show les-halles-full-storey --seed 7
cargo run -p buildings-playground -- show les-halles-livable-full-storey
cargo run -p buildings-playground -- show les-halles-livable-full-storey --extent 72,4,54 --seed 42
cargo run -p buildings-playground -- show les-halles-livable-full-storey-examples
cargo run -p buildings-playground -- show mixed-use-les-halles-monotower
cargo run -p buildings-playground -- show mixed-use-les-halles-monotower --extent 72,16,54 --seed 42
# Request specific shaft openings (cyan/amber = request, magenta = fitted shaft):
cargo run -p buildings-playground -- show les-halles-floor-plan --seed 42 \
  --opening 'se:shaft:8,0,-14:12,3.5,-10' \
  --opening 'nw:shaft:-12,0,8:-8,3.5,14'
# Rectangular pitch (no end triangles):
cargo run -p buildings-playground -- show pitch --rise 1 --run 2 --length 6 --tile-width 1
# Ridge longer than eave (flipped ends via from_eave_ridge):
cargo run -p buildings-playground -- show pitch --rise 1 --run 2 --eave 4 --ridge 6 --tile-width 1
# Explicit ends (positive = upright / eave-long, negative = flipped / ridge-long):
cargo run -p buildings-playground -- show pitch --rise 1 --run 2 --length 4 --left 1 --right -0.5
cargo run -p buildings-playground -- show tessellated-triangle
cargo run -p buildings-playground -- show tessellated-triangle --a 0,0 --b 3,0 --c 0,2
# Triangle / quad / strip with clips:
cargo run -p buildings-playground -- show clipped-tessellated-triangle
cargo run -p buildings-playground -- show clipped-quad-panel
cargo run -p buildings-playground -- show clipped-ruled-strip
cargo run -p buildings-playground -- show tube
cargo run -p buildings-playground -- show connecting-hall
cargo run -p buildings-playground -- show arc-tower
cargo run -p buildings-playground -- show connecting-shells
cargo run -p buildings-playground -- show trazaloid
cargo run -p buildings-playground -- show pitched-rectangular-roof
cargo run -p buildings-playground -- show pitched-rectangular-roof --gables
cargo run -p buildings-playground -- show pitched-rectangular-roof --no-hips --gables
cargo run -p buildings-playground -- show pitched-rectangular-roof --skylight
# Big open gable with a window in the +X end wall:
cargo run -p buildings-playground -- show pitched-rectangular-roof \
  --no-hips --gables --ridge-inset 0 \
  --footprint-x 16 --footprint-z 10 --ridge-height 7 --eave-height 2.5 \
  --gable-window
# Same idea with an explicit AABB on the +X gable wall:
cargo run -p buildings-playground -- show pitched-rectangular-roof \
  --no-hips --gables --ridge-inset 0 \
  --footprint-x 16 --footprint-z 10 --ridge-height 7 --eave-height 2.5 \
  --opening 'gable_win:window:7.6,2.6,-1.4:8.4,5.2,1.4'
# Orthogonal AABB roof complex (L / T / single) with valley gizmos:
cargo run -p buildings-playground -- show rectangular-pitched-roof-complex
cargo run -p buildings-playground -- show rectangular-pitched-roof-complex --preset t
cargo run -p buildings-playground -- show rectangular-pitched-roof-complex --preset l --end-cap gable
cargo run -p buildings-playground -- show rectangular-pitched-roof-complex --preset single --overhang-ratio 0.15
cargo run -p buildings-playground -- show rectangle
cargo run -p buildings-playground -- show rectangle --preset wall
cargo run -p buildings-playground -- show clipped-rectangle
cargo run -p buildings-playground -- show clipped-rectangular-strip
cargo run -p buildings-playground -- show fitted-rectangle --preset wall
cargo run -p buildings-playground -- show clipped-fitted-rectangle
cargo run -p buildings-playground -- show clipped-fitted-rectangular-strip
cargo run -p buildings-playground -- show rectangular-n-tube
cargo run -p buildings-playground -- show arc-sweep
cargo run -p buildings-playground -- show clipped-arc-sweep
cargo run -p buildings-playground -- show noisy-rectangular-wall
# Folded quad (default ~90° crease + joint):
cargo run -p buildings-playground -- show quad-panel
# Coplanar quad (no joint):
cargo run -p buildings-playground -- show quad-panel --a0 0,0,0 --a1 3,0,0 --b0 0,0,3 --b1 3,0,3
# Panel complex (default mild trapezoid); compact mesh syntax:
cargo run -p buildings-playground -- show panel-complex
cargo run -p buildings-playground -- show panel-complex \
  --mesh '1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,4},{1,4,3}'
# Same geometry as one quad face:
cargo run -p buildings-playground -- show quad-panel-complex \
  --mesh '1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}'
# Ruled pitch strip (default funky 5+5 eave/ridge):
cargo run -p buildings-playground -- show ruled-pitch
```

In-game: press `/` for the command console (same clap commands as argv).

## Commands

- `help`
- `show linear|arc-90|arc-180|slice-90` — partition leaves (`panels/.../rectangle_001`, `arcs/.../arc_*`)
- `show pitch [--rise R] [--run R] [--length L] [--tile-width W] [--left B] [--right B] | [--eave E --ridge R]` — shepherd's-thatch pitched face; omit `--length`/`--left`/`--right` for optional regions; `--eave`+`--ridge` uses equal end triangles
- `show tessellated-triangle [--a X,Z] [--b X,Z] [--c X,Z]` — rough-stone floor fill of a panel-space triangle
- `show clipped-tessellated-triangle [--a|--b|--c X,Y,Z] [--clip 'x,y,z;…'] [--min-dihedral R] [--no-joint]` — world triangle − closed clip (clipped to bounds) → `PanelComplex`
- `show clipped-quad-panel [--a0|--a1|--b0|--b1 X,Y,Z] [--clip 'x,y,z;…'] [--min-dihedral R] [--no-joint]` — ruled quad − closed clip
- `show clipped-ruled-strip [--min-dihedral R] [--no-joint]` — multi-bay strip with a clip on the middle bay
- `show tube [--min-dihedral R] [--no-joint] [--no-floor] [--no-ceiling] [--no-left] [--no-right]` — trapezoid cross-section polyline → four clipped ruled strips (bend + pitch + slight roll; left-wall opening); `--no-*` omits faces
- `show connecting-hall` — one-kink `ConnectingHall` tube between two oriented openings (gizmos: opening quads, orientation arrows, A→mid→B path)
- `show connecting-stairwell [--case stacked|opposite|quarter-turn|tiny|tall|stacked-pair] [--kind circular|rectangular|run-and-landing] [--tread-fill F]` — exclusive AABB well (circular helix, wall-hugging rectangular, or half-well I + walk-off landing). Gizmos: cyan well box, lime walk-on, orange walk-off, yellow landings, magenta last leading
- `show connecting-stairwell-examples [--kind circular|rectangular|run-and-landing]` — 4×3 gallery of pathological wells (door pairs, skinny/pancake/needle, min/max fill, stacked pair, one-riser squat)
- `show arc-tower [--radius R] [--floor-count N] [--storey-height H] [--floor-hole M] [--no-base-floor] [--no-ceiling]` — stacked circular `ArcTower` shell (explicit cardinal openings; no noise)
- `show connecting-shells` — demo joining `ArcTower` to `Trazaloid` via `ConnectingHall`
- `show trazaloid […] [--floor] [--no-ceiling] [--floor-hole M] [--ceiling-hole M] [--door-thickness M] [--face-post-count N]` — two-band trapezoidal-pyramid shell; floor/ceiling optional with centered square holes
- `show rect-floor [--footprint-x X] [--footprint-z Z] [--storey-height H] [--floor] [--ceiling] [--door-south]… [--opening SPEC]…` — orthonormal rectangular storey; openings fit authored AABBs (`side=south` or full min:max); gizmos show plan voids + mapped quads
- `show rounded-rect-floor […] [--corner-radius R] [--corner-segments N] [--door-south]… [--opening SPEC]…` — rounded-rect storey (straight rectangle kits + ruled corners)
- `show i-floor […] [--plan-l|--stem-only] [--no-top-left]… [--door-south]… [--opening SPEC]…` — I/T/U/L/Z storey from central bar + flanges; `side=` picks nearest outer edge
- `show rect-ring-floor [--outer-x X] [--outer-z Z] [--inner-x X] [--inner-z Z] [--floor] [--ceiling] [--door-east]… [--opening SPEC]…` — courtyard rectangular ring; one opening → one outer/inner side (nearest); wide openings open that side — use several openings for a U / half-ring
- `show circ-ring-floor [--outer-radius R] [--inner-radius R] [--floor] [--ceiling] [--opening id:label:t=0.25[,ring=inner]]…` — circular courtyard ring; `t=` openings map to nearest ring (or `ring=inner|outer`)
- `show rectangle [--preset floor|wall|ceiling] [--origin X,Y,Z] [--edge X,Y,Z] [--height H] [--thickness T] [--roll R]` — oriented `buildings::Rectangle` (lowest-edge vector + height + roll; `0` roll ⇒ top toward `+Y`)
- `show clipped-rectangle [--origin|--edge X,Y,Z] [--height H] [--thickness T] [--roll R] [--left|--right|--bottom|--top M]` — oriented rectangle with inset framed by rectangle kits
- `show clipped-rectangular-strip [--inset M] [--min-dihedral R] [--no-joint]` — node-chain oriented rectangle strip; middle bay inset frame; crease joints on bay folds
- `show fitted-rectangle [--preset floor|wall|ceiling|skew] [--a0|--a1|--b0|--b1 X,Y,Z]` — best-fit `FittedRectangle` from four (possibly skew) corners
- `show clipped-fitted-rectangle [--a0|--a1|--b0|--b1 X,Y,Z] [--left|--right|--bottom|--top M]` — best-fit rectangle with inset frame
- `show clipped-fitted-rectangular-strip [--inset M] [--min-dihedral R] [--no-joint]` — two-rail best-fit strip; middle bay inset; crease joints on folds
- `show rectangular-n-tube [--inset M] [--min-dihedral R] [--no-joint] [--omit-face I]…` — closed square cross-section polyline → four clipped rectangle strips (face-1 middle bay inset); `--omit-face i` skips edge `i→i+1` (square: `0` floor, `2` ceiling)
- `show arc-sweep [--radius R] [--height H] [--sweep-degrees D] [--start-yaw-deg D]` — circular fitted `arcs::ArcSweep` (not IR `partitions::ArcSweep`)
- `show clipped-arc-sweep [--radius R] [--height H] [--sweep-degrees D] [--start-yaw-deg D]` — same with hardcoded angular clip openings
- `show noisy-rectangular-wall [--distance D] [--seed N] …` — `wall_demo` noisy path → rectangle strip (+ mid portal inset)
- `show quad-panel [--a0|--a1|--b0|--b1 X,Y,Z] [--t-a0|--t-a1|--t-b0|--t-b1 T] [--min-dihedral R] [--no-joint]` — two lines → two tessellated triangles + optional crease `JointNode` (default corners are a ~90° fold; thicknesses default to 0.4)
- `show panel-complex [--mesh 'id=(x,y,z) … {a,b,c}'] [--min-dihedral R] [--no-joint]` — point-id triangle mesh + crease joints; optional thickness as 4th tuple component
- `show quad-panel-complex [--mesh 'id=(x,y,z) … {a0,a1,b0,b1}'] [--min-dihedral R] [--no-joint]` — quad-face mesh (diagonal a0–b1) → same presentation path
- `show ruled-pitch [--min-dihedral R] [--no-joint]` — `RuledPitch` (eave/`rail_a`, ridge/`rail_b`; default funky 5+5) → ruled quad strip + crease joints
- `show wizards-tower [--noise 0.5]` — authored tower hierarchy (`LodScene` composition)
- `show stacked-rings [--floor-count N] [--floor-height H] [--radius R]` — circular wall stack (kit scale check)
- `show bedroom [--extent X,Y,Z] [--noise 0.5] [--spaciousness 1.0] [--occupancy 0.55] [--door]` — hierarchical bedroom; bed-first multi-fill under spaciousness/occupancy; `--door` adds a −Z circulation exclusion
- `show residential-bathroom [--extent X,Y,Z] [--seed N] [--door]` — full residential bathroom shell (label + passage keep-outs)
- `show residential-half-bathroom [--extent X,Y,Z] [--seed N] [--door]` — half bathroom shell (label + passage keep-outs)
- `show residential-bathroom-examples` — gallery of full + half residential bathrooms (passage AABBs drawn as gizmos)
- `show kitchen-examples` — gallery of Kitchen cells (passage AABBs drawn as gizmos)
- `show dining-room-examples` — gallery of DiningRoom cells (passage AABBs drawn as gizmos)
- `show living-room-examples` — gallery of LivingRoom cells (passage AABBs drawn as gizmos)
- `show sitting-room-examples` — gallery of SittingRoom cells (passage AABBs drawn as gizmos)
- `show study-examples` — gallery of Study cells (passage AABBs drawn as gizmos)
- `show commercial-stall [--extent X,Y,Z] [--seed N]` — single commercial stall Label placeholder
- `show commercial-stall-strip [--extent X,Y,Z] [--seed N]` — packed stall Labels along a band
- `show les-halles-floor-plan-examples` — gallery of Les Halles floor plans (commercial + livable sampling; corner vs mid-side shafts; strip/shaft gizmos)
- `show les-halles-full-storey` — shell plus gallery `CommercialStallStrip` Label fills (face text via gizmo patch)
- `show les-halles-livable-full-storey [--extent X,Y,Z] [--seed N]` — shell plus lengthwise gallery `RectangularLivableArea` bays (default footprint `72,4,54`)
- `show les-halles-livable-full-storey-examples` — gallery of livable Les Halles storeys (varied extents/seeds)
- `show mixed-use-les-halles-monotower [--extent X,Y,Z] [--seed N]` — commercial-below / livable-above Les Halles monotower (default `72,16,54`)

WASD / Space / Shift + mouse look.
