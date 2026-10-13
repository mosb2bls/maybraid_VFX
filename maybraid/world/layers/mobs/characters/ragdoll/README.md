# Character ragdoll

Persistent procedural corpses for character visuals.

`CharacterRagdollPlugin` converts a damage [`Downed`](../../../../../damage/src/lifecycle.rs)
body into two lifetimes:

1. The `CharacterRoot` visual becomes a `Corpse` while remaining in its existing
   hierarchy, and initializes a ragdoll when its body rig is ready.
2. The body loses gameplay and physics ownership, then the complete retained
   hierarchy expires through `DespawnAfter`.

The solver does not turn rendered bones into physics bodies. It captures named
bones into world-space particles, applies inherited velocity, impact, gravity,
drag, and lightweight distance constraints, then sphere-casts only a small set
of body proxies against Fixed geometry. Simulation is frame-bounded at a low
rate and stops after a short settling window; sleeping poses are not remarshalled.
The same solver has profiles for humanoid, quadruped, and forelimbed body rigs.
Forelimbed rigs use lower gravity and greater drag as a basic buoyancy approximation.

Ragdoll state is keyed by bone name rather than entity, so a refreshed LOD bone
map can receive the current pose. `SuspendAnimation` prevents clip output from
competing with the corpse pose; marshalling runs after structural rig pose and
before transform propagation.

External systems can write `RagdollImpulse` to disturb an awake or sleeping
corpse without depending on solver internals. `CharacterRagdollSettings`
configures corpse lifetime, visual-handoff timeout, solver iterations,
simulation rate, and maximum settling time.
