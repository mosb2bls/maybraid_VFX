# Tracy flights

Release `maybraid` with Bevy `trace_tracy`, driven by
[`maybraid/agent`](../maybraid/agent/README.md) scripts, captured on this
machine's GPU (not lavapipe).

Bevy 0.19.1 pulls `tracy-client` 0.18.4 / `tracy-client-sys` 0.28.0, which is
**Tracy 0.13.1** — the copy on the Nix flake PATH. Do not mix Homebrew 0.14.x
captures with this binary.

```bash
nix develop --command bash -lc '
  cargo build -p maybraid --release --features bevy/trace_tracy
  profiling/tracy-flight.sh origin-idle baseline 1
'
```

CSV summaries land in [`captures/`](captures/). `*.tracy` files are gitignored
(often tens of MB); keep them next to the CSVs locally when size allows.

`tracy-flight.sh` starts `tracy-capture` first, then the game with
`MAYBRAID_VSYNC=off --start-at=0,0 --agent-quit`, and exports zone / message
CSVs plus a top-zone summary.
