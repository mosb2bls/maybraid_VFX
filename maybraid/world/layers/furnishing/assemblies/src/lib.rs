//! Painted furniture assemblies fitted into [`FurnitureNode`](building_components::FurnitureNode) slots.
//!
//! Chico-shaped: `FooParams` → `params.build()` → `Foo`. [`unit_from_num`](bed::BedParams::unit_from_num)
//! keys the palette only. Parts explode for [`material_ref::MaterialRef`] paint
//! and instance as posed GLBs via [`furniture_components`].
//!
//! Kit remap lives in furniture-components. Slot facing / abutment come from
//! the node (`placement.yaw`); assemblies do not re-read a floor plan.
//! [`generation`] walks building High slots. The 50 m cell stream lives in `maputo`.

pub mod basin;
pub mod bed;
pub mod bread;
pub mod chair;
pub mod chest;
pub mod cookware;
pub mod counter;
pub mod faucet;
pub mod fill;
pub mod food_display;
pub mod fridge;
pub mod fruit;
pub mod generation;
pub mod on_buildings;
pub mod palette;
pub mod partition;
pub mod plugin;
pub mod present;
pub mod range;
pub mod shelf;
pub mod table;

pub use basin::{Basin, BasinParams};
pub use bed::{Bed, BedParams};
pub use bread::{Bread, BreadParams};
pub use chair::{Chair, ChairParams};
pub use chest::{Chest, ChestParams};
pub use cookware::{Cookware, CookwareParams};
pub use counter::{Counter, CounterParams};
pub use faucet::{Faucet, FaucetParams};
pub use fill::{abutment_wall, posed_assembly, try_assembly};
pub use food_display::{FoodDisplay, FoodDisplayParams};
pub use fridge::{Fridge, FridgeParams};
pub use fruit::{Fruit, FruitParams};
pub use furniture_components::{
	assembly_scene, pose_parts, posed_kit, posed_kit_part, FurnitureKitPart, PartKind, PlacedPart,
	BOX_KIT_TO_UNIT, LEG_KIT_TO_UNIT,
};
pub use generation::{
	collect_furniture_slots, generate_assemblies, generate_assemblies_from_nodes,
};
pub use on_buildings::{paint_host_furniture, PaintedFurniture};
pub use partition::{Partition, PartitionParams};
pub use plugin::{FurnitureAssembliesPlugin, FurnitureKitMeshes};
pub use present::{filled_slot_scene, filled_slots_scene};
pub use range::{Range, RangeParams};
pub use shelf::{Shelf, ShelfParams};
pub use table::{Table, TableParams};

use building_components::FurnitureGeometry;

/// Built assembly: topology is seed-invariant; [`MaterialRef`](material_ref::MaterialRef) is not.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembly {
	pub geometry: FurnitureGeometry,
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}
