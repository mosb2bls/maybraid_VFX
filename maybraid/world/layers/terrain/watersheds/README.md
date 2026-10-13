# marazion-watersheds

Pure Marazion watershed stamps ([RFC-127](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds)) — no LOD `GenerationScheme` wiring. Durham models consume `HydroComplex` / `WaterFill` after pre-watershed terrain is composed.

## Docs

- **[State of the API](STATE_OF_THE_API.md)** — what HydroNodes buy us now, wet basin vs drainage basin, and where basin-scale grading goes next
- [Watershed correction extents](src/WATERSHED_CORRECTION.md) — `max_correction_extent` and cellular discoverability
- [Node blend notes](src/NODE_BLEND.md)
- Durham cellular / water / shore rules: [`../durham/CONTRIBUTING.md`](../durham/CONTRIBUTING.md)

## Layout

```text
src/authored/    pocket hierarchy + leaf plans (lake, stream, bog, streams_graph)
src/primitive/   HydroNode, HydroComplex, params, backfill, WaterFill, hydro fields
```
