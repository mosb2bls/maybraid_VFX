# Authoring character motion

V0 clips sample a semantic pose. The resolver maps those fields onto the imported parent-space swing / flex / twist compose (`R(swing) * R(twist) * R(flex) * rest`). Field names are the authoring API. They are **not** yet a character-space anatomical contract: hip flexion is still femur swing about parent Y, elbow flexion is still forearm flex about parent Z, and walk lean is still spine twist about parent X.

Do not put a minus sign on the right knee. Right shin and right forearm already use `−Z` flex.

Character space is the armature’s local frame: **+X** right, **+Y** up, **+Z** fight-forward. Bevy’s world stays **+Y** up and **−Z** camera-forward.

`/character joint --flexion` drives the same V0 channels as clips: flexion is flex, lateral is swing, axial is twist.

Inspected `humanoid_rig.glb` (production biped bodies share these binds): T-pose `shoulder.L` is `Rz(−90°)`, `shoulder.R` is `Rz(+90°)`, humerus/forearm rest identity, `pelvis.L` is `(-0.5,-0.5,-0.5,0.5)`, femur `Rx(−90°)` at 0.25 m from the pelvis, shin identity at 0.50 m from the femur. Femur length is that shin-origin distance (0.50 m), not the pelvis offset. Parent **Y** (DEFAULT swing) is the previous flap / arm-pump stroke (T-pose +Y sweeps in XZ). Parent **Z** (DEFAULT flex, right forearm `−Z`) is the elbow hinge. Character **X** is the arm length and would only twist those joints.

Inspected `quadruped_rig.glb` (production quadruped bodies share these binds): `anterior_mid_back` `(0, s2, s2, 0)` parents `upper_back`; `shoulder.L` is `Rz(+90°)`; `anterior_thigh` is `Rx(−90°)`; shin identity. The V0 definition still parents `upper_back` to `back_ridge`; judge the live fold as `mid_back * shoulder * thigh * shin`. The old shin hinge is parent **Z** (right `−Z`).

Inspected `forelimbed_rig.glb`: `shoulder.L` is the same `Rz(−90°)` as the humanoid T-pose. Fin sweep is DEFAULT parent-Y swing.

## V0 channels

Angles are radians. These are the knobs the adapter writes:

| Semantic field | Compose channel | Typical imported axis |
| --- | --- | --- |
| `shoulder_forward`, `hip_flexion`, stride | swing | parent Y, right femur `−Y` |
| `elbow_flexion`, `knee_flexion`, hinge, `shoulder_lift` | flex | parent Z, right shin/forearm `−Z` |
| spine `forward_bend`, nod, pelvis flexion | twist | parent X |

Walk lean uses `add_root_forward` → twist so the torso pitches. That is an intentional correction of the old walk yaw. Held and looping squat share `add_stack_forward` (0.28, 0.26, 0.24, 0.22). Jab waist is `add_waist_forward` (0.40, 0.35, 0.15, 0.10). `add_turn` yaws lumbar, mid-back, and upper back.

A jab aim is a character-space direction on the humerus, plus a roll. The elbow stays V0 forearm flex.

Flapping keeps the previous parent-Y front/back stroke. A vertical wing beat would be a separate authoring change.

## What a sample may depend on

A sample reads the clip definition, the effective rest, the clip parameters, and progress. Sampling again does not accumulate. Bones the clip does not write stay at rest. `ArmatureOffset::IDENTITY` means the armature is not visually offset. Gameplay locomotion is not that offset.

Identity rest is not a production T-pose. Clip tests that need bind geometry use `HumanoidV0Rig::for_clip_test()` / `QuadrupedV0Rig::for_clip_test()`, which seed the inspected local translations and rotations.

Segment lengths are joint-to-joint. Femur length is the shin origin’s translation. A missing distal joint keeps the family default (0.5 m).

## Playground

After `/`:

- `/character assemble --species braidman --animation jab` spawns the default clothed Braidman preview (parts + character shaders) through the runtime fixed-assembly path. Sided gestures take `--side left|right`.
- `/character animate jab --side right` switches the mailbox clip without respawning. The sample clock restarts at 0 so a one-shot reads from the first frame.
- `/character playback --pause` freezes the clock. `--resume` starts it. `--speed 0.5` scales time. `--progress 0.4` scrubs to 0.4 seconds. `--once` stops at one second. `--looping` keeps running. `--phase 0.5` samples the opposite gait phase. `--leg-scale 0.75` or `--leg-scale 1.25` scales femur and shin rest translations before sampling. Femur length follows the scaled shin origin.
- `/character camera front`, `side`, or `three-quarter` frames the rig.
- `/character joint femur.L` draws bone-local axes (bright RGB), parent-local axes (shorter, dimmer), and character-space axes on the rig. The opposite bone is drawn dimmer so the two sides can be compared. A yellow line is the effective-rest length direction. A white cross marks the posed segment end. `--hide-rest` drops the rest line.
- `/character joint femur.L --flexion 40 --lateral 0 --axial 0` replaces that bone's clip rotation with the V0 flex / swing / twist channels. `--clear` returns the bone to the clip.

To inspect a new one-shot (Think Again, salute, shrug), add the clip to `AnimationMode` and the `animate_limbs` match — `/character animate` and playback/joint then work without a respawn.
