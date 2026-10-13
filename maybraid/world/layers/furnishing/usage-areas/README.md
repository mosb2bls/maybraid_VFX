# Furniture usage areas

Expand Richmond [`FurnitureUsageNode`](../../urbanization/buildings/components/src/furniture/usage.rs) regions into ensembles of [`FurnitureNode`](../../urbanization/buildings/components/src/furniture/node.rs) slots.

Richmond packers keep labels such as `BitesCounter` and `BitesKitchen` on the packed AABB. They do **not** remap a whole band to one stretched `Counter`. This crate fills those boxes:

- [`BitesCounterUsage`](src/bites_counter.rs) — ~0.85 m passage-face stations, outward display, leftover aisle grid (screens / stock / lounge)
- [`BitesKitchenUsage`](src/bites_kitchen.rs) — wall-run plus peninsula, cooktop, fridge; leftover screens and shelves
- [`BitesSeatingUsage`](src/bites_seating.rs) — communal tables (~2.2 × 1.4 m, up to six chairs) on a 3.8 m grid

Depends on [`building-components`](../../urbanization/buildings/components/) only. [`furniture-assemblies`](../assemblies/) `collect_furniture_slots` and Maputo's 50 m cell stream call [`expand_usages`](src/expand.rs) after collecting High usage nodes.
