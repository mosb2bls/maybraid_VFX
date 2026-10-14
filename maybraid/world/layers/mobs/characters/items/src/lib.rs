//! Items that map onto characters.
//!
//! This crate owns the catalog of wearable and carried items (clothing,
//! firearms, grenades, and skill maps), the shared item color palette, clothing surface
//! recipes, hashed display names, and the inventory bag used at character
//! creation. Species
//! crates describe *how* an item attaches to a particular character (rig, slot,
//! normalization); this crate describes *what* items exist and how they are
//! presented to menus.

pub mod clothing;
pub mod clothing_material;
pub mod clothing_roll;
pub mod drop;
pub mod firearm;
pub mod firearm_material;
pub mod firearm_roll;
pub mod grenade;
pub mod grenade_roll;
pub mod inventory;
pub mod names;
pub mod palette;
pub mod skill_map;
pub mod stats;

mod menu_traits;

pub use clothing::{ClothingColor, ClothingHost, ClothingKind, ClothingMesh, ClothingSlot};
pub use clothing_material::{ClothingMaterial, ClothingMaterialChoice};
pub use clothing_roll::{generate_clothing_stats, ClothingBuff, ClothingPriors};
pub use drop::{
	ActiveInventoryDropPolicy, InventoryDrop, InventoryDropError, InventoryDropPolicy,
	InventoryDropRefused, InventoryDropRequest, InventoryDropped,
};
pub use firearm::{
	FirearmBarrel, FirearmGrip, FirearmKitSpec, FirearmLooks, FirearmMesh, FirearmScales,
	FirearmSight, FirearmSpec, FirearmStock, FirearmTriggerBox, SlotLook, SlotRest, SlotScale,
	IRON_SIGHT_FOV, LENGTH_MILLI_MAX, LENGTH_MILLI_MIN, SCALE_MILLI_UNIT, THICKNESS_MILLI_MAX,
	THICKNESS_MILLI_MIN,
};
pub use firearm_material::{BoltMaterial, FirearmMaterial};
pub use firearm_roll::{
	generate_firearm_stats, realize_firearm_stats, Dist, FirearmBuff, FirearmPriors,
};
pub use grenade::{GrenadeMesh, GrenadeRecharge, GrenadeSpec, GrenadeStats, STANDARD_LAUNCH_SPEED};
pub use grenade_roll::{
	effect_scale_for_blast, generate_grenade_stats, realize_grenade_stats, BLAST_DAMAGE_MAX,
	BLAST_DAMAGE_MIN, BLAST_RADIUS_MAX, BLAST_RADIUS_MIN, FIERY_EXPLOSION_METERS, LAUNCH_SPEED_MAX,
	LAUNCH_SPEED_MIN,
};
pub use inventory::{
	icon_catalog_key_for_label, Inventory, InventoryItem, InventorySlot, ItemRng, LootFraction,
	MaterialRefParams, ITEM_ICON_CATALOG_TAG, SKILL_MAP_BAG_LIMIT, SKILL_MAP_QUEUE_LIMIT,
	STARTER_CLOTHING_COUNT, STARTER_GRENADE_COUNT, STARTER_SKILL_MAP_COUNT, STARTER_WEAPON_COUNT,
	WEAPON_QUEUE_LIMIT, WORN_CLOTHING_LIMIT,
};
pub use palette::ItemColor;
pub use skill_map::{SkillMapKind, SkillMapSpec};
pub use stats::{CharacterSheet, ClothingStats, FireMode, FirearmStats, ProjectileKind};
