# Grenade user

Held grenade, cook / windup / release, and owned recharge clocks.

Throw realization reads addressed
[`EquippedItemUse`](../../controllers/character/src/dispatch.rs) for the
grenade [`EquippedItemHandler`](../../controllers/character/src/dispatch.rs).
A missing use is not a release. Capture keeps the fuse ticking; suspension
freezes both fuse and windup in [`advance_throw`](src/throw.rs). Disconnect /
reassign / removal apply [`HoldEffect::ReleaseNeutral`](../../controllers/core/src/router.rs)
through [`apply_throw_routing`](src/throw.rs).
[`GrenadeThrow::take_live_cook`](src/throw.rs) is the single swap / death /
cancel drop owner. [`spawn_held_grenade`](src/hold.rs) installs the grenade
handler. Headless clocks use [`register_grenade_user_runtime`](src/lib.rs).

See [ARCHITECTURE.md](../../controllers/ARCHITECTURE.md) for the D1 / D3
contract.
