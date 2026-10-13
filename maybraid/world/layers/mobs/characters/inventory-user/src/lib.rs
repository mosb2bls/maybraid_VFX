//! Inventory bag as a 1:1 Bevy relationship, persisted beside the character.

use bevy::prelude::*;
use character_items::{
	BoltMaterial, ClothingMaterial, ClothingMesh, ClothingStats, FirearmBarrel, FirearmGrip,
	FirearmKitSpec, FirearmLooks, FirearmMaterial, FirearmMesh, FirearmScales, FirearmSight,
	FirearmSpec, FirearmStats, FirearmStock, FirearmTriggerBox, GrenadeMesh, GrenadeRecharge,
	GrenadeSpec, GrenadeStats, Inventory, InventoryItem, InventorySlot, ItemColor, SkillMapKind,
	SkillMapSpec, WORN_CLOTHING_LIMIT,
};
use character_persist::{CharacterId, PersistError, SaveRoot};
use serde::{Deserialize, Serialize};
use std::fs;

const VERSION: u32 = 5;

/// Capsule/session using an inventory bag.
///
/// 1:1 onto the bag entity (`bag`). Inserting it stamps [`CarriedBy`] so
/// despawn/replace stays consistent.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = CarriedBy)]
pub struct InventoryUser {
	#[relationship]
	pub bag: Entity,
	pub settings: InventoryUserSettings,
}

impl InventoryUser {
	pub fn carrying(bag: Entity) -> Self {
		Self { bag, settings: InventoryUserSettings::default() }
	}
}

#[derive(Clone, Copy, Debug)]
pub struct InventoryUserSettings {
	pub worn_limit: usize,
}

impl Default for InventoryUserSettings {
	fn default() -> Self {
		Self { worn_limit: WORN_CLOTHING_LIMIT }
	}
}

/// Bag-side 1:1 target of [`InventoryUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = InventoryUser)]
pub struct CarriedBy(Entity);

pub struct InventoryUserPlugin;

impl Plugin for InventoryUserPlugin {
	fn build(&self, _app: &mut App) {}
}

#[derive(Serialize, Deserialize)]
struct InventoryFile {
	version: u32,
	id: CharacterId,
	items: Vec<InventoryItemFile>,
	#[serde(default)]
	clothing: Vec<usize>,
	#[serde(default)]
	weapons: Vec<usize>,
	#[serde(default)]
	skills: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
struct InventoryFileV1 {
	version: u32,
	id: CharacterId,
	items: Vec<ClothingItemFile>,
	worn: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum InventoryItemFile {
	Clothing {
		mesh: ClothingMesh,
		material: ClothingMaterial,
		color: ItemColor,
		#[serde(default)]
		stats: Option<ClothingStats>,
	},
	Firearm {
		mesh: FirearmMesh,
		#[serde(default)]
		kit: Option<FirearmKitFile>,
		#[serde(default)]
		scales: Option<FirearmScales>,
		#[serde(default)]
		looks: Option<FirearmLooks>,
		#[serde(default)]
		material: Option<FirearmMaterial>,
		#[serde(default)]
		color: Option<ItemColor>,
		#[serde(default)]
		bolt: Option<BoltMaterial>,
		#[serde(default)]
		stats: Option<FirearmStats>,
	},
	SkillMap {
		skill: SkillMapKind,
		seed: u32,
	},
	Grenade {
		#[serde(default)]
		mesh: GrenadeMesh,
		#[serde(default)]
		stats: Option<GrenadeStats>,
		#[serde(default)]
		recharge: f32,
	},
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct FirearmKitFile {
	barrel: FirearmBarrel,
	grip: FirearmGrip,
	trigger_box: FirearmTriggerBox,
	stock: FirearmStock,
	#[serde(default)]
	sight: FirearmSight,
}

impl FirearmKitFile {
	fn from_spec(kit: FirearmKitSpec) -> Self {
		Self {
			barrel: kit.barrel,
			grip: kit.grip,
			trigger_box: kit.trigger_box,
			stock: kit.stock,
			sight: kit.sight,
		}
	}

	fn into_spec(self, body: FirearmMesh) -> FirearmKitSpec {
		FirearmKitSpec {
			body,
			barrel: self.barrel,
			grip: self.grip,
			trigger_box: self.trigger_box,
			stock: self.stock,
			sight: self.sight,
		}
	}
}

#[derive(Serialize, Deserialize)]
struct ClothingItemFile {
	mesh: ClothingMesh,
	material: ClothingMaterial,
	color: ItemColor,
}

impl InventoryItemFile {
	fn from_item(item: &InventoryItem) -> Self {
		match item {
			InventoryItem::Clothing { mesh, material, stats } => Self::Clothing {
				mesh: *mesh,
				material: material.id,
				color: material.color,
				stats: Some(*stats),
			},
			InventoryItem::Firearm { spec, stats } => Self::Firearm {
				mesh: spec.kit.body,
				kit: Some(FirearmKitFile::from_spec(spec.kit)),
				scales: Some(spec.scales),
				looks: Some(spec.looks),
				material: Some(spec.looks.body.material),
				color: Some(spec.looks.body.color),
				bolt: Some(spec.bolt),
				stats: Some(*stats),
			},
			InventoryItem::SkillMap { spec } => {
				Self::SkillMap { skill: spec.kind, seed: spec.seed }
			}
			InventoryItem::Grenade { spec, stats, recharge } => {
				Self::Grenade { mesh: spec.mesh, stats: Some(*stats), recharge: recharge.remaining }
			}
		}
	}

	fn into_item(self) -> InventoryItem {
		match self {
			Self::Clothing { mesh, material, color, stats } => InventoryItem::Clothing {
				mesh,
				material: character_items::MaterialRefParams::new(material, color),
				stats: stats.unwrap_or_else(|| ClothingStats::generate(mesh, material, color)),
			},
			Self::Firearm { mesh, kit, scales, looks, material, color, bolt, stats } => {
				let spec = FirearmSpec {
					kit: kit.map(|kit| kit.into_spec(mesh)).unwrap_or_else(|| mesh.concept_kit()),
					scales: scales.unwrap_or(FirearmScales::UNIT),
					looks: looks.unwrap_or_else(|| {
						FirearmLooks::uniform(
							material.unwrap_or(FirearmMaterial::BrushedMetal),
							color.unwrap_or(ItemColor::Natural),
						)
					}),
					bolt: bolt.unwrap_or(BoltMaterial::PlainLaser),
				};
				InventoryItem::Firearm {
					spec,
					stats: stats.unwrap_or_else(|| FirearmStats::generate(&spec)),
				}
			}
			Self::SkillMap { skill, seed } => {
				InventoryItem::skill_map(SkillMapSpec::new(skill, seed))
			}
			Self::Grenade { mesh, stats, recharge } => InventoryItem::Grenade {
				spec: GrenadeSpec { mesh },
				stats: stats.unwrap_or_else(GrenadeStats::standard),
				recharge: GrenadeRecharge { remaining: recharge },
			},
		}
	}
}

impl ClothingItemFile {
	fn into_item(self) -> InventoryItem {
		InventoryItem::clothing(self.mesh, self.material, self.color)
	}
}

fn sanitize_selection(
	items: &[InventoryItem],
	indices: impl IntoIterator<Item = usize>,
	slot: InventorySlot,
) -> Vec<usize> {
	let mut selected = Vec::new();
	for index in indices {
		if index >= items.len() {
			continue;
		}
		if items[index].slot() != slot {
			continue;
		}
		if selected.contains(&index) {
			continue;
		}
		selected.push(index);
		if selected.len() >= slot.capacity() {
			break;
		}
	}
	selected
}

/// Write `inventories/{id}.json`. Missing parent dirs are created.
pub fn save(root: &SaveRoot, id: CharacterId, inventory: &Inventory) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let file = InventoryFile {
		version: VERSION,
		id,
		items: inventory.items.iter().map(InventoryItemFile::from_item).collect(),
		clothing: sanitize_selection(
			&inventory.items,
			inventory.clothing.iter().copied(),
			InventorySlot::Clothing,
		),
		weapons: sanitize_selection(
			&inventory.items,
			inventory.weapons.iter().copied(),
			InventorySlot::Weapons,
		),
		skills: sanitize_selection(
			&inventory.items,
			inventory.skills.iter().copied(),
			InventorySlot::Skills,
		),
	};
	let json = serde_json::to_string_pretty(&file)?;
	fs::write(root.inventory_path(id), json)?;
	Ok(())
}

/// Load the bag. A missing file is an empty inventory, not an error.
pub fn load(root: &SaveRoot, id: CharacterId) -> Result<Inventory, PersistError> {
	let path = root.inventory_path(id);
	let json = match fs::read_to_string(&path) {
		Ok(json) => json,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
			return Ok(Inventory::default());
		}
		Err(error) => return Err(error.into()),
	};
	let value: serde_json::Value = serde_json::from_str(&json)?;
	let version = value.get("version").and_then(|version| version.as_u64()).unwrap_or(1);
	if version >= 2 {
		let file: InventoryFile = serde_json::from_value(value)?;
		let items: Vec<InventoryItem> =
			file.items.into_iter().map(InventoryItemFile::into_item).collect();
		let clothing = sanitize_selection(&items, file.clothing, InventorySlot::Clothing);
		let weapons = sanitize_selection(&items, file.weapons, InventorySlot::Weapons);
		let skills = sanitize_selection(&items, file.skills, InventorySlot::Skills);
		return Ok(Inventory { items, clothing, weapons, skills });
	}
	let file: InventoryFileV1 = serde_json::from_value(value)?;
	let items: Vec<InventoryItem> =
		file.items.into_iter().map(ClothingItemFile::into_item).collect();
	let clothing = sanitize_selection(&items, file.worn, InventorySlot::Clothing);
	Ok(Inventory { items, clothing, weapons: Vec::new(), skills: Vec::new() })
}

pub fn spawn_bag(commands: &mut Commands, host: Entity, inventory: Inventory) -> Entity {
	let bag = commands.spawn(inventory).id();
	commands.entity(host).insert(InventoryUser::carrying(bag));
	bag
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{ClothingMesh, ItemRng};

	#[test]
	fn starter_outfit_round_trips() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		let mut rng = ItemRng::from_seed(3);
		let items = rng.random_starter_loadout();
		let inventory = Inventory::with_starter_outfit(items);
		save(&root, id, &inventory).expect("save");
		let loaded = load(&root, id).expect("load");
		assert_eq!(loaded, inventory);
		assert_eq!(loaded.weapons.len(), 3);
		assert!(loaded.items.iter().any(|item| item.grenade_spec().is_some()));
		assert!(loaded.items[0].clothing_stats().is_some_and(|stats| stats.weight > 0));
		assert!(loaded
			.items
			.iter()
			.any(|item| item.firearm_stats().is_some_and(|stats| stats.damage > 0)));
	}

	#[test]
	fn missing_file_is_empty_bag() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let loaded = load(&root, CharacterId(1)).expect("load");
		assert_eq!(loaded, Inventory::default());
	}

	#[test]
	fn clothing_labels_are_kebab_case() {
		assert_eq!(ClothingMesh::TankTop.label(), "tank-top");
		let json = serde_json::to_string(&ClothingMesh::TankTop).expect("json");
		assert_eq!(json, "\"tank-top\"");
	}

	#[test]
	fn mixed_bag_round_trips_firearms() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(9);
		let items = vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Reltor),
		];
		let inventory =
			Inventory { items, clothing: vec![0], weapons: vec![1, 2], skills: Vec::new() };
		save(&root, id, &inventory).expect("save");
		let json = fs::read_to_string(root.inventory_path(id)).expect("read");
		assert!(json.contains("\"kind\": \"firearm\""));
		assert_eq!(load(&root, id).expect("load"), inventory);
		assert!(json.contains("\"stats\""));
	}

	#[test]
	fn v2_file_without_stats_rolls_from_identity() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		root.ensure_dirs().expect("dirs");
		let id = CharacterId(11);
		let json = r#"{
			"version": 2,
			"id": "0000000000000000000000000000000b",
			"items": [
				{ "kind": "clothing", "mesh": "pants", "material": "cloth", "color": "natural" },
				{ "kind": "firearm", "mesh": "bullpup" }
			],
			"clothing": [0],
			"weapons": [1]
		}"#;
		fs::write(root.inventory_path(id), json).expect("write");
		let loaded = load(&root, id).expect("load");
		assert_eq!(loaded.items.len(), 2);
		assert_eq!(
			loaded.items[0],
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural
			)
		);
		assert_eq!(loaded.items[1], InventoryItem::firearm(FirearmMesh::Bullpup));
		assert_eq!(loaded.clothing, vec![0]);
		assert_eq!(loaded.weapons, vec![1]);
	}

	#[test]
	fn v1_clothing_file_still_loads() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		root.ensure_dirs().expect("dirs");
		let id = CharacterId(4);
		let json = r#"{
			"version": 1,
			"id": "00000000000000000000000000000004",
			"items": [
				{ "mesh": "pants", "material": "cloth", "color": "natural" },
				{ "mesh": "tank-top", "material": "cloth", "color": "red" }
			],
			"worn": [0, 1, 1, 99]
		}"#;
		fs::write(root.inventory_path(id), json).expect("write");
		let loaded = load(&root, id).expect("load");
		assert_eq!(loaded.items.len(), 2);
		assert_eq!(loaded.clothing, vec![0, 1]);
		assert!(loaded.weapons.is_empty());
		assert!(loaded.skills.is_empty());
		assert_eq!(loaded.items[0].mesh(), Some(ClothingMesh::Pants));
	}

	#[test]
	fn skill_map_round_trips() {
		use character_items::{SkillMapKind, SkillMapSpec};

		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(13);
		let items = vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 0xA11A_5EED)),
		];
		let inventory =
			Inventory { items, clothing: vec![0], weapons: Vec::new(), skills: vec![1] };
		save(&root, id, &inventory).expect("save");
		let json = fs::read_to_string(root.inventory_path(id)).expect("read");
		assert!(json.contains("\"kind\": \"skill-map\""));
		assert!(json.contains("\"dumbwave\""));
		assert_eq!(load(&root, id).expect("load"), inventory);
	}

	#[test]
	fn grenade_round_trips_remaining_recharge() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(15);
		let items = vec![InventoryItem::Grenade {
			spec: GrenadeSpec::standard(),
			stats: GrenadeStats::standard(),
			recharge: GrenadeRecharge { remaining: 2.5 },
		}];
		let inventory =
			Inventory { items, clothing: Vec::new(), weapons: vec![0], skills: Vec::new() };
		save(&root, id, &inventory).expect("save");
		let json = fs::read_to_string(root.inventory_path(id)).expect("read");
		assert!(json.contains("\"kind\": \"grenade\""));
		assert!(json.contains("2.5"));
		assert_eq!(load(&root, id).expect("load"), inventory);
	}

	#[test]
	fn v4_file_without_skills_stays_empty() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		root.ensure_dirs().expect("dirs");
		let id = CharacterId(14);
		let json = r#"{
			"version": 4,
			"id": "0000000000000000000000000000000e",
			"items": [
				{ "kind": "clothing", "mesh": "pants", "material": "cloth", "color": "natural" }
			],
			"clothing": [0],
			"weapons": []
		}"#;
		fs::write(root.inventory_path(id), json).expect("write");
		let loaded = load(&root, id).expect("load");
		assert!(loaded.skills.is_empty());
		assert_eq!(loaded.clothing, vec![0]);
	}
}
