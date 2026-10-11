# Tracy flights

Release `maybraid` with Bevy `trace_tracy`, driven by
[`maybraid/agent`](../maybraid/agent/README.md) scripts, captured on this
machine's GPU (not lavapipe).

Capture with **`tracy-capture` only** (Nix Tracy 0.13.1). Do not also open
the Tracy GUI (`tracy` / `tracy-profiler`) — only one listener can take the
game's connection, and a competing app will steal or drop the flight.

Bevy 0.19.1 emits via `tracy-client` 0.18.4 / `tracy-client-sys` 0.28.0
(protocol **0.13.1**). Match that capture binary; do not mix Homebrew 0.14.x.

```bash
nix develop --command bash -lc '
  cargo build -p maybraid --release --features bevy/trace_tracy
  profiling/tracy-flight.sh origin-idle baseline 1
'
```

## Capture output stays out of git

`tracy-flight.sh` writes under `profiling/captures/` by default
(override with `MAYBRAID_TRACY_CAPTURES`). That directory, `*.tracy`,
unwrap/zone/message CSVs, event JSONL, and capture/game logs are
`.gitignore`d. Do not commit them.

Do **not** write `tracy-csvexport -u` unwrap CSVs — they are hundreds of
MB to multi-GB. Zone means come from the aggregate export.

Publish tables and per-scenario summaries as a **secret GitHub Gist**
(`gh gist create`, default secret — not `--public`) and link the Gist
from the PR. Do not upload `.tracy` files unless one is specifically
worth sharing; mention it instead of attaching a large binary.

`tracy-flight.sh` starts `tracy-capture` first, then the game with
`MAYBRAID_VSYNC=off --start-at=0,0 --agent-quit`, and exports zone /
message CSVs plus a top-zone summary.

## A/B on a shared Mac

macOS can switch the game into a throttled regime mid-session (render
thread blocked ~12 ms in `prepare_windows`, main thread waiting in
`RenderExtractApp`, multi-ms preemption spikes in ordinary systems).
`present wait` in `compare-windows.py` output flags it. Before/after
blocks captured minutes apart can land in different regimes, so
interleave them instead: save the baseline binary and alternate it with
the current build via `MAYBRAID_BIN`.

```bash
MAYBRAID_BIN=/tmp/maybraid-base profiling/tracy-flight.sh crowd-16 ab-base 1
profiling/tracy-flight.sh crowd-16 ab-after 1
```
