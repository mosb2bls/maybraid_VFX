use character_items::ClothingMesh;
use character_ui_menu::{MenuComponent, MenuNode, SelectGroup};
use characters::{
	presets::{BuildPreset, GenderPreset},
	species::{
		braidman::BraidmanConfig,
		brodler::{BrodlerConfig, BrodlerHeadMesh, HornMesh},
		common::{BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh},
		tuberwaber::TuberwaberConfig,
	},
};

use crate::{
	character::{CharacterMenu, ConceptSpecies},
	characters::{braidman::BraidmanMenu, brodler::BrodlerMenu, tuberwaber::TuberwaberMenu},
	MenuEvent,
};

/// In-range, off-default slider sample. Preview defaults are 1.0; menus clamp to 0.8..=1.2.
const NON_DEFAULT_SLIDER: f32 = 1.15;

/// Persistable appearance species. Independent of [`ConceptSpecies::HUMANOIDS`]: create
/// menus must not grow just because the preview catalog added a humanoid.
const CREATE_SPECIES: &[ConceptSpecies] = &[
	ConceptSpecies::Braidman,
	ConceptSpecies::Brodler,
	ConceptSpecies::Mygr,
	ConceptSpecies::Dui,
	ConceptSpecies::Wumbus,
	ConceptSpecies::Lero,
	ConceptSpecies::Spibmom,
	ConceptSpecies::Tuberwaber,
];

fn inventory_section<'a>(
	nodes: &'a [MenuNode<MenuEvent>],
) -> anyhow::Result<&'a MenuNode<MenuEvent>> {
	nodes.labeled("Inventory")
}

fn inventory_child<'a>(
	nodes: &'a [MenuNode<MenuEvent>],
	label: &str,
) -> anyhow::Result<&'a MenuNode<MenuEvent>> {
	let MenuNode::Section { children, .. } = inventory_section(nodes)? else {
		anyhow::bail!("Inventory must be a Section");
	};
	for child in children {
		if let MenuNode::Section { label: child_label, children } = child {
			if *child_label == label {
				return children.first().ok_or_else(|| anyhow::anyhow!("{label} section is empty"));
			}
		}
	}
	anyhow::bail!("expected Inventory child {label}")
}

trait MenuNodesExt {
	fn labeled(&self, label: &str) -> anyhow::Result<&MenuNode<MenuEvent>>;
	fn species_select(&self)
		-> anyhow::Result<(&[SelectGroup<MenuEvent>], &[MenuNode<MenuEvent>])>;
}

impl MenuNodesExt for [MenuNode<MenuEvent>] {
	fn labeled(&self, label: &str) -> anyhow::Result<&MenuNode<MenuEvent>> {
		self.iter()
			.find(|node| match node {
				MenuNode::ShortText { label: found, .. }
				| MenuNode::Section { label: found, .. }
				| MenuNode::SectionSelect { label: found, .. }
				| MenuNode::Action { label: found, .. }
				| MenuNode::ItemMultiSelect { label: found, .. }
				| MenuNode::GridCatalog { label: found, .. }
				| MenuNode::BlockAsset { label: found, .. }
				| MenuNode::LabeledCycle { label: found, .. }
				| MenuNode::LabeledSlider { label: found, .. }
				| MenuNode::LabeledSwatch { label: found, .. } => *found == label,
				_ => false,
			})
			.ok_or_else(|| anyhow::anyhow!("expected a node labeled {label}"))
	}

	fn species_select(
		&self,
	) -> anyhow::Result<(&[SelectGroup<MenuEvent>], &[MenuNode<MenuEvent>])> {
		match self.labeled("Species")? {
			MenuNode::SectionSelect { groups, children, .. } => Ok((groups, children)),
			_ => anyhow::bail!("Species must be a SectionSelect"),
		}
	}
}

trait SelectGroupExt {
	fn species_values(&self) -> Vec<ConceptSpecies>;
}

impl SelectGroupExt for SelectGroup<MenuEvent> {
	fn species_values(&self) -> Vec<ConceptSpecies> {
		self.choices
			.iter()
			.filter_map(|choice| match choice.event {
				MenuEvent::SetSpecies(species) => Some(species),
				_ => None,
			})
			.collect()
	}
}

trait SelectGroupsExt {
	fn labeled(&self, label: &str) -> anyhow::Result<&SelectGroup<MenuEvent>>;
	fn selected_species(&self) -> anyhow::Result<ConceptSpecies>;
}

impl SelectGroupsExt for [SelectGroup<MenuEvent>] {
	fn labeled(&self, label: &str) -> anyhow::Result<&SelectGroup<MenuEvent>> {
		self.iter()
			.find(|group| group.label == Some(label))
			.ok_or_else(|| anyhow::anyhow!("expected species group {label}"))
	}

	fn selected_species(&self) -> anyhow::Result<ConceptSpecies> {
		self.iter()
			.flat_map(|group| &group.choices)
			.find_map(|choice| match choice.event {
				MenuEvent::SetSpecies(species) if choice.selected => Some(species),
				_ => None,
			})
			.ok_or_else(|| anyhow::anyhow!("expected a selected species"))
	}
}

fn section_labels(nodes: &[MenuNode<MenuEvent>]) -> Vec<&'static str> {
	nodes
		.iter()
		.filter_map(|node| match node {
			MenuNode::Section { label, .. } => Some(*label),
			_ => None,
		})
		.collect()
}

fn catalog_labels(choices: &[character_ui_menu::GridCatalogChoice<MenuEvent>]) -> Vec<&str> {
	choices.iter().map(|choice| choice.label.as_str()).collect()
}

/// Config → species menu → config. Conversion is lossless for fields the menu owns;
/// callers must leave deliberately normalized fields (skin-synced head/ears) matching
/// the restore rule, or this equality fails.
fn assert_lossless_menu_round_trip<C, M>(config: C)
where
	M: for<'a> From<&'a C>,
	C: for<'a> From<&'a M> + PartialEq + std::fmt::Debug,
{
	let restored = C::from(&M::from(&config));
	assert_eq!(config, restored);
}

#[test]
fn braidman_config_round_trip() {
	use character_items::ItemColor;

	let mut config = BraidmanConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.build = BuildPreset::Athletic;
	config.body = BodyMesh::Full;
	config.head = HeadMesh::Gaunt;
	config.eye = EyeMesh::Falcon;
	config.nose = NoseMesh::Broad;
	config.mouth = MouthMesh::Standard;
	config.ear = EarMesh::Round;
	config.hair = HairMesh::ThickBraids;
	config.colors.body = ItemColor::Red;
	config.colors.sync_skin_from_body();
	config.colors.eyes = ItemColor::Gold;
	config.colors.mouth = ItemColor::Cool;
	config.colors.hair = ItemColor::Light;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, BraidmanMenu>(config);
}

#[test]
fn brenal_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::brenal::{BrenalConfig, BrenalHornMesh};

	use crate::characters::brenal::BrenalMenu;

	let mut config = BrenalConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.horns = BrenalHornMesh::HarrowedCrown;
	config.eye = EyeMesh::Falcon;
	config.colors.body = ItemColor::Red;
	config.colors.sync_skin_from_body();
	config.colors.tail = ItemColor::Cool;
	config.colors.eyes = ItemColor::Gold;
	config.colors.mouth = ItemColor::Warm;
	config.colors.horns = ItemColor::Dark;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, BrenalMenu>(config);
}

#[test]
fn caole_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::caole::{CaoleBodyMesh, CaoleConfig};

	use crate::characters::caole::CaoleMenu;

	let mut config = CaoleConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.body = CaoleBodyMesh::Rumbler;
	config.eye = EyeMesh::Falcon;
	config.colors.body = ItemColor::Red;
	config.colors.sync_skin_from_body();
	config.colors.tail = ItemColor::Cool;
	config.colors.eyes = ItemColor::Gold;
	config.colors.mouth = ItemColor::Warm;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, CaoleMenu>(config);
}

#[test]
fn epiphant_config_round_trip() {
	use characters::species::epiphant::{EpiphantColor, EpiphantConfig};

	use crate::characters::epiphant::EpiphantMenu;

	let mut config = EpiphantConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.eye = EyeMesh::Falcon;
	config.colors.body = EpiphantColor::Charcoal;
	config.colors.head = EpiphantColor::Charcoal;
	config.colors.ears = EpiphantColor::Steel;
	config.colors.tail = EpiphantColor::MistGray;
	config.colors.eyes = EpiphantColor::SoftLavender;
	config.colors.nose = EpiphantColor::Steel;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, EpiphantMenu>(config);
}

#[test]
fn hars_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::hars::HarsConfig;

	use crate::characters::hars::HarsMenu;

	let mut config = HarsConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.eye = EyeMesh::Falcon;
	config.colors.body = ItemColor::Red;
	config.colors.sync_skin_from_body();
	config.colors.tail = ItemColor::Cool;
	config.colors.eyes = ItemColor::Blue;
	config.colors.mouth = ItemColor::Gold;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, HarsMenu>(config);
}

#[test]
fn ylter_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::ylter::YilterConfig;

	use crate::characters::ylter::YilterMenu;

	let mut config = YilterConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.colors.body = ItemColor::Red;
	config.colors.head = ItemColor::Red;
	config.colors.neck = ItemColor::Red;
	config.colors.tail = ItemColor::Cool;
	config.colors.eyes = ItemColor::Blue;
	config.colors.mouth = ItemColor::Gold;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, YilterMenu>(config);
}

#[test]
fn sonyak_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::sonyak::SonyakConfig;

	use crate::characters::sonyak::SonyakMenu;

	let mut config = SonyakConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.colors.body = ItemColor::Red;
	config.colors.head = ItemColor::Red;
	config.colors.hair = ItemColor::Gold;
	config.colors.eyes = ItemColor::Blue;
	config.colors.mouth = ItemColor::Cool;
	config.colors.tail = ItemColor::Gold;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, SonyakMenu>(config);
}

#[test]
fn claber_config_round_trip() {
	use characters::species::claber::{ClaberColor, ClaberConfig, ClaberHornMesh};

	use crate::characters::claber::ClaberMenu;

	let mut config = ClaberConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.horns = ClaberHornMesh::None;
	config.eye = EyeMesh::Falcon;
	config.colors.body = ClaberColor::DesertBrown;
	config.colors.sync_skin_from_body();
	config.colors.tail = ClaberColor::SoftRed;
	config.colors.eyes = ClaberColor::SoftRed;
	config.colors.mouth = ClaberColor::SoftGold;
	config.colors.horns = ClaberColor::SoftRed;
	config.sliders.arm_length = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, ClaberMenu>(config);
}

#[test]
fn croconot_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::croconot::{CroconotConfig, CroconotHornMesh};

	use crate::characters::croconot::CroconotMenu;

	let mut config = CroconotConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.horns = CroconotHornMesh::HarrowedCrown;
	config.eye = EyeMesh::Falcon;
	config.colors.body = ItemColor::Red;
	config.colors.sync_skin_from_body();
	config.colors.tail = ItemColor::Cool;
	config.colors.eyes = ItemColor::Blue;
	config.colors.mouth = ItemColor::Warm;
	config.colors.horns = ItemColor::Dark;
	config.sliders.arm_length = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, CroconotMenu>(config);
}

#[test]
fn brodler_config_round_trip() {
	use characters::species::brodler::{BrodlerEyeColor, BrodlerSkinColor};

	let mut config = BrodlerConfig::default_preview();
	config.head = BrodlerHeadMesh::Full;
	config.horns = HornMesh::LorkenCrown;
	config.eye = EyeMesh::Falcon;
	config.nose = NoseMesh::Broad;
	config.ear = EarMesh::Round;
	config.hair = HairMesh::ThickBraids;
	config.colors.skin = BrodlerSkinColor::Umber;
	config.colors.eyes = BrodlerEyeColor::Yellow;
	config.colors.mouth = character_items::ItemColor::Cool;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, BrodlerMenu>(config);
}

#[test]
fn dui_config_round_trip() {
	use characters::species::dui::{DuiConfig, DuiMouthColor, DuiNoseMesh, DuiSkinColor};

	use crate::characters::dui::DuiMenu;

	let mut config = DuiConfig::default_preview();
	config.nose = DuiNoseMesh::Tbar;
	config.hair = HairMesh::ThickBraids;
	config.colors.skin = DuiSkinColor::Gold;
	config.colors.mouth = DuiMouthColor::Blue;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, DuiMenu>(config);
}

#[test]
fn lidder_config_round_trip() {
	use characters::species::lidder::{
		LidderBeakColor, LidderBeakMesh, LidderConfig, LidderEyeColor, LidderPlumageColor,
	};

	use crate::characters::lidder::LidderMenu;

	let mut config = LidderConfig::default_preview();
	config.beak = LidderBeakMesh::Hook;
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = LidderPlumageColor::Ink;
	config.colors.eyes = LidderEyeColor::Gold;
	config.colors.beak = LidderBeakColor::Coral;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, LidderMenu>(config);
}

#[test]
fn chupri_config_round_trip() {
	use characters::species::chupri::{
		ChupriBeakColor, ChupriBeakMesh, ChupriConfig, ChupriEyeColor, ChupriPlumageColor,
	};

	use crate::characters::chupri::ChupriMenu;

	let mut config = ChupriConfig::default_preview();
	config.beak = ChupriBeakMesh::Hook;
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = ChupriPlumageColor::Cyan;
	config.colors.eyes = ChupriEyeColor::Lemon;
	config.colors.beak = ChupriBeakColor::Coral;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, ChupriMenu>(config);
}

#[test]
fn brokker_config_round_trip() {
	use characters::species::brokker::{
		BrokkerConfig, BrokkerEyeColor, BrokkerPlumageColor, BrokkerSnoutColor,
	};

	use crate::characters::brokker::BrokkerMenu;

	let mut config = BrokkerConfig::default_preview();
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = BrokkerPlumageColor::Rust;
	config.colors.eyes = BrokkerEyeColor::Gold;
	config.colors.snout = BrokkerSnoutColor::Charcoal;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, BrokkerMenu>(config);
}

#[test]
fn tipple_config_round_trip() {
	use characters::species::tipple::{
		TippleBeakColor, TippleConfig, TippleEyeColor, TipplePlumageColor,
	};

	use crate::characters::tipple::TippleMenu;

	let mut config = TippleConfig::default_preview();
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = TipplePlumageColor::Cherry;
	config.colors.eyes = TippleEyeColor::Lemon;
	config.colors.beak = TippleBeakColor::Pink;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, TippleMenu>(config);
}

#[test]
fn topple_config_round_trip() {
	use characters::species::topple::{
		ToppleBeakColor, ToppleBeakMesh, ToppleConfig, ToppleEyeColor, TopplePlumageColor,
	};

	use crate::characters::topple::ToppleMenu;

	let mut config = ToppleConfig::default_preview();
	config.beak = ToppleBeakMesh::Hook;
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = TopplePlumageColor::Blush;
	config.colors.eyes = ToppleEyeColor::SoftBlue;
	config.colors.beak = ToppleBeakColor::Cream;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, ToppleMenu>(config);
}

#[test]
fn kispar_config_round_trip() {
	use characters::species::kispar::{
		KisparBeakColor, KisparBeakMesh, KisparConfig, KisparEyeColor, KisparPlumageColor,
	};

	use crate::characters::kispar::KisparMenu;

	let mut config = KisparConfig::default_preview();
	config.beak = KisparBeakMesh::Hook;
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = KisparPlumageColor::Rust;
	config.colors.eyes = KisparEyeColor::Gold;
	config.colors.beak = KisparBeakColor::Rust;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, KisparMenu>(config);
}

#[test]
fn tapp_config_round_trip() {
	use characters::species::tapp::{
		TappBeakColor, TappBeakMesh, TappConfig, TappEyeColor, TappPlumageColor,
	};

	use crate::characters::tapp::TappMenu;

	let mut config = TappConfig::default_preview();
	config.beak = TappBeakMesh::Hook;
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = TappPlumageColor::Pearl;
	config.colors.eyes = TappEyeColor::SoftAmber;
	config.colors.beak = TappBeakColor::Pearl;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, TappMenu>(config);
}

#[test]
fn kaller_config_round_trip() {
	use characters::species::kaller::{
		KallerConfig, KallerCrownColor, KallerEyeColor, KallerPlumageColor, KallerSnoutColor,
	};

	use crate::characters::kaller::KallerMenu;

	let mut config = KallerConfig::default_preview();
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = KallerPlumageColor::Moss;
	config.colors.eyes = KallerEyeColor::Gold;
	config.colors.snout = KallerSnoutColor::Olive;
	config.colors.crown = KallerCrownColor::Bone;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, KallerMenu>(config);
}

#[test]
fn kappler_config_round_trip() {
	use characters::species::kappler::{
		KapplerBeakColor, KapplerBeakMesh, KapplerConfig, KapplerEyeColor, KapplerPlumageColor,
	};

	use crate::characters::kappler::KapplerMenu;

	let mut config = KapplerConfig::default_preview();
	config.beak = KapplerBeakMesh::Hook;
	config.eye = EyeMesh::Standard;
	config.hair = HairMesh::ThickBraids;
	config.colors.plumage = KapplerPlumageColor::Blush;
	config.colors.eyes = KapplerEyeColor::SoftBlue;
	config.colors.beak = KapplerBeakColor::Cream;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, KapplerMenu>(config);
}

#[test]
fn mygr_config_round_trip() {
	use characters::species::mygr::{MygrConfig, MygrEyeColor, MygrSkinColor};

	use crate::characters::mygr::MygrMenu;

	let mut config = MygrConfig::default_preview();
	config.eye = EyeMesh::Falcon;
	config.hair = HairMesh::ThickBraids;
	config.colors.skin = MygrSkinColor::Charcoal;
	config.colors.eyes = MygrEyeColor::Amber;
	config.colors.mouth = character_items::ItemColor::Cool;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, MygrMenu>(config);
}

#[test]
fn wumbus_config_round_trip() {
	use characters::species::wumbus::{
		WumbusConfig, WumbusEarColor, WumbusEyeColor, WumbusHornColor, WumbusHornMesh,
		WumbusMouthColor, WumbusSkinColor, WumbusSpineColor,
	};

	use crate::characters::wumbus::WumbusMenu;

	let mut config = WumbusConfig::default_preview();
	config.horns = WumbusHornMesh::HarrowedCrown;
	config.eye = EyeMesh::Falcon;
	config.hair = HairMesh::ThickBraids;
	config.colors.skin = WumbusSkinColor::Espresso;
	config.colors.eyes = WumbusEyeColor::Honey;
	config.colors.ears = WumbusEarColor::Sandy;
	config.colors.mouth = WumbusMouthColor::DustyRose;
	config.colors.horns = WumbusHornColor::Wheat;
	config.colors.spine = WumbusSpineColor::PaleGold;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, WumbusMenu>(config);
}

#[test]
fn lero_config_round_trip() {
	use characters::species::lero::{
		LeroConfig, LeroEyeColor, LeroMouthColor, LeroMouthMesh, LeroSkinColor, LeroSpineColor,
		LeroTailColor,
	};

	use crate::characters::lero::LeroMenu;

	let mut config = LeroConfig::default_preview();
	config.mouth = LeroMouthMesh::Robrek;
	config.hair = HairMesh::ThickBraids;
	config.colors.skin = LeroSkinColor::FadedRed;
	config.colors.eyes = LeroEyeColor::Amber;
	config.colors.mouth = LeroMouthColor::PaleRose;
	config.colors.tail = LeroTailColor::Sand;
	config.colors.spine = LeroSpineColor::PaleMint;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, LeroMenu>(config);
}

#[test]
fn spibmom_config_round_trip() {
	use characters::species::spibmom::{
		SpibmomConfig, SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomMouthColor,
		SpibmomSkinColor, SpibmomSpineColor,
	};

	use crate::characters::spibmom::SpibmomMenu;

	let mut config = SpibmomConfig::default_preview();
	config.eye = EyeMesh::Falcon;
	config.hair = HairMesh::ThickBraids;
	config.colors.skin = SpibmomSkinColor::MistBlue;
	config.colors.eyes = SpibmomEyeColor::Frost;
	config.colors.ears = SpibmomEarColor::Charcoal;
	config.colors.mouth = SpibmomMouthColor::DeepSlate;
	config.colors.crown = SpibmomCrownColor::DeepBronze;
	config.colors.spine = SpibmomSpineColor::DarkUmber;
	config.colors.hair = character_items::ItemColor::Gold;
	assert_lossless_menu_round_trip::<_, SpibmomMenu>(config);
}

#[test]
fn tuberwaber_config_round_trip() {
	use character_items::ItemColor;
	use characters::species::tuberwaber::TuberwaberColor;

	let mut config = TuberwaberConfig::default_preview();
	config.gender = GenderPreset::Female;
	config.build = BuildPreset::Athletic;
	config.eye = EyeMesh::Falcon;
	config.nose = NoseMesh::Balloon;
	config.hair = HairMesh::ThickBraids;
	config.colors.body = TuberwaberColor::Ice;
	config.colors.sync_skin_from_body();
	config.colors.eyes = TuberwaberColor::Jade;
	config.colors.mouth = TuberwaberColor::Saffron;
	config.colors.horns = TuberwaberColor::Amber;
	config.colors.hair = ItemColor::Gold;
	config.sliders.shoulder_width = NON_DEFAULT_SLIDER;
	assert_lossless_menu_round_trip::<_, TuberwaberMenu>(config);
}

#[test]
fn grener_config_round_trip() {
	use characters::species::grener::{GrenerBodyColor, GrenerConfig};

	use crate::characters::grener::GrenerMenu;

	let mut config = GrenerConfig::default_preview();
	config.colors.body = GrenerBodyColor::Steel;
	assert_lossless_menu_round_trip::<_, GrenerMenu>(config);
}

#[test]
fn thumplus_config_round_trip() {
	use characters::species::thumplus::{ThumplusBodyColor, ThumplusConfig};

	use crate::characters::thumplus::ThumplusMenu;

	let mut config = ThumplusConfig::default_preview();
	config.colors.body = ThumplusBodyColor::Midnight;
	assert_lossless_menu_round_trip::<_, ThumplusMenu>(config);
}

#[test]
fn mistler_config_round_trip() {
	use characters::species::mistler::{MistlerBodyColor, MistlerConfig};

	use crate::characters::mistler::MistlerMenu;

	let mut config = MistlerConfig::default_preview();
	config.colors.body = MistlerBodyColor::Aqua;
	assert_lossless_menu_round_trip::<_, MistlerMenu>(config);
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
	let MenuNode::ShortText { value, max_len, .. } = nodes.labeled("Name")? else {
		anyhow::bail!("Name must be ShortText");
	};
	assert!(value.is_empty());
	assert_eq!(*max_len, crate::CHARACTER_NAME_MAX_LEN);

	let (groups, children) = nodes.species_select()?;
	// Default appearance picker hardcodes family lists; catalog slices are the
	// independent membership source (not a total-count snapshot).
	assert_eq!(groups.labeled("Humanoids")?.species_values(), ConceptSpecies::HUMANOIDS);
	assert_eq!(groups.labeled("Quadrupeds")?.species_values(), ConceptSpecies::QUADRUPEDS);
	assert_eq!(groups.labeled("Birds")?.species_values(), ConceptSpecies::BIRDS);
	assert_eq!(groups.labeled("Aquatic")?.species_values(), ConceptSpecies::AQUATIC);
	assert_eq!(groups.selected_species()?, ConceptSpecies::Braidman);
	for label in ["Presets", "Body", "Head & Features", "Hair", "Clothing", "Animation"] {
		assert!(section_labels(children).contains(&label), "braidman tree should include {label}");
	}
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
	let (groups, _) = nodes.species_select()?;
	assert!(
		groups.iter().all(|group| group.label == Some("Humanoids")),
		"create picker is humanoid-only"
	);
	assert_eq!(groups.labeled("Humanoids")?.species_values(), CREATE_SPECIES);
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
	let tools = inventory_child(&nodes, "Tools")?;
	let MenuNode::GridCatalog { max_selected, choices, .. } = tools else {
		anyhow::bail!("expected Inventory Tools to lower to a GridCatalog");
	};
	assert_eq!(*max_selected, WEAPON_QUEUE_LIMIT);
	assert_eq!(choices.len(), 2);
	assert_eq!(choices[0].rank, Some(1));
	assert_eq!(choices[1].rank, Some(2));
	assert!(choices[0].selected);
	assert_eq!(choices[0].label, expected_name);
	assert_eq!(choices[0].image_key, expected_key);
	assert!(!choices[0].detail.is_empty());
	assert!(menu.overlay_editable("Inventory"));
	assert!(menu.overlay_editable("Tools"));
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
	let skills = inventory_child(&nodes, "Skill Maps")?;
	let MenuNode::GridCatalog { max_selected, choices, .. } = skills else {
		anyhow::bail!("expected Inventory Skill Maps to lower to a GridCatalog");
	};
	assert_eq!(*max_selected, SKILL_MAP_QUEUE_LIMIT);
	assert_eq!(choices.len(), 2);
	assert_eq!(choices[0].rank, Some(1));
	assert_eq!(choices[1].rank, Some(2));
	assert_eq!(choices[0].label, expected_name);
	assert_ne!(choices[0].preview, character_ui_menu::PreviewColor::WHITE);
	assert_ne!(choices[0].preview, choices[1].preview);
	assert_eq!(choices[0].image_key, expected_key);
	assert!(menu.overlay_editable("Inventory"));
	assert!(menu.overlay_editable("Skill Maps"));
	assert!(menu.overlay_editable("Tanzes"));
	assert!(menu.apply(MenuEvent::ToggleInventory(0)));
	let inventory = menu.inventory.as_ref().expect("create inventory");
	assert_eq!(inventory.skills, vec![1]);
	Ok(())
}

#[test]
fn create_menu_inventory_is_one_section_with_four_headers() -> anyhow::Result<()> {
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor, SkillMapKind,
		SkillMapSpec,
	};

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red),
		InventoryItem::firearm(FirearmMesh::Bullpup),
		InventoryItem::standard_grenade(),
		InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
	];
	let firearm_name = items[1].name();
	let grenade_name = items[2].name();
	let menu = CharacterMenu::for_create(items);
	let nodes = menu.menu_nodes();
	let MenuNode::Section { children, .. } = inventory_section(&nodes)? else {
		anyhow::bail!("Inventory must be a Section");
	};
	assert_eq!(section_labels(children), ["Clothing", "Tools", "Skill Maps", "Tanzes", "Lubikan"]);
	assert!(nodes
		.iter()
		.all(|node| !matches!(node, MenuNode::Section { label: "Weapons" | "Skill Maps", .. })));
	let (_, species) = nodes.species_select()?;
	assert!(species
		.iter()
		.all(|node| !matches!(node, MenuNode::Section { label: "Clothing", .. })));
	let tools = inventory_child(&nodes, "Tools")?;
	let MenuNode::GridCatalog { choices, .. } = tools else {
		anyhow::bail!("Tools must be a GridCatalog");
	};
	assert_eq!(catalog_labels(choices), [firearm_name.as_str(), grenade_name.as_str()]);
	let lubikan = inventory_child(&nodes, "Lubikan")?;
	let MenuNode::GridCatalog { choices, .. } = lubikan else {
		anyhow::bail!("Lubikan must be a catalog tile");
	};
	let self_tile = choices
		.iter()
		.find(|choice| choice.label == "Self")
		.ok_or_else(|| anyhow::anyhow!("expected Lubikan Self tile"))?;
	assert_eq!(self_tile.detail, "Always equipped");
	assert_eq!(self_tile.image_key, Some(character_items::self_lubikan_catalog_key()));
	assert!(self_tile.selected);
	assert!(choices.iter().all(|choice| choice.label == "Self"));
	Ok(())
}

#[test]
fn appearance_only_menu_keeps_clothing_under_species() -> anyhow::Result<()> {
	let menu = CharacterMenu::default();
	assert!(menu.inventory.is_none());
	let nodes = menu.menu_nodes();
	assert!(nodes.labeled("Inventory").is_err());
	let (_, children) = nodes.species_select()?;
	assert!(section_labels(children).contains(&"Clothing"));
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
	assert!(
		cards.iter().any(|card| card.title == weapon_name),
		"loadout should include a card for {weapon_name}"
	);
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
	let (_, children) = nodes.species_select()?;
	assert!(
		section_labels(children).contains(&"Body"),
		"body attributes stay visible after appearance lock"
	);
	assert!(!section_labels(children).contains(&"Clothing"));
	assert!(nodes.labeled("Inventory").is_ok());
	assert!(nodes.iter().all(|node| !matches!(node, MenuNode::Action { .. })));
	assert!(!menu.overlay_editable("Species"));
	assert!(!menu.overlay_editable("Body"));
	assert!(menu.overlay_editable("Inventory"));
	assert!(menu.overlay_editable("Clothing"));
	assert!(menu.overlay_editable("Tools"));
	assert!(menu.overlay_editable("Skill Maps"));
	assert!(menu.overlay_editable("Tanzes"));
	assert!(menu.overlay_editable("Lubikan"));
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

#[test]
fn body_preview_focus_contains_every_species_bounds() {
	use character_ui_menu::{CameraFocus, ListValues};

	for &species in ConceptSpecies::values() {
		let mut menu = CharacterMenu::default();
		assert!(
			menu.apply(MenuEvent::SetSpecies(species)) || species == ConceptSpecies::Braidman,
			"{species:?} should be selectable from the default menu"
		);
		let bounds = menu.preview_aabb();
		let focus = menu.body_preview_focus();
		assert!(
			focus.contains_preview_bounds(bounds, CameraFocus::BODY_PREVIEW_FOV, 1.0),
			"{species:?} framing must contain its rest-pose hull {bounds:?} from {focus:?}"
		);
	}
}

#[test]
fn body_preview_focus_scales_with_species_size() {
	let braidman = CharacterMenu::default();
	let mut spibmom = CharacterMenu::default();
	assert!(spibmom.apply(MenuEvent::SetSpecies(ConceptSpecies::Spibmom)));
	let mut chupri = CharacterMenu::default();
	assert!(chupri.apply(MenuEvent::SetSpecies(ConceptSpecies::Chupri)));
	let mut thumplus = CharacterMenu::default();
	assert!(thumplus.apply(MenuEvent::SetSpecies(ConceptSpecies::Thumplus)));

	let braidman_dist = braidman
		.body_preview_focus()
		.camera_offset
		.distance(braidman.body_preview_focus().look_at_offset);
	let spibmom_dist = spibmom
		.body_preview_focus()
		.camera_offset
		.distance(spibmom.body_preview_focus().look_at_offset);
	let chupri_dist = chupri
		.body_preview_focus()
		.camera_offset
		.distance(chupri.body_preview_focus().look_at_offset);
	let thumplus_dist = thumplus
		.body_preview_focus()
		.camera_offset
		.distance(thumplus.body_preview_focus().look_at_offset);

	assert!(
		spibmom_dist > braidman_dist,
		"Spibmom's 2× head + stature should sit farther than Braidman ({spibmom_dist} vs {braidman_dist})"
	);
	assert!(
		chupri_dist < braidman_dist,
		"Chupri should sit closer than Braidman ({chupri_dist} vs {braidman_dist})"
	);
	assert!(
		thumplus_dist > spibmom_dist,
		"Thumplus should sit farther than Spibmom ({thumplus_dist} vs {spibmom_dist})"
	);
}

#[test]
fn set_species_requests_body_preview_focus() {
	let mut menu = CharacterMenu::default();
	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Spibmom)));
	let focus = menu
		.camera_focus_for_event(MenuEvent::SetSpecies(ConceptSpecies::Spibmom))
		.expect("species change should reframe");
	assert_eq!(focus, menu.body_preview_focus());
}

#[test]
fn human_scale_body_focus_misses_spibmom() {
	use character_ui_menu::CameraFocus;

	let mut menu = CharacterMenu::default();
	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Spibmom)));
	let bounds = menu.preview_aabb();
	assert!(
		!crate::BODY_FOCUS.contains_preview_bounds(bounds, CameraFocus::BODY_PREVIEW_FOV, 1.0),
		"legacy human-scale BODY_FOCUS should crop a tall Spibmom"
	);
}

#[test]
fn drop_inventory_removes_an_item_and_unequips_it() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor};

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
		InventoryItem::firearm(FirearmMesh::Bullpup),
		InventoryItem::firearm(FirearmMesh::Reltor),
	];
	let mut menu = CharacterMenu::for_create(items);
	assert!(menu.apply(MenuEvent::DropInventory(2)));
	let inventory = menu.inventory.as_ref().ok_or_else(|| anyhow::anyhow!("create inventory"))?;
	assert_eq!(inventory.items.len(), 3);
	assert_eq!(inventory.weapons, vec![2]);
	assert_eq!(
		inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
		Some(FirearmMesh::Reltor)
	);
	assert_eq!(inventory.clothing, vec![0, 1]);
	assert!(menu.apply(MenuEvent::DropInventory(0)));
	let inventory = menu.inventory.as_ref().ok_or_else(|| anyhow::anyhow!("create inventory"))?;
	assert_eq!(inventory.items.len(), 2);
	assert_eq!(inventory.clothing, vec![0]);
	assert_eq!(inventory.items[0].mesh(), Some(ClothingMesh::TankTop));
	assert!(!menu.apply(MenuEvent::DropInventory(9)));
	let no_equipped = character_items::InventoryDropPolicy {
		allow_equipped: false,
		..character_items::InventoryDropPolicy::PLACEHOLDER
	};
	assert!(!menu.apply_drop(0, no_equipped));
	assert_eq!(menu.inventory.as_ref().map(|bag| bag.items.len()), Some(2));
	Ok(())
}

#[test]
fn saved_menu_can_drop_after_appearance_lock() -> anyhow::Result<()> {
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};

	let items = vec![
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Natural),
		InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
	];
	let mut created = CharacterMenu::for_create(items);
	created.name = String::from("Misty");
	let appearance = created.appearance();
	let inventory = created.inventory.clone().ok_or_else(|| anyhow::anyhow!("inventory"))?;
	let mut menu = CharacterMenu::for_saved(created.saved_name(), &appearance, inventory);
	assert!(menu.appearance_locked());
	assert!(MenuEvent::DropInventory(0).edits_inventory());
	assert!(menu.apply(MenuEvent::DropInventory(0)));
	let inventory = menu.inventory.as_ref().ok_or_else(|| anyhow::anyhow!("inventory"))?;
	assert_eq!(inventory.items.len(), 1);
	assert_eq!(inventory.clothing, vec![0]);
	Ok(())
}

#[test]
fn default_character_menu_uses_braidman_preview_without_recursing() {
	use characters::ConceptAnimation;

	let from_default = CharacterMenu::default();
	let explicit = CharacterMenu::from_braidman(
		&BraidmanConfig::default_preview(),
		ConceptAnimation::default(),
	);
	assert_eq!(from_default, explicit);
	assert_eq!(from_default.species.value, ConceptSpecies::Braidman);
	assert!(from_default.name.is_empty());
	assert!(from_default.inventory.is_none());
	assert!(!from_default.appearance_locked);
}

#[test]
fn from_species_constructors_set_selected_and_keep_inactive_defaults() {
	use characters::{
		presets::GenderPreset,
		species::{
			brenal::{BrenalConfig, BrenalHornMesh},
			grener::{GrenerBodyColor, GrenerConfig},
			tuberwaber::TuberwaberConfig,
		},
		ConceptAnimation,
	};

	use crate::characters::brenal::BrenalMenu;

	let mut brenal_config = BrenalConfig::default_preview();
	brenal_config.horns = BrenalHornMesh::HarrowedCrown;
	let menu = CharacterMenu::from_brenal(&brenal_config, ConceptAnimation::Run);
	assert_eq!(menu.species.value, ConceptSpecies::Brenal);
	assert_eq!(menu.brenal_config().horns, BrenalHornMesh::HarrowedCrown);
	assert_eq!(menu.animation(), ConceptAnimation::Run);
	assert_eq!(menu.braidman, BraidmanMenu::default());
	assert_eq!(menu.tuberwaber, TuberwaberMenu::default());
	assert!(menu.name.is_empty());
	assert!(menu.inventory.is_none());
	assert!(!menu.appearance_locked);

	let mut tuber = TuberwaberConfig::default_preview();
	tuber.gender = GenderPreset::Male;
	let menu = CharacterMenu::from_tuberwaber(&tuber, ConceptAnimation::Walk);
	assert_eq!(menu.species.value, ConceptSpecies::Tuberwaber);
	assert_eq!(menu.tuberwaber_config().gender, GenderPreset::Male);
	assert_eq!(menu.animation(), ConceptAnimation::Walk);
	assert_eq!(menu.braidman, BraidmanMenu::default());
	assert_eq!(menu.brenal, BrenalMenu::default());

	let mut grener = GrenerConfig::default_preview();
	grener.colors.body = GrenerBodyColor::Steel;
	let menu = CharacterMenu::from_grener(&grener, ConceptAnimation::Still);
	assert_eq!(menu.species.value, ConceptSpecies::Grener);
	assert_eq!(menu.grener_config().colors.body, GrenerBodyColor::Steel);
	assert_eq!(menu.braidman, BraidmanMenu::default());
	assert_eq!(menu.brenal, BrenalMenu::default());
}

#[test]
fn switching_species_keeps_per_species_edits() {
	use crate::CharacterField;

	let mut menu = CharacterMenu::default();
	let braidman_before = menu.braidman.body.value.sliders.shoulder_width.value;
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::ShoulderWidth, 0.15)));
	let braidman_after = menu.braidman.body.value.sliders.shoulder_width.value;
	assert_ne!(braidman_before, braidman_after);

	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Brenal)));
	assert_eq!(menu.braidman.body.value.sliders.shoulder_width.value, braidman_after);
	let brenal_before = menu.brenal.body.value.sliders.shoulder_width.value;
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::ShoulderWidth, 0.1)));
	let brenal_after = menu.brenal.body.value.sliders.shoulder_width.value;
	assert_ne!(brenal_before, brenal_after);

	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Braidman)));
	assert_eq!(menu.braidman.body.value.sliders.shoulder_width.value, braidman_after);
	assert_eq!(menu.brenal.body.value.sliders.shoulder_width.value, brenal_after);
}

#[test]
fn slider_delta_clamps_and_rejects_unsupported_fields() {
	use crate::CharacterField;

	let mut menu = CharacterMenu::default();
	let shoulder = menu.braidman.body.value.sliders.shoulder_width;
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::ShoulderWidth, 10.0)));
	assert_eq!(menu.braidman.body.value.sliders.shoulder_width.value, shoulder.max);
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::ShoulderWidth, -10.0)));
	assert_eq!(menu.braidman.body.value.sliders.shoulder_width.value, shoulder.min);
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::NoseWidth, 0.05)));
	assert!(!menu.apply(MenuEvent::SliderDelta(CharacterField::SnoutLength, 0.1)));

	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Brenal)));
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::EarWidth, 0.05)));
	assert!(!menu.apply(MenuEvent::SliderDelta(CharacterField::NoseWidth, 0.1)));
	assert!(!menu.apply(MenuEvent::SliderDelta(CharacterField::SnoutLength, 0.1)));

	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Croconot)));
	let snout_before = menu.croconot.head_features.value.feature_sliders.snout_length.value;
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::SnoutLength, 0.1)));
	assert_ne!(menu.croconot.head_features.value.feature_sliders.snout_length.value, snout_before);

	assert!(menu.apply(MenuEvent::SetSpecies(ConceptSpecies::Tuberwaber)));
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::NoseWidth, 0.05)));
	assert!(menu.apply(MenuEvent::SliderDelta(CharacterField::MouthHeight, 0.05)));
	assert!(!menu.apply(MenuEvent::SliderDelta(CharacterField::EarWidth, 0.1)));
}
