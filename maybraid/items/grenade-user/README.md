# Grenade user

Held grenade, cook / windup / release, and owned recharge clocks.

Throw realization reads addressed
[`EquippedItemUse`](../../controllers/character/src/dispatch.rs) for the
grenade [`EquippedItemHandler`](../../controllers/character/src/dispatch.rs).
A missing use (capture, suspend, other actor) freezes the cook — it is not a
release. [`GrenadeThrow::take_live_cook`](src/throw.rs) is the single swap /
death drop owner.

See [ARCHITECTURE.md](../../controllers/ARCHITECTURE.md) for the D1 / D3
contract.
