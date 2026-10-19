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

Publish captures worth keeping as assets on a `vX.Y.Z Traces` GitHub release,
each with a same-named `.md` of findings — see **Traces Releases** in
[`CONTRIBUTING.md`](../CONTRIBUTING.md). Link the release from the PR.

`tracy-flight.sh` starts `tracy-capture` first, then the game with
`MAYBRAID_VSYNC=off --start-at=0,0 --agent-quit`, and exports zone /
message CSVs plus a top-zone summary. Home-first picker / character-switch
scripts must not latch `PlayerSpawnXz`: set `MAYBRAID_NO_START_AT=1`.
Profile scripts omit `stats vsync` because that command *toggles* present
mode and would re-enable vsync over the env var.

## Scripts outside the repo

Both flight scripts take a scenario name (`maybraid/agent/scripts/<name>.jsonl`)
or a path to any JSONL script, such as one downloaded from a Traces release.
The measure window is the script's last non-`-warmup` `scenario` start mark,
so a script needs a `<name>-warmup` / `<name>` pair. A leading `start`
command overrides `--start-at=0,0`. Unknown scripts get a 300 s wall clock;
raise it with `MAYBRAID_FLIGHT_TIMEOUT`.

```bash
gh release download v0.1.3-traces -R ramate-io/maybraid -p 'abc1234_20261011_script_*.jsonl' -D /tmp/scripts
profiling/tracy-flight.sh /tmp/scripts/abc1234_20261011_script_ridge-run.jsonl daily 1
```

## GPU flights (Metal)

Bevy does not open a Tracy GPU context on Metal, so Tracy has no GPU zones on
macOS. `metal-flight.sh` records the real per-pass GPU time with Instruments
instead: it launches the game, waits for the measure scenario's start mark,
attaches `xctrace record --template 'Metal System Trace'`, exports the encoder,
GPU-interval, and GPU performance-state tables, and summarizes them with
`metal-passes.py`.

```bash
profiling/metal-flight.sh crowd-0 baseline 1
```

Output goes to `profiling/captures/gpu/<phase>/<scenario>/r<repeat>/`:
`*-summary.md` (GPU busy per frame, pass buckets, top encoder labels,
performance state), `*-passes.csv`, `*-stats.json`, and `*.trace.zip` (the
upload form of the `.trace` bundle; ~650 MB for 40 s). The exported XML is
10s–100s of MB and is deleted after parsing.

- Needs Xcode (`/usr/bin/xctrace`) and Accessibility permission for
  `osascript`: macOS throttles an occluded game window, so the script keeps it
  frontmost while recording.
- `MAYBRAID_GPU_TRACE_LIMIT` (default 28 s) is how long to record after the
  start mark. Set it to at least the measure window; the summary warns when
  the recording was shorter than the scenario.
- Compare **GPU busy** (union of channels) against the 16.67 ms budget. Apple
  GPUs downclock when lightly loaded, so read the performance-state table
  before comparing GPU ms across runs.
- Encoder labels are occasionally dropped in a repeat recording (the opaque
  pass shows as `unlabeled`); discard that run rather than averaging it in.

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
