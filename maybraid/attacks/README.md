# Attacks

Reusable attack timing, hazards, and telegraphs. Encounter scripts and summoned
owners invoke the same [`AttackDefinition`](src/definition.rs) values; this crate
does not know about Bikanwarda, Self, or any other content cast.

Gameplay writes [`Hit`](../damage/src/lib.rs). Presentation is a subscriber:

```text
BeginAttack → Windup (telegraph) → Execute (hazard Hits) → Recover → cooldown
                 │                      │
                 └─ AttackPresentationEvent ─┘
```

VFX lanes ([#1122](https://github.com/ramate-io/maybraid/issues/1122),
[#1123](https://github.com/ramate-io/maybraid/issues/1123)) listen for
[`AttackPresentationEvent`](src/presentation.rs) and tag spawned effects with
[`AttackPresentationBinding`](src/presentation.rs). Damage still applies when no
presenter is installed.

[`HazardShape::Wavefront`](src/hazard.rs) is a discrete ring / disc emitted from
a mouth or socket. It travels in world space along a horizontal heading (optional
pitch) at a constant radius, not as a radially expanding ground ring. Sequential
waves use an emission interval. Each wave hits a target at most once when cadence
is `<= 0`, otherwise it follows the authored cadence. Hazard presentation events
carry [`WavefrontPresentation`](src/presentation.rs) (`origin`, `direction`,
`speed`, `range`, `radius`, `wave_index`).

Default damage, cooldown, interruption, and friendly-fire are authored in
[`AttackDefaults::AUTHORED`](src/defaults.rs). Callers copy or override those
fields; they are never implied at runtime.

```shell
cargo run -p attacks-playground
```

Software-Vulkan captures of the playground (windup telegraph, execute hazard + hits, post-cancel cleanup):

```shell
cargo build -p attacks-playground
xvfb-run -a -s "-screen 0 1280x720x24" maybraid/attacks/preview/capture.sh
```

Images land in [`preview/`](preview/).
