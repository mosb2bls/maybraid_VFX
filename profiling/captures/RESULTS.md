# Tracy flight results (PR 1183)

Hardware: Apple M4 Pro, 48 GiB, Metal 3, 1080p60 display.
Build: `cargo build -p maybraid --release --features bevy/trace_tracy`.
Capture: Nix Tracy 0.13.1 `tracy-capture` only, `MAYBRAID_VSYNC=off`,
`--start-at=0,0`.
Means are **whole-capture aggregates** (warmup included) unless noted.
Measure-window unwraps exist only for the first idle/locomotion baselines;
later flights skip `-u` because those CSVs were multi-GB.

The first after-opt `origin-idle` flight (`…T141940Z`) still showed the
2 ms fulfill tax. That build recorded `last_generation` only when the
Seen walk finished; the walk itself filled the 2 ms budget, so the skip
never armed. Tables below use flights after `4add3fbe4`.

Clean `biome-traverse` baselines (`r2`, `r3`) and extra `locomotion-mix`
baseline `r2` were recaptured from the pre-opt source
(`f927705dc` `terrain-chunk-ref`) after the display stayed up. The
incomplete `biome-traverse` `r1` (`…T140802Z`) is display-sleep only.

## `fulfill_terrain_chunk_refs` (ms/frame)

| scenario | phase | repeats | n | mean | max |
| --- | --- | --- | ---: | ---: | ---: |
| origin-idle | baseline | r1, r2 | 2822 / 2807 | 1.972 / 1.972 | 3.657 / 2.385 |
| origin-idle | after | r1–r3 | 2802 / 2812 / 2814 | 0.195 / 0.191 / 0.192 | 2.146 / 2.120 / 6.118 |
| locomotion-mix | baseline | r1, r2 | 3157 / 3079 | 1.981 / 1.974 | 4.412 / 2.297 |
| locomotion-mix | after | r1–r3 | 3532 / 3499 / 2604 | 0.156 / 0.159 / 0.211 | 2.124 / 2.207 / 2.099 |
| biome-traverse | baseline | r2, r3 | 11204 / 10886 | 1.996 / 1.996 | 4.457 / 3.726 |
| biome-traverse | after | r1, r2 | 16004 / 15557 | 0.040 / 0.040 | 2.078 / ~2.1 |

Idle/locomotion fulfill mean dropped **~10×** (1.97 ms → 0.19 ms).
Biome-traverse dropped **~50×** (2.00 ms → 0.04 ms) because the long
route spends most frames on a quiet mailbox. The remaining 2 ms max is
the stream-in prefix.

## `schedule{name=Update}` (ms/frame)

| scenario | phase | mean |
| --- | --- | ---: |
| origin-idle | baseline r1 / r2 | 4.512 / 4.547 |
| origin-idle | after r1 / r2 / r3 | 2.891 / 2.883 / 2.849 |
| locomotion-mix | baseline r1 / r2 | 5.461 / 4.677 |
| locomotion-mix | after r1 / r2 / r3 | 2.827 / 2.852 / 3.127 |
| biome-traverse | baseline r2 / r3 | 4.248 / 4.202 |
| biome-traverse | after r1 / r2 | 2.213 / 2.364 |

Update schedule dropped **~1.6–2.0 ms**, matching the fulfill skip.

## Whole-frame `update` (ms/frame)

| scenario | phase | mean |
| --- | --- | ---: |
| origin-idle | baseline | 17.06 / 17.13 |
| origin-idle | after | 17.14 / 17.11 / 17.08 |
| locomotion-mix | baseline | 19.01 / 16.22 |
| locomotion-mix | after | 17.02 / 17.05 / 17.61 |
| biome-traverse | baseline | 13.08 / 15.00 |
| biome-traverse | after | 7.46 / 15.69 |

Whole-frame time is still GPU/present bound and noisy across biome
repeats. The CPU Update schedule improved; frame time did not move
on idle because `RenderExtractApp` / `render_system` still dominate.

## Measure-window (unwrap, first baselines only)

`origin-idle` measure window (~25 s idle after 20 s warmup): fulfill
**2.004 / 2.005 ms**. `locomotion-mix` measure window: fulfill **2.014 ms**.

## Discarded / incomplete

- `origin-idle` baseline `…T135845Z`: `stats vsync` re-enabled vsync.
- `biome-traverse` baseline `…T140802Z`: display sleep (`Monitor removed`);
  1513 frames only.
- `origin-idle` after `…T141940Z`: first opt without the generation-stamp
  fix; fulfill still 1.969 ms.
