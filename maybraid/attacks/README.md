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
