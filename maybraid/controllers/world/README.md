# World controller

Compatibility re-export of [`maybraid-session`](../../gameplay/session/src/lib.rs).
Session capabilities live in `gameplay/session`. This crate keeps the existing
package name (`maybraid-world-controller`).

Owns [`SessionCapabilities`](../../gameplay/session/src/capabilities.rs): actor/session participation
that is installed or removed. Delivery freeze is an owned
[`InputRouter`](../core/src/router.rs) [`SuspensionToken`](../core/src/router.rs).
Motor freeze uses [`player::MotorSuppressionToken`].
[`WorldGameplayEnabled`](../../gameplay/session/src/suspension.rs) is the live-play projection of
installed gameplay minus the pause suspension.

Capabilities are independent — not a game-mode enum and not a per-frame
`SessionActivity` table. Loading streams with gameplay off. Pause writes the
pause token; motors stay installed; NPCs keep simulating. Leave / Home
uninstalls play and resumes the pause owner only.

Capture (U5) is a third operation: it redirects or blocks delivery without
freezing simulation. [`SessionCapabilitySystems`](../../gameplay/session/src/lib.rs) applies a
pending pause after Bind and before ResetMotorRequests / Tanz execute / WorldFilter.

`apply_world_motor_filter` declares token-owned `MotorIntent` suppression
for the local player (`WORLD_GAMEPLAY`) or every `CharacterController`
(`SIMULATION` when the Simulation bit is absent). It does not write live
`player::MoveWish` or the Discovery vegetation motor path.

Standard ↔ Tanzer is per-actor participation, written once on change. It
does not restamp a global mode table.

Item / camera / Lubikan interpreters join this session after Bind.
[`PlayerSystems::Intent`](../../player/lib/src/lib.rs) is ordered after
[`SessionCapabilitySystems`](../../gameplay/session/src/lib.rs) so item realization sees the
same-frame pause. See [ARCHITECTURE.md](../ARCHITECTURE.md) for the phase 5
D1–D7 and C5 contracts.
