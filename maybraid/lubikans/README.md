# Lubikans

Shared Lubikan interface. Other lanes depend on these types; keep the surface
small and stable. Self is a normal definition. Missing equipped selection
resolves to Self through the same model.

This crate has **no Bikanwarda dependency**. General gameplay and future
summons consume the same identity, charge, activation, selection, and owner
lifetime types.

```text
LubikanDefinitionId / LubikanAccountId
        ↓
LubikanLedger  (owned accounts + equipped selection)
        ↓
LubikanCharge::apply → LubikanChargeMutation
        ↓
LubikanChargeChanged   (authoritative award / spend)
```

HUD, occupancy, input, and combat rules read this store. They do not create
charge state by being visible.

Damage history stays generic. [`lubikans-damage`](damage/) copies
[`LubikanAccountBinding`](src/ledger.rs) onto `damage::DamageCredit` so the
`damage` crate does not depend on this one.

## Public types

| Type | Role |
|------|------|
| [`LubikanDefinitionId`](src/identity.rs) / [`LubikanAccountId`](src/identity.rs) | Catalog vs owned-account identity. Self is `LubikanDefinitionId::SELF` / `LubikanAccountId::SELF`. |
| [`LubikanDefinition`](src/identity.rs) | Capacity and activation cost for one catalog entry. |
| [`LubikanSelfConfig`](src/catalog.rs) | Configurable Self limits and lifetime policy. |
| [`LubikanSelfHealConfig`](src/heal.rs) / [`LubikanSelfHeal`](src/heal.rs) / [`LubikanSelfHealChannel`](src/heal.rs) | Self heal rate, duration, caps, and the HUD-readable channel (`active`, `remaining`, `rate`). |
| [`LubikanCatalog`](src/catalog.rs) | Registered definitions. A normal game registers Self here. |
| [`LubikanCharge`](src/charge.rs) / [`LubikanChargeMutation`](src/charge.rs) | Bounded charge and the result of a signed apply (requested, applied, new value, clamp flags). |
| [`LubikanChargeRequest`](src/change.rs) / [`LubikanChargeChanged`](src/change.rs) / [`LubikanChargeReason`](src/change.rs) | Request → one apply → one change. Occupancy and combat write requests; Award is the sole owner. |
| [`LubikanCombatAward`](src/award.rs) | Combat credits: hit = applied HP, kill = opponent max health (stacked by default). Zero-effect hits award nothing; overkill does not use requested damage. |
| [`LubikanActivationRequest`](src/activation.rs) / [`LubikanActivation`](src/activation.rs) / [`LubikanReadiness`](src/activation.rs) | Activation request, session phase, and derived eligibility. |
| [`LubikanActivationSession`](src/activation.rs) / [`LubikanActivationRefused`](src/activation.rs) / [`LubikanActivationReport`](src/activation.rs) / [`LubikanActivationFinished`](src/activation.rs) | Handoff: one owner accepts (spend once) or refuses (no spend). #1110 / summons consume the session and report. HUD reads `Finished` (account + session id); Self returns to Idle after a natural complete. |
| [`LubikanSelection`](src/selection.rs) | Equipped account, or Self when none is equipped. |
| [`LubikanLedger`](src/ledger.rs) / [`LubikanAccount`](src/ledger.rs) / [`LubikanOwner`](src/ledger.rs) / [`LubikanAccountBinding`](src/ledger.rs) | Per-owner owned accounts, plus a bind-time snapshot for attribution. Self cannot be removed. |
| [`LubikanOwnerLifetime`](src/lifetime.rs) / [`LubikanLifetimePolicy`](src/lifetime.rs) | Equip, unequip, body replace, and session restart. |
| [`LubikanSystems`](src/lib.rs) | Schedule sets other lanes order against (`Owner`, `Activate`, `Heal`, `Award`). |
| [`fixtures`](src/fixtures.rs) | Test helper (`test-support` feature or this crate's tests). |

[`LubikanInterfacePlugin`](src/lib.rs) only registers the shared messages.
[`LubikansPlugin`](src/plugin.rs) is the only place that creates or restores
a player's ledger, and the sole owner of [`LubikanActivationRequest`](src/activation.rs):
it spends once, emits [`LubikanActivationSession`](src/activation.rs) or
[`LubikanActivationRefused`](src/activation.rs), and applies
[`LubikanActivationReport`](src/activation.rs) to the captured account.
Consumers order after [`LubikanSystems::Owner`](src/lib.rs) (PreUpdate).
Self healing runs in [`LubikanSystems::Heal`](src/lib.rs) after Activate: it
consumes Self [`LubikanActivationSession`](src/activation.rs)s, regenerates
from `DamageApplied.applied` / [`DamageHistory`](../damage/src/history.rs),
and reports [`LubikanActivationReport`](src/activation.rs) plus
[`LubikanActivationFinished`](src/activation.rs) (account + session id). A
natural complete returns the captured account to Idle in the same frame so a
later activate in the same life can spend again. HUD shows completed from
`Finished`, not by holding `Completed`. It does not spend charge or call
`try_activate`. World capsules get the owner marker from
`grant_world_player_lubikan_owner` (do not insert a fresh
`LubikanLedger::with_self`). Pause copies `WorldGameplayEnabled` onto
[`LubikanHealingEnabled`](src/heal.rs) and freezes healing without cancelling.
`WorldPlaySessionEnded` is teardown. Input cycles through
[`LubikanLedger::select`](src/ledger.rs) only (`None` / `Some(Self)` are the
Self fallback) and writes [`LubikanActivationRequest`](src/activation.rs) from
`CharacterIntent::PowerUseItem` (pad **X** + fire). The world Self loop also
adds [`LubikanDamagePlugin`](damage/) so bindings copy onto generic
`DamageCredit` before history ingest. The plugin has no Bikanwarda or HUD
dependency.

## Consumer fixture

```rust
use lubikans::fixtures::{install_interface, spawn_self_owner};

let mut app = App::new();
install_interface(&mut app);
let owner = spawn_self_owner(app.world_mut());
```

Enable `lubikans` with the `test-support` feature from a consumer's
`dev-dependencies`.
