# Agent drive / observe

Text commands and events so a slow-inference agent (or a Tracy flight) can
drive Discovery without frame-by-frame steering.

The crate is Bevy-free. Parse, script load, and clock playback live here.
The world plugin (only when `--agent-script` / `--agent-pipe` is set) copies
each [`PadIntent`](src/pad.rs) onto the virtual pad after HID produce, so
injected input uses the same `VirtualPad` → `CharacterIntent` path as a
gamepad or keyboard.

## Opt-in

```bash
maybraid --start-at=0,0 \
  --agent-script maybraid/agent/scripts/origin-walk.jsonl \
  --agent-events /tmp/maybraid-events.jsonl \
  --agent-quit
```

`--agent-pipe PATH` (`-` = stdin) appends live JSON lines behind the script.
`MAYBRAID_AGENT_SCRIPT`, `MAYBRAID_AGENT_EVENTS`, `MAYBRAID_AGENT_PIPE`, and
`MAYBRAID_AGENT_QUIT` are the env equivalents. With none of those set the
plugin is not added.

## Commands

Line-delimited JSON. `#` and `//` comments are skipped. Internally tagged
with `"cmd"`.

| `cmd` | Fields | Clock |
|---|---|---|
| `start` | `xz` | spawn point when leading; otherwise like `teleport` |
| `hold` | `move`, `look` (`[x,y]`, gamepad-style −1..1), `focus`, `fire`, `buttons`, `secs` | holds for `secs` |
| `press` | `button` | one frame |
| `wait` | `secs` | idle |
| `goto` | `xz`, `timeout` (default 30), `radius` (default 3), `sprint`, optional `look` | steer until reached |
| `teleport` | `xz`, optional `y` | instant (utility) |
| `console` | `line` (`stats vsync`, `sky noon`, …) | instant |
| `scenario` | `name`, `mark` (`start` / `end`) | instant; Tracy message |
| `quit` | | request process exit |

Buttons: `jump` (`a`), `sprint` (`l3`), `stance` (`b`), `interact` (`x`),
`swap` (`y`), `ads` (`lb`), `fire` (`rt`), `focus` (`lt`), `menu`, `map`,
`swap_pov` (`r3`), d-pad.

See [`scripts/origin-walk.jsonl`](scripts/origin-walk.jsonl),
[`scripts/origin-idle.jsonl`](scripts/origin-idle.jsonl),
[`scripts/biome-traverse.jsonl`](scripts/biome-traverse.jsonl),
[`scripts/locomotion-mix.jsonl`](scripts/locomotion-mix.jsonl),
[`scripts/crowd-16.jsonl`](scripts/crowd-16.jsonl), and
[`scripts/remote-start.jsonl`](scripts/remote-start.jsonl).

Profile scripts mark a `*-warmup` scenario (stream-in / first steps) and then
the measure window. `goto.look` holds the look stick while steering so a
traversal can sweep the camera without a separate `hold`.

Do **not** send `stats vsync` when the process already has `MAYBRAID_VSYNC=off`:
that command *toggles*, so it turns vsync back on and locks the Frame plot.

## Start location

A script picks where it starts with a `start` command before its first
duration command (`hold` / `press` / `wait` / `goto`):

```json
{"cmd":"start","xz":[-1500,-600]}
```

That replaces `--start-at` (and `MAYBRAID_START_AT`), so the player spawns
there and terrain streams in there — no origin load followed by a teleport.
The game logs `script start X,Z replaces --start-at`. A `start` after the
first duration command, or one sent over `--agent-pipe`, behaves like
`teleport`. Without a `start`, the script runs from `--start-at` (the flight
scripts pass `0,0`). See [`scripts/remote-start.jsonl`](scripts/remote-start.jsonl).

## Script API version

[`SCRIPT_API_VERSION`](src/command.rs) is logged when a script loads. Bump
it when a command is added or changes meaning, and add a row here. Scripts
published outside the repo (Traces releases) state the version they target.

| Version | Change |
|---|---|
| 1 | `hold`, `press`, `wait`, `goto` (+ `look`), `teleport`, `console`, `scenario`, `quit` |
| 2 | `start` — script-chosen spawn point |

## Events

Line-delimited JSON to `--agent-events`. Every line has `t` (game seconds)
and `frame`.

| `kind` | When |
|---|---|
| `ready` | world surface + gameplay are up (or ready-timeout) |
| `player` | periodic pose (`x,y,z`, `yaw`, `move`) |
| `command` | `acknowledged` / `started` / `finished` / `failed` |
| `waypoint` | `goto` reached or timed out |
| `scenario` | script markers |
| `chunk_loads` | count of new cascade chunks this frame (cheap summary) |
| `actors` | `hosts` / `bodies` / `brains` census when it changes (at most once per sample period) |
| `quit` | process is exiting |

Scenario / command / waypoint / ready also emit Tracy messages when the
binary is built with `bevy/trace_tracy`.
