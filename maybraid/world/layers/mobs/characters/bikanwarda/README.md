# Bikanwarda character assembly (`bikanwarda-character`)

Boss-scale Epiphant assembly for Bikanwarda ([#1134](https://github.com/ramate-io/maybraid/issues/1134)): measured **20 m** encounter standing height, sockets, collider fit, visual bounds, and Bray / sleep animation envelopes. Uses existing Epiphant meshes (placeholder presentation until final tattoos on [#1231](https://github.com/ramate-io/maybraid/issues/1231)).

No NPC/world health bars and no `combat-hud` dependency — gameplay hooks are spawn markers + physics hulls only.

**Preview stills** (Lavapipe + `character-concepts-playground`):

```bash
./maybraid/world/layers/mobs/characters/bikanwarda/previews/1134/capture.sh
```

**Tests:**

```bash
export CXX=g++-13 CC=gcc-13 LIBRARY_PATH=/usr/lib/gcc/x86_64-linux-gnu/13
cargo test -p bikanwarda-character --all-targets
```
