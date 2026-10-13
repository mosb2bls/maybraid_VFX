Here’s the full spec I’d use.

```rust
/// A transition into an animation from a captured source pose.
///
/// `Transition` is not a general two-animation mixer. It represents the
/// moment where an animation takes control of a rig from whatever pose the rig
/// was already in when the transition began.
///
/// The source pose is captured once at construction time. During `apply`, the
/// target animation is sampled normally, then the rig is blended from the
/// captured source pose into that sampled target pose using `progress` and
/// `curve`.
pub struct Transition<A, R> {
    /// The animation being transitioned into.
    pub animation: A,

    /// Pose captured at transition start.
    ///
    /// This must remain stable for the lifetime of the transition. Do not
    /// update it every frame.
    pub from_pose: RigPose,

    /// Linear transition progress before curve remapping.
    ///
    /// Expected range: `0.0..=1.0`.
    /// Values are clamped during sampling.
    pub progress: f32,

    /// Curve used to remap progress into blend weight.
    ///
    /// For example, `SmoothStep` turns linear time into eased blend weight.
    pub curve: BlendCurve,

    _rig: std::marker::PhantomData<R>,
}
```

I’d define the curve as a concrete enum first:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendCurve {
    Linear,
    SmoothStep,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Default for BlendCurve {
    fn default() -> Self {
        Self::SmoothStep
    }
}

impl BlendCurve {
    pub fn sample(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        match self {
            Self::Linear => t,

            Self::SmoothStep => t * t * (3.0 - 2.0 * t),

            Self::EaseIn => t * t,

            Self::EaseOut => {
                let inv = 1.0 - t;
                1.0 - inv * inv
            }

            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
}
```

Constructor API:

```rust
impl<A, R> Transition<A, R>
where
    R: HumanoidRig,
{
    /// Creates a transition into `animation`, capturing the rig's current pose.
    pub fn new(animation: A, rig: &R, progress: f32) -> Self {
        Self {
            animation,
            from_pose: snapshot_pose(rig),
            progress,
            curve: BlendCurve::default(),
            _rig: std::marker::PhantomData,
        }
    }

    /// Creates a transition from an explicit captured pose.
    pub fn from_pose(animation: A, from_pose: RigPose, progress: f32) -> Self {
        Self {
            animation,
            from_pose,
            progress,
            curve: BlendCurve::default(),
            _rig: std::marker::PhantomData,
        }
    }

    pub fn with_curve(mut self, curve: BlendCurve) -> Self {
        self.curve = curve;
        self
    }

    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = progress;
        self
    }

    pub fn weight(&self) -> f32 {
        self.curve.sample(self.progress)
    }

    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}
```

Animation implementation:

```rust
impl<A, R> Animation<R> for Transition<A, R>
where
    A: Animation<R>,
    R: HumanoidRig,
{
    fn apply(&self, rig: &mut R) -> Effects {
        let rest = snapshot_pose(rig);

        let (target_pose, target_effects) = sample(&self.animation, rig, &rest);

        let weight = self.weight();

        blend_pose(rig, &self.from_pose, &target_pose, weight);

        mix_effects(Effects::default(), target_effects, weight)
    }
}
```

Expected semantics:

```rust
// At transition start:
let transition = Transition::<_, HumanoidV0Rig>::new(
    Run::default(),
    rig,
    0.0,
)
.with_curve(BlendCurve::SmoothStep);

// Each frame:
transition.progress = elapsed / duration;
transition.animation.phase = run_phase;
transition.apply(rig);
```

Important invariants:

```rust
// Good:
from_pose is captured once when the state changes.

// Bad:
from_pose is re-captured every frame.
```

I’d describe the rule as:

```text
Transition<A> = captured current pose -> sampled A pose
Mix<A, B>     = sampled A pose -> sampled B pose
Smooth<A, B>  = sampled A pose -> sampled B pose with smoothed weight
```

So `Transition` is specifically the “enter this animation from the current rig state” primitive.
