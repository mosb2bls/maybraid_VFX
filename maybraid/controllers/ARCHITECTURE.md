# Composable controllers

Tracking: [#1436](https://github.com/ramate-io/maybraid/issues/1436).
This document is the stacked phase 1–5 contract
([#1437](https://github.com/ramate-io/maybraid/issues/1437),
[#1438](https://github.com/ramate-io/maybraid/issues/1438),
[#1439](https://github.com/ramate-io/maybraid/issues/1439),
[#1440](https://github.com/ramate-io/maybraid/issues/1440),
[#1441](https://github.com/ramate-io/maybraid/issues/1441)). Later phases
stack on it; they should update symbol mappings when they move a writer, not
invent a second inventory.

A **consumer/interpreter** receives an event, sample, or intent and interprets
it into behavior, state, or further events. A **controller** is a
consumer/interpreter that produces follow-on events or intents, possibly with
persistent state.

There is no universal intent-interpret stage and no giant intent enum.
[`ControllerPipeline::ResetMotorRequests`](core/src/schedule.rs) only clears
per-actor motor slots. Reusable systems are registered once; actor- or
session-scoped participation is installed, removed, or suspended. Domain
interpreters own realization; upstream controllers must not compete by
writing the same motor components. Continuous input carries neutral / release
semantics; analog samples are not heap messages.

## Flow

Target lane (from [#1436](https://github.com/ramate-io/maybraid/issues/1436)):

```text
Device input (A1) → virtual-pad controller (A2) → input-routing controller (A3)
  → menu controller (A4 / M1) → MenuNavImpulse → menu consumer (M2)
  → character controller (A5)
       Look / Focus / Ads / SwapPov (B1–B4)      → camera interpreter (C1–C4)
       UseItem / PowerUseItem / queue (B5–B7)    → item interpreter (D1–D9)
       InGameMenu / Inventory / ToggleMap (B8–B10) → shell interpreter (E1–E3)
       movement + gestures (B11–B13)             → installed actor controllers
            ordinary locomotion → MotorIntent → compose → apply (P3 / P4)
            Tanz gesture → TanzInput
                 → Tanz selection → ExecuteTanz → execution → motor + pose
                 → gesture-to-map → SkillMapFlick → skill-map interpreter
            standard flick controller → SkillMapFlick
NPC movement controller (A6) → MotorIntent
```

Phase 1 realizes A3 as the sole [`InputRouter`](core/src/router.rs), U1–U6
contracts (assignment, token-scoped capture / suspension, hold / gesture
effects), M1 targeting, and P1–P4 as **generic** `SystemSet`s. Phase 2
realizes ordinary locomotion (B11), the movement-specific motor contract
(`MotorIntent` contributions → `compose_motor` → `ResolvedMotorRequest` →
`apply_motor`), NPC request + recovery / separation modifiers (A6), and
scoped WorldFilter suppression. Phase 3 realizes Tanz mode switch +
recognition on the single routed actor sample, `ExecuteTanz` selection,
and actor/session-addressed `SkillMapFlick` with source/actor ownership
and `RoutingEvent` cancel / freeze. Phase 4 realizes session capabilities
and pause tokens. Phase 5 addresses UseItem / PowerUseItem through
[`dispatch_equipped_item`](../items/equipment/src/dispatch.rs) (D1), stamps
[`EquippedItemHandler`](../items/equipment/src/dispatch.rs) at inventory / holster
commit, and hands recoil through [`CameraRecoilHandoff`](../player/camera/src/look.rs).
Character produce is still a broadcast adapter for look / shell / queue
intents; item interpreters consume D1, not the pad.

## Dependency direction

```text
bevy / maybraid-input
        ▲
        │
controllers/core          (router, capture, generic schedule — no game deps)
        ▲
        │
controllers/character     controllers/menu     gameplay/session
        ▲                        ▲                     ▲
        │                        │                     │
player/lib (motor)         menu widgets          gameplay/lib + world + game
        ▲
        │
controllers/locomotion / items/tanz-user / intelligence
player/camera / items/skill-map / items/{equipment,firearm-user,grenade-user}
movement/realization
```

Rules:

- `controllers/core` depends only on `maybraid-input` and Bevy.
- `controllers/character` depends on `core` + `input`.
- The player motor contract (`MotorIntent` / `ResolvedMotorRequest`) must not
  depend on locomotion or Tanz.
- Locomotion and Tanz write the motor contract; they do not own apply.
- Apply realizes translation / jump / stance / sprint / cap without checking
  whether the producer was Tanz.
- `game` assembles World + Gameplay + shell. `gameplay/lib` is the sole owner
of session, equipment, and item *runtime* registration
([`register_gameplay_runtime`](../gameplay/lib/src/lib.rs) /
[`ItemRuntimes`](../gameplay/lib/src/equipment.rs)). Those registers are
idempotent: item plugins may call the same functions; a second call is a
no-op. Headless stack tests use that same registration. `GameplayPlugin`
adds item *presentation* plugins after the runtime. `world` keeps terrain /
POI / population / presentation / `spawn_surface` plus world-specific
adapters (loadout, stash, map, occupancy). They are not controllers unless
a type in them produces follow-on intents (E1–E4 today).

### Crate layout

Package names stay; only directories moved. See [#1450](https://github.com/ramate-io/maybraid/issues/1450).

| Directory | Package | Owns |
|---|---|---|
| `player/lib` | `player` | Capsule, motor compose / apply, handoff slots |
| `player/camera` | `player-camera` | Follow camera, look, recoil apply |
| `movement/realization` | `movement-realization` | Recovery modifier |
| `items/skill-map` | `maybraid-skill-map` | Map session + flick |
| `items/equipment` | `maybraid-equipment` | Category, holster, equipped dispatch |
| `items/firearm-user` | `firearm-user` | Trigger / routing / recoil (`runtime` / `presentation`) |
| `items/firearms` | `firearms` | Cadence clock + `WeaponShotRequest` (`runtime` / `presentation`) |
| `items/grenade-user` | `grenade-user` | Cook / throw request (`runtime` / `presentation`) |
| `items/grenades` | `grenades` | Fuse / blast + `SpawnThrownGrenade` realize |
| `items/tanz-user` | `tanz-user` | Tanz recognize / execute |
| `gameplay/session` | `maybraid-session` | Capabilities, pause, play-session end |
| `gameplay/lib` | `maybraid-gameplay` | Cross-domain schedule, item-runtime owner, adapters |
| `world/lib` | `maybraid-world` | Terrain, POIs, population, presentation, `spawn_surface` |
| `game/src` | `maybraid` | Assembles World + Gameplay + shell / flow |

`controllers/world` is a compatibility re-export of `gameplay/session`.
Persistent equipment state lives in `items/equipment`, grenade cooking in
`grenade-user`, firing cadence in `firearms`. `gameplay/lib` adapters only
translate or coordinate.

## Scheduling boundaries

```text
PreUpdate
  InputSystems
    → VirtualPadSystems::Produce     (A1 → A2)
    → VirtualPadSystems::Derive      (MenuNavPad, PadCursor)

Update
  ControllerPipeline::Bind           (P1)  character produce, menu focus+dispatch
    CharacterControlSystems ⊂ Bind   (item / camera / shell alias)
  camera / item / shell branch       (C*, D*, E*) after Bind
  ControllerPipeline::ResetMotorRequests (P2)  reset MotorIntent slots
  ControllerPipeline::MotorRequest   (P3)  generic
    MotorRequestStage::Standard      locomotion + NPC drive (beside those systems)
    MotorRequestStage::Tanz          Tanz execution
    MotorRequestStage::WorldFilter   pause / text / map suppress
    recovery + separation modifiers
  ControllerPipeline::Motor          (P4)  compose + apply, then PlayerSystems::Body

PostUpdate
  PadRumbleSystems
```

`ControllerPipeline` is the shared schedule contract. Plugin install order is
not. Tanz / Standard / WorldFilter names do **not** belong in generic core;
phase 2 registers those sets beside locomotion / Tanz / world systems
(`player::MotorRequestStage`).

`InputRouter` is the sole routing owner. Sources have an explicit actor and
optional UI assignment. `SourceConnection::Connected` with no actor is
awaiting host fallback; `Disconnected` is an explicit source-lifecycle
state. Assignment loss, disconnect, removal, and the first blocking capture
clear actor delivery and emit `HoldEffect::ReleaseNeutral` +
`GestureEffect::Cancel`. The last blocking capture release emits
`Uncaptured` (`None` / `None`). Nested captures emit only when effective
delivery changes. Suspension emits `HoldEffect::Freeze` +
`GestureEffect::Freeze`. Captures and suspensions are token- and
owner-scoped: releasing one owner cannot release another.

Lifecycle events are a Bevy `Message` (`RoutingEvent`). Each event carries
`previous` / `next` actor identity; consumers apply
[`RoutingEvent::lost_actor`](core/src/router.rs) /
[`RoutingEvent::gained_actor`](core/src/router.rs) /
[`RoutingEvent::applies_to`](core/src/router.rs) instead of reconstructing
ownership from the router's later state. Each consumer uses a
`MessageReader` cursor and sees every event exactly once, regardless of
stage. Mutating systems should use `InputRouting` so the message is
published in the same system. `take_events` is only the unpublished buffer.

Controllers read `InputRouter::actor` / `delivered_actor` / `local_actor`.
They never rediscover a "current player". The lowest-ID `Player` fallback is
resolved **once** at the host assignment boundary
(`InputRouter::assign_lowest_id_actor`, called from `player`'s
`assign_local_player`) and only while the source is connected. Reconnect is
`InputRouter::connect` from the source lifecycle.

Same-frame pause and capture (invariant 10): menu focus + capture write run
in the same `Update` chain as dispatch. Pause declares a token-owned
`MotorIntent` suppression on the local player in `WorldFilter` the same
frame; compose emits a released request and apply drops sprint, jump, and
caps. NPC slots are not suppressed.

## Role inventory

Each row is a spec role. **Symbol** is the live type or system. **Now** is
the current owner. Later phases change the owner column, not the role id.

### A — controllers / sources

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| A1 | Device HID | `bevy_input` `Gamepad` / `ButtonInput` / mouse; Apple `hid/gamecontroller.rs` | OS / gilrs / GameController | `input::produce::{keyboard,mouse,gamepad}` | `VirtualPadPlugin` | Hardware only. Stays in `input`. |
| A2 | Virtual-pad controller | `VirtualPad`, `VirtualPadSystems` | A1 producers + `PadOverlay` | A3–A5, plus leftover hardware readers below | `VirtualPadPlugin` | Canonical analog + edges. |
| A3 | Input-routing controller | `InputRouter`, `InputSource`, `Routed<T>`, `RouteTarget` | A2 (`InputSource::LocalPad`) | A4 / A5 / U5 / U6 | `ControllersCorePlugin` | Sole owner of assignment, capture, and suspension. One local pad. |
| A4 | Menu controller | `MenuControllerPlugin`, `refresh_menu_focus`, `dispatch_menu_nav` | A2 `MenuNavPad` | M2 via `MenuNavImpulse` | `MenuControllerPlugin` | Targeted path (see M1). |
| A5 | Character controller | `produce_character_intents` → `CharacterIntent`; `RoutedActorControl` | A2 `VirtualPad` + `InputRouter::delivered_actor` | B* intents (broadcast) + B12 / B13 routed sample | `CharacterControllerPlugin` | Bind. Intents still untargeted; analog / A / B are actor-addressed. |
| A6 | NPC movement controller | `intelligence::movement::drive_movement` | planner / `MovementIntelligence` | `MotorIntent` NPC slot | movement-intelligence plugin | **Phase 2.** Declares; does not write `MoveWish`. |

### B — character / actor samples

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| B1 | Look | `CharacterIntent::Look` | A5 | C1 `apply_look_intents` | character crate | Broadcast. |
| B2 | Focus | `CharacterIntent::Focus` | A5 | C2; also drops sprint in `interpret_standard_locomotion` | character crate | Broadcast. |
| B3 | Ads | `CharacterIntent::Ads` | A5 | C3 | character crate | Broadcast. |
| B4 | Swap POV | `CharacterIntent::SwapPov` | A5 | C4 | character crate | Broadcast. |
| B5 | Use item | `CharacterIntent::UseItem` | A5 | D1 `dispatch_equipped_item` | character crate (sample); `items/equipment` (D1) | **Phase 5.** Broadcast Bind sample. D1 is the only interpreter. |
| B6 | Power-use item | `CharacterIntent::PowerUseItem` | A5 | D1 → `EquippedPowerUse` | character crate | **Phase 5.** Broadcast Bind sample. D7 reads the addressed message. |
| B7 | Equipped queue | `SwapActive`, `CycleEquippedQueue`, `SelectInventoryCyclingCategory` | A5 | D5 / D6 | character crate | Broadcast. |
| B8 | In-game menu | `CharacterIntent::InGameMenu` | A5 | E1 `toggle_world_pause` | character crate | Broadcast. |
| B9 | Inventory | `CharacterIntent::Inventory` | A5 | E2 `stash.rs` | character crate | Broadcast. |
| B10 | Toggle map | `CharacterIntent::ToggleMap` | A5 | E3 `map_view.rs` | character crate | Broadcast. |
| B11 | Ordinary locomotion | `controllers/locomotion` `interpret_standard_locomotion` | A5 + `InputRouter::delivered_local_actor` | `MotorIntent` Standard contribution | `LocomotionControllerPlugin` | **Phase 2.** One delivered actor. Capture / disconnect / suspend do not write Standard. No lowest-ID lookup. |
| B12 | Tanz gesture | `TanzInput` (`user: Entity`) | `tanz_user::recognize_tanz_inputs` from `RoutedActorControl` | Tanz match → `ExecuteTanz`; skill-map fan-out | `TanzUser` component | **Phase 3.** Source/actor-owned history; physical A/B, not Jump. `RoutingEvent` messages cancel partial gestures. |
| B13 | Standard flick | `SkillMapFlick` | `SkillMapController::detect_skill_map_flicks` from `RoutedActorControl` | skill-map interpreter | `SkillMapController` resource | **Phase 3.** Actor/session-addressed; `look_throw`. History scoped to owner; session change cannot complete against the new session. |

### C — camera interpreter

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| C1 | Look | `player_camera::look::apply_look_intents` (`Look`) | B1 | `CameraController` yaw / pitch | `player/camera` | **Phase 5.** Follow camera; gated on `InputRouter::delivered_actor`. |
| C2 | Focus | same system (`Focus`) | B2 | `CameraController.focus` | `player/camera` | **Phase 5.** Intent only — no parallel mouse read. |
| C3 | Ads | same system (`Ads`) | B3 | `CameraController.ads` | `player/camera` | **Phase 5.** Intent only — no parallel mouse read. |
| C4 | Swap POV | same system (`SwapPov`) | B4 | `CameraController::toggle` | `player/camera` | Gated by `CameraPovLocked`. **Phase 5.** |
| C5 | Recoil | `CameraRecoilHandoff` via `apply_recoil_handoff` | D4 `WeaponFired` | `CameraController` follow yaw / pitch + shooter `PlayerLook` | camera is the only follow-yaw writer | **Phase 5.** Denied / zero-recoil shots write nothing. |

### D — item interpreter

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| D1 | Equipped-item dispatch | `dispatch_equipped_item` → `EquippedItemUse` / `EquippedPowerUse` | A5 Bind + `EquippedItemHandler` | D2 / D3 / D7 | `items/equipment` | **Phase 5.** Character produces `UseItem`; equipment writes only for `delivered_actor`. Absence is not a release. |
| D2 | Firearm trigger | `firearm_user::fire::apply_fire_intents` | D1 firearm + held | `WeaponTrigger` on held gun | firearm-user | **Phase 5.** `With<Player>` + addressed use. NPCs keep intelligence `WeaponTrigger`. |
| D3 | Grenade throw | `grenade_user::throw::apply_throw_intents` | D1 grenade | cook / latch | grenade-user | **Phase 5.** Missing delivery freezes; `take_live_cook` owns swap / death drop. |
| D4 | Recoil handoff | `queue_weapon_recoil` → `advance_weapon_recoil` | `WeaponFired` | C5 `CameraRecoilHandoff` | firearm-user | **Phase 5.** Camera applies; firearm-user does not write look / camera. |
| D5 | Inventory category | `apply_inventory_category_selection` | B7 `SelectInventoryCyclingCategory` | per-actor `InventoryCycling`; HUD `ActiveInventoryCyclingCategory` | `items/equipment` `selection.rs` | **Phase 5.** Actor component is the owner; resource is a mirror. |
| D6 | Queue cycle / swap | `cycle_equipped_inventory_queues` / `begin_equipment_holster` + `commit_equipment_holster` | B7 + `InventoryCycling` | skill-map / Lubikan cycle, or `EquipmentHolster` commit | `items/equipment` | **Phase 5.** Weapons commit the handler at the holster midpoint. Item crates realize hold / teardown. |
| D7 | Lubikan activation | `gameplay/lib/src/powers.rs` `request_lubikan_activation` | D1 `EquippedPowerUse` | `LubikanActivationRequest::equipped(owner)` | `gameplay/lib` | **Phase 5.** Spends once; refusal keeps charge. |
| D8 | Interaction / stash | `stash.rs` `StartInteraction` | `CharacterIntent::StartInteraction` | stash / crate loot | world | Broadcast. |
| D9 | Item presentation | `PlayerPoseSystems::Item` / `WeaponSwap` | D6, Tanz pose | held meshes | player presentation | Not a controller. Listed so D6 does not own pose. |

### E — shell interpreter

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| E1 | In-game menu / pause | `game::toggle_world_pause` | B8 + `VirtualPad` (A vs Start) | `WorldPause`, `SessionCapabilities` pause token, `WorldGameplayEnabled` | `GamePlugin` + `gameplay/session` | Pause writes the session token; session projects live-play. |
| E2 | Inventory sheet | `stash.rs` Inventory reader | B9 | inventory editor | world | Broadcast. |
| E3 | World map | `map_view.rs` ToggleMap reader | B10 | `WorldMapView` | world | Broadcast. Two camera-lock writers remain. |
| E4 | Session gameplay gate | `SessionCapabilities`, `WorldGameplayEnabled` | E1, `game/src/flow.rs` enter, pause | B11 world filter, HUD, cycling | `gameplay/session` | Live. Capabilities already replace the old `SessionActivity` / `WorldGameplayEnabled` gates. Session projects live-play; world keeps scenery / menu-shell bits. |

### U — routing, capture, lifecycle

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| U1 | Source routing | `InputSource` | A3 | `Routed`, `CaptureLayer`, `InputRouter` | `controllers/core` | Local pad only. |
| U2 | Actor routing | `ActorTarget`, `InputRouter::actor`, `RoutedActorControl` | host `assign_local_player` | A5; B11 / B12 / B13 | `InputRouter` (sole owner) | Lowest-ID `Player` assigned **once** at the host boundary. Controllers do not rediscover. |
| U3 | UI routing | `UiTarget`, `RouteTarget::Ui`, `InputRouter::ui` | A4 capture; host `assign_ui` | M1 / M2 | `InputRouter` | Menu path uses token-scoped capture; UI assignment is available for hosts. |
| U4 | Focus pick | `MenuController::pick_focus` / `resolve` | screen subtree | A4 | `MenuController` | Overlay > HUD > text. |
| U5 | Scoped capture | `CaptureToken`, `ScopeOwner`, `CaptureStack` via `InputRouter` | A4 `refresh_menu_focus` (`MenuNavCapture`); console / map / text | A4 dispatch; A5 `delivered_actor`; pad policy | `InputRouter` | Token- and owner-scoped. Nested menu / text / overlay. First/last blocking capture emits `RoutingKind::{Captured,Uncaptured}`. Updating a claim keeps stack order. |
| U6 | Suspension | `SuspensionToken`, `InputRouter::suspend` / `set_paused` | E1 pause; hosts | A5 delivery; Tanz execute (session-local); E4 projection | `InputRouter` | Delivery freeze. Motor freeze is `MotorSuppressionToken`. Pause is not session end. Session writes go through `InputRouting` so later stages see the messages. |
| U7 | Lifecycle cleanup | `RoutingEvent` hold / gesture effects; Tanz `reset_input` / `apply_toggle`; item latch / trigger; death despawn | A3 events; mode flip; death | wishes, execution, items | `InputRouter` + per-domain | Consumers read `MessageReader<RoutingEvent>` (`InputRouting` publishes). `ReleaseNeutral` / `Cancel` drop Standard holds, partial Tanz / flick history, and leftover item holds (disconnect / capture / reassignment / removal). A live grenade cook is kept on `Captured`. |

### M — menu

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| M1 | Menu controller | `MenuController`, `ExclusiveNav`, `dispatch_menu_nav` | A2 `MenuNavPad` | M2 | `MenuControllerPlugin` | **Phase 1 migrated path.** `Routed<MenuNav>` → `MenuNavImpulse` on one `UiTarget`. Exclusive modal nav acquires a `CaptureToken` on `InputRouter`. |
| M2 | Menu consumer | `On<MenuNavImpulse>` in `TextMenu`, `HudMenu`, scroll, short-text, screens, playground prompts | M1 | widget state, `MenuActivate` | each widget | Already entity-addressed. Keep. |

### P — pipeline / motor

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| P1 | Bind | `ControllerPipeline::Bind` | A4, A5 | C* / D* / E* / B11 | `ControllersCorePlugin` | Live. Character produce + menu dispatch. |
| P2 | Reset motor requests | `ControllerPipeline::ResetMotorRequests` / `reset_motor_intents` | — | P3 | `PlayerPlugin` | **Phase 2.** Clears declared contributions each frame. `MotorSelection` persists. |
| P3 | Motor request | `player::MotorRequestStage` + recovery / separation | B11, B12, A6, world | P4 | domain sets beside those systems | **Phase 2.** Declared contributions only. |
| P4 | Motor apply | `compose_motor` then `apply_motor` then `PlayerSystems::Body` | `ResolvedMotorRequest` | `MoveWish` / jump / sprint / cap | `PlayerPlugin` | **Phase 2.** Compose selects; apply realizes without producer checks. |

## Realized-state writer inventory

Who currently writes the live motor / pose bits that interpreters should own.

| State | Writer | File | Policy | Role after the stack |
|---|---|---|---|---|
| `CharacterIntent` | `produce_character_intents` | `controllers/character/src/produce.rs` | `InputRouter::actor`, capture blocks, per-actor `OrdinaryLocomotion` | A5 Bind. Single producer. Becomes `Routed` per actor. |
| `RoutedActorControl` | `produce_actor_control_samples` | `controllers/character/src/sample.rs` | `InputRouter::delivered_actor` | Bind. Analog + physical A/B for one actor. Sole pad sample for B12 / B13. |
| `ExecuteTanz` | `match_tanz_inputs` | `items/tanz-user/src/execute.rs` | ordered `TanzStack` + cooldown | Selection only. Independent of `SkillMapFlick`. |
| `MotorIntent` Standard | `interpret_standard_locomotion` | `controllers/locomotion/src/standard.rs` | `InputRouter::delivered_local_actor`; per-actor `OrdinaryLocomotion` | P3 Standard. One delivered actor. |
| `MotorIntent` Tanz | `apply_tanz_movement` | `items/tanz-user/src/execute.rs` | `TanzUser.mode.is_tanzer()` | P3 Tanz. Declares sprint/cap in the sample. Compose selects; apply does not inspect Tanz. |
| `SkillMapFlick` | Tanz `publish_tanz_map_inputs` / Standard `detect_skill_map_flicks` | `tanz-user` / `skill-map` | actor + optional session | Independent of Tanz match / execution success. |
| `MotorIntent` suppress | `apply_world_motor_filter` | `world/lib/src/control.rs` | `WORLD_GAMEPLAY` on Player; `SIMULATION` on every `CharacterController` when the Simulation bit is absent | P3 WorldFilter. Token-owned; zeros leftover vegetation `MoveWish`. |
| `MotorIntent` NPC | `drive_movement` | `intelligence/movement` | `MovementIntelligence` | A6. Declares. |
| `MotorIntent` recovery | `realize_movement` | `movement/realization` | `MotorModifier::Recovery` | P3 modifier. |
| `MotorIntent` separation | `declare_npc_soft_bump` | `player/lib/src/separation.rs` | `MotorModifier::Separation` | P3 modifier. |
| `ResolvedMotorRequest` | `compose_motor` | `player/lib/src/motor/compose.rs` | Tanz else NPC else Standard; then modifiers / suppression | Composition only. |
| `MoveWish` / `JumpWish` / `Sprinting` / `MoveSpeedCap` | `apply_motor` | `player/lib/src/motor/apply.rs` | composed request | **Only apply path.** |
| mode flip | `TanzUser::apply_toggle` / `set_mode` | `items/tanz-user/src/lib.rs` | bumper chord / agent | Clears execution only. Contributor loss releases stale sprint / jump / cap. |
| vegetation `MoveWish` / `MovementAction` | `keyboard_movement_input` | `world/player/src/player.rs` | `PadMovementEnabled` (false in Discovery) | Playground host only. Discovery duplicate removed. |
| velocity | `apply_character_movement` | `world/player/src/player.rs` | `VegetationPlayerMotor` (false in Discovery) | Playground host motor. |
| `CameraController` look / POV | `apply_look_intents` | `player/camera/src/look.rs` | `CameraLookSuppressed`, `CameraPovLocked`, `delivered_actor` | C1–C4. **Phase 5.** |
| Follow camera recoil yaw / pitch | `apply_recoil_handoff` | `player/camera/src/look.rs` | `CameraRecoilHandoff` from `WeaponFired` | C5. **Phase 5.** Sole follow-yaw recoil writer. |
| `EquippedItemHandler` | `PendingEquipmentHold::request` / item `spawn_held_*` | `items/equipment` + item crates | holster commit requests; items stamp at realize | D1 stamp. **Phase 5.** World `hold_primary_weapon` still arms the first loadout. |
| `EquippedItemUse` / `EquippedPowerUse` | `dispatch_equipped_item` | `items/equipment/src/dispatch.rs` | `delivered_actor` + handler | D1. **Phase 5.** Sole Bind writer. |
| `WeaponTrigger` (player) | `apply_fire_intents` | `items/firearm-user/src/fire.rs` | `Player` + addressed firearm use | D2. **Phase 5.** |
| `WeaponTrigger` (NPC) | `intelligence::combat::firearm` | `intelligence/combat/firearm/src/combat.rs` | planner cadence | A6 item path. Keep. |
| `WorldGameplayEnabled` | `toggle_world_pause`, flow enter | `game/src/lib.rs`, world control | pause / shell | E1 / E4. Phase 4. |

## Direct hardware reads (outside `input`)

Invariant 3: one normalized input source. These reads stay until the listed
phase. Phase 1 inventories them and migrates menu dispatch only.

| Location | Reads | Should consume | Phase |
|---|---|---|---|
| `controllers/character/src/produce.rs` | `VirtualPad` | A2 (this **is** the character Bind reader) | keep; later route the output |
| `controllers/menu/src/dispatch.rs` | `MenuNavPad` (derived, not raw HID) | A2 surface | keep |
| `items/tanz-user` | — | `RoutedActorControl` (A5 / U2) | **Phase 3 migrated** |
| `items/skill-map/src/controller.rs` / `cursor.rs` | — | `RoutedActorControl` + addressed `SkillMapFlick` | **Phase 3 migrated** |
| `items/firearm-user/src/fire.rs` | — | D1 `EquippedItemUse` | **Phase 5 migrated** |
| `game/src/lib.rs` `toggle_world_pause` | `VirtualPad` (A vs Start) | B8 | 4 |
| `world/lib/src/player_lifecycle.rs` `drive_respawn_picker` | `VirtualPad` stick / D-pad | shell | 5 |
| `world/lib/src/camera.rs` `fly_world_camera` | `VirtualPad` | playground free-fly | keep (host debug) |
| `world/player/src/player.rs` `keyboard_movement_input` | `VirtualPad` | playground B11 | playground host |
| `world/player/src/camera.rs` | `VirtualPad` | playground camera | keep (host debug) |
| `world/lib/src/stash.rs` | `Query<&Gamepad>` | menu select | 5 |
| `menu/components` short-text / modal / prompt | `VirtualPad` / `Gamepad` | M1 / text capture | 4–5 |
| `menu/playground` drop / leave / preview | `VirtualPad` / `Gamepad` | M1 | playground hosts |
| `util/camera-controls`, `util/game-commands` | `Gamepad` | debug overlays | keep (util) |

## Query-wide dispatch

Invariant 1: explicit targeting. Broadcast `Messages<T>` plus `Query` that
applies the last sample to **every** matching entity.

| Path | Mechanism | Phase 1 | Removal |
|---|---|---|---|
| **Menu nav (M1)** | `Routed<MenuNav>` → `MenuNavImpulse` on one `UiTarget`; exclusive modal acquires a `CaptureToken` | **Migrated** | — |
| `CharacterIntent` (B1–B11) | `MessageWriter` / `MessageReader`, no actor | adapter | 3–5 per consumer. B11 consumes via `InputRouter::delivered_local_actor`. |
| `apply_move_intents` | removed | **Phase 2** | locomotion + compose / apply |
| `apply_intents_to_movement` | removed live writes | **Phase 2** | `apply_world_motor_filter` suppress |
| `apply_look_intents` | broadcast B1–B4; gated on `delivered_actor` | **Phase 5 gated** | remaining: addressed look sample |
| `apply_fire_intents` | D1 → one `Player` `FirearmUser` | **Phase 5 migrated** | — |
| `grenade_user::throw` | D1 grenade; missing use freezes | **Phase 5 migrated** | — |
| `SkillMapFlick` | actor/session-addressed | **Phase 3 migrated** | — |
| `TanzInput` | produced from `RoutedActorControl` for one actor | **Phase 3 migrated** | — |
| `drive_movement` | each NPC declares its own `MotorIntent` (OK) from a planner | **Phase 2** | keep |

## Temporary adapters

Do not add a second copy of these. Remove in the named phase.

| Adapter | Why it exists | Removal |
|---|---|---|
| Broadcast `CharacterIntent` | Look / shell / queue consumers still read the process-wide message. UseItem / PowerUseItem now go through D1. | Remaining look / shell / queue consumers. |
| `CharacterControlSystems` ⊂ `Bind` | Leftover `.after(CharacterControlSystems)` item / camera / shell hosts. Motor hosts joined the pipeline. | Remaining hosts; Bind alias only. |
| Domain `MotorRequestStage` lives in player | Generic core must not name Tanz / Standard / WorldFilter. | Keep beside domain systems. |
| `PlayerSystems::Body` outside P4 | **Removed.** Body is in `ControllerPipeline::Motor`. Intent stays for item users (D*). | — |
| World adapter live `player::MoveWish` write | **Removed.** WorldFilter declares token-owned suppress only. | — |
| `MenuOwnsBack` written from menu focus | **Removed.** Capture owns **B**. | — |
| `ExclusiveNav` fallback in `dispatch_menu_nav` | **Removed.** Dispatch reads `InputRouter::top_capture` only. | — |
| `PadGameplayEnabled` + `KeyboardPadMap` lanes | **Removed.** `InputRouter::pad_policy` writes `PadInputPolicy`. | — |
| `OrdinaryLocomotionEnabled` pad-level resource | **Removed.** Per-actor `OrdinaryLocomotion` is the participation bit. | — |
| `SkillMapSteerPolicy` pad-level resource | **Removed.** Tanz `SkillMapFlick` and per-actor `OrdinaryLocomotion` gate Standard flick. | — |
| Direct `VirtualPad` / `Gamepad` reads in the hardware table | Bind is the only blessed reader. Tanz / skill-map migrated. | Phase 5 leftovers. |

## Routing contracts

| Event | Held inputs | Accumulated gestures | Delivery |
|---|---|---|---|
| First assign | `HoldEffect::None` | `GestureEffect::None` | Source delivers to the assigned actor / UI |
| Reassignment | `ReleaseNeutral` | `Cancel` | Old target loses the source; new target receives later samples |
| Disconnect | `ReleaseNeutral` | `Cancel` | Assignment cleared; `delivered_*` is `None`; host fallback must not assign until `connect` |
| Removal (actor / UI gone, or host `clear_*`) | `ReleaseNeutral` | `Cancel` | That side of the assignment is cleared |
| First blocking capture | `ReleaseNeutral` | `Cancel` | Assignment kept; actor delivery blocked |
| Nested capture / release | (no event) | (no event) | Effective actor delivery unchanged |
| Last blocking capture released | `None` | `None` | Actor delivery restored if still assigned and not suspended |
| Suspension | `Freeze` | `Freeze` | Assignment kept; `delivered_*` is `None` until resume |
| Resume | `None` | `None` | Delivery returns if still assigned and not captured |

Capture redirects / blocks actor delivery without clearing the assignment.
Releasing `CaptureToken` / `SuspensionToken` A, or `ScopeOwner` A, cannot
release owner B.

Tanz recognition and Standard flick history are scoped to the
`(InputSource, ActorTarget)` owner. `GestureEffect::Cancel` drops partial
flicks and bumper holds. Changing a map session cannot complete a gesture
against the new session.

## Motor contributions

Composition (`MotorIntent::compose` / `compose_motor`) selects contributions
and writes an explicit [`ResolvedMotorRequest`]. Apply realizes that request
and does not inspect the producer.

| Kind | Source | Policy |
|---|---|---|
| Base movement | Tanz else NPC else Standard | The selected locomotion / execution sample. `planned_translation()` uses this same order. |
| Modifier | `MotorModifier::Recovery` / `Separation` | Recovery may replace translation and OR jump. Separation adds an XZ split. |
| Suppression | `MotorSuppressionToken` | Scoped, token-owned. Any live token yields a released request. Releasing A cannot release B. |

When the previously selected base contributor disappears, translation is the
new base (or zero if none) and that contributor's unclaimed sprint / cap /
jump holds are released. "No request" does not preserve stale ownership.
`apply_routing_hold_effects` reads `MessageReader<RoutingEvent>` and treats
`HoldEffect::ReleaseNeutral` (disconnect, first capture, reassignment,
removal) as Standard contributor loss on the actor that just lost delivery.

## Invariants (checklist)

Copied from [#1436](https://github.com/ramate-io/maybraid/issues/1436) so
later PRs do not drift.

1. Explicit targeting (no query-wide broadcast).
2. Scoped, composable UI capture.
3. One normalized input source (no raw `VirtualPad` / gamepad reads outside `input`).
4. Mode config changes participation, not a per-frame global table.
5. Input blocking is not simulation suspension.
6. Suspension preserves capabilities.
7. Motor ownership: one compose path, one apply path.
8. Gesture fan-out: an unmatched Tanz still drives the map.
9. Lifecycle cleanup.
10. Same-frame behavior for pause and capture.

## Phase 5 — items, camera, Lubikan

Tracking: [#1441](https://github.com/ramate-io/maybraid/issues/1441). Stack tip of
[#1436](https://github.com/ramate-io/maybraid/issues/1436).

### Item / camera flow

```text
VirtualPad → produce_character_intents → CharacterIntent (Bind broadcast)
InputRouter::delivered_actor + EquippedItemHandler
       → dispatch_equipped_item
            → EquippedItemUse(actor, handler, squeeze, held)
                 → apply_fire_intents      (firearm + held → WeaponTrigger;
                                            re-checks delivered_actor after
                                            SessionCapabilitySystems)
                 → apply_throw_intents     (grenade; missing use is not a release)
                 → apply_throw_routing     (MessageReader: each event on
                                            lost_actor; drop on disconnect /
                                            reassign / removal; capture
                                            cancels leftover latch)
                 → advance_throw           (suspension freezes fuse + windup;
                                            capture keeps a live fuse;
                                            release writes SpawnThrownGrenade)
            → EquippedPowerUse(actor)
                 → LubikanActivationRequest::equipped(owner)
InventoryCycling (per-actor) ← SelectInventoryCyclingCategory
       → HUD ActiveInventoryCyclingCategory (mirror)
       → cycle_equipped_inventory_queues | begin_equipment_holster
            → commit_equipment_holster requests hold; item crates stamp the handler
WeaponFired → queue_weapon_recoil → CameraRecoilHandoff → apply_recoil_handoff
```

### Crate layout

| Crate | Phase 5 types / systems |
|---|---|
| `controllers/character` | `CharacterIntent::UseItem` / `PowerUseItem` / queue samples; `InventoryCyclingCategory` |
| `items/equipment` | `EquippedItemHandler`, `EquippedItemUse`, `EquippedPowerUse`, `dispatch_equipped_item`, `InventoryCycling`, `EquipmentHolster` |
| `player/camera` | `CameraRecoilHandoff`, `apply_recoil_handoff`; look gated on delivery |
| `items/firearm-user` | `apply_fire_intents` reads D1; `apply_fire_routing` clears a held trigger on `lost_actor`; recoil writes the handoff only; `register_firearm_user_runtime` / `register_firearm_user_presentation`; realizes hold / teardown |
| `items/grenade-user` | `GrenadeThrow::{apply_use,try_start_cook,apply_hold,apply_routing,cancel_partial_hold,take_live_cook}`; `register_grenade_user_runtime` / `register_grenade_user_presentation`; throw writes `SpawnThrownGrenade` |
| `items/firearms` | `decide_weapon_fire` clock + `WeaponShotRequest` / `WeaponFired`; `register_firearm_runtime` / `register_firearm_presentation`. Presentation `fire_weapons` realizes shot requests. |
| `gameplay/lib` | `register_gameplay_runtime` / `ItemRuntimes` (sole item-runtime owner); `request_lubikan_activation`; actor teardown coordinate |
| `gameplay/session` | capabilities, pause token, play-session end |
| `world` | loadout / arm / VFX adapters; world-dependent Lubikan owner grant |
| `intelligence/combat/firearm` | unchanged NPC `WeaponTrigger` writer |

### Controller I/O

| In | Out | Owner |
|---|---|---|
| `CharacterIntent::UseItem` / `PowerUseItem` | `EquippedItemUse` / `EquippedPowerUse` | D1 Bind |
| `EquippedItemUse` firearm + held | `WeaponTrigger` | D2 |
| `WeaponTrigger` + cadence | `WeaponShotRequest` / `WeaponFired` | `decide_weapon_fire` |
| `EquippedItemUse` grenade | cook / latch / explicit release | D3 |
| throw release | `SpawnThrownGrenade` | D3 → grenades interpreter |
| `RoutingEvent` disconnect / reassign / removal | `lost_actor` + `take_live_cook` + `DropLiveGrenade` | D3 |
| `RoutingEvent` `Captured` | cancel leftover item latch / trigger; keep a live cook | D3 / D2 |
| `InputRouter` suspension | freeze `advance_throw` clocks | D3 |
| `WeaponFired` | `CameraRecoilHandoff` | D4 → C5 |
| `EquippedPowerUse` | `LubikanActivationRequest::equipped` | D7 |
| `SelectInventoryCyclingCategory` | `InventoryCycling` + HUD mirror | D5 |
| `SwapActive` / `CycleEquippedQueue` | skill-map / Lubikan cycle, or `WeaponSwap` | D6 |

### Removed legacy paths

- `firearm-user` no longer reads `VirtualPad` or `MouseButton`.
- `grenade-user` no longer treats a missing `UseItem` broadcast as release.
- `world_lubikan` no longer reads `CharacterIntent::PowerUseItem`.
- Recoil no longer writes `CameraController` / `PlayerLook` from firearm-user.
- Camera focus / ADS no longer OR in a parallel mouse read.

### Tests

Unit: `dispatch.rs`, `fire.rs`, `throw/` (`missing_use_does_not_release_a_live_cook`,
`disconnect_drops_only_the_named_grenade_user`,
`reassignment_then_capture_in_one_frame_cleans_the_old_actor`,
`release_emits_exactly_one_spawn_request`),
`items/equipment` selection / transition, `gameplay/lib` `powers.rs` / `actor_lifecycle.rs`,
`player/camera` `look.rs`. World still tests loadout arm and Lubikan owner grant.

Stack tip (`gameplay/lib/src/controller_stack.rs`) calls the same headless
runtime registration production uses
(`register_gameplay_runtime`, `register_player_camera_runtime`) and
steps `app.update()`. A second `register_gameplay_runtime` or a later
item-plugin runtime call is a no-op. GPU / audio systems stay on the
full plugins (`GameplayPlugin` adds those after the runtime):

- two delivered actors; two item modes
- two simultaneous grenade users; a local-pad disconnect drops only the
  named actor
- reassignment plus capture in one frame still cleans the old cook
- grenade handler does not squeeze a firearm
- throw release emits exactly one `SpawnThrownGrenade` (owner, fuse,
  origin, velocity)
- denied shot runs real fire + recoil clocks (cooldown denies; a later
  ready shot kicks)
- capture keeps ticking a live fuse and cancels a leftover item latch /
  held firearm trigger; suspension freezes fuse and windup
- reassignment / disconnect emit `ReleaseNeutral` and drop via
  `take_live_cook` on `lost_actor`; disconnect stays disconnected under
  production `PlayerPlugin` / `assign_local_player`
- Start + held trigger in one pad frame leaves an idle firearm / ready
  grenade (`PlayerSystems::Intent` after `SessionCapabilitySystems`)
- category then cycle in one frame
- Lubikan spends once, then refuses
- nested captures release independently
- same-frame pause blocks queue cycle
- loading → play → inventory pause → resume → Home
- NPC `FirearmUser` does not consume player D1

### Remaining limitations

- `CharacterIntent` is still a Bind broadcast for look, shell, and queue
  intents (B1–B4, B7–B10).
- `CharacterControlSystems` remains a Bind alias for leftover
  `.after(CharacterControlSystems)` hosts.
- `apply_look_intents` is still query-wide `FollowCamera` (delivery-gated).
- Shell HID leftovers: `player_lifecycle` respawn picker `VirtualPad`,
  `stash` `Gamepad` presence, `game::toggle_world_pause` Start / A.
- Full `WorldPlugin` is not headless-safe; stack tests use the extracted
  headless runtime registration, not terrain / vegetation assemble.

## Review baselines

Useful commits may be pulled from these branches; do not push to them.

- [#1394](https://github.com/ramate-io/maybraid/pull/1394) `9ba9504d` — explicit `ControllerPipeline` (generic stages now in `controllers/core`). Named Tanz / Standard / WorldFilter sets live beside domain systems.
- [#1395](https://github.com/ramate-io/maybraid/pull/1395) `9c683129` — `SessionActivity` + per-actor ordinary locomotion. Phase 4 replaces the global table.

Do not touch [#1342](https://github.com/ramate-io/maybraid/pull/1342).
