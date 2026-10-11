# Damage

Generic hit receiver. Producers write [`Hit`](src/lib.rs); this crate mutates
[`Health`](src/lib.rs) and emits [`DamageApplied`](src/lib.rs) / [`Died`](src/lib.rs).
`Died` is materialized once as a durable [`Downed`](src/lifecycle.rs) component.
[`DespawnAfter`](src/lifecycle.rs) provides a shared game-time queue so consumers
can finish visual or event handoffs before removing an entity hierarchy.

Accepted hits are also copied into a bounded [`DamageHistory`](src/history.rs)
during [`DamageSystems::History`](src/lib.rs), after apply and before down. Records
use generation-aware [`DamageActorId`](src/attribution.rs) keys so queries stay
meaningful after source or victim despawn and do not match recycled occupants.
[`ResponsibleOwner`](src/attribution.rs) is the summon-owner seam: the live
attacker stays on the record, and the owner is snapshotted separately.

Lubikan charge credit is named through the shared [`lubikans`](../lubikans/)
interface ([#1107](https://github.com/ramate-io/maybraid/issues/1107)): a
[`LubikanAccountBinding`](../lubikans/src/ledger.rs) snapshot (`account`,
`definition`, `owner`; `generation()` is `owner.to_bits()`). Selection is
[`LubikanSelection`](../lubikans/src/selection.rs) / `LubikanLedger::selection()`.
[`LubikanOwnerLifetime`](../lubikans/src/lifetime.rs) is the lifetime *event*,
not the snapshot token. History does not mutate charge. Healing lookback uses
[`DamageHistory::received_positive_damage_since`](src/history.rs); an active
healing session should latch that answer rather than depend on the retention
window forever.

Default retention is 45s and 256 records (oldest dropped first). Override
[`DamageHistoryPolicy`](src/history.rs). Write [`ClearDamageHistory`](src/history.rs)
on session reset.

[`projectiles`](../projectiles/) stay geometry-only. A contact adapter copies
[`HitPayload`](src/lib.rs) off the projectile into `Hit`. An optional
[`HeadshotBand`](src/lib.rs) on the target scales that amount when the contact
sits above `min_local_y` in the target's local Y. Firearms, melee, and
playgrounds do not apply HP themselves; they stamp payload and (if they want a
bonus) the band.

```text
ProjectileContact + HitPayload  →  Hit
Hit + optional HeadshotBand     →  Health, DamageApplied, Died
DamageApplied                   →  DamageHistory (bounded)
Died                            →  Downed
```
