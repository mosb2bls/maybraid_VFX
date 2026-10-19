# Projectiles

Flight, penetration, and first-contact for query-only bolts and bullets.

Weapons (muzzle, cooldown, lasers) stay in item crates. [`ProjectileSystems::Sweep`](src/lib.rs)
shapecasts each flight in ≤2 m segments so fast bolts still register thin
colliders. Visual impacts listen to [`ProjectileContact`](src/lib.rs); this crate
does not depend on Hanabi or firearms. Identical ballistic specifications share
their mesh and material assets, and empty penetration segments avoid reverse
and endpoint queries that cannot contribute a solid span. Contact consumers
order after `ProjectileSystems::Sweep`.

```text
Weapon fire                →  spawn_flight
ProjectileSystems::Sweep   →  Flight budgets + ProjectileContact
item VFX                   →  consume contact
```
