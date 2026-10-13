Yes, that makes sense. I’d make `progress` the central animation sampling coordinate.

Here’s the spec I’d use.

```rust
/// Applies an animation to a rig at a specific normalized progress value.
///
/// `progress` is the animation's sampling coordinate.
///
/// For cyclic animations, progress wraps by taking `progress.fract()`.
///
/// Examples:
///
/// - `0.0` samples the start of the cycle
/// - `0.5` samples halfway through the cycle
/// - `1.0` samples the start of the next cycle
/// - `1.5` samples halfway through the next cycle
///
/// Non-cyclic animations may clamp or otherwise interpret progress beyond
/// `0.0..=1.0`.
pub trait Animation<R> {
    fn apply_for(&self, rig: &mut R, progress: f32);
    fn effects_for(&self, rig: &R, progress: f32) -> Effects { Effects::default() }
    fn apply(&self, rig: &mut R, progress: f32) -> Effects {
        self.apply_for(rig, progress);
        self.effects_for(rig, progress)
    }
}
```

I’d add a small helper type/convention:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress(pub f32);

impl Progress {
    pub fn new(value: f32) -> Self {
        Self(value)
    }

    /// Cyclic progress in [0.0, 1.0).
    pub fn cycle(self) -> f32 {
        self.0.rem_euclid(1.0)
    }

    /// Clamped one-shot progress in [0.0, 1.0].
    pub fn clamp(self) -> f32 {
        self.0.clamp(0.0, 1.0)
    }

    /// Whole completed cycles.
    pub fn cycles(self) -> f32 {
        self.0.floor()
    }

    /// True once a one-shot animation has reached or passed its end.
    pub fn is_complete(self) -> bool {
        self.0 >= 1.0
    }
}
```

Then locomotion uses a rig-agnostic frontend converted for humanoids:

```rust
/// Rig-agnostic run cycle (stride, bounce, rotation).
pub struct Run {
    pub stride: f32,
    pub bounce: f32,
    pub rotation: f32,
}

impl<R: HumanoidRig> Animation<R> for Run {
    fn apply(&self, rig: &mut R, progress: f32) -> Effects {
        UprightRun::from_run(self).apply(rig, progress)
    }
}
```

Art-level biped tuning lives in `UprightRun<R>` / `UprightWalk<R>`; [`UprightRun::from_run`](upright_run.rs) scales the tuned template from the three frontend knobs (`bounce` and `rotation` default to `1.0`).

```rust
impl<R: HumanoidRig> Animation<R> for UprightRun<R> {
    fn apply(&self, rig: &mut R, progress: f32) -> Effects {
        let phase = Progress(progress).cycle();

        let left_arm_swing = -arm_swing(phase);
        let right_arm_swing = arm_swing(phase + 0.5);
        let run = self;

        apply_leg(rig, Side::Left, phase, -1.0, run);
        apply_leg(rig, Side::Right, phase, 1.0, run);

        apply_run_arm(
            rig,
            Side::Left,
            left_arm_swing,
            phase,
            rig.forearm_flex_sign(Side::Left),
            -run.arm_down,
            run,
        );

        apply_run_arm(
            rig,
            Side::Right,
            right_arm_swing,
            phase,
            rig.forearm_flex_sign(Side::Right),
            run.arm_down,
            run,
        );

        Effects::default()
    }
}
```

This likely means `Run` should no longer store per-bone tuning:

```rust
pub struct Run {
    pub stride: f32,
    pub bounce: f32,
    pub rotation: f32,
}
```

Then animation state owns progress:

```rust
pub struct Playing<A> {
    pub animation: A,
    pub progress: f32,
    pub speed: f32,
}
```

And the controller does:

```rust
playing.progress += delta_seconds * playing.speed;
playing.animation.apply(rig, playing.progress);
```

For `Transition`, I’d define it as:

```rust
pub struct Transition<A, R> {
    pub animation: A,
    pub from_pose: RigPose,
    pub curve: TransitionCurve,
    _rig: PhantomData<R>,
}
```

with:

```rust
impl<A, R> Transition<A, R>
where
    A: Animation<R>,
    R: HumanoidRig,
{
    pub fn apply(
        &self,
        rig: &mut R,
        animation_progress: f32,
        transition_progress: f32,
    ) -> Effects {
        let rest = snapshot_pose(rig);

        restore_pose(rig, &rest);
        let effects = self.animation.apply(rig, animation_progress);
        let target_pose = snapshot_pose(rig);

        let weight = self.curve.sample(transition_progress.clamp(0.0, 1.0));

        blend_pose(rig, &self.from_pose, &target_pose, weight);
        mix_effects(Effects::default(), effects, weight)
    }
}
```

Important distinction:

```rust
animation_progress   = where the target animation is in its cycle
transition_progress  = how far the transition blend has completed
```

So usage becomes:

```rust
transition.apply(
    rig,
    run_progress,        // can be 0.0, 0.5, 1.5, 20.25, etc.
    transition_progress, // usually clamped 0.0..=1.0
);
```

I would **not** overload one progress value to mean both things. A run cycle and a transition blend are two different clocks.

Core API rule:

```text
Animation progress:
    cyclic animations wrap
    one-shot animations clamp or complete
    values above 1.0 are valid

Transition progress:
    always 0.0..=1.0
    controls blend weight only
    values above 1.0 mean transition is finished
```

This is a much cleaner model than storing `phase` inside each animation. The animation becomes a pure sampler, while the animation controller owns time.

## Stylized arm aim (jab and related punches)

Full IK is not required for fight snaps. The jab uses a **body-space target** plus
`HumanoidRig::humerus_along_with_roll`:

| Convention | Meaning |
|------------|---------|
| `target` space | Body COM; **+X** right, **+Y** up, **+Z** fight-forward |
| Humerus placement | Aim bone length (local Y) along a direction built from drop / forward / lateral weights, then roll about that length |
| Punch travel | Humerus +Z whip + elbow uncoil (not elbow alone) |
| Cover arm | Same guard along-frame; tucked elbow; no whip |

Related concepts (cross, hook, uppercut) should keep this contract: knobs stay in body
space, humanoid apply aims with along-with-roll, and swing/flex stacking is avoided for
humerus aim. See [`jab.rs`](jab.rs).
