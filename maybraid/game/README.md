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

## Error recovery

The executable has two boundaries. Neither replaces fixing the bug.

### ECS errors (in-process)

Bevy 0.19 routes failed commands, `Result` systems, observers, run conditions, and system-parameter validation through one fallback handler. Errors at `Severity::Error` or `Severity::Panic` (the default when a site does not set a severity) are logged, appended to `errors.log` next to `panic.log`, and recovered:

| State when the error happened | Destination |
|---|---|
| `GameFlow::World` | The active game: menus and pending show requests are cleared (`MenuScreens::clear`), any character editor session ends (`end_character_editor`), and `WorldPause::Playing` is set |
| `GameFlow::Home` / `GameFlow::Characters` | Home (same-state `set` re-runs `OnExit` / `OnEnter` in 0.19) |
| `GameFlow::LoadingWorld` | Home, after the loading shell tears down |

A short toast names the log file. More than three recoveries in ten seconds, or an error while a recovery is already applying, escalates from the world to Home and then exits (code `2`) if Home keeps failing. Render-world errors are logged and count toward that guard but do not change game state on their own.

`MAYBRAID_STRICT_ERRORS=1` keeps today's panic-on-ECS-error behavior. Development builds and `--agent-*` runs default to strict. Release builds default to recover. A playtest that opts out of strict still treats any recovery as a failure (the process exits `2`).

Sites that expect a miss should keep using `try_*` or `.with_severity(Severity::Warning)` so they do not enter recovery.

### Panics (relaunch)

An `unwrap`, `expect`, out-of-bounds index, or `panic!` still leaves `App::update`. The panic hook writes `panic.log` (see [`panic_log.rs`](src/panic_log.rs)). Release builds then run a supervisor in the same binary: the parent has no window, the child is the game, and an abnormal exit (non-zero, signal, or a `panic.log` newer than the child start) relaunches with `--recover` (Home, or Discovery when `last_session` and the active character are valid). At most three relaunches in two minutes; then the parent stops and prints the log path.

`--agent-*` runs never relaunch, so playtests see the real exit code. `--no-supervise` skips the parent. `MAYBRAID_SUPERVISE=1` forces a supervisor in debug builds; `MAYBRAID_SUPERVISED=1` marks the child.

The supervisor lives in the game binary, not the packaging scripts. A macOS `.app` therefore shows two processes while the game is up (parent + child). Clean window / Cmd+Q exit is `0` and must not relaunch.

### Reading the logs

`MAYBRAID_LOGS` overrides the directory. Otherwise:

- macOS: `~/Library/Logs/Maybraid`
- Windows: `%APPDATA%/Maybraid/logs`
- Linux: `$XDG_STATE_HOME/maybraid/logs` or `~/.local/state/maybraid/logs`

`panic.log` is a Rust panic (process died). `errors.log` is a recovered ECS error (process stayed up).
