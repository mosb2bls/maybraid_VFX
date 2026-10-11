# Maybraid Game

The actual Maybraid game.

Starts on the home screen. Discovery enters the streamed world. Characters opens
the character screen. Reliquary is not wired yet. Training Ground is listed but
temporarily removed until it is rebuilt. In world, Start (Enter on
keyboard) overlays the in-game menu; Start again dismisses it. Leave on that
menu returns home. Settings opens the same pause settings overlay (shadows, render
scale, mob HUD); Back / Escape returns home.

```bash
cargo run -p maybraid
```

Assets live in this crate’s `assets/` symlink (`maybraid/assets`). A packaged
`.app` reads `Contents/Resources/assets` instead; see [packaging/](../../packaging/).

```bash
packaging/scripts/package-macos.sh
open dist/Maybraid.app
```
