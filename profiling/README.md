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

Flights write under `profiling/captures/` by default (gitignored: `*.tracy`,
zone/message CSVs, events, logs, summaries). Override with
`MAYBRAID_PROFILING_OUT` if needed. Do **not** commit capture data or result
tables. Publish the summary as a GitHub Gist and link that gist from the PR.

Do **not** write `tracy-csvexport -u` unwrap CSVs — they are hundreds of MB
to multi-GB. Zone means come from the aggregate export.

`tracy-flight.sh` starts `tracy-capture` first, then the game with
`MAYBRAID_VSYNC=off --start-at=0,0 --agent-quit`, and exports zone / message
CSVs plus a top-zone summary. Profile scripts omit `stats vsync` because that
console line *toggles* present mode and would re-enable vsync over the env var.
