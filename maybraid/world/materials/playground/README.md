# Maybraid World Materials Playground

A [`MaterialRef`](../../../material-ref) on blast-style hulls. Uses the composed world material lib so named recipes resolve the same way they do in Maybraid World.

```text
cargo run -p world-materials-playground
cargo run -p world-materials-playground -- show pulse
cargo run -p world-materials-playground -- mesh blast
cargo run -p world-materials-playground -- mesh ring
```

In-game (press `/`):

- `show hex|pulse|tail|muzzle-flame|standard` — authored energy / flame / Standard looks
- `show <recipe>` — any world recipe name (`furniture_wood`, …)
- `mesh sphere|capsule` — solid baselines (the original hosts)
- `mesh core` — tiny orb
- `mesh shell` — faceted energy hull
- `mesh ring` — flat shockwave annulus
- `mesh cards` — outward flipbook cards
- `mesh blast` — core + shell + ring stacked
- `shoot` — fire repeating copies of the **current** hull and recipe
- `shoot stop` — cancel the launcher
