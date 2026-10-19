# World controller

Owns [`WorldGameplayEnabled`](src/enabled.rs): the live-play flag that
gates character intents, combat HUD, skill-map sync, inventory cycling,
and Lubikan activation.

`WorldGameplayEnabled::accepts_character_input` is live play with no text
console and no world map sheet. `WorldGameplayEnabled::combat_hud_visible`
is the same without the console (vitals stay up while typing).

Pause writes this resource. Pause is not a play-session teardown.
