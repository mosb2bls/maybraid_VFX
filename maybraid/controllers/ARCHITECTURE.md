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

Phase 2 realizes ordinary locomotion (B11), the shared motor contract
(`MotorIntent` / one resolver / one apply), NPC request + recovery /
separation modifiers (A6), and WorldFilter suppress. Phase 3 realizes Tanz
mode switch + recognition on routed actor samples, `ExecuteTanz` selection,
and actor/session-addressed `SkillMapFlick`. Phase 4 replaces the #1395
global `SessionActivity` table with [`SessionCapabilities`](world/src/capabilities.rs)
install / remove, a composable [`CaptureStack`](core/src/capture.rs) resolver
(U5), and owned [`SuspensionStack`](core/src/suspension.rs) tokens (U6). The
three operations are distinct: install/remove participation, capture input,
suspend simulation. Character produce is still a broadcast adapter; camera /
item chains stay for phase 5. #1442 / #1443 will change routing and motor
shapes; lifecycle tokens (`CapabilityToken`) are the swap point.

## Dependency direction

```text
bevy / maybraid-input
        ▲
        │
controllers/core          (targeting, capture, schedule — no game deps)
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
  SessionCapabilitySystems           pending pause + scenery/editor write
  camera / item / shell / map        (C*, D*, E*) after Bind + session
  ControllerPipeline::Interpret      (P2)  reset MotorIntent slots
  ControllerPipeline::MotorRequest   (P3)
    MotorRequestStage::Standard      locomotion + NPC drive
    MotorRequestStage::Tanz          Tanz execution (declared slot)
    MotorRequestStage::WorldFilter   pause / capture / map suppress
    recovery + separation modifiers
  ControllerPipeline::Motor          (P4)  resolve + apply, then PlayerSystems::Body

PostUpdate
  PadRumbleSystems
```

`ControllerPipeline` / `MotorRequestStage` are the contract. Plugin install
order is not. Phase 2 keeps the #1394 names and replaces overwrite priority
with one actor-local resolver.

Same-frame pause and capture (invariant 10): menu focus + capture write run
in the same `Update` chain as dispatch. `SessionCapabilitySystems` honors a
pending `WorldPause` after Bind and before map / items / WorldFilter. Pause
declares `MotorIntent` suppress on the local player; apply releases sprint,
jump, and caps. NPC slots and installed motors stay. Capture blocks the
local pad without suspending simulation.

## Role inventory

Each row is a spec role. **Symbol** is the live type or system. **Now** is
the current owner. Later phases change the owner column, not the role id.

### A — controllers / sources

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| A1 | Device HID | `bevy_input` `Gamepad` / `ButtonInput` / mouse; Apple `hid/gamecontroller.rs` | OS / gilrs / GameController | `input::produce::{keyboard,mouse,gamepad}` | `VirtualPadPlugin` | Hardware only. Stays in `input`. |
| A2 | Virtual-pad controller | `VirtualPad`, `VirtualPadSystems` | A1 producers + `PadOverlay` | A3–A5, plus leftover hardware readers below | `VirtualPadPlugin` | Canonical analog + edges. |
| A3 | Input-routing controller | `InputSource`, `Routed<T>`, `RouteTarget` | A2 (`InputSource::LocalPad`) | A4 / A5 / U5 | `ControllersCorePlugin` | New in phase 1. One local pad. |
| A4 | Menu controller | `MenuControllerPlugin`, `refresh_menu_focus`, `dispatch_menu_nav` | A2 `MenuNavPad` | M2 via `MenuNavImpulse` | `MenuControllerPlugin` | Targeted path (see M1). |
| A5 | Character controller | `produce_character_intents` → `CharacterIntent`; `RoutedActorControl` | A2 `VirtualPad` | B* intents (broadcast) + B12 / B13 routed sample | `CharacterControllerPlugin` | Bind. Intents still untargeted; analog/A/B are actor-addressed. |
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
| B11 | Ordinary locomotion | `controllers/locomotion` `interpret_standard_locomotion` | A5 + `LocalActor` / lowest-id `Player` | `MotorIntent` Standard slot | `LocomotionControllerPlugin` | **Phase 2.** One actor. |
| B12 | Tanz gesture | `TanzInput` (`user: Entity`) | `tanz_user::recognize_tanz_inputs` from `RoutedActorControl` | Tanz match → `ExecuteTanz`; skill-map fan-out | `TanzUser` component | **Phase 3/4.** One routed actor. Mode change writes `OrdinaryLocomotion` once for that actor; no global `SkillMapSteerPolicy` restamp. |
| B13 | Standard flick | `SkillMapFlick` | `SkillMapController::detect_skill_map_flicks` from `RoutedActorControl` | skill-map interpreter | `SkillMapController` resource | **Phase 3.** Actor/session-addressed; `look_throw`, not raw `Gamepad`. |

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
| E1 | In-game menu / pause | `game::toggle_world_pause` + `resolve_session_capabilities` | B8 | `WorldPause`, `SuspensionStack::set_pause` | `GamePlugin` | **Phase 4.** Toggle only sets `NextState`; session systems write the pause token the same frame. |
| E2 | Inventory sheet | `stash.rs` Inventory reader | B9 | inventory editor | world | Broadcast. Live edit follow is the scenery/editor writer. |
| E3 | World map | `map_view.rs` ToggleMap reader | B10 | `WorldMapView` + map `CaptureLayer` | world | Broadcast. Camera lock/follow is one writer. |
| E4 | Session gameplay gate | `SessionCapabilities` + `SuspensionStack` → `WorldGameplayEnabled` / physics / streaming / menu-shell | E1, flow enter, leave | B11 world filter, HUD, cycling | `WorldControllerPlugin` | **Phase 4.** Install / remove. Suspend is a separate owned token. |

### U — routing, capture, lifecycle

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| U1 | Source routing | `InputSource` | A3 | `Routed`, `CaptureLayer` | `controllers/core` | Local pad only. |
| U2 | Actor routing | `ActorTarget`, `LocalActor`, `RoutedActorControl` | locomotion / hosts | B11 / B12 / B13 | `controllers/core` + character Bind | **Phase 3.** Local pad actor. |
| U3 | UI routing | `UiTarget`, `RouteTarget::Ui` | A4 | M1 / M2 | `controllers/core` + menu | Used by the migrated menu path. |
| U4 | Focus pick | `MenuController::pick_focus` / `resolve` | screen subtree | A4 | `MenuController` | Overlay > HUD > text. |
| U5 | Scoped capture | `CaptureStack`, `CaptureLayer`, `CapabilityToken` | A4 refresh, text lock, console, map | A4 dispatch, A5 produce, WorldFilter | `ControllersCorePlugin` | **Phase 4.** Nested claims release by token. Capture owns **B**. Not simulation halt. |
| U6 | Suspension | `SuspensionStack`, `SuspensionClaim` | E1 pending pause | WorldFilter, stamp (motors stay) | `WorldControllerPlugin` | **Phase 4.** Owned freeze; releasing pause cannot release another token. NPCs keep simulating. |
| U7 | Lifecycle cleanup | Tanz `reset_input` / `apply_toggle`; death despawn; `uninstall_play` | mode flip, death, Home | wishes, execution, items, motors | per-domain + session | Shared uninstall on leave. Phase 5 still owns item/death edges. |

### M — menu

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| M1 | Menu controller | `MenuController`, `ExclusiveNav`, `dispatch_menu_nav` | A2 `MenuNavPad` | M2 | `MenuControllerPlugin` | **Phase 1 migrated path.** `Routed<MenuNav>` → `MenuNavImpulse` on one `UiTarget`. Refresh writes `CaptureStack`; dispatch does not re-resolve exclusive nav. |
| M2 | Menu consumer | `On<MenuNavImpulse>` in `TextMenu`, `HudMenu`, scroll, short-text, screens, playground prompts | M1 | widget state, `MenuActivate` | each widget | Already entity-addressed. Keep. |

### P — pipeline / motor

| Id | Role | Symbol | Producers | Consumers | Lifecycle owner | Now |
|---|---|---|---|---|---|---|
| P1 | Bind | `ControllerPipeline::Bind` | A4, A5 | C* / D* / E* / B11 | `ControllersCorePlugin` | Live. Character produce + menu dispatch. |
| P2 | Interpret | `reset_motor_intents` | — | P3 | `PlayerPlugin` | **Phase 2.** Clears slots each frame. |
| P3 | Motor request | `MotorRequestStage::{Standard,Tanz,WorldFilter}` + recovery / separation | B11, B12, A6, world | P4 | named sets | **Phase 2.** Declared slots only. |
| P4 | Motor apply | `apply_motor` then `PlayerSystems::Body` | `ResolvedMotorRequest` | `MoveWish` / jump / sprint / cap | `PlayerPlugin` | **Phase 2.** One resolver, one apply. |

## Realized-state writer inventory

Who currently writes the live motor / pose bits that interpreters should own.

| State | Writer | File | Policy | Role after the stack |
|---|---|---|---|---|
| `CharacterIntent` | `produce_character_intents` | `controllers/character/src/produce.rs` | `CaptureStack` + per-actor `OrdinaryLocomotion` | A5 Bind. Single producer. Becomes `Routed` per actor. |
| `MotorIntent` Standard slot | `interpret_standard_locomotion` | `controllers/locomotion/src/standard.rs` | `LocalActor` / lowest-id `Player`; per-actor `OrdinaryLocomotion` | P3 Standard. One actor. |
| `RoutedActorControl` | `produce_actor_control_samples` / `route_actor_control_samples` | `controllers/character` + `controllers/locomotion` | `LocalActor` / lowest-id `Player` | Bind. Analog + physical A/B for one actor. |
| `ExecuteTanz` | `match_tanz_inputs` | `items/tanz-user/src/execute.rs` | ordered `TanzStack` + cooldown | Selection only. Execution submits. |
| `MotorIntent` Tanz slot | `apply_tanz_movement` | `items/tanz-user/src/execute.rs` | `TanzUser.mode.is_tanzer()` | P3 Tanz. Declares; apply owns cap / sprint release. |
| `SkillMapFlick` | Tanz `publish_tanz_map_inputs` / Standard `detect_skill_map_flicks` | `tanz-user` / `skill-map` | actor + optional session | Independent of Tanz match. |
| `MotorIntent` suppress | `apply_world_motor_filter` | `world/lib/src/control.rs` | suspended gameplay / capture / map / non-character mode | P3 WorldFilter. Player only; zeros leftover vegetation `MoveWish`. NPCs keep their slot. |
| `MotorIntent` NPC slot | `drive_movement` | `intelligence/movement` | `MovementIntelligence` | A6. Declares. |
| `MotorIntent` recovery | `realize_movement` | `movement-realization` | `Without<Player>` | P3 modifier. |
| `MotorIntent` separation | `declare_npc_soft_bump` | `player/src/separation.rs` | `Npc` | P3 modifier. |
| `MoveWish` / `JumpWish` / `Sprinting` / `MoveSpeedCap` | `apply_motor` | `player/src/motor.rs` | resolved request | **Only apply path.** |
| mode flip | `TanzUser::apply_toggle` / `set_mode` | `items/tanz-user/src/lib.rs` | bumper chord / agent | Clears execution only. Resolver releases stale sprint / jump / cap. |
| vegetation `MoveWish` / `MovementAction` | `keyboard_movement_input` | `world/player/src/player.rs` | `PadMovementEnabled` (false in Discovery) | Playground host only. Discovery duplicate removed. |
| velocity | `apply_character_movement` | `world/player/src/player.rs` | `VegetationPlayerMotor` (false in Discovery) | Playground host motor. |
| `CameraController` look / POV | `apply_look_intents` | `player-camera/src/look.rs` | `CameraLookSuppressed`, `CameraPovLocked` | C1–C4. Phase 5. |
| `WeaponTrigger` | `apply_fire_intents` | `items/firearm-user/src/fire.rs` | `Player` + `FirearmUser` | D2. Phase 5. |
| `WorldGameplayEnabled` | `resolve_session_capabilities` / `apply_shell_look` | `game/src/shell.rs` | `SessionCapabilities::gameplay` | E1 / E4. Projection only. |
| `WorldSceneryVisible` / `InventoryEditCameraFollow` | `sync_scenery_and_editor` | `game/src/shell.rs` | session scenery + pause editor | One writer. |
| `CameraPovLocked` / `CameraLookSuppressed` / `CameraFollow` | `sync_camera_session_state` | `world/lib/src/camera.rs` | map + inventory edit + gameplay | One writer. |

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
| `game/src/lib.rs` `toggle_world_pause` | `VirtualPad` (A vs Start) | B8 | leftover; Start vs A still distinguished here |
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
| **Menu nav (M1)** | `Routed<MenuNav>` → `MenuNavImpulse` on one `UiTarget`; exclusive modal writes `CaptureStack` | **Migrated** | — |
| `CharacterIntent` (B1–B11) | `MessageWriter` / `MessageReader`, no actor | adapter | 2–5 per consumer |
| `apply_move_intents` | removed | **Phase 2** | locomotion + `apply_motor` |
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
| Broadcast `CharacterIntent` | Consumers are not actor-addressed yet. A5 still binds from the local pad into a process-wide message. | Phases 3–5 as each consumer takes `Routed` / `ActorTarget`. B11 consumes it into one actor. |
| `CharacterControlSystems` ⊂ `Bind` | Leftover `.after(CharacterControlSystems)` item / camera / shell hosts. Motor hosts joined the pipeline. | Phases 3–5. |
| `MotorRequestStage` sets with no members | **Removed.** Standard / Tanz / WorldFilter have writers. | — |
| `PlayerSystems::Body` outside P4 | **Removed.** Body is in `ControllerPipeline::Motor`. Intent stays for item users (D*). | — |
| World adapter live `player::MoveWish` write | **Removed.** WorldFilter declares suppress only. | — |
| `MenuOwnsBack` written from menu focus | **Removed.** Capture owns **B**. | — |
| `ExclusiveNav` fallback in `dispatch_menu_nav` | **Removed.** Dispatch reads `CaptureStack` only. | — |
| `exclusive_nav_menu` free function | **Removed.** Call `ExclusiveNav::resolve`. | — |
| `PadGameplayEnabled` + `KeyboardPadMap` lanes | **Removed.** `CaptureStack::pad_policy` writes `PadInputPolicy`. | — |
| `OrdinaryLocomotionEnabled` pad-level resource | **Removed.** Per-actor `OrdinaryLocomotion` is the participation bit. | — |
| Global `SkillMapSteerPolicy` restamp from Tanz mode | **Removed.** Mode change writes that actor's `OrdinaryLocomotion` once. | — |
| Direct `VirtualPad` / `Gamepad` reads in the hardware table | Bind is the only blessed reader. Tanz / skill-map migrated. | Phase 5 leftovers. |
| `ActorTarget` unused by a live path | **Removed.** B11 uses `LocalActor` / `ActorTarget`. | — |

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

- [#1394](https://github.com/ramate-io/maybraid/pull/1394) `9ba9504d` — explicit `ControllerPipeline` / `MotorRequestStage` (now in `controllers/core`). Motor-writer move is phase 2.
- [#1395](https://github.com/ramate-io/maybraid/pull/1395) `9c683129` — `SessionActivity` + per-actor ordinary locomotion. Phase 4 replaced the global table with `SessionCapabilities`.

Do not touch [#1342](https://github.com/ramate-io/maybraid/pull/1342).
