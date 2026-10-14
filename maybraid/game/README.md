# Maybraid Game

The actual Maybraid game.

Starts on the home screen ([`HomeMenuChoice`](../menu/screens/src/home.rs)):

| Row | Behavior |
| --- | --- |
| **Discovery** | Enters the streamed world (`PlaySession::Discovery`). Locked until the save has at least one character. |
| **Characters** | Opens the character screen. Shows **Start here** when the roster is empty. |
| **Reliquary** | Shown as **Coming soon** (locked). The executable route is still unimplemented. |
| **Training Ground** | Shown as **Temporarily removed** (locked) while it is rebuilt. |
| **Settings** | Opens the shared in-game settings overlay (shadows, render scale, mob HUD). **Back** / **Escape** / **B** returns to the home screen. |

In world, **Start** (**Enter** on keyboard) toggles the in-game pause menu.
**Start** again resumes play. **Leave** on that menu returns home. **Settings**
there opens the same overlay; **Back** dismisses back to the pause menu (or the
character screen when opened from pause).

```bash
cargo run -p maybraid
```

Headless / Tracy flights can inject pad input from a script (same
`VirtualPad` path as a gamepad). See [`maybraid/agent/README.md`](../agent/README.md).

```bash
MAYBRAID_VSYNC=off cargo run -p maybraid -- \
  --start-at=0,0 \
  --agent-script maybraid/agent/scripts/origin-walk.jsonl \
  --agent-events /tmp/maybraid-events.jsonl \
  --agent-quit
```

Assets live in this crate’s `assets/` symlink (`maybraid/assets`). A packaged
`.app` reads `Contents/Resources/assets` instead; see [packaging/](../../packaging/).

```bash
packaging/scripts/package-macos.sh
open dist/Maybraid.app
```
