# Damage

Generic hit receiver. Producers write [`Hit`](src/lib.rs); this crate mutates
[`Health`](src/lib.rs) and emits [`DamageApplied`](src/lib.rs) / [`Died`](src/lib.rs).
`Died` is materialized once as a durable [`Downed`](src/lifecycle.rs) component.
Further [`Hit`]s on already-dead `Health` are ignored, so a body that never
receives a corpse clock would stand there un-injurable. [`mark_downed`](src/lifecycle.rs)
and [`ensure_downed_have_despawn`](src/lifecycle.rs) therefore stamp a fallback
[`DespawnAfter`](src/lifecycle.rs) ([`DEFAULT_CORPSE_SECS`](src/lifecycle.rs));
consumers may replace it with a tighter timer. [`down_zero_health`](src/lifecycle.rs)
covers zero-HP bodies that never emitted `Died`. The shared game-time queue lets
handoffs finish before the hierarchy is removed.

[`projectiles`](../projectiles/) stay geometry-only. A contact adapter copies
[`HitPayload`](src/lib.rs) off the projectile into `Hit`. An optional
[`HeadshotBand`](src/lib.rs) on the target scales that amount when the contact
sits above `min_local_y` in the target's local Y. Firearms, melee, and
playgrounds do not apply HP themselves; they stamp payload and (if they want a
bonus) the band.

```text
ProjectileContact + HitPayload  →  Hit
Hit + optional HeadshotBand     →  Health, DamageApplied, Died → Downed
```
