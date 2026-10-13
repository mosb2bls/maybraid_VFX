# Contributing to world-player

Character / player host and diagnostics library for [`maybraid-world`](../lib/).
Assembled world runs [`maybraid-world-playground`](../playground/). This
crate is no longer a standalone binary; restore the retired app from
[PLAYGROUNDS.md](../../PLAYGROUNDS.md).

Forest stream knobs live on each mode's `VegetationLayerConfig`. Mode, mesh stats, and
character attach stay here until they move to a non-playground crate.

## Verify

```bash
cargo check -p world-player
```
