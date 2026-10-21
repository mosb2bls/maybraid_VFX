# World controller

Owns [`WorldGameplayEnabled`](src/enabled.rs): the live-play flag that
gates character intents, combat HUD, skill-map sync, inventory cycling,
and Lubikan activation.

`WorldGameplayEnabled::accepts_character_input` is live play with no text
console and no world map sheet. `WorldGameplayEnabled::combat_hud_visible`
is the same without the console (vitals stay up while typing).

Pause writes this resource. Pause is not a play-session teardown.
`apply_world_motor_filter` declares token-owned `MotorIntent` suppression
for the local player in `MotorRequestStage::WorldFilter`; it does not write
live `player::MoveWish` or the Discovery vegetation motor path.
