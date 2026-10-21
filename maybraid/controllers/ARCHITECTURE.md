# Composable controllers

Tracking: [#1436](https://github.com/ramate-io/maybraid/issues/1436).
This document is the stacked phase 1–3 contract
([#1437](https://github.com/ramate-io/maybraid/issues/1437),
[#1438](https://github.com/ramate-io/maybraid/issues/1438),
[#1439](https://github.com/ramate-io/maybraid/issues/1439)). Later phases
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
and `RoutingEvent` cancel / freeze. Character produce is still a
broadcast adapter; camera / item / shell chains stay for later phases.

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
controllers/locomotion / tanz-user / intelligence / player-camera / skill-map / items
```

Rules:

- `controllers/core` depends only on `maybraid-input` and Bevy.
- `controllers/character` depends on `core` + `input`.
- The player motor contract (`MotorIntent` / `ResolvedMotorRequest`) must not
  depend on locomotion or Tanz.
- Locomotion and Tanz write the motor contract; they do not own apply.
- Apply realizes translation / jump / stance / sprint / cap without checking
  whether the producer was Tanz.
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
    CharacterControlSystems ⊂ Bind   (item / camera / shell alias)
  camera / item / shell branch       (C*, D*, E*) after Bind
  ControllerPipeline::Interpret      (P2)  reset MotorIntent slots
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
optional UI assignment. Assignment loss, disconnect, and removal clear
delivery and emit `HoldEffect::ReleaseNeutral` + `GestureEffect::Cancel`.
Suspension emits `HoldEffect::Freeze` + `GestureEffect::Freeze`. Captures and
suspensions are token- and owner-scoped: releasing one owner cannot release
another.

Controllers read `InputRouter::actor` / `delivered_actor` / `local_actor`.
They never rediscover a "current player". The lowest-ID `Player` fallback is
resolved **once** at the host assignment boundary
(`InputRouter::assign_lowest_id_actor`, called from `player`'s
`assign_local_player`).

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
| B5 | Use item | `CharacterIntent::UseItem` | A5 | D1 / D2 / D3 | character crate | Broadcast. Firearm-user also re-reads the pad (D2). |
| B6 | Power-use item | `CharacterIntent::PowerUseItem` | A5 | D7 `world_lubikan` | character crate | Broadcast. |
| B7 | Equipped queue | `SwapActive`, `CycleEquippedQueue`, `SelectInventoryCyclingCategory` | A5 | D5 / D6 | character crate | Broadcast. |
| B8 | In-game menu | `CharacterIntent::InGameMenu` | A5 | E1 `toggle_world_pause` | character crate | Broadcast. |
| B9 | Inventory | `CharacterIntent::Inventory` | A5 | E2 `stash.rs` | character crate | Broadcast. |
| B10 | Toggle map | `CharacterIntent::ToggleMap` | A5 | E3 `map_view.rs` | character crate | Broadcast. |
| B11 | Ordinary locomotion | `controllers/locomotion` `interpret_standard_locomotion` | A5 + `InputRouter::local_actor` | `MotorIntent` Standard contribution | `LocomotionControllerPlugin` | **Phase 2.** One routed actor. No lowest-ID lookup. |
| B12 | Tanz gesture | `TanzInput` (`user: Entity`) | `tanz_user::recognize_tanz_inputs` from `RoutedActorControl` | Tanz match → `ExecuteTanz`; skill-map fan-out | `TanzUser` component | **Phase 3.** Source/actor-owned history; physical A/B, not Jump. `RoutingEvent` cancels partial gestures. |
| B13 | Standard flick | `SkillMapFlick` | `SkillMapController::detect_skill_map_flicks` from `RoutedActorControl` | skill-map interpreter | `SkillMapController` resource | **Phase 3.** Actor/session-addressed; `look_throw`. History scoped to owner; session change cannot complete against the new session. |

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
| U2 | Actor routing | `ActorTarget`, `InputRouter::actor`, `RoutedActorControl` | host `assign_local_player` | A5; B11 / B12 / B13 | `InputRouter` (sole owner) | Lowest-ID `Player` assigned **once** at the host boundary. Controllers do not rediscover. |
| U3 | UI routing | `UiTarget`, `RouteTarget::Ui`, `InputRouter::ui` | A4 capture; host `assign_ui` | M1 / M2 | `InputRouter` | Menu path uses token-scoped capture; UI assignment is available for hosts. |
| U4 | Focus pick | `MenuController::pick_focus` / `resolve` | screen subtree | A4 | `MenuController` | Overlay > HUD > text. |
| U5 | Scoped capture | `CaptureToken`, `ScopeOwner`, `CaptureStack` via `InputRouter` | A4 `refresh_menu_focus` (`MenuNavCapture`); console / map / text | A4 dispatch; A5 `delivered_actor`; pad policy | `InputRouter` | Token- and owner-scoped. Nested menu / text / overlay. Updating a claim keeps stack order. |
| U6 | Suspension | `SuspensionToken`, `InputRouter::suspend` / `set_paused` | E1 pause; hosts | A5 delivery; Tanz execute (session-local); E4 projection | `InputRouter` | Delivery freeze. Motor freeze is `MotorSuppressionToken`. Pause is not session end. |
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
| P2 | Interpret | `reset_motor_intents` | — | P3 | `PlayerPlugin` | **Phase 2.** Clears declared contributions each frame. `MotorSelection` persists. |
| P3 | Motor request | `player::MotorRequestStage` + recovery / separation | B11, B12, A6, world | P4 | domain sets beside those systems | **Phase 2.** Declared contributions only. |
| P4 | Motor apply | `compose_motor` then `apply_motor` then `PlayerSystems::Body` | `ResolvedMotorRequest` | `MoveWish` / jump / sprint / cap | `PlayerPlugin` | **Phase 2.** Compose selects; apply realizes without producer checks. |

## Realized-state writer inventory

Who currently writes the live motor / pose bits that interpreters should own.

| State | Writer | File | Policy | Role after the stack |
|---|---|---|---|---|
| `CharacterIntent` | `produce_character_intents` | `controllers/character/src/produce.rs` | `InputRouter::actor`, capture blocks, per-actor `OrdinaryLocomotion` | A5 Bind. Single producer. Becomes `Routed` per actor. |
| `RoutedActorControl` | `produce_actor_control_samples` | `controllers/character/src/sample.rs` | `InputRouter::delivered_actor` | Bind. Analog + physical A/B for one actor. Sole pad sample for B12 / B13. |
| `ExecuteTanz` | `match_tanz_inputs` | `items/tanz-user/src/execute.rs` | ordered `TanzStack` + cooldown | Selection only. Independent of `SkillMapFlick`. |
| `MotorIntent` Standard | `interpret_standard_locomotion` | `controllers/locomotion/src/standard.rs` | `InputRouter::local_actor`; per-actor `OrdinaryLocomotion` | P3 Standard. One routed actor. |
| `MotorIntent` Tanz | `apply_tanz_movement` | `items/tanz-user/src/execute.rs` | `TanzUser.mode.is_tanzer()` | P3 Tanz. Declares sprint/cap in the sample. Compose selects; apply does not inspect Tanz. |
| `SkillMapFlick` | Tanz `publish_tanz_map_inputs` / Standard `detect_skill_map_flicks` | `tanz-user` / `skill-map` | actor + optional session | Independent of Tanz match / execution success. |
| `MotorIntent` suppress | `apply_world_motor_filter` | `world/lib/src/control.rs` | `WORLD_GAMEPLAY` on Player; `SIMULATION` on every `CharacterController` when the Simulation bit is absent | P3 WorldFilter. Token-owned; zeros leftover vegetation `MoveWish`. |
| `MotorIntent` NPC | `drive_movement` | `intelligence/movement` | `MovementIntelligence` | A6. Declares. |
| `MotorIntent` recovery | `realize_movement` | `movement-realization` | `MotorModifier::Recovery` | P3 modifier. |
| `MotorIntent` separation | `declare_npc_soft_bump` | `player/src/separation.rs` | `MotorModifier::Separation` | P3 modifier. |
| `ResolvedMotorRequest` | `compose_motor` | `player/src/motor.rs` | Tanz else NPC else Standard; then modifiers / suppression | Composition only. |
| `MoveWish` / `JumpWish` / `Sprinting` / `MoveSpeedCap` | `apply_motor` | `player/src/motor.rs` | composed request | **Only apply path.** |
| mode flip | `TanzUser::apply_toggle` / `set_mode` | `items/tanz-user/src/lib.rs` | bumper chord / agent | Clears execution only. Contributor loss releases stale sprint / jump / cap. |
| vegetation `MoveWish` / `MovementAction` | `keyboard_movement_input` | `world/player/src/player.rs` | `PadMovementEnabled` (false in Discovery) | Playground host only. Discovery duplicate removed. |
| velocity | `apply_character_movement` | `world/player/src/player.rs` | `VegetationPlayerMotor` (false in Discovery) | Playground host motor. |
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
| `items/tanz-user` | — | `RoutedActorControl` (A5 / U2) | **Phase 3 migrated** |
| `skill-map/src/controller.rs` / `cursor.rs` | — | `RoutedActorControl` + addressed `SkillMapFlick` | **Phase 3 migrated** |
| `items/firearm-user/src/fire.rs` | `VirtualPad` + `MouseButton` | D1 / B5 | 5 |
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
| `CharacterIntent` (B1–B11) | `MessageWriter` / `MessageReader`, no actor | adapter | 3–5 per consumer. B11 consumes via `InputRouter::local_actor`. |
| `apply_move_intents` | removed | **Phase 2** | locomotion + compose / apply |
| `apply_intents_to_movement` | removed live writes | **Phase 2** | `apply_world_motor_filter` suppress |
| `apply_look_intents` | broadcast → every `FollowCamera` | adapter | 5 |
| `apply_fire_intents` | pad → every `FirearmUser` with `Player` | adapter | 5 |
| `grenade_user::throw` | broadcast `UseItem` | adapter | 5 |
| `SkillMapFlick` | actor/session-addressed | **Phase 3 migrated** | — |
| `TanzInput` | produced from `RoutedActorControl` for one actor | **Phase 3 migrated** | — |
| `drive_movement` | each NPC declares its own `MotorIntent` (OK) from a planner | **Phase 2** | keep |

## Temporary adapters

Do not add a second copy of these. Remove in the named phase.

| Adapter | Why it exists | Removal |
|---|---|---|
| Broadcast `CharacterIntent` | Consumers are not actor-addressed yet. A5 binds only when `InputRouter` has an actor, then still writes a process-wide message. | Phases 3–5 as each consumer takes `Routed` / `ActorTarget`. B11 already reads the router. |
| `CharacterControlSystems` ⊂ `Bind` | Leftover `.after(CharacterControlSystems)` item / camera / shell hosts. Motor hosts joined the pipeline. | Phases 3–5. |
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
| Disconnect | `ReleaseNeutral` | `Cancel` | Assignment cleared; `delivered_*` is `None` |
| Removal (actor / UI gone, or host `clear_*`) | `ReleaseNeutral` | `Cancel` | That side of the assignment is cleared |
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

## Review baselines

Useful commits may be pulled from these branches; do not push to them.

- [#1394](https://github.com/ramate-io/maybraid/pull/1394) `9ba9504d` — explicit `ControllerPipeline` (generic stages now in `controllers/core`). Named Tanz / Standard / WorldFilter sets live beside domain systems.
- [#1395](https://github.com/ramate-io/maybraid/pull/1395) `9c683129` — `SessionActivity` + per-actor ordinary locomotion. Phase 4 replaces the global table.

Do not touch [#1342](https://github.com/ramate-io/maybraid/pull/1342).
