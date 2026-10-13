# Richmond Urbanization

Richmond is an cellular urbanization model for Maybraid. It covers:

- Connected transport networks
- Development regions
- Building footprints
- Room subdivisions
- Ornamental details

See [CONTRIBUTING.md](CONTRIBUTING.md) for how buildings author domain nodes and emit them under LOD.

## Hierarchy

The current planned hierarchy is as follows:

1. **World Transport Networks:** similar to the large hydrology graph, we generate low-resolution transport networks over a very large grid (150km+ cells).
2. **Settlements:** a settlment cell (15km+ cells) is picked for a certain settlement type based on noise and transport network node count. The settlement type also describes a coarse internal transport network, adding more transport nodes.
3. **Neighborhoods:** a neighborhood cell (500m+ cells) is picked for a certain neighborhood type based on noise and settlement node count. The neighborhood type also describes a coarse internal transport network, adding more transport nodes.
4. **Development:** a development cell (100m+ cells) is picked for a certain development type based on noise and neighborhood node count. The development cell is responsible for allocating buildings--some of which may be connected. Development cells are the first level in the hierarchy which we plan to have implement an actual `LodScene`. They can be reposnible for hiding or unifying certain building components at long ranges--similar to the intent for groves in the vegetation hierarchy.  
5. **Buildings:** within a development cell, you'll find buildings. Buildings implement [`building_components::BuildingComponents`](buildings/components/src/lib.rs) and expose domain nodes via `*_nodes_for_level`. Component-only buildings present as [`ComponentsOnly`](buildings/components/src/lib.rs)`<T>` for `LodScene`; types with hosts, silhouettes, or non-node extras keep a custom `LodScene`. Many types compose each other by extending each other's nodes. It's a good idea not to make a building too big as there may be structural culling tricks which could be unnecessarily delayed if the building cannot distinguish smaller subsections. For this reason, **Development** cells will often knit particularly large effective buildings from several smaller building parts. 
6. **Building Components:** building components are the smallest units in the hierarchy. Their `LodScene` is made up of meshes and their LOD bands are primarily concerned with limiting the triangle count of these meshes which is sent to the GPU. The `Panel` primitive is very commonly used for general linear geometry. Other primitives tend to have specific functions. 
