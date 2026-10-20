# Character controller

Maps [`VirtualPad`](../../input/README.md) onto gameplay intents. Analog is
the current stick / trigger. Digital is a press or release edge. Chords are
resolved here so consumers do not re-read the pad.

Bind runs in [`ControllerPipeline::Bind`](../core/src/schedule.rs) from
[`controllers/core`](../core/README.md). Intents are still a process-wide
[`CharacterIntent`](src/intent.rs) broadcast — that adapter is removed as
consumers become actor-addressed (phases 2–5). See
[ARCHITECTURE.md](../ARCHITECTURE.md).

`RightTrigger` + `X` is `PowerUseItem` and suppresses `StartInteraction`.
L3 hold is sprint (`StartSprint` every hold frame, `StopSprint` on release).
Shift maps to L3. R3 click swaps first / third person.
LT is optic `Focus`; left bumper is iron `Ads`. **B** tap is `ChangeSquat`,
**B** hold (~0.35 s) is `ChangeProne`. Menu still owns **B** as `MenuNav::Back`
when a `MenuController` has focus; character collect skips stance then.
Discover maps read stick flicks in `maybraid-skill-map`, not a bumper chord.
