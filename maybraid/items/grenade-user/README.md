# Grenade user

Held grenade, cook / windup / release, and owned recharge clocks.

Throw realization reads addressed
[`EquippedItemUse`](../equipment/src/dispatch.rs) for the
grenade [`EquippedItemHandler`](../equipment/src/dispatch.rs).
A missing use is not a release. [`apply_throw_routing`](src/throw.rs) reads
[`RoutingEvent`](../../controllers/core/src/router.rs) via `MessageReader`.
Capture keeps a live fuse ticking and only cancels a leftover use latch;
suspension freezes both fuse and windup in [`advance_throw`](src/throw.rs).
Disconnect / reassign / removal drop through
[`GrenadeThrow::apply_routing`](src/throw.rs).
[`GrenadeThrow::take_live_cook`](src/throw.rs) is the single swap / death /
cancel drop owner. [`spawn_held_grenade`](src/hold.rs) installs the grenade
handler. Execution (cook / windup, routing, recharge, equipment realize) lives
in [`runtime`](src/runtime.rs) via [`register_grenade_user_runtime`](src/runtime.rs).
HUD / pose / feel live in [`presentation`](src/presentation.rs). Throw builds
[`ThrownGrenadeSpawn`](../grenades/src/runtime.rs) and realizes it immediately;
a follow-up will publish that payload as `SpawnThrownGrenade`.

See [ARCHITECTURE.md](../../controllers/ARCHITECTURE.md) for the D1 / D3
contract.
