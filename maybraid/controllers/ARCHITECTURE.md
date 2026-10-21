# Composable controllers

Tracking: [#1436](https://github.com/ramate-io/maybraid/issues/1436).
This document is the phase 1 contract
([#1437](https://github.com/ramate-io/maybraid/issues/1437)). Later phases
stack on it; they should update symbol mappings when they move a writer, not
invent a second inventory.

A **consumer/interpreter** receives an event, sample, or intent and interprets
it into behavior, state, or further events. A **controller** is a
consumer/interpreter that produces follow-on events or intents, possibly with
persistent state.

There is no universal Interpret stage and no giant intent enum. Reusable
systems are registered once; actor- or session-scoped participation is
installed, removed, or suspended. Domain interpreters own realization;
upstream controllers must not compete by writing the same motor components.
Continuous input carries neutral / release semantics; analog samples are not
heap messages.

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
            ordinary locomotion → MotorIntent → motor interpreter (P3 / P4)
            Tanz gesture → TanzInput
                 → Tanz selection → ExecuteTanz → execution → motor + pose
                 → gesture-to-map → SkillMapFlick → skill-map interpreter
            standard flick controller → SkillMapFlick
NPC movement controller (A6) → MotorIntent
```

Phase 1 realizes A3 as the sole [`InputRouter`](core/src/router.rs), U1–U6
contracts (assignment, token-scoped capture / suspension, hold / gesture
effects), M1 targeting, and P1–P4 as **generic** `SystemSet`s. Motor request
writers and broadcast `CharacterIntent` stay on adapters until phases 2–5.

## Dependency direction

```text
bevy / maybraid-input
        ▲
        │
controllers/core          (router, capture, generic schedule — no game deps)
        ▲
        │
controllers/character     controllers/menu     controllers/world
        ▲                        ▲                     ▲
        │                        │                     │
player (motor contract)    menu widgets           world / game assemble
        ▲
        │
player-camera / tanz-user / skill-map / item users / intelligence
```

Rules:

- `controllers/core` depends only on `maybraid-input` and Bevy.
- `controllers/character` depends on `core` + `input`.
- The player motor contract (`MoveWish` / future `MotorIntent`) must not
  depend on locomotion or Tanz.
- Locomotion and Tanz write the motor contract; they do not own apply.
- `game` / `world` assemble plugins. They are not controllers unless a type
  in them produces follow-on intents (E1–E4 today).

## Scheduling boundaries

```text
PreUpdate
  InputSystems
    → VirtualPadSystems::Produce     (A1 → A2)
    → VirtualPadSystems::Derive      (MenuNavPad, PadCursor)

Update
  ControllerPipeline::Bind           (P1)  character produce, menu focus+dispatch
    CharacterControlSystems ⊂ Bind   (temporary alias)
  camera / item / shell branch       (C*, D*, E*) after Bind
  ControllerPipeline::Interpret      (P2)  empty in phase 1
  ControllerPipeline::MotorRequest   (P3)  generic; domain order beside domain systems
  ControllerPipeline::Motor          (P4)  PlayerSystems::Body (not joined yet)

PostUpdate
  PadRumbleSystems
```

`ControllerPipeline` is the shared schedule contract. Plugin install order is
not. Tanz / Standard / WorldFilter names do **not** belong in generic core;
phase 2 registers those sets beside locomotion / Tanz / world systems.

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

Lifecycle events are a Bevy `Message` (`RoutingEvent`). Each consumer uses a
`MessageReader` cursor and sees every event exactly once, regardless of
stage. Mutating systems should use `InputRouting` so the message is
published in the same system. `take_events` is only the unpublished buffer.

Controllers read `InputRouter::actor` / `delivered_actor`. They never
rediscover a "current player". The lowest-ID `Player` fallback is resolved
**once** at the host assignment boundary
(`InputRouter::assign_lowest_id_actor`, called from `player`'s
`assign_local_player`) and only while the source is connected. Reconnect is
`InputRouter::connect` from the source lifecycle.

Same-frame pause and capture (invariant 10): menu focus + capture write run
in the same `Update` chain as dispatch. Pause still zeros wishes in the world
adapter the same frame it flips `WorldGameplayEnabled` — that writer is not
yet inside `WorldFilter`.

## Role inventory

Each row is a spec role. **Symbol** is the live type or system. **Now** is
the phase 1 owner. Later phases change the owner column, not the role id.

### A — controllers / sources

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| A1 | Device HID | `bevy_input` `Gamepad` / `ButtonInput` / mouse; Apple `hid/gamecontroller.rs` | OS / gilrs / GameController | `input::produce::{keyboard,mouse,gamepad}` | `VirtualPadPlugin` | Hardware only. Stays in `input`. |
| A2 | Virtual-pad controller | `VirtualPad`, `VirtualPadSystems` | A1 producers + `PadOverlay` | A3–A5, plus leftover hardware readers below | `VirtualPadPlugin` | Canonical analog + edges. |
| A3 | Input-routing controller | `InputRouter`, `InputSource`, `Routed<T>`, `RouteTarget` | A2 (`InputSource::LocalPad`) | A4 / A5 / U5 / U6 | `ControllersCorePlugin` | Sole owner of assignment, capture, and suspension. One local pad. |
| A4 | Menu controller | `MenuControllerPlugin`, `refresh_menu_focus`, `dispatch_menu_nav` | A2 `MenuNavPad` | M2 via `MenuNavImpulse` | `MenuControllerPlugin` | Targeted path (see M1). |
| A5 | Character controller | `produce_character_intents` → `CharacterIntent` | A2 `VirtualPad` + `InputRouter::actor` | B* consumers via broadcast `Messages` | `CharacterControllerPlugin` | Bind only. Produces only when the host has assigned an actor; payload is still an untargeted broadcast (adapter). |
| A6 | NPC movement controller | `intelligence::movement::drive_movement` | planner / `MovementIntelligence` | `player::MoveWish` | movement-intelligence plugin | Writes the motor wish directly. Phase 2 → `MotorIntent`. |

### B — character / actor samples

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| B1 | Look | `CharacterIntent::Look` | A5 | C1 `apply_look_intents` | character crate | Broadcast. |
| B2 | Focus | `CharacterIntent::Focus` | A5 | C2; also drops sprint in `apply_move_intents` | character crate | Broadcast. |
| B3 | Ads | `CharacterIntent::Ads` | A5 | C3 | character crate | Broadcast. |
| B4 | Swap POV | `CharacterIntent::SwapPov` | A5 | C4 | character crate | Broadcast. |
| B5 | Use item | `CharacterIntent::UseItem` | A5 | D1 / D2 / D3 | character crate | Broadcast. Firearm-user also re-reads the pad (D2). |
| B6 | Power-use item | `CharacterIntent::PowerUseItem` | A5 | D7 `world_lubikan` | character crate | Broadcast. |
| B7 | Equipped queue | `SwapActive`, `CycleEquippedQueue`, `SelectInventoryCyclingCategory` | A5 | D5 / D6 | character crate | Broadcast. |
| B8 | In-game menu | `CharacterIntent::InGameMenu` | A5 | E1 `toggle_world_pause` | character crate | Broadcast. |
| B9 | Inventory | `CharacterIntent::Inventory` | A5 | E2 `stash.rs` | character crate | Broadcast. |
| B10 | Toggle map | `CharacterIntent::ToggleMap` | A5 | E3 `map_view.rs` | character crate | Broadcast. |
| B11 | Ordinary locomotion | `Move` / `Jump` / `StartSprint` / `StopSprint` / `ChangeSquat` / `ChangeProne` | A5 when `OrdinaryLocomotionEnabled` | P3 Standard `apply_move_intents`; world adapter | character + player | Broadcast + query-wide apply. |
| B12 | Tanz gesture | `TanzInput` (`user: Entity`) | `tanz_user::recognize_tanz_inputs` reading `VirtualPad` | Tanz match + skill-map fan-out | `TanzUser` component | Actor field exists; pad read is still global. Phase 3. |
| B13 | Standard flick | `SkillMapFlick` | `SkillMapController::detect_skill_map_flicks` | skill-map interpreter | `SkillMapController` resource | Untargeted message; also reads raw `Gamepad`. Phase 3. |

### C — camera interpreter

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| C1 | Look | `player_camera::look::apply_look_intents` (`Look`) | B1 | `CameraController` yaw / pitch | `player-camera` | Query-wide `FollowCamera`. Phase 5. |
| C2 | Focus | same system (`Focus` + RMB) | B2 | `CameraController.focus` | `player-camera` | Phase 5. |
| C3 | Ads | same system (`Ads` + MMB) | B3 | `CameraController.ads` | `player-camera` | Phase 5. |
| C4 | Swap POV | same system (`SwapPov`) | B4 | `CameraController::toggle` | `player-camera` | Gated by `CameraPovLocked`. Phase 5. |
| C5 | Recoil | `firearm_user::fire::{queue,advance}_weapon_recoil` | `WeaponFired` + live pad / look | `PlayerLook`, `CameraController` | firearm-user | Not a `WeaponFired`-only handoff yet. Phase 5. |

### D — item interpreter

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| D1 | Equipped-item dispatch | (missing) intended owner of B5 / B6 | A5 | D2 / D3 / D7 | none — each user reads the pad or the broadcast | Phase 5. |
| D2 | Firearm trigger | `firearm_user::fire::apply_fire_intents` | `VirtualPad` + `MouseButton::Left` | `WeaponTrigger` on held gun | firearm-user | Direct pad read. Phase 5. |
| D3 | Grenade throw | `grenade_user::throw` | B5 `UseItem` broadcast | grenade spawn / fuse | grenade-user | Query-wide. Phase 5. |
| D4 | Recoil handoff | `queue_weapon_recoil` | `WeaponFired` | C5 | firearm-user | Coupled to C5. Phase 5. |
| D5 | Inventory category | `apply_inventory_category_selection` | B7 `SelectInventoryCyclingCategory` | `ActiveInventoryCyclingCategory` | `world/lib` `inventory_cycling.rs` | One writer. Keep. |
| D6 | Queue cycle / swap | `cycle_equipped_inventory_queues`, `world/lib/src/weapon.rs` | B7 `SwapActive` / `CycleEquippedQueue` | loadout + `WeaponSwap` | two writers today | Phase 5: one category owner + timed commit. |
| D7 | Lubikan activation | `world_lubikan` PowerUseItem reader | B6 | `LubikanActivationRequest` | world | Broadcast. Phase 5. |
| D8 | Interaction / stash | `stash.rs` `StartInteraction` | `CharacterIntent::StartInteraction` | stash / crate loot | world | Broadcast. |
| D9 | Item presentation | `PlayerPoseSystems::Item` / `WeaponSwap` | D6, Tanz pose | held meshes | player presentation | Not a controller. Listed so D6 does not own pose. |

### E — shell interpreter

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| E1 | In-game menu / pause | `game::toggle_world_pause` | B8 + `VirtualPad` (A vs Start) | `WorldPause`, `WorldGameplayEnabled` | `GamePlugin` | Also writes gameplay. Phase 4. |
| E2 | Inventory sheet | `stash.rs` Inventory reader | B9 | inventory editor | world | Broadcast. |
| E3 | World map | `map_view.rs` ToggleMap reader | B10 | `WorldMapView` | world | Broadcast. Two camera-lock writers remain. |
| E4 | Session gameplay gate | `WorldGameplayEnabled`, `WorldSceneryVisible`, `WorldMenuShell` | E1, flow enter, pause | B11 world filter, HUD, cycling | world + game | Independent bools, several writers. Phase 4 replaces with capabilities. [#1395](https://github.com/ramate-io/maybraid/pull/1395) `SessionActivity` is a review baseline, not merged. |

### U — routing, capture, lifecycle

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| U1 | Source routing | `InputSource` | A3 | `Routed`, `CaptureLayer`, `InputRouter` | `controllers/core` | Local pad only. |
| U2 | Actor routing | `ActorTarget`, `InputRouter::actor` | host `assign_local_player` | A5; future B* / motor | `InputRouter` (sole owner) | Lowest-ID `Player` assigned **once** at the host boundary. Controllers do not rediscover. |
| U3 | UI routing | `UiTarget`, `RouteTarget::Ui`, `InputRouter::ui` | A4 capture; host `assign_ui` | M1 / M2 | `InputRouter` | Menu path uses token-scoped capture; UI assignment is available for hosts. |
| U4 | Focus pick | `MenuController::pick_focus` / `resolve` | screen subtree | A4 | `MenuController` | Overlay > HUD > text. |
| U5 | Scoped capture | `CaptureToken`, `ScopeOwner`, `CaptureStack` via `InputRouter` | A4 `refresh_menu_focus` (`MenuNavCapture`) | A4 `dispatch_menu_nav` | `InputRouter` | Token- and owner-scoped. Releasing one owner cannot release another. Menu-nav layer only today. Phase 4 nests text/overlay independently. |
| U6 | Suspension | `SuspensionToken`, `InputRouter::suspend` / `resume` / `resume_owner` | hosts (none live yet) | future E4 / phase 4 | `InputRouter` | Vocabulary + effects. Input blocking ≠ simulation halt. Phase 4 installs participation. |
| U7 | Lifecycle cleanup | `RoutingEvent` hold / gesture effects; Tanz `reset_input` / `apply_toggle`; death despawn | A3 events; mode flip; death | wishes, execution, items | `InputRouter` + per-domain | Router defines reassignment / disconnect / removal / suspension effects. Domain uninstall is phase 4–5. |

### M — menu

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| M1 | Menu controller | `MenuController`, `ExclusiveNav`, `dispatch_menu_nav` | A2 `MenuNavPad` | M2 | `MenuControllerPlugin` | **Phase 1 migrated path.** `Routed<MenuNav>` → `MenuNavImpulse` on one `UiTarget`. Exclusive modal nav acquires a `CaptureToken` on `InputRouter`. |
| M2 | Menu consumer | `On<MenuNavImpulse>` in `TextMenu`, `HudMenu`, scroll, short-text, screens, playground prompts | M1 | widget state, `MenuActivate` | each widget | Already entity-addressed. Keep. |

### P — pipeline / motor

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| P1 | Bind | `ControllerPipeline::Bind` | A4, A5 | C* / D* / E* / B11 | `ControllersCorePlugin` | Live. Character produce + menu dispatch. |
| P2 | Interpret | `ControllerPipeline::Interpret` | — | — | core | Empty. Phase 2. |
| P3 | Motor request | `ControllerPipeline::MotorRequest` | B11, B12, A6 | P4 | core graph; writers still outside the set | Phase 2 joins writers and places domain order beside those systems. |
| P4 | Motor apply | `PlayerSystems::Body` (`apply_wish_movement`, jump, stance hulls) | P3 wishes | capsule velocity | `PlayerPlugin` | One apply path already. Request side still has several writers. |

## Realized-state writer inventory

Who currently writes the live motor / pose bits that interpreters should own.

| State | Writer | File | Policy | Role after the stack |
|---|---|---|---|---|
| `CharacterIntent` | `produce_character_intents` | `controllers/character/src/produce.rs` | `OrdinaryLocomotionEnabled`, `PadGameplayEnabled`, `MenuOwnsBack` | A5 Bind. Single producer. Becomes `Routed` per actor. |
| `player::MoveWish` / `JumpWish` / `Sprinting` / stance | `apply_move_intents` | `player/src/intent.rs` | pad `OrdinaryLocomotionEnabled`; **all** `Player` + `CharacterController` | P3 Standard. Phase 2: `MotorIntent` for one actor. |
| vegetation `MoveWish` / `MovementAction` + zeros `player::MoveWish` | `apply_intents_to_movement` | `world/lib/src/control.rs` | `WorldGameplayEnabled.accepts_character_input`, `PlaygroundMode`, ordinary flag | P3 WorldFilter. Phase 2 drops live `player::MoveWish` writes ([#1394](https://github.com/ramate-io/maybraid/pull/1394) baseline). |
| `player::MoveWish` / `MoveSpeedCap` | `apply_tanz_movement` | `items/tanz-user/src/execute.rs` | `TanzUser.mode.is_tanzer()` | P3 Tanz. Phase 2–3: `MotorIntent` from execution. |
| zeros `MoveWish` on mode flip | `TanzUser::apply_toggle` / `toggle_control_mode` | `items/tanz-user/src/lib.rs` | bumper chord | U7. Does not touch Tanz `ModeToggle` in `input.rs` ([#1385](https://github.com/ramate-io/maybraid/issues/1385)). |
| `player::MoveWish` | `drive_movement` | `intelligence/movement` | `MovementIntelligence` | A6. Phase 2: declared `MotorIntent`. |
| `player::MoveWish` / `JumpWish` | `realize_movement` | `movement-realization` | `Without<Player>` | Phase 2: declared recovery modifier. |
| additive `MoveWish` | `apply_npc_soft_bump` | `player/src/separation.rs` | `Npc` | Phase 2: declared separation modifier. |
| vegetation `MoveWish` / `MovementAction` | `keyboard_movement_input` | `world/player/src/player.rs` | `PadMovementEnabled` (false in world) | Legacy host. Phase 2 removes the Discovery duplicate. |
| velocity | `apply_character_movement` | `world/player/src/player.rs` | `VegetationPlayerMotor` (false in world) | Legacy host motor. Phase 2. |
| `CameraController` look / POV | `apply_look_intents` | `player-camera/src/look.rs` | `CameraLookSuppressed`, `CameraPovLocked` | C1–C4. Phase 5. |
| `WeaponTrigger` | `apply_fire_intents` | `items/firearm-user/src/fire.rs` | `Player` + `FirearmUser` | D2. Phase 5. |
| `WorldGameplayEnabled` | `toggle_world_pause`, flow enter | `game/src/lib.rs`, world control | pause / shell | E1 / E4. Phase 4. |

## Direct hardware reads (outside `input`)

Invariant 3: one normalized input source. These reads stay until the listed
phase. Phase 1 inventories them and migrates menu dispatch only.

| Location | Reads | Should consume | Phase |
|---|---|---|---|
| `controllers/character/src/produce.rs` | `VirtualPad` | A2 (this **is** the character Bind reader) | keep; later route the output |
| `controllers/menu/src/dispatch.rs` | `MenuNavPad` (derived, not raw HID) | A2 surface | keep |
| `items/tanz-user/src/lib.rs` `toggle_control_mode` / `recognize_tanz_inputs` | `VirtualPad` | routed actor sample | 3 |
| `items/tanz-user/src/input.rs` | `VirtualPad` in the recognizer | same | 3 |
| `skill-map/src/controller.rs` `detect_skill_map_flicks` | `VirtualPad` + raw `Query<&Gamepad>` | routed flick | 3 |
| `skill-map/src/cursor.rs` `sync_skill_map_held` | `VirtualPad` | routed modifier | 3 |
| `items/firearm-user/src/fire.rs` | `VirtualPad` + `MouseButton` | D1 / B5 | 5 |
| `game/src/lib.rs` `toggle_world_pause` | `VirtualPad` (A vs Start) | B8 | 4 |
| `world/lib/src/player_lifecycle.rs` `drive_respawn_picker` | `VirtualPad` stick / D-pad | shell | 5 |
| `world/lib/src/camera.rs` `fly_world_camera` | `VirtualPad` | playground free-fly | keep (host debug) |
| `world/player/src/player.rs` `keyboard_movement_input` | `VirtualPad` | B11 | 2 |
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
| `CharacterIntent` (B1–B11) | `MessageWriter` / `MessageReader`, no actor | adapter | 2–5 per consumer |
| `apply_move_intents` | reads broadcast, writes every `Player` + `CharacterController` | adapter | 2 |
| `apply_intents_to_movement` | same, every vegetation + player wish | adapter | 2 |
| `apply_look_intents` | broadcast → every `FollowCamera` | adapter | 5 |
| `apply_fire_intents` | pad → every `FirearmUser` with `Player` | adapter | 5 |
| `grenade_user::throw` | broadcast `UseItem` | adapter | 5 |
| `SkillMapFlick` | untargeted message | adapter | 3 |
| `TanzInput` | message **with** `user` but produced from one global pad for every Tanzer | half-targeted | 3 |
| `drive_movement` | each NPC writes its own wish (OK) from a planner, not from pad broadcast | keep shape | 2 (declare as `MotorIntent`) |

## Temporary adapters

Do not add a second copy of these. Remove in the named phase.

| Adapter | Why it exists | Removal |
|---|---|---|
| Broadcast `CharacterIntent` | Consumers are not actor-addressed yet. A5 binds only when `InputRouter` has an actor, then still writes a process-wide message. | Phases 2–5 as each consumer takes `Routed` / `ActorTarget`. Duplicate character / locomotion producers are removed as descendants update. |
| `CharacterControlSystems` ⊂ `Bind` | Existing `.after(CharacterControlSystems)` hosts (player intent, Tanz, world cycling, stash, Lubikan). | Phase 2: those hosts join `ControllerPipeline` sets. |
| Domain motor-request order not in core | Tanz / Standard / WorldFilter names were removed from generic core. | Phase 2 registers those sets beside locomotion / Tanz / world. |
| `PlayerSystems::{Intent,Body}` outside P3 / P4 | Motor apply is already one path; request writers are not in the pipeline sets. | Phase 2. |
| World adapter live `player::MoveWish` write | Discovery vegetation + pause zeroing share `apply_intents_to_movement`. | Phase 2 (world filter zeros only; vegetation motor removed). |
| `MenuOwnsBack` written from menu focus | Character Bind still consults it to skip stance. Capture should own **B**. | Phase 4 (U5 / U6). |
| `ExclusiveNav` fallback in `dispatch_menu_nav` when the router has no menu capture | Hosts that run dispatch without `refresh_menu_focus` (or before it). Refresh always acquires the token when the plugin is installed. | Phase 4 once every host goes through refresh. |
| `exclusive_nav_menu` free function | Thin wrapper around `ExclusiveNav::resolve` for existing imports. | Phase 4. |
| `PadGameplayEnabled` + `KeyboardPadMap` lanes | Global table, not participation install. | Phase 4. |
| `OrdinaryLocomotionEnabled` pad-level resource | Any Tanzer flips walking for everyone. [#1395](https://github.com/ramate-io/maybraid/pull/1395) per-actor flag is the review baseline. | Phase 2 / 4. |
| Direct `VirtualPad` / `Gamepad` reads in the hardware table | Bind is the only blessed reader. | Phases 3 and 5. |

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

## Invariants (checklist)

Copied from [#1436](https://github.com/ramate-io/maybraid/issues/1436) so
later PRs do not drift.

1. Explicit targeting (no query-wide broadcast).
2. Scoped, composable UI capture.
3. One normalized input source (no raw `VirtualPad` / gamepad reads outside `input`).
4. Mode config changes participation, not a per-frame global table.
5. Input blocking is not simulation suspension.
6. Suspension preserves capabilities.
7. Motor ownership: one resolver, one apply path.
8. Gesture fan-out: an unmatched Tanz still drives the map.
9. Lifecycle cleanup.
10. Same-frame behavior for pause and capture.

## Review baselines

Useful commits may be pulled from these branches; do not push to them.

- [#1394](https://github.com/ramate-io/maybraid/pull/1394) `9ba9504d` — explicit `ControllerPipeline` (generic stages now in `controllers/core`). Named Tanz / Standard / WorldFilter sets move to domain crates in phase 2. Motor-writer move is phase 2.
- [#1395](https://github.com/ramate-io/maybraid/pull/1395) `9c683129` — `SessionActivity` + per-actor ordinary locomotion. Phase 4 replaces the global table.

Do not touch [#1342](https://github.com/ramate-io/maybraid/pull/1342).
