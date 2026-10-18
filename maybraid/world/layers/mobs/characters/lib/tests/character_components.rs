//! CharacterComponents / Clothed composition tests.

use anyhow::{anyhow, Result};
use character_items::{ClothingMaterial, ClothingMesh};
use characters::{
	species::{
		braidman::{pose::BraidmanPose, Braidman, BraidmanConfig},
		brenal::{Brenal, BrenalConfig},
		brodler::{Brodler, BrodlerConfig},
		caole::{Caole, CaoleConfig},
		claber::{Claber, ClaberConfig},
		common::{nodes as humanoid, BodyMesh},
		croconot::{Croconot, CroconotConfig},
		dui::DuiConfig,
		epiphant::{Epiphant, EpiphantConfig},
		hars::{Hars, HarsConfig},
		sonyak::{Sonyak, SonyakConfig},
		spibmom::{Spibmom, SpibmomConfig},
		topple::{Topple, ToppleConfig},
		wumbus::{Wumbus, WumbusConfig},
		ylter::{Yilter, YilterConfig},
	},
	BuildPreset, CharacterComponents, CharacterPartSlot, CharacterRecipe, Clothed, HeadCapsule,
	HitCapsule, Layer, LocomotionCapsule, PartNode, RigId,
};
use lod::scene::LodSceneLevel;
use scene_ref::MirrorAxis;

fn assert_right_features_reflected(parts: &[characters::PartNode]) {
	let right_ear = parts
		.iter()
		.find(|p| p.slot == characters::CharacterPartSlot::EarRight)
		.expect("right ear");
	assert_eq!(right_ear.scene.mirror, Some(MirrorAxis::X));
	assert!(right_ear.scene.reflect_instance);
	let right_eye = parts
		.iter()
		.find(|p| p.slot == characters::CharacterPartSlot::EyeRight)
		.expect("right eye");
	assert_eq!(right_eye.scene.mirror, Some(MirrorAxis::X));
	assert!(right_eye.scene.reflect_instance);
}

fn assert_same_motor(hull: LocomotionCapsule, expected: LocomotionCapsule) {
	assert!((hull.radius - expected.radius).abs() < 1e-5);
	assert!((hull.length - expected.length).abs() < 1e-5);
	assert_eq!(hull.pronograde, expected.pronograde);
}

fn assert_ear_socket_locals(parts: &[characters::PartNode]) {
	let local = |slot| {
		parts
			.iter()
			.find(|p| p.slot == slot)
			.and_then(|p| p.socket)
			.map(|s| s.local)
			.expect("socket")
	};
	let left_ear = local(characters::CharacterPartSlot::EarLeft);
	let right_ear = local(characters::CharacterPartSlot::EarRight);
	assert_eq!(left_ear, humanoid::ear_left_socket_local());
	assert_eq!(right_ear, humanoid::ear_right_socket_local());
	assert_ne!(left_ear, right_ear);
	assert!(right_ear.translation.x < 0.0);
	assert_eq!(
		local(characters::CharacterPartSlot::EyeLeft),
		local(characters::CharacterPartSlot::EyeRight)
	);
}

#[test]
fn braidman_emits_rigs_and_parts_at_every_band() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	let high_rigs = braidman.rig_nodes_for_level(LodSceneLevel::High);
	let low_rigs = braidman.rig_nodes_for_level(LodSceneLevel::Low);
	assert_eq!(high_rigs.len(), 2);
	assert_eq!(high_rigs.len(), low_rigs.len());

	let high_parts = braidman.part_nodes_for_level(LodSceneLevel::High);
	let ultra = braidman.part_nodes_for_level(LodSceneLevel::UltraLow);
	assert_eq!(high_parts.len(), ultra.len());
	assert!(high_parts.len() >= 8);
}

#[test]
fn braidman_eyes_and_mouth_use_face_recipes() {
	use character_shaders::{RECIPE_FACE_EYE, RECIPE_FACE_MOUTH};
	use material_ref::MaterialId;

	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	let parts = braidman.part_nodes_for_level(LodSceneLevel::High).flatten();
	let name = |slot| {
		parts
			.iter()
			.find(|p| p.slot == slot)
			.map(|p| p.material.name.clone())
			.expect("part")
	};
	assert_eq!(name(CharacterPartSlot::EyeLeft), MaterialId::named(RECIPE_FACE_EYE));
	assert_eq!(name(CharacterPartSlot::EyeRight), MaterialId::named(RECIPE_FACE_EYE));
	assert_eq!(name(CharacterPartSlot::Mouth), MaterialId::named(RECIPE_FACE_MOUTH));
	assert!(!matches!(name(CharacterPartSlot::HeadMesh), MaterialId::Name(_)));
}

#[test]
fn custom_and_quadruped_faces_use_face_recipes() {
	use character_shaders::{RECIPE_FACE_EYE, RECIPE_FACE_MOUTH};
	use material_ref::MaterialId;

	let name_of = |parts: &[PartNode], slot| {
		parts
			.iter()
			.find(|p| p.slot == slot)
			.map(|p| p.material.name.clone())
			.expect("part")
	};

	let dui = characters::species::dui::Dui::from_config(&DuiConfig::default_preview());
	let dui_parts = dui.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_eq!(name_of(&dui_parts, CharacterPartSlot::EyeLeft), MaterialId::named(RECIPE_FACE_EYE));
	assert_eq!(name_of(&dui_parts, CharacterPartSlot::Mouth), MaterialId::named(RECIPE_FACE_MOUTH));

	let caole = Caole::from_config(&CaoleConfig::default_preview());
	let caole_parts = caole.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_eq!(
		name_of(&caole_parts, CharacterPartSlot::EyeLeft),
		MaterialId::named(RECIPE_FACE_EYE)
	);
	assert_eq!(
		name_of(&caole_parts, CharacterPartSlot::Mouth),
		MaterialId::named(RECIPE_FACE_MOUTH)
	);

	let wumbus = Wumbus::from_config(&WumbusConfig::default_preview());
	let wumbus_parts = wumbus.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_eq!(
		name_of(&wumbus_parts, CharacterPartSlot::EyeLeft),
		MaterialId::named(RECIPE_FACE_EYE)
	);
	assert_eq!(
		name_of(&wumbus_parts, CharacterPartSlot::Mouth),
		MaterialId::named(RECIPE_FACE_MOUTH)
	);

	let hars = Hars::from_config(&HarsConfig::default_preview());
	let hars_parts = hars.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_eq!(
		name_of(&hars_parts, CharacterPartSlot::EyeLeft),
		MaterialId::named(RECIPE_FACE_EYE)
	);
	assert_eq!(
		name_of(&hars_parts, CharacterPartSlot::Mouth),
		MaterialId::named(RECIPE_FACE_MOUTH)
	);
}

#[test]
fn clothed_braidman_adds_clothing_layer() {
	let mut config = BraidmanConfig::default_preview();
	config.clothing.push(ClothingMesh::Tunic);
	let inner = Braidman::from_config(&config);
	let clothed = config.clothed();
	let inner_len = inner.part_nodes_for_level(LodSceneLevel::High).len();
	let clothed_parts = clothed.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(clothed_parts.len(), inner_len + 1);
	assert!(clothed_parts.labeled.contains_key(&Layer::new("clothing")));
	assert_eq!(clothed.rig_nodes_for_level(LodSceneLevel::High).len(), 2);
}

#[test]
fn braidman_uses_the_humanoid_hull() {
	let config = BraidmanConfig::default_preview();
	let expected =
		LocomotionCapsule::humanoid_from_pose(&BraidmanPose::from_config(&config).resolve(), 1.0);
	let braidman = Braidman::from_config(&config);
	assert_eq!(braidman.locomotion_capsule(), expected);
	assert!(braidman.locomotion_capsule().hit_capsule().is_none());
	let clothed = config.clothed();
	assert_eq!(clothed.locomotion_capsule(), expected);
}

#[test]
fn whelp_and_quadruped_hulls_differ_from_humanoid() {
	let topple = Topple::from_config(&ToppleConfig::default_preview());
	assert_eq!(
		topple.locomotion_capsule(),
		LocomotionCapsule::HUMANOID.scaled(0.30).with_head_scale(1.85)
	);
	let brenal = Brenal::from_config(&BrenalConfig::default_preview());
	assert_same_motor(
		brenal.locomotion_capsule(),
		LocomotionCapsule::quadruped_for_limb_length(1.0),
	);
	assert!(brenal.locomotion_capsule().girdle > 0.0);
	assert_ne!(topple.locomotion_capsule(), LocomotionCapsule::HUMANOID);
	assert_ne!(brenal.locomotion_capsule(), LocomotionCapsule::HUMANOID);
}

#[test]
fn hars_hull_follows_rest_limb_length() {
	let hars = Hars::from_config(&HarsConfig::default_preview());
	assert_same_motor(
		hars.locomotion_capsule(),
		LocomotionCapsule::quadruped_for_limb_length(1.35),
	);

	let lanky = Hars::from_config(&HarsConfig::default_preview().with_build(BuildPreset::Lanky));
	assert_same_motor(
		lanky.locomotion_capsule(),
		LocomotionCapsule::quadruped_for_limb_length(1.35 * 1.05),
	);

	let mut long_legs = HarsConfig::default_preview();
	long_legs.sliders.leg_length = 1.2;
	let long = Hars::from_config(&long_legs);
	assert_same_motor(
		long.locomotion_capsule(),
		LocomotionCapsule::quadruped_for_limb_length(1.35 * 1.2),
	);
}

#[test]
fn ylter_and_croconot_hulls_follow_species_limb_length() {
	let ylter = Yilter::from_config(&YilterConfig::default_preview());
	assert_same_motor(
		ylter.locomotion_capsule(),
		LocomotionCapsule::quadruped_for_limb_length(1.4 * 1.1),
	);
	let croconot = Croconot::from_config(&CroconotConfig::default_preview());
	assert_same_motor(
		croconot.locomotion_capsule(),
		LocomotionCapsule::quadruped_for_limb_length(0.8),
	);
}

#[test]
fn biped_hull_follows_legs_shoulders_and_head() -> Result<()> {
	let average = Braidman::from_config(&BraidmanConfig::default_preview());
	let mut long = BraidmanConfig::default_preview();
	long.sliders.leg_length = 1.2;
	let long = Braidman::from_config(&long);
	assert!(long.locomotion_capsule().half_height() > average.locomotion_capsule().half_height());

	let brodler = Brodler::from_config(&BrodlerConfig::default_preview());
	assert!(brodler.locomotion_capsule().radius > LocomotionCapsule::HUMANOID.radius);
	assert!(brodler.locomotion_capsule().hit_capsule().is_none());

	let spibmom = Spibmom::from_config(&SpibmomConfig::default_preview());
	assert!(spibmom.locomotion_capsule().half_height() > LocomotionCapsule::HUMANOID.half_height());
	assert!(spibmom.locomotion_capsule().hit_capsule().is_none());
	let head = spibmom
		.locomotion_capsule()
		.head_capsule()
		.ok_or_else(|| anyhow!("Spibmom 2× head should get a head volume"))?;
	assert!((head.radius - HeadCapsule::REST_HALF).abs() < 1e-5);
	assert!(head.length > HeadCapsule::REST_HALF * 2.0);
	assert!(
		head.crown_y() > spibmom.locomotion_capsule().half_height() + HeadCapsule::REST_HALF * 2.0
	);
	Ok(())
}

#[test]
fn quadruped_hit_girdle_follows_rest_pose_bones() -> Result<()> {
	let average = Hars::from_config(&HarsConfig::default_preview());
	let mut wide = HarsConfig::default_preview();
	wide.build = BuildPreset::Stocky;
	wide.sliders.hip_width = 1.4;
	let wide = Hars::from_config(&wide);
	assert!(wide.locomotion_capsule().girdle > average.locomotion_capsule().girdle);
	assert!(average.locomotion_capsule().girdle > HitCapsule::REST_HALF_WIDTH);
	let hit = wide
		.locomotion_capsule()
		.hit_capsule()
		.ok_or_else(|| anyhow!("pronograde Hars should stamp a hit hull"))?;
	assert!((hit.radius - wide.locomotion_capsule().girdle).abs() < 1e-5);
	Ok(())
}

#[test]
fn clothing_layer_uses_named_shader_recipe() {
	let mut config = BraidmanConfig::default_preview();
	config.clothing.push(ClothingMesh::Tunic);
	config.colors.clothing_material = ClothingMaterial::Glitter;
	let part = config
		.clothed()
		.part_nodes_for_level(LodSceneLevel::High)
		.flatten()
		.into_iter()
		.find(|part| part.slot == CharacterPartSlot::Clothing)
		.expect("clothing part");
	assert_eq!(part.material.name, material_ref::MaterialId::named("clothing_glitter"));
	assert_eq!(
		part.material.palette.first().copied(),
		Some(config.colors.clothing_color(ClothingMesh::Tunic).color())
	);
}

#[test]
fn clothing_layers_use_per_item_shader_recipes() {
	let mut config = BraidmanConfig::default_preview();
	config.clothing.push(ClothingMesh::Tunic);
	config.clothing.push(ClothingMesh::Pants);
	config
		.colors
		.set_clothing_material(ClothingMesh::Tunic, ClothingMaterial::Tattered);
	config
		.colors
		.set_clothing_material(ClothingMesh::Pants, ClothingMaterial::Glitter);
	let recipes: Vec<_> = config
		.clothed()
		.part_nodes_for_level(LodSceneLevel::High)
		.flatten()
		.into_iter()
		.filter(|part| part.slot == CharacterPartSlot::Clothing)
		.map(|part| part.material.name)
		.collect();
	assert_eq!(
		recipes,
		vec![
			material_ref::MaterialId::named("clothing_tattered"),
			material_ref::MaterialId::named("clothing_glitter"),
		]
	);
}

fn clothing_scene_path(config: &impl CharacterRecipe) -> String {
	config
		.clothed()
		.part_nodes_for_level(LodSceneLevel::High)
		.flatten()
		.into_iter()
		.find(|part| part.slot == CharacterPartSlot::Clothing)
		.map(|part| part.scene.path)
		.expect("clothing part")
}

#[test]
fn tank_top_on_standard_braidman_uses_humanoid_fit() {
	let mut config = BraidmanConfig::default_preview();
	config.body = BodyMesh::Standard;
	config.clothing.push(ClothingMesh::TankTop);
	assert_eq!(
		clothing_scene_path(&config),
		"characters/clothes/body/humanoid_full_body/tank_top.glb"
	);
}

#[test]
fn tank_top_on_full_braidman_uses_leron_fit() {
	let mut config = BraidmanConfig::default_preview();
	config.body = BodyMesh::Full;
	config.clothing.push(ClothingMesh::TankTop);
	assert_eq!(
		clothing_scene_path(&config),
		"characters/clothes/body/leron_biped_full_body/tank_top.glb"
	);
}

#[test]
fn tank_top_on_dui_uses_igeo_fit() {
	let mut config = DuiConfig::default_preview();
	config.clothing.push(ClothingMesh::TankTop);
	assert_eq!(
		clothing_scene_path(&config),
		"characters/clothes/body/igeo_biped_full_body/tank_top.glb"
	);
}

#[test]
fn fitted_coat_on_standard_braidman_uses_humanoid_fit() {
	let mut config = BraidmanConfig::default_preview();
	config.body = BodyMesh::Standard;
	config.clothing.push(ClothingMesh::FittedCoat);
	assert_eq!(
		clothing_scene_path(&config),
		"characters/clothes/body/humanoid_full_body/fitted_coat.glb"
	);
}

#[test]
fn long_dress_on_standard_braidman_uses_humanoid_fit() {
	let mut config = BraidmanConfig::default_preview();
	config.body = BodyMesh::Standard;
	config.clothing.push(ClothingMesh::LongDress);
	assert_eq!(
		clothing_scene_path(&config),
		"characters/clothes/body/humanoid_full_body/long_dress.glb"
	);
}

#[test]
fn short_dress_on_standard_braidman_uses_humanoid_fit() {
	let mut config = BraidmanConfig::default_preview();
	config.body = BodyMesh::Standard;
	config.clothing.push(ClothingMesh::ShortDress);
	assert_eq!(
		clothing_scene_path(&config),
		"characters/clothes/body/humanoid_full_body/short_dress.glb"
	);
}

#[test]
fn robe_on_standard_braidman_uses_humanoid_fit() {
	let mut config = BraidmanConfig::default_preview();
	config.body = BodyMesh::Standard;
	config.clothing.push(ClothingMesh::Robe);
	assert_eq!(clothing_scene_path(&config), "characters/clothes/body/humanoid_full_body/robe.glb");
}

#[test]
fn right_features_use_scene_ref_instance_reflect() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	assert_right_features_reflected(&braidman.part_nodes_for_level(LodSceneLevel::High).flatten());
}

#[test]
fn right_ear_keeps_right_socket_local() {
	let braidman = Braidman::from_config(&BraidmanConfig::default_preview());
	assert_ear_socket_locals(&braidman.part_nodes_for_level(LodSceneLevel::High).flatten());
}

#[test]
fn clothed_from_config_matches_helper() {
	let config = BraidmanConfig::default_preview();
	let a = config.clothed();
	let b = Clothed::new(Braidman::from_config(&config), Vec::new());
	assert_eq!(
		a.part_nodes_for_level(LodSceneLevel::High).len(),
		b.part_nodes_for_level(LodSceneLevel::High).len()
	);
}

#[test]
fn brodler_emits_rigs_and_parts_at_every_band() {
	let brodler = Brodler::from_config(&BrodlerConfig::default_preview());
	let high_rigs = brodler.rig_nodes_for_level(LodSceneLevel::High);
	assert_eq!(high_rigs.len(), 2);
	assert_eq!(high_rigs.len(), brodler.rig_nodes_for_level(LodSceneLevel::Low).len());

	let high_parts = brodler.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(high_parts.len(), brodler.part_nodes_for_level(LodSceneLevel::UltraLow).len());
	assert!(high_parts.len() >= 9);
}

#[test]
fn clothed_brodler_adds_clothing_layer() {
	let mut config = BrodlerConfig::default_preview();
	config.clothing.push(ClothingMesh::Tunic);
	let inner = Brodler::from_config(&config);
	let clothed = CharacterRecipe::clothed(&config);
	let inner_len = inner.part_nodes_for_level(LodSceneLevel::High).len();
	let clothed_parts = clothed.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(clothed_parts.len(), inner_len + 1);
	assert!(clothed_parts.labeled.contains_key(&Layer::new("clothing")));
}

#[test]
fn brodler_right_features_match_shared_humanoid_helpers() {
	let brodler = Brodler::from_config(&BrodlerConfig::default_preview());
	let parts = brodler.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_features_reflected(&parts);
	assert_ear_socket_locals(&parts);
}

fn assert_bands(name: &str, character: &(impl CharacterComponents + ?Sized)) {
	let high_rigs = character.rig_nodes_for_level(LodSceneLevel::High);
	let low_rigs = character.rig_nodes_for_level(LodSceneLevel::Low);
	assert!(high_rigs.len() >= 1, "{name}: expected ≥1 rig");
	assert_eq!(high_rigs.len(), low_rigs.len(), "{name}: rig High/Low band equality");

	let high_parts = character.part_nodes_for_level(LodSceneLevel::High);
	let low_parts = character.part_nodes_for_level(LodSceneLevel::Low);
	assert!(high_parts.len() >= 1, "{name}: expected ≥1 part");
	assert_eq!(high_parts.len(), low_parts.len(), "{name}: part High/Low band equality");
}

#[test]
fn orthograde_humanoid_species_emit_rigs_and_parts_at_every_band() {
	use characters::species::{
		brokker::{Brokker, BrokkerConfig},
		chupri::{Chupri, ChupriConfig},
		dui::{Dui, DuiConfig},
		kaller::{Kaller, KallerConfig},
		kappler::{Kappler, KapplerConfig},
		kispar::{Kispar, KisparConfig},
		lero::{Lero, LeroConfig},
		lidder::{Lidder, LidderConfig},
		mygr::{Mygr, MygrConfig},
		spibmom::{Spibmom, SpibmomConfig},
		tapp::{Tapp, TappConfig},
		tipple::{Tipple, TippleConfig},
		topple::{Topple, ToppleConfig},
		tuberwaber::{Tuberwaber, TuberwaberConfig},
		wumbus::{Wumbus, WumbusConfig},
	};

	let species: [(&str, Box<dyn CharacterComponents>); 15] = [
		("brokker", Box::new(Brokker::from_config(&BrokkerConfig::default_preview()))),
		("mygr", Box::new(Mygr::from_config(&MygrConfig::default_preview()))),
		("dui", Box::new(Dui::from_config(&DuiConfig::default_preview()))),
		("chupri", Box::new(Chupri::from_config(&ChupriConfig::default_preview()))),
		("lidder", Box::new(Lidder::from_config(&LidderConfig::default_preview()))),
		("kispar", Box::new(Kispar::from_config(&KisparConfig::default_preview()))),
		("tapp", Box::new(Tapp::from_config(&TappConfig::default_preview()))),
		("tipple", Box::new(Tipple::from_config(&TippleConfig::default_preview()))),
		("topple", Box::new(Topple::from_config(&ToppleConfig::default_preview()))),
		("kappler", Box::new(Kappler::from_config(&KapplerConfig::default_preview()))),
		("kaller", Box::new(Kaller::from_config(&KallerConfig::default_preview()))),
		("lero", Box::new(Lero::from_config(&LeroConfig::default_preview()))),
		("wumbus", Box::new(Wumbus::from_config(&WumbusConfig::default_preview()))),
		("tuberwaber", Box::new(Tuberwaber::from_config(&TuberwaberConfig::default_preview()))),
		("spibmom", Box::new(Spibmom::from_config(&SpibmomConfig::default_preview()))),
	];
	for (name, character) in &species {
		assert_bands(name, character.as_ref());
	}
}

fn assert_band_equality(character: &impl CharacterComponents, rigs: usize, min_parts: usize) {
	let high_rigs = character.rig_nodes_for_level(LodSceneLevel::High);
	assert_eq!(high_rigs.len(), rigs);
	assert_eq!(high_rigs.len(), character.rig_nodes_for_level(LodSceneLevel::Low).len());
	let high_parts = character.part_nodes_for_level(LodSceneLevel::High);
	assert_eq!(high_parts.len(), character.part_nodes_for_level(LodSceneLevel::UltraLow).len());
	assert!(high_parts.len() >= min_parts);
}

fn assert_right_slot_reflected_without_scale_flip(parts: &[PartNode], slot: CharacterPartSlot) {
	let part = parts.iter().find(|p| p.slot == slot).expect("right feature slot");
	assert_eq!(part.scene.mirror, Some(MirrorAxis::X));
	assert!(part.scene.reflect_instance);
	let local = part.socket.expect("socket").local;
	assert!(local.scale.x > 0.0, "socket local must be placement-only");
}

fn assert_head_parent(character: &impl CharacterComponents, parent: RigId, bone: &str) {
	let head = character
		.rig_nodes_for_level(LodSceneLevel::High)
		.flatten()
		.into_iter()
		.find(|rig| rig.id == RigId::Head)
		.expect("head rig");
	let socket = head.socket.expect("head socket");
	assert_eq!(socket.rig, Some(parent.into()));
	assert_eq!(socket.bone, Some(bone));
}

fn assert_empty_clothing(inner: &impl CharacterComponents, clothed: &impl CharacterComponents) {
	assert_eq!(
		inner.part_nodes_for_level(LodSceneLevel::High).len(),
		clothed.part_nodes_for_level(LodSceneLevel::High).len()
	);
}

#[test]
fn hars_emits_rigs_and_parts_at_every_band() {
	let hars = Hars::from_config(&HarsConfig::default_preview());
	assert_band_equality(&hars, 3, 9);
	assert_head_parent(&hars, RigId::Neck, "head_socket");
	let parts = hars.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EarRight);
	assert_empty_clothing(&hars, &HarsConfig::default_preview().clothed());
}

#[test]
fn ylter_emits_rigs_and_parts_at_every_band() {
	let ylter = Yilter::from_config(&YilterConfig::default_preview());
	assert_band_equality(&ylter, 3, 7);
	assert_head_parent(&ylter, RigId::Neck, "head_socket");
	let parts = ylter.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_empty_clothing(&ylter, &YilterConfig::default_preview().clothed());
}

#[test]
fn caole_emits_rigs_and_parts_at_every_band() {
	let caole = Caole::from_config(&CaoleConfig::default_preview());
	assert_band_equality(&caole, 2, 8);
	assert_head_parent(&caole, RigId::Body, "head_socket");
	let parts = caole.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EarRight);
	assert_empty_clothing(&caole, &CaoleConfig::default_preview().clothed());
}

#[test]
fn brenal_emits_rigs_and_parts_at_every_band() {
	let brenal = Brenal::from_config(&BrenalConfig::default_preview());
	assert_band_equality(&brenal, 2, 8);
	assert_head_parent(&brenal, RigId::Body, "head_socket");
	let parts = brenal.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EarRight);
	assert_empty_clothing(&brenal, &BrenalConfig::default_preview().clothed());
}

#[test]
fn claber_emits_rigs_and_parts_at_every_band() {
	let claber = Claber::from_config(&ClaberConfig::default_preview());
	assert_band_equality(&claber, 2, 9);
	assert_head_parent(&claber, RigId::Body, "head_socket");
	let parts = claber.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EarRight);
	assert_empty_clothing(&claber, &ClaberConfig::default_preview().clothed());
}

#[test]
fn croconot_emits_rigs_and_parts_at_every_band() {
	let croconot = Croconot::from_config(&CroconotConfig::default_preview());
	assert_band_equality(&croconot, 2, 8);
	assert_head_parent(&croconot, RigId::Body, "head_socket");
	let parts = croconot.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EarRight);
	assert_empty_clothing(&croconot, &CroconotConfig::default_preview().clothed());
}

#[test]
fn epiphant_emits_rigs_and_parts_at_every_band() {
	let epiphant = Epiphant::from_config(&EpiphantConfig::default_preview());
	assert_band_equality(&epiphant, 2, 8);
	assert_head_parent(&epiphant, RigId::Body, "head_socket");
	let parts = epiphant.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EarRight);
	assert_empty_clothing(&epiphant, &EpiphantConfig::default_preview().clothed());
}

#[test]
fn sonyak_emits_rigs_and_parts_at_every_band() {
	let sonyak = Sonyak::from_config(&SonyakConfig::default_preview());
	assert_band_equality(&sonyak, 2, 7);
	assert_head_parent(&sonyak, RigId::Body, "head_socket");
	let parts = sonyak.part_nodes_for_level(LodSceneLevel::High).flatten();
	assert_right_slot_reflected_without_scale_flip(&parts, CharacterPartSlot::EyeRight);
	assert_empty_clothing(&sonyak, &SonyakConfig::default_preview().clothed());
}
