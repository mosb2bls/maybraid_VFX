use character_items::ClothingMesh;
use character_ui_menu::{MenuComponent, MenuNode};
use characters::species::{
	braidman::BraidmanConfig, brodler::BrodlerConfig, tuberwaber::TuberwaberConfig,
};

use crate::{
	character::{CharacterMenu, ConceptSpecies},
	characters::{braidman::BraidmanMenu, brodler::BrodlerMenu, tuberwaber::TuberwaberMenu},
	MenuEvent,
};

#[test]
fn braidman_config_round_trip() -> anyhow::Result<()> {
	let config = BraidmanConfig::default_preview();
	let menu = BraidmanMenu::from(&config);
	let restored = BraidmanConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn brenal_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::brenal::BrenalConfig::default_preview();
	let menu = crate::characters::brenal::BrenalMenu::from(&config);
	let restored = characters::species::brenal::BrenalConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn caole_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::caole::CaoleConfig::default_preview();
	let menu = crate::characters::caole::CaoleMenu::from(&config);
	let restored = characters::species::caole::CaoleConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn epiphant_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::epiphant::EpiphantConfig::default_preview();
	let menu = crate::characters::epiphant::EpiphantMenu::from(&config);
	let restored = characters::species::epiphant::EpiphantConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.nose, restored.nose);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn hars_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::hars::HarsConfig::default_preview();
	let menu = crate::characters::hars::HarsMenu::from(&config);
	let restored = characters::species::hars::HarsConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn ylter_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::ylter::YilterConfig::default_preview();
	let menu = crate::characters::ylter::YilterMenu::from(&config);
	let restored = characters::species::ylter::YilterConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn sonyak_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::sonyak::SonyakConfig::default_preview();
	let menu = crate::characters::sonyak::SonyakMenu::from(&config);
	let restored = characters::species::sonyak::SonyakConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.colors.hair, restored.colors.hair);
	assert_eq!(config.sliders.shoulder_width, restored.sliders.shoulder_width);
	Ok(())
}

#[test]
fn claber_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::claber::ClaberConfig::default_preview();
	let menu = crate::characters::claber::ClaberMenu::from(&config);
	let restored = characters::species::claber::ClaberConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.sliders.arm_length, restored.sliders.arm_length);
	assert_eq!(config.colors.body, restored.colors.body);
	Ok(())
}

#[test]
fn croconot_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::croconot::CroconotConfig::default_preview();
	let menu = crate::characters::croconot::CroconotMenu::from(&config);
	let restored = characters::species::croconot::CroconotConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.sliders.arm_length, restored.sliders.arm_length);
	assert_eq!(config.colors.body, restored.colors.body);
	Ok(())
}

#[test]
fn brodler_config_round_trip() -> anyhow::Result<()> {
	let config = BrodlerConfig::default_preview();
	let menu = BrodlerMenu::from(&config);
	let restored = BrodlerConfig::from(&menu);
	assert_eq!(config.head, restored.head);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.colors.skin, restored.colors.skin);
	Ok(())
}

#[test]
fn dui_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::dui::DuiConfig::default_preview();
	let menu = crate::characters::dui::DuiMenu::from(&config);
	let restored = characters::species::dui::DuiConfig::from(&menu);
	assert_eq!(config.nose, restored.nose);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.nose_color, restored.colors.nose_color);
	Ok(())
}

#[test]
fn lidder_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::lidder::LidderConfig::default_preview();
	let menu = crate::characters::lidder::LidderMenu::from(&config);
	let restored = characters::species::lidder::LidderConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn chupri_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::chupri::ChupriConfig::default_preview();
	let menu = crate::characters::chupri::ChupriMenu::from(&config);
	let restored = characters::species::chupri::ChupriConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn brokker_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::brokker::BrokkerConfig::default_preview();
	let menu = crate::characters::brokker::BrokkerMenu::from(&config);
	let restored = characters::species::brokker::BrokkerConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.snout, restored.colors.snout);
	Ok(())
}

#[test]
fn tipple_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::tipple::TippleConfig::default_preview();
	let menu = crate::characters::tipple::TippleMenu::from(&config);
	let restored = characters::species::tipple::TippleConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn topple_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::topple::ToppleConfig::default_preview();
	let menu = crate::characters::topple::ToppleMenu::from(&config);
	let restored = characters::species::topple::ToppleConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn kispar_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::kispar::KisparConfig::default_preview();
	let menu = crate::characters::kispar::KisparMenu::from(&config);
	let restored = characters::species::kispar::KisparConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn tapp_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::tapp::TappConfig::default_preview();
	let menu = crate::characters::tapp::TappMenu::from(&config);
	let restored = characters::species::tapp::TappConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn kaller_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::kaller::KallerConfig::default_preview();
	let menu = crate::characters::kaller::KallerMenu::from(&config);
	let restored = characters::species::kaller::KallerConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.snout, restored.colors.snout);
	assert_eq!(config.colors.crown, restored.colors.crown);
	Ok(())
}

#[test]
fn kappler_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::kappler::KapplerConfig::default_preview();
	let menu = crate::characters::kappler::KapplerMenu::from(&config);
	let restored = characters::species::kappler::KapplerConfig::from(&menu);
	assert_eq!(config.beak, restored.beak);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.colors.plumage, restored.colors.plumage);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.beak, restored.colors.beak);
	Ok(())
}

#[test]
fn mygr_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::mygr::MygrConfig::default_preview();
	let menu = crate::characters::mygr::MygrMenu::from(&config);
	let restored = characters::species::mygr::MygrConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	Ok(())
}

#[test]
fn wumbus_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::wumbus::WumbusConfig::default_preview();
	let menu = crate::characters::wumbus::WumbusMenu::from(&config);
	let restored = characters::species::wumbus::WumbusConfig::from(&menu);
	assert_eq!(config.horns, restored.horns);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.eyes, restored.colors.eyes);
	assert_eq!(config.colors.ears, restored.colors.ears);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	assert_eq!(config.colors.horns, restored.colors.horns);
	assert_eq!(config.colors.spine, restored.colors.spine);
	Ok(())
}

#[test]
fn lero_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::lero::LeroConfig::default_preview();
	let menu = crate::characters::lero::LeroMenu::from(&config);
	let restored = characters::species::lero::LeroConfig::from(&menu);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	assert_eq!(config.colors.tail, restored.colors.tail);
	assert_eq!(config.colors.spine, restored.colors.spine);
	Ok(())
}

#[test]
fn spibmom_config_round_trip() -> anyhow::Result<()> {
	let config = characters::species::spibmom::SpibmomConfig::default_preview();
	let menu = crate::characters::spibmom::SpibmomMenu::from(&config);
	let restored = characters::species::spibmom::SpibmomConfig::from(&menu);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.colors.skin, restored.colors.skin);
	assert_eq!(config.colors.crown, restored.colors.crown);
	assert_eq!(config.colors.spine, restored.colors.spine);
	assert_eq!(config.colors.mouth, restored.colors.mouth);
	Ok(())
}

#[test]
fn tuberwaber_config_round_trip() -> anyhow::Result<()> {
	let config = TuberwaberConfig::default_preview();
	let menu = TuberwaberMenu::from(&config);
	let restored = TuberwaberConfig::from(&menu);
	assert_eq!(config.gender, restored.gender);
	assert_eq!(config.build, restored.build);
	assert_eq!(config.body, restored.body);
	assert_eq!(config.head, restored.head);
	assert_eq!(config.eye, restored.eye);
	assert_eq!(config.nose, restored.nose);
	assert_eq!(config.mouth, restored.mouth);
	assert_eq!(config.hair, restored.hair);
	assert_eq!(config.clothing, restored.clothing);
	assert_eq!(config.colors, restored.colors);
	Ok(())
}

#[test]
fn clothing_toggle_and_color() -> anyhow::Result<()> {
	use character_items::ItemColor;

	use crate::{CharacterField, MenuEvent, SwatchValue};

	let mut menu = CharacterMenu::default();
	let coat = ClothingMesh::FittedCoat;
	assert!(menu.apply(MenuEvent::ToggleClothing(coat)));
	assert!(menu.braidman.clothing.value.layers.contains(coat));
	assert!(menu.apply(MenuEvent::SetSwatch(
		CharacterField::Clothing(coat),
		SwatchValue::Item(ItemColor::Red),
	)));
	assert_eq!(menu.braidman.clothing_color(coat), ItemColor::Red);
	Ok(())
}

#[test]
fn clothing_material_applies_to_braidman() -> anyhow::Result<()> {
	use character_items::ClothingMaterial;

	use crate::{AssetValue, CharacterField, MenuEvent};

	let mut menu = CharacterMenu::default();
	let tunic = ClothingMesh::Tunic;
	assert!(menu.apply(MenuEvent::ToggleClothing(tunic)));
	assert_eq!(menu.braidman.clothing.value.material_for(tunic), ClothingMaterial::Cloth);
	assert!(menu.apply(MenuEvent::SetAsset(
		CharacterField::ClothingMaterial(tunic),
		AssetValue::ClothingMaterial(ClothingMaterial::Glitter),
	)));
	assert_eq!(menu.braidman.clothing.value.material_for(tunic), ClothingMaterial::Glitter);
	let config = characters::species::braidman::BraidmanConfig::from(&menu.braidman);
	assert_eq!(config.colors.clothing_material, ClothingMaterial::Cloth);
	assert_eq!(config.colors.clothing_material_for(tunic), ClothingMaterial::Glitter);
	assert_eq!(config.colors.clothing_material_for(ClothingMesh::Pants), ClothingMaterial::Cloth);
	Ok(())
}

#[test]
fn character_menu_lowers_to_species_select_tree() -> anyhow::Result<()> {
	let menu = CharacterMenu::default();
	let nodes = menu.menu_nodes();
	assert_eq!(nodes.len(), 2);
	let MenuNode::ShortText { label, value, max_len } = &nodes[0] else {
		anyhow::bail!("expected a Name ShortText at the root");
	};
	assert_eq!(*label, "Name");
	assert!(value.is_empty());
	assert_eq!(*max_len, crate::CHARACTER_NAME_MAX_LEN);
	let MenuNode::SectionSelect { label, groups, children } = &nodes[1] else {
		anyhow::bail!("expected a species SectionSelect after the name field");
	};
	assert_eq!(*label, "Species");
	assert_eq!(groups.len(), 4);
	assert_eq!(groups[0].label, Some("Humanoids"));
	assert_eq!(groups[1].label, Some("Quadrupeds"));
	assert_eq!(groups[2].label, Some("Birds"));
	assert_eq!(groups[3].label, Some("Aquatic"));
	let choice_count: usize = groups.iter().map(|group| group.choices.len()).sum();
	assert_eq!(choice_count, 28);
	assert!(groups[0].choices[0].selected, "default species should be braidman");
	assert_eq!(groups[0].choices[0].label, "braidman");
	// Braidman: presets, body, head & features, hair, clothing, animation.
	assert_eq!(children.len(), 6);
	assert!(children.iter().all(|child| matches!(child, MenuNode::Section { .. })));
	Ok(())
}

#[test]
fn clothing_swatches_only_lower_for_worn_layers() -> anyhow::Result<()> {
	use crate::MenuEvent;

	let mut menu = CharacterMenu::default();
	let coat = ClothingMesh::FittedCoat;
	assert!(menu.apply(MenuEvent::ToggleClothing(coat)));
	let nodes = menu.braidman.clothing.value.menu_nodes();
	let MenuNode::ItemMultiSelect { rows, .. } = &nodes[0] else {
		anyhow::bail!("expected clothing to lower to an ItemMultiSelect");
	};
	for row in rows {
		assert_eq!(row.asset.selected, !row.colors.is_empty());
		assert_eq!(row.asset.selected, !row.materials.is_empty());
	}
	Ok(())
}

#[test]
fn body_color_syncs_skin() -> anyhow::Result<()> {
	use character_items::ItemColor;

	use crate::{CharacterField, MenuEvent, SwatchValue};

	let mut menu = CharacterMenu::default();
	assert!(menu.apply(MenuEvent::SetSwatch(
		CharacterField::BodyColor,
		SwatchValue::Item(ItemColor::Warm),
	)));
	let config = menu.braidman_config();
	assert_eq!(config.colors.body, ItemColor::Warm);
	assert_eq!(config.colors.head, ItemColor::Warm);
	assert_eq!(config.colors.nose, ItemColor::Warm);
	Ok(())
}

#[test]
fn create_menu_restricts_species_to_humanoids() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};

	let items = vec![InventoryItem::clothing(
		ClothingMesh::TankTop,
		ClothingMaterial::Cloth,
		ItemColor::Natural,
	)];
	let mut menu = CharacterMenu::for_create(items);
	let nodes = menu.menu_nodes();
	let MenuNode::SectionSelect { groups, .. } = &nodes[1] else {
		anyhow::bail!("expected species SectionSelect");
	};
	assert_eq!(groups.len(), 1);
	assert_eq!(groups[0].choices.len(), ConceptSpecies::HUMANOIDS.len());
	assert!(!menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Brenal)));
	assert_eq!(menu.species.value, ConceptSpecies::Braidman);
	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Brodler)));
	assert_eq!(menu.species.value, ConceptSpecies::Brodler);
	Ok(())
}

#[test]
fn create_menu_clothing_is_grid_catalog() -> anyhow::Result<()> {
	use character_items::{
		ClothingMaterial, ClothingMesh, InventoryItem, ItemColor, WORN_CLOTHING_LIMIT,
	};

	let items: Vec<_> = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Hawaiian, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Hawaiian, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Hawaiian, ItemColor::Red),
	];
	let expected_name = items[0].name();
	let expected_key = items[0].icon_catalog_key();
	let mut menu = CharacterMenu::for_create(items);
	let nodes = menu.braidman.clothing.value.menu_nodes();
	let MenuNode::GridCatalog { max_selected, choices, .. } = &nodes[0] else {
		anyhow::bail!("expected clothing to lower to a GridCatalog");
	};
	assert_eq!(*max_selected, WORN_CLOTHING_LIMIT);
	assert_eq!(choices.len(), 3);
	assert!(choices[0].selected);
	assert!(choices[1].selected);
	assert!(!choices[2].selected);
	assert_eq!(choices[0].rank, Some(1));
	assert_eq!(choices[1].rank, Some(2));
	assert_eq!(choices[2].rank, None);
	assert_eq!(choices[0].label, expected_name);
	assert_eq!(choices[0].image_key, expected_key);
	assert!(!choices[0].detail.is_empty());
	assert!(menu.apply(MenuEvent::ToggleInventory(0)));
	assert!(!menu.inventory.as_ref().expect("create inventory").is_worn(0));
	assert!(!menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Hars)));
	Ok(())
}

#[test]
fn create_menu_weapons_is_ranked_grid_catalog() -> anyhow::Result<()> {
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor, WEAPON_QUEUE_LIMIT,
	};

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Hawaiian, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Hawaiian, ItemColor::Red),
		InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Hawaiian, ItemColor::Red),
		InventoryItem::firearm(FirearmMesh::Bullpup),
		InventoryItem::firearm(FirearmMesh::Reltor),
	];
	let expected_name = items[3].name();
	let expected_key = items[3].icon_catalog_key();
	let mut menu = CharacterMenu::for_create(items);
	let nodes = menu.menu_nodes();
	let weapons = nodes.iter().find_map(|node| match node {
		MenuNode::Section { label: "Weapons", children } => children.first(),
		_ => None,
	});
	let Some(MenuNode::GridCatalog { max_selected, choices, .. }) = weapons else {
		anyhow::bail!("expected a top-level Weapons GridCatalog");
	};
	assert_eq!(*max_selected, WEAPON_QUEUE_LIMIT);
	assert_eq!(choices.len(), 2);
	assert_eq!(choices[0].rank, Some(1));
	assert_eq!(choices[1].rank, Some(2));
	assert!(choices[0].selected);
	assert_eq!(choices[0].label, expected_name);
	assert_eq!(choices[0].image_key, expected_key);
	assert!(!choices[0].detail.is_empty());
	assert!(menu.overlay_editable("Weapons"));
	assert!(menu.apply(MenuEvent::ToggleInventory(3)));
	let inventory = menu.inventory.as_ref().expect("create inventory");
	assert_eq!(inventory.weapons, vec![4]);
	assert_eq!(inventory.clothing, vec![0, 1]);
	assert_eq!(inventory.rank(4), Some(1));
	Ok(())
}

#[test]
fn create_menu_skill_maps_is_ranked_grid_catalog() -> anyhow::Result<()> {
	use character_items::{InventoryItem, SkillMapKind, SkillMapSpec, SKILL_MAP_QUEUE_LIMIT};

	let items = vec![
		InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
		InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
	];
	let expected_name = items[0].name();
	let expected_key = items[0].icon_catalog_key();
	let mut menu = CharacterMenu::for_create(items);
	let nodes = menu.menu_nodes();
	let skills = nodes.iter().find_map(|node| match node {
		MenuNode::Section { label: "Skill Maps", children } => children.first(),
		_ => None,
	});
	let Some(MenuNode::GridCatalog { max_selected, choices, .. }) = skills else {
		anyhow::bail!("expected a top-level Skill Maps GridCatalog");
	};
	assert_eq!(*max_selected, SKILL_MAP_QUEUE_LIMIT);
	assert_eq!(choices.len(), 2);
	assert_eq!(choices[0].rank, Some(1));
	assert_eq!(choices[1].rank, Some(2));
	assert_eq!(choices[0].label, expected_name);
	assert_ne!(choices[0].preview, character_ui_menu::PreviewColor::WHITE);
	assert_ne!(choices[0].preview, choices[1].preview);
	assert_eq!(choices[0].image_key, expected_key);
	assert!(menu.overlay_editable("Skill Maps"));
	assert!(menu.apply(MenuEvent::ToggleInventory(0)));
	let inventory = menu.inventory.as_ref().expect("create inventory");
	assert_eq!(inventory.skills, vec![1]);
	Ok(())
}

#[test]
fn create_menu_loadout_compiles_character_sheet() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor};

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::firearm(FirearmMesh::Bullpup),
	];
	let sheet = {
		use character_items::Inventory;
		Inventory::with_starter_outfit(items.clone()).character_sheet()
	};
	let weapon_name = items[1].name();
	let menu = CharacterMenu::for_create(items);
	let nodes = menu.menu_nodes();
	let loadout = nodes.iter().find_map(|node| match node {
		MenuNode::Section { label: "Loadout", children } => Some(children),
		_ => None,
	});
	let Some(children) = loadout else {
		anyhow::bail!("expected a top-level Loadout section");
	};
	let Some(MenuNode::StatGrid { cards }) = children.first() else {
		anyhow::bail!("expected Loadout to lower to a StatGrid");
	};
	assert_eq!(cards[0].title, "Total Stats");
	assert_eq!(cards[1].title, "Base Stats");
	assert_eq!(cards[2].title, "Buffs");
	let health = cards[0]
		.rows
		.iter()
		.find(|row| row.label == "Health")
		.map(|row| row.value.as_str());
	assert_eq!(health, Some(sheet.health.to_string()).as_deref());
	assert!(cards[0].rows.iter().any(|row| row.label == "Damage Bonus"));
	assert!(cards[0].rows.iter().any(|row| row.label == "Added Weight"));
	assert!(cards[1].rows.iter().any(|row| row.label == "Damage Bonus"));
	assert!(cards[1].rows.iter().any(|row| row.label == "Added Weight"));
	let pace = cards[0]
		.rows
		.iter()
		.find(|row| row.label == "Pace")
		.map(|row| row.value.as_str());
	assert_eq!(pace, Some(sheet.pace_equation()).as_deref());
	assert_eq!(
		cards[0].rows.iter().find(|row| row.label == "Pace").map(|row| row.tone),
		Some(character_ui_menu::StatTone::Formula)
	);
	assert_eq!(cards.len(), 4);
	assert_eq!(cards[3].title, weapon_name);
	assert!(menu.overlay_editable("Loadout"));
	Ok(())
}

#[test]
fn create_menu_omits_save_from_the_editor_tree() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};

	let items = vec![InventoryItem::clothing(
		ClothingMesh::TankTop,
		ClothingMaterial::Cloth,
		ItemColor::Natural,
	)];
	let mut menu = CharacterMenu::for_create(items);
	let nodes = menu.menu_nodes();
	assert!(nodes.iter().all(|node| !matches!(node, MenuNode::Action { .. })));
	assert!(menu.is_create());
	assert!(menu.apply(MenuEvent::Save));
	Ok(())
}

#[test]
fn saved_menu_locks_body_and_keeps_inventory() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
	];
	let mut created = CharacterMenu::for_create(items);
	created.name = String::from("Misty");
	let appearance = created.appearance();
	let inventory = created.inventory.clone().expect("inventory");
	let mut menu = CharacterMenu::for_saved(created.saved_name(), &appearance, inventory);
	assert!(menu.appearance_locked());
	assert!(!menu.is_create());
	let nodes = menu.menu_nodes();
	let MenuNode::SectionSelect { children, .. } = &nodes[1] else {
		anyhow::bail!("expected species SectionSelect so body attributes stay visible");
	};
	assert!(children
		.iter()
		.any(|node| matches!(node, MenuNode::Section { label: "Body", .. })));
	assert!(children
		.iter()
		.any(|node| matches!(node, MenuNode::Section { label: "Clothing", .. })));
	assert!(nodes.iter().all(|node| !matches!(node, MenuNode::Action { .. })));
	assert!(!menu.overlay_editable("Species"));
	assert!(!menu.overlay_editable("Body"));
	assert!(menu.overlay_editable("Clothing"));
	assert!(menu.overlay_editable("Weapons"));
	assert!(menu.overlay_editable("Skill Maps"));
	assert!(menu.overlay_editable("Loadout"));
	assert!(!menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Brodler)));
	assert_eq!(menu.species.value, ConceptSpecies::Braidman);
	assert!(!menu.apply(MenuEvent::Cycle(crate::event::CharacterField::Gender, 1)));
	assert!(menu.apply(MenuEvent::ToggleInventory(0)));
	assert!(!menu.inventory.as_ref().expect("inventory").is_worn(0));
	assert!(!menu.apply(MenuEvent::Save));
	Ok(())
}

#[test]
fn saved_menu_strips_clothing_from_appearance() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};
	use characters::CharacterAppearance;

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
	];
	let mut menu = CharacterMenu::for_create(items);
	menu.name = String::from("Misty");
	let appearance = menu.appearance();
	let CharacterAppearance::Braidman(config) = &appearance else {
		anyhow::bail!("expected braidman");
	};
	assert!(config.clothing.is_empty());
	assert!(config.colors.clothing.is_empty());
	let inventory = menu.inventory.clone().expect("inventory");
	let restored = CharacterMenu::for_saved(menu.saved_name(), &appearance, inventory);
	assert_eq!(restored.name, "Misty");
	assert_eq!(restored.appearance(), appearance);
	assert_eq!(restored.inventory.as_ref().expect("inventory").worn().len(), 2);
	Ok(())
}
