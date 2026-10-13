use character_items::{Inventory, InventoryItem};
use character_ui_menu::{
	CameraFocus, LabelOption, ListValues, MenuComponent, MenuNode, SingleSelect,
};
use characters::{
	species::{
		braidman::BraidmanConfig, brenal::BrenalConfig, brodler::BrodlerConfig,
		brokker::BrokkerConfig, caole::CaoleConfig, chupri::ChupriConfig, claber::ClaberConfig,
		croconot::CroconotConfig, dui::DuiConfig, epiphant::EpiphantConfig, grener::GrenerConfig,
		hars::HarsConfig, kaller::KallerConfig, kappler::KapplerConfig, kispar::KisparConfig,
		lero::LeroConfig, lidder::LidderConfig, mistler::MistlerConfig, mygr::MygrConfig,
		sonyak::SonyakConfig, spibmom::SpibmomConfig, tapp::TappConfig, thumplus::ThumplusConfig,
		tipple::TippleConfig, topple::ToppleConfig, tuberwaber::TuberwaberConfig,
		wumbus::WumbusConfig, ylter::YilterConfig,
	},
	CharacterAppearance, ConceptAnimation,
};

use crate::{
	characters::{
		braidman::BraidmanMenu,
		brenal::{BrenalAnimationClip, BrenalMenu},
		brodler::BrodlerMenu,
		brokker::BrokkerMenu,
		caole::{CaoleAnimationClip, CaoleMenu},
		chupri::ChupriMenu,
		claber::{ClaberAnimationClip, ClaberMenu},
		croconot::{CroconotAnimationClip, CroconotMenu},
		dui::DuiMenu,
		epiphant::{EpiphantAnimationClip, EpiphantMenu},
		grener::GrenerMenu,
		hars::{HarsAnimationClip, HarsMenu},
		kaller::KallerMenu,
		kappler::KapplerMenu,
		kispar::KisparMenu,
		lero::LeroMenu,
		lidder::LidderMenu,
		mistler::MistlerMenu,
		mygr::MygrMenu,
		sonyak::{SonyakAnimationClip, SonyakMenu},
		spibmom::SpibmomMenu,
		tapp::TappMenu,
		thumplus::ThumplusMenu,
		tipple::TippleMenu,
		topple::ToppleMenu,
		tuberwaber::TuberwaberMenu,
		wumbus::WumbusMenu,
		ylter::{YilterAnimationClip, YilterMenu},
	},
	cycle_value,
	event::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue},
	shared::{clothing_menu_from_inventory, loadout_section, skills_catalog, weapons_catalog},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConceptSpecies {
	Braidman,
	Brenal,
	Caole,
	Epiphant,
	Hars,
	Yilter,
	Sonyak,
	Claber,
	Croconot,
	Brodler,
	Mygr,
	Dui,
	Lidder,
	Chupri,
	Brokker,
	Tipple,
	Topple,
	Kispar,
	Tapp,
	Kaller,
	Kappler,
	Wumbus,
	Lero,
	Spibmom,
	Grener,
	Thumplus,
	Mistler,
	Tuberwaber,
}

impl ConceptSpecies {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Braidman => "braidman",
			Self::Brenal => "brenal",
			Self::Caole => "caole",
			Self::Epiphant => "epiphant",
			Self::Hars => "hars",
			Self::Yilter => "ylter",
			Self::Sonyak => "sonyak",
			Self::Claber => "claber",
			Self::Croconot => "croconot",
			Self::Brodler => "brodler",
			Self::Mygr => "mygr",
			Self::Dui => "dui",
			Self::Lidder => "lidder",
			Self::Chupri => "chupri",
			Self::Brokker => "brokker",
			Self::Tipple => "tipple",
			Self::Topple => "topple",
			Self::Kispar => "kispar",
			Self::Tapp => "tapp",
			Self::Kaller => "kaller",
			Self::Kappler => "kappler",
			Self::Wumbus => "wumbus",
			Self::Lero => "lero",
			Self::Spibmom => "spibmom",
			Self::Grener => "grener",
			Self::Thumplus => "thumplus",
			Self::Mistler => "mistler",
			Self::Tuberwaber => "tuberwaber",
		}
	}
}

impl ListValues for ConceptSpecies {
	fn values() -> &'static [Self] {
		&[
			// Humanoids
			Self::Braidman,
			Self::Brodler,
			Self::Mygr,
			Self::Dui,
			Self::Wumbus,
			Self::Lero,
			Self::Spibmom,
			Self::Tuberwaber,
			// Quadrupeds
			Self::Brenal,
			Self::Caole,
			Self::Epiphant,
			Self::Hars,
			Self::Yilter,
			Self::Sonyak,
			Self::Claber,
			Self::Croconot,
			// Birds
			Self::Lidder,
			Self::Chupri,
			Self::Brokker,
			Self::Tipple,
			Self::Topple,
			Self::Kispar,
			Self::Tapp,
			Self::Kaller,
			Self::Kappler,
			// Aquatic
			Self::Grener,
			Self::Thumplus,
			Self::Mistler,
		]
	}
}

impl ConceptSpecies {
	pub const HUMANOIDS: &'static [Self] = &[
		Self::Braidman,
		Self::Brodler,
		Self::Mygr,
		Self::Dui,
		Self::Wumbus,
		Self::Lero,
		Self::Spibmom,
		Self::Tuberwaber,
	];

	pub const fn is_humanoid(self) -> bool {
		matches!(
			self,
			Self::Braidman
				| Self::Brodler
				| Self::Mygr | Self::Dui
				| Self::Wumbus
				| Self::Lero | Self::Spibmom
				| Self::Tuberwaber
		)
	}
}

impl LabelOption for ConceptSpecies {
	fn label(&self) -> &'static str {
		match *self {
			Self::Braidman => "braidman",
			Self::Brenal => "brenal",
			Self::Caole => "caole",
			Self::Epiphant => "epiphant",
			Self::Hars => "hars",
			Self::Yilter => "ylter",
			Self::Sonyak => "sonyak",
			Self::Claber => "claber",
			Self::Croconot => "croconot",
			Self::Brodler => "brodler",
			Self::Mygr => "mygr",
			Self::Dui => "dui",
			Self::Lidder => "lidder",
			Self::Chupri => "chupri",
			Self::Brokker => "brokker",
			Self::Tipple => "tipple",
			Self::Topple => "topple",
			Self::Kispar => "kispar",
			Self::Tapp => "tapp",
			Self::Kaller => "kaller",
			Self::Kappler => "kappler",
			Self::Wumbus => "wumbus",
			Self::Lero => "lero",
			Self::Spibmom => "spibmom",
			Self::Grener => "grener",
			Self::Thumplus => "thumplus",
			Self::Mistler => "mistler",
			Self::Tuberwaber => "tuberwaber",
		}
	}
}

/// Hard cap for the character display name.
pub const CHARACTER_NAME_MAX_LEN: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterMenu {
	pub name: String,
	pub inventory: Option<character_items::Inventory>,
	/// Saved characters keep appearance; only inventory (and name) stay editable.
	pub appearance_locked: bool,
	pub species: SingleSelect<ConceptSpecies>,
	pub braidman: BraidmanMenu,
	pub brenal: BrenalMenu,
	pub caole: CaoleMenu,
	pub epiphant: EpiphantMenu,
	pub hars: HarsMenu,
	pub ylter: YilterMenu,
	pub sonyak: SonyakMenu,
	pub claber: ClaberMenu,
	pub croconot: CroconotMenu,
	pub brodler: BrodlerMenu,
	pub mygr: MygrMenu,
	pub dui: DuiMenu,
	pub lidder: LidderMenu,
	pub chupri: ChupriMenu,
	pub brokker: BrokkerMenu,
	pub tipple: TippleMenu,
	pub topple: ToppleMenu,
	pub kispar: KisparMenu,
	pub tapp: TappMenu,
	pub kaller: KallerMenu,
	pub kappler: KapplerMenu,
	pub wumbus: WumbusMenu,
	pub lero: LeroMenu,
	pub spibmom: SpibmomMenu,
	pub grener: GrenerMenu,
	pub thumplus: ThumplusMenu,
	pub mistler: MistlerMenu,
	pub tuberwaber: TuberwaberMenu,
}

impl CharacterMenu {
	pub fn for_create(items: Vec<InventoryItem>) -> Self {
		let mut menu = Self::default();
		menu.inventory = Some(Inventory::with_starter_outfit(items));
		menu.appearance_locked = false;
		menu.sync_inventory_clothing();
		menu
	}

	pub fn for_saved(name: String, appearance: &CharacterAppearance, inventory: Inventory) -> Self {
		let mut menu = match appearance {
			CharacterAppearance::Braidman(config) => {
				Self::from_braidman(config, ConceptAnimation::default())
			}
			CharacterAppearance::Brodler(config) => {
				Self::from_brodler(config, ConceptAnimation::default())
			}
			CharacterAppearance::Mygr(config) => {
				Self::from_mygr(config, ConceptAnimation::default())
			}
			CharacterAppearance::Dui(config) => Self::from_dui(config, ConceptAnimation::default()),
			CharacterAppearance::Wumbus(config) => {
				Self::from_wumbus(config, ConceptAnimation::default())
			}
			CharacterAppearance::Lero(config) => {
				Self::from_lero(config, ConceptAnimation::default())
			}
			CharacterAppearance::Spibmom(config) => {
				Self::from_spibmom(config, ConceptAnimation::default())
			}
			CharacterAppearance::Tuberwaber(config) => {
				Self::from_tuberwaber(config, ConceptAnimation::default())
			}
		};
		menu.name = name;
		menu.inventory = Some(inventory);
		menu.appearance_locked = true;
		menu.sync_inventory_clothing();
		menu
	}

	pub fn appearance(&self) -> CharacterAppearance {
		let mut appearance = match self.species.value {
			ConceptSpecies::Braidman => CharacterAppearance::Braidman(self.braidman_config()),
			ConceptSpecies::Brodler => CharacterAppearance::Brodler(self.brodler_config()),
			ConceptSpecies::Mygr => CharacterAppearance::Mygr(self.mygr_config()),
			ConceptSpecies::Dui => CharacterAppearance::Dui(self.dui_config()),
			ConceptSpecies::Wumbus => CharacterAppearance::Wumbus(self.wumbus_config()),
			ConceptSpecies::Lero => CharacterAppearance::Lero(self.lero_config()),
			ConceptSpecies::Spibmom => CharacterAppearance::Spibmom(self.spibmom_config()),
			ConceptSpecies::Tuberwaber => CharacterAppearance::Tuberwaber(self.tuberwaber_config()),
			_ => CharacterAppearance::Braidman(self.braidman_config()),
		};
		appearance.strip_clothing();
		appearance
	}

	pub fn saved_name(&self) -> String {
		let name = self.name.trim();
		if name.is_empty() {
			String::from("Unnamed")
		} else {
			name.to_string()
		}
	}

	pub fn is_create(&self) -> bool {
		self.inventory.is_some() && !self.appearance_locked
	}

	pub fn appearance_locked(&self) -> bool {
		self.appearance_locked
	}

	/// Overlay interiors for body catalogs stay visible when locked; clothing
	/// and weapons stay editable.
	pub fn overlay_editable(&self, key: &str) -> bool {
		!self.appearance_locked()
			|| key == "Clothing"
			|| key == "Weapons"
			|| key == "Skill Maps"
			|| key == "Loadout"
	}

	fn sync_inventory_clothing(&mut self) {
		let Some(inventory) = self.inventory.clone() else {
			return;
		};
		let clothing = clothing_menu_from_inventory(&inventory);
		self.braidman.clothing.value = clothing.clone();
		self.brodler.clothing.value = clothing.clone();
		self.mygr.clothing.value = clothing.clone();
		self.dui.clothing.value = clothing.clone();
		self.wumbus.clothing.value = clothing.clone();
		self.lero.clothing.value = clothing.clone();
		self.spibmom.clothing.value = clothing.clone();
		self.tuberwaber.clothing.value = clothing;
	}

	pub fn from_braidman(config: &BraidmanConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Braidman),
			braidman: BraidmanMenu::from(config).with_animation(animation),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_brenal(config: &BrenalConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Brenal),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::from(config).with_animation(animation),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_caole(config: &CaoleConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Caole),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::from(config).with_animation(animation),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_epiphant(config: &EpiphantConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Epiphant),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::from(config).with_animation(animation),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_hars(config: &HarsConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Hars),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::from(config).with_animation(animation),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_ylter(config: &YilterConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Yilter),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::from(config).with_animation(animation),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_sonyak(config: &SonyakConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Sonyak),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::from(config).with_animation(animation),
			claber: ClaberMenu::default(),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_croconot(config: &CroconotConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Croconot),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::from(config).with_animation(animation),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_claber(config: &ClaberConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Claber),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			claber: ClaberMenu::from(config).with_animation(animation),
			croconot: CroconotMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_brodler(config: &BrodlerConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Brodler),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::from(config).with_animation(animation),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_mygr(config: &MygrConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Mygr),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::from(config).with_animation(animation),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_dui(config: &DuiConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Dui),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::from(config).with_animation(animation),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_lidder(config: &LidderConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Lidder),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::from(config).with_animation(animation),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_chupri(config: &ChupriConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Chupri),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::from(config).with_animation(animation),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_brokker(config: &BrokkerConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Brokker),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::from(config).with_animation(animation),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_tipple(config: &TippleConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Tipple),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::from(config).with_animation(animation),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_topple(config: &ToppleConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Topple),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::from(config).with_animation(animation),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_kispar(config: &KisparConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Kispar),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::from(config).with_animation(animation),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_tapp(config: &TappConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Tapp),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::from(config).with_animation(animation),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_kaller(config: &KallerConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Kaller),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::from(config).with_animation(animation),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_kappler(config: &KapplerConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Kappler),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::from(config).with_animation(animation),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_wumbus(config: &WumbusConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Wumbus),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::from(config).with_animation(animation),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_lero(config: &LeroConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Lero),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::from(config).with_animation(animation),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_spibmom(config: &SpibmomConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Spibmom),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::from(config).with_animation(animation),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_grener(config: &GrenerConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Grener),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::from(config).with_animation(animation),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_thumplus(config: &ThumplusConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Thumplus),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::from(config).with_animation(animation),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_mistler(config: &MistlerConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Mistler),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::from(config).with_animation(animation),
			tuberwaber: TuberwaberMenu::default(),
		}
	}

	pub fn from_tuberwaber(config: &TuberwaberConfig, animation: ConceptAnimation) -> Self {
		Self {
			name: String::new(),
			inventory: None,
			appearance_locked: false,
			species: SingleSelect::new(ConceptSpecies::Tuberwaber),
			braidman: BraidmanMenu::default(),
			brenal: BrenalMenu::default(),
			caole: CaoleMenu::default(),
			epiphant: EpiphantMenu::default(),
			hars: HarsMenu::default(),
			ylter: YilterMenu::default(),
			sonyak: SonyakMenu::default(),
			croconot: CroconotMenu::default(),
			claber: ClaberMenu::default(),
			brodler: BrodlerMenu::default(),
			mygr: MygrMenu::default(),
			dui: DuiMenu::default(),
			lidder: LidderMenu::default(),
			chupri: ChupriMenu::default(),
			brokker: BrokkerMenu::default(),
			tipple: TippleMenu::default(),
			topple: ToppleMenu::default(),
			kispar: KisparMenu::default(),
			tapp: TappMenu::default(),
			kaller: KallerMenu::default(),
			kappler: KapplerMenu::default(),
			wumbus: WumbusMenu::default(),
			lero: LeroMenu::default(),
			spibmom: SpibmomMenu::default(),
			grener: GrenerMenu::default(),
			thumplus: ThumplusMenu::default(),
			mistler: MistlerMenu::default(),
			tuberwaber: TuberwaberMenu::from(config).with_animation(animation),
		}
	}

	/// Lowers the currently selected species menu; the other species' state is
	/// retained but not part of the tree.
	fn species_node(&self) -> MenuNode<MenuEvent> {
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.menu_node(),
			ConceptSpecies::Brenal => self.brenal.menu_node(),
			ConceptSpecies::Caole => self.caole.menu_node(),
			ConceptSpecies::Epiphant => self.epiphant.menu_node(),
			ConceptSpecies::Hars => self.hars.menu_node(),
			ConceptSpecies::Yilter => self.ylter.menu_node(),
			ConceptSpecies::Sonyak => self.sonyak.menu_node(),
			ConceptSpecies::Claber => self.claber.menu_node(),
			ConceptSpecies::Croconot => self.croconot.menu_node(),
			ConceptSpecies::Brodler => self.brodler.menu_node(),
			ConceptSpecies::Mygr => self.mygr.menu_node(),
			ConceptSpecies::Dui => self.dui.menu_node(),
			ConceptSpecies::Lidder => self.lidder.menu_node(),
			ConceptSpecies::Chupri => self.chupri.menu_node(),
			ConceptSpecies::Brokker => self.brokker.menu_node(),
			ConceptSpecies::Tipple => self.tipple.menu_node(),
			ConceptSpecies::Topple => self.topple.menu_node(),
			ConceptSpecies::Kispar => self.kispar.menu_node(),
			ConceptSpecies::Tapp => self.tapp.menu_node(),
			ConceptSpecies::Kaller => self.kaller.menu_node(),
			ConceptSpecies::Kappler => self.kappler.menu_node(),
			ConceptSpecies::Wumbus => self.wumbus.menu_node(),
			ConceptSpecies::Lero => self.lero.menu_node(),
			ConceptSpecies::Spibmom => self.spibmom.menu_node(),
			ConceptSpecies::Grener => self.grener.menu_node(),
			ConceptSpecies::Thumplus => self.thumplus.menu_node(),
			ConceptSpecies::Mistler => self.mistler.menu_node(),
			ConceptSpecies::Tuberwaber => self.tuberwaber.menu_node(),
		}
	}

	pub fn animation(&self) -> ConceptAnimation {
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.animation(),
			ConceptSpecies::Brenal => self.brenal.animation(),
			ConceptSpecies::Caole => self.caole.animation(),
			ConceptSpecies::Epiphant => self.epiphant.animation(),
			ConceptSpecies::Hars => self.hars.animation(),
			ConceptSpecies::Yilter => self.ylter.animation(),
			ConceptSpecies::Sonyak => self.sonyak.animation(),
			ConceptSpecies::Claber => self.claber.animation(),
			ConceptSpecies::Croconot => self.croconot.animation(),
			ConceptSpecies::Brodler => self.brodler.animation(),
			ConceptSpecies::Mygr => self.mygr.animation(),
			ConceptSpecies::Dui => self.dui.animation(),
			ConceptSpecies::Lidder => self.lidder.animation(),
			ConceptSpecies::Chupri => self.chupri.animation(),
			ConceptSpecies::Brokker => self.brokker.animation(),
			ConceptSpecies::Tipple => self.tipple.animation(),
			ConceptSpecies::Topple => self.topple.animation(),
			ConceptSpecies::Kispar => self.kispar.animation(),
			ConceptSpecies::Tapp => self.tapp.animation(),
			ConceptSpecies::Kaller => self.kaller.animation(),
			ConceptSpecies::Kappler => self.kappler.animation(),
			ConceptSpecies::Wumbus => self.wumbus.animation(),
			ConceptSpecies::Lero => self.lero.animation(),
			ConceptSpecies::Spibmom => self.spibmom.animation(),
			ConceptSpecies::Grener => self.grener.animation(),
			ConceptSpecies::Thumplus => self.thumplus.animation(),
			ConceptSpecies::Mistler => self.mistler.animation(),
			ConceptSpecies::Tuberwaber => self.tuberwaber.animation(),
		}
	}

	pub fn braidman_config(&self) -> BraidmanConfig {
		BraidmanConfig::from(&self.braidman)
	}

	pub fn brenal_config(&self) -> BrenalConfig {
		BrenalConfig::from(&self.brenal)
	}

	pub fn caole_config(&self) -> CaoleConfig {
		CaoleConfig::from(&self.caole)
	}

	pub fn epiphant_config(&self) -> EpiphantConfig {
		EpiphantConfig::from(&self.epiphant)
	}

	pub fn hars_config(&self) -> HarsConfig {
		HarsConfig::from(&self.hars)
	}

	pub fn ylter_config(&self) -> YilterConfig {
		YilterConfig::from(&self.ylter)
	}

	pub fn sonyak_config(&self) -> SonyakConfig {
		SonyakConfig::from(&self.sonyak)
	}

	pub fn claber_config(&self) -> ClaberConfig {
		ClaberConfig::from(&self.claber)
	}

	pub fn croconot_config(&self) -> CroconotConfig {
		CroconotConfig::from(&self.croconot)
	}

	pub fn brodler_config(&self) -> BrodlerConfig {
		BrodlerConfig::from(&self.brodler)
	}

	pub fn mygr_config(&self) -> MygrConfig {
		MygrConfig::from(&self.mygr)
	}

	pub fn dui_config(&self) -> DuiConfig {
		DuiConfig::from(&self.dui)
	}

	pub fn lidder_config(&self) -> LidderConfig {
		LidderConfig::from(&self.lidder)
	}

	pub fn chupri_config(&self) -> ChupriConfig {
		ChupriConfig::from(&self.chupri)
	}

	pub fn brokker_config(&self) -> BrokkerConfig {
		BrokkerConfig::from(&self.brokker)
	}

	pub fn tipple_config(&self) -> TippleConfig {
		TippleConfig::from(&self.tipple)
	}

	pub fn topple_config(&self) -> ToppleConfig {
		ToppleConfig::from(&self.topple)
	}

	pub fn kispar_config(&self) -> KisparConfig {
		KisparConfig::from(&self.kispar)
	}

	pub fn tapp_config(&self) -> TappConfig {
		TappConfig::from(&self.tapp)
	}

	pub fn kaller_config(&self) -> KallerConfig {
		KallerConfig::from(&self.kaller)
	}

	pub fn kappler_config(&self) -> KapplerConfig {
		KapplerConfig::from(&self.kappler)
	}

	pub fn wumbus_config(&self) -> WumbusConfig {
		WumbusConfig::from(&self.wumbus)
	}

	pub fn lero_config(&self) -> LeroConfig {
		LeroConfig::from(&self.lero)
	}

	pub fn spibmom_config(&self) -> SpibmomConfig {
		SpibmomConfig::from(&self.spibmom)
	}

	pub fn grener_config(&self) -> GrenerConfig {
		GrenerConfig::from(&self.grener)
	}

	pub fn thumplus_config(&self) -> ThumplusConfig {
		ThumplusConfig::from(&self.thumplus)
	}

	pub fn mistler_config(&self) -> MistlerConfig {
		MistlerConfig::from(&self.mistler)
	}

	pub fn tuberwaber_config(&self) -> TuberwaberConfig {
		TuberwaberConfig::from(&self.tuberwaber)
	}

	pub fn apply(&mut self, event: MenuEvent) -> bool {
		match event {
			MenuEvent::Save => return self.is_create(),
			MenuEvent::SetSpecies(species) => {
				if self.appearance_locked() {
					return false;
				}
				if self.inventory.is_some() && !species.is_humanoid() {
					return false;
				}
				if self.species.value == species {
					return false;
				}
				self.species.value = species;
				self.sync_inventory_clothing();
				return true;
			}
			MenuEvent::ToggleInventory(index) => {
				let Some(inventory) = self.inventory.as_mut() else {
					return false;
				};
				if !inventory.toggle(index) {
					return false;
				}
				self.sync_inventory_clothing();
				return true;
			}
			MenuEvent::ToggleClothing(_) if self.inventory.is_some() => return false,
			MenuEvent::ToggleSection(_) => return false,
			_ if self.appearance_locked() && !event.edits_inventory() => return false,
			_ => {}
		}
		match self.species.value {
			ConceptSpecies::Braidman => self.apply_braidman(event),
			ConceptSpecies::Brenal => self.apply_brenal(event),
			ConceptSpecies::Caole => self.apply_caole(event),
			ConceptSpecies::Epiphant => self.apply_epiphant(event),
			ConceptSpecies::Hars => self.apply_hars(event),
			ConceptSpecies::Yilter => self.apply_ylter(event),
			ConceptSpecies::Sonyak => self.apply_sonyak(event),
			ConceptSpecies::Claber => self.apply_claber(event),
			ConceptSpecies::Croconot => self.apply_croconot(event),
			ConceptSpecies::Brodler => self.apply_brodler(event),
			ConceptSpecies::Mygr => self.apply_mygr(event),
			ConceptSpecies::Dui => self.apply_dui(event),
			ConceptSpecies::Lidder => self.apply_lidder(event),
			ConceptSpecies::Chupri => self.apply_chupri(event),
			ConceptSpecies::Brokker => self.apply_brokker(event),
			ConceptSpecies::Tipple => self.apply_tipple(event),
			ConceptSpecies::Topple => self.apply_topple(event),
			ConceptSpecies::Kispar => self.apply_kispar(event),
			ConceptSpecies::Tapp => self.apply_tapp(event),
			ConceptSpecies::Kaller => self.apply_kaller(event),
			ConceptSpecies::Kappler => self.apply_kappler(event),
			ConceptSpecies::Wumbus => self.apply_wumbus(event),
			ConceptSpecies::Lero => self.apply_lero(event),
			ConceptSpecies::Spibmom => self.apply_spibmom(event),
			ConceptSpecies::Grener => self.apply_grener(event),
			ConceptSpecies::Thumplus => self.apply_thumplus(event),
			ConceptSpecies::Mistler => self.apply_mistler(event),
			ConceptSpecies::Tuberwaber => self.apply_tuberwaber(event),
		}
	}

	pub fn camera_focus_for_event(&self, event: MenuEvent) -> Option<CameraFocus> {
		if matches!(event, MenuEvent::ToggleInventory(_)) {
			return Some(crate::BODY_FOCUS);
		}
		let field = match event {
			MenuEvent::SetAsset(field, _) | MenuEvent::Cycle(field, _) => Some(field),
			MenuEvent::ToggleClothing(clothing) => Some(CharacterField::Clothing(clothing)),
			_ => None,
		}?;
		match self.species.value {
			ConceptSpecies::Braidman => self.braidman.camera_focus_for_field(field),
			ConceptSpecies::Brenal => self.brenal.camera_focus_for_field(field),
			ConceptSpecies::Caole => self.caole.camera_focus_for_field(field),
			ConceptSpecies::Epiphant => self.epiphant.camera_focus_for_field(field),
			ConceptSpecies::Hars => self.hars.camera_focus_for_field(field),
			ConceptSpecies::Yilter => self.ylter.camera_focus_for_field(field),
			ConceptSpecies::Sonyak => self.sonyak.camera_focus_for_field(field),
			ConceptSpecies::Claber => self.claber.camera_focus_for_field(field),
			ConceptSpecies::Croconot => self.croconot.camera_focus_for_field(field),
			ConceptSpecies::Brodler => self.brodler.camera_focus_for_field(field),
			ConceptSpecies::Mygr => self.mygr.camera_focus_for_field(field),
			ConceptSpecies::Dui => self.dui.camera_focus_for_field(field),
			ConceptSpecies::Lidder => self.lidder.camera_focus_for_field(field),
			ConceptSpecies::Chupri => self.chupri.camera_focus_for_field(field),
			ConceptSpecies::Brokker => self.brokker.camera_focus_for_field(field),
			ConceptSpecies::Tipple => self.tipple.camera_focus_for_field(field),
			ConceptSpecies::Topple => self.topple.camera_focus_for_field(field),
			ConceptSpecies::Kispar => self.kispar.camera_focus_for_field(field),
			ConceptSpecies::Tapp => self.tapp.camera_focus_for_field(field),
			ConceptSpecies::Kaller => self.kaller.camera_focus_for_field(field),
			ConceptSpecies::Kappler => self.kappler.camera_focus_for_field(field),
			ConceptSpecies::Wumbus => self.wumbus.camera_focus_for_field(field),
			ConceptSpecies::Lero => self.lero.camera_focus_for_field(field),
			ConceptSpecies::Spibmom => self.spibmom.camera_focus_for_field(field),
			ConceptSpecies::Grener => self.grener.camera_focus_for_field(field),
			ConceptSpecies::Thumplus => self.thumplus.camera_focus_for_field(field),
			ConceptSpecies::Mistler => self.mistler.camera_focus_for_field(field),
			ConceptSpecies::Tuberwaber => self.tuberwaber.camera_focus_for_field(field),
		}
	}

	fn apply_braidman(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.braidman;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BodyMesh, AssetValue::Body(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::HeadMesh, AssetValue::Head(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Ear, AssetValue::Ear(value)) => {
					menu.head_features.value.ear.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_braidman_slider(menu, field, delta),
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HairColor => {
					menu.hair.value.color.value = color;
					true
				}
				CharacterField::Clothing(clothing) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleInventory(_) => {
				false
			}
		}
	}

	fn apply_brenal(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.brenal;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::BrenalHorns, delta) => {
				menu.head_features.value.horns.value =
					cycle_value(menu.head_features.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BrenalBody, AssetValue::BrenalBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::BrenalHead, AssetValue::BrenalHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::BrenalMouth, AssetValue::BrenalMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = BrenalAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_brenal_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_caole(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.caole;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::CaoleBody, AssetValue::CaoleBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::CaoleMouth, AssetValue::CaoleMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = CaoleAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_caole_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_epiphant(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.epiphant;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::EpiphantBody, AssetValue::EpiphantBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::EpiphantNose, AssetValue::EpiphantNose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = EpiphantAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_epiphant_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Epiphant(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::NoseColor => {
					menu.head_features.value.nose_color.value = color;
					true
				}
				CharacterField::EarColor => {
					menu.head_features.value.ear_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_hars(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.hars;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::HarsBody, AssetValue::HarsBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::HarsMouth, AssetValue::HarsMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = HarsAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_hars_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_ylter(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.ylter;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::YilterBody, AssetValue::YilterBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::YilterMouth, AssetValue::YilterMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = YilterAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_ylter_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_sonyak(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.sonyak;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::SonyakBody, AssetValue::SonyakBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::SonyakMouth, AssetValue::SonyakMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = SonyakAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_sonyak_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HairColor => {
					menu.head_features.value.hair_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_croconot(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.croconot;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::CroconotHorns, delta) => {
				menu.head_features.value.horns.value =
					cycle_value(menu.head_features.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::CroconotBody, AssetValue::CroconotBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::CroconotHead, AssetValue::CroconotHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::CroconotMouth, AssetValue::CroconotMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = CroconotAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_croconot_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_claber(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.claber;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::ClaberHorns, delta) => {
				menu.head_features.value.horns.value =
					cycle_value(menu.head_features.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::ClaberBody, AssetValue::ClaberBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::ClaberHead, AssetValue::ClaberHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::ClaberMouth, AssetValue::ClaberMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = ClaberAnimationClip::from(value);
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_claber_slider(menu, field, delta),
			MenuEvent::SetSwatch(field, SwatchValue::Claber(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				CharacterField::TailColor => {
					menu.body.value.tail_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _)
			| MenuEvent::Cycle(_, _)
			| MenuEvent::ToggleClothing(_)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_brodler(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.brodler;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BrodlerHead, AssetValue::BrodlerHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Horns, AssetValue::Horns(value)) => {
					menu.head.value.horns.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Ear, AssetValue::Ear(value)) => {
					menu.head_features.value.ear.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::SkinColor, SwatchValue::BrodlerSkin(color)) => {
					menu.head.value.skin.value = color;
					menu.head_features.value.skin_color = color;
					true
				}
				(CharacterField::BrodlerEyeColor, SwatchValue::BrodlerEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::HornColor, SwatchValue::BrodlerHorn(color)) => {
					menu.head_features.value.horn_color.value = color;
					menu.head.value.horn_color = color;
					true
				}
				(CharacterField::MouthColor, SwatchValue::Item(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_mygr(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.mygr;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::MygrHead, AssetValue::MygrHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::MygrMouth, AssetValue::MygrMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::MygrSkinColor, SwatchValue::MygrSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::MygrEyeColor, SwatchValue::MygrEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::MouthColor, SwatchValue::Item(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_dui(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.dui;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::DuiNose, delta) => {
				menu.head_features.value.nose.value =
					cycle_value(menu.head_features.value.nose.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::DuiHead, AssetValue::DuiHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::DuiEye, AssetValue::DuiEye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::DuiMouth, AssetValue::DuiMouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::DuiSkinColor, SwatchValue::DuiSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::DuiMouthColor, SwatchValue::DuiMouth(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_lidder(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.lidder;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::LidderHead, AssetValue::LidderHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::LidderBeak, AssetValue::LidderBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::LidderPlumageColor, SwatchValue::LidderPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::LidderEyeColor, SwatchValue::LidderEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::LidderBeakColor, SwatchValue::LidderBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_chupri(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.chupri;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::ChupriHead, AssetValue::ChupriHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::ChupriBeak, AssetValue::ChupriBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::ChupriPlumageColor, SwatchValue::ChupriPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::ChupriEyeColor, SwatchValue::ChupriEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::ChupriBeakColor, SwatchValue::ChupriBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_brokker(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.brokker;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::BrokkerHead, AssetValue::BrokkerHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::BrokkerPlumageColor, SwatchValue::BrokkerPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::BrokkerEyeColor, SwatchValue::BrokkerEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::BrokkerSnoutColor, SwatchValue::BrokkerSnout(color)) => {
					menu.head_features.value.snout_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_tipple(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.tipple;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::TippleHead, AssetValue::TippleHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::TippleBeak, AssetValue::TippleBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::TipplePlumageColor, SwatchValue::TipplePlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::TippleEyeColor, SwatchValue::TippleEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::TippleBeakColor, SwatchValue::TippleBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_topple(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.topple;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::ToppleHead, AssetValue::ToppleHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::ToppleBeak, AssetValue::ToppleBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::TopplePlumageColor, SwatchValue::TopplePlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::ToppleEyeColor, SwatchValue::ToppleEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::ToppleBeakColor, SwatchValue::ToppleBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_kispar(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.kispar;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::KisparHead, AssetValue::KisparHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::KisparBeak, AssetValue::KisparBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::KisparPlumageColor, SwatchValue::KisparPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::KisparEyeColor, SwatchValue::KisparEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::KisparBeakColor, SwatchValue::KisparBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_tapp(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.tapp;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::TappHead, AssetValue::TappHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::TappBeak, AssetValue::TappBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::TappPlumageColor, SwatchValue::TappPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::TappEyeColor, SwatchValue::TappEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::TappBeakColor, SwatchValue::TappBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_kaller(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.kaller;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::KallerHead, AssetValue::KallerHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::KallerPlumageColor, SwatchValue::KallerPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::KallerEyeColor, SwatchValue::KallerEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::KallerSnoutColor, SwatchValue::KallerSnout(color)) => {
					menu.head_features.value.snout_color.value = color;
					true
				}
				(CharacterField::KallerCrownColor, SwatchValue::KallerCrown(color)) => {
					menu.head_features.value.crown_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_kappler(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.kappler;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::KapplerHead, AssetValue::KapplerHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::KapplerBeak, AssetValue::KapplerBeak(value)) => {
					menu.head_features.value.beak.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::KapplerPlumageColor, SwatchValue::KapplerPlumage(color)) => {
					menu.head.value.plumage.value = color;
					true
				}
				(CharacterField::KapplerEyeColor, SwatchValue::KapplerEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::KapplerBeakColor, SwatchValue::KapplerBeak(color)) => {
					menu.head_features.value.beak_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_wumbus(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.wumbus;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::WumbusHorns, delta) => {
				menu.head.value.horns.value = cycle_value(menu.head.value.horns.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::WumbusHead, AssetValue::WumbusHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::WumbusMouth, AssetValue::WumbusMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::WumbusSkinColor, SwatchValue::WumbusSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::WumbusEyeColor, SwatchValue::WumbusEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::WumbusEarColor, SwatchValue::WumbusEar(color)) => {
					menu.head_features.value.ear_color.value = color;
					true
				}
				(CharacterField::WumbusMouthColor, SwatchValue::WumbusMouth(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::WumbusHornColor, SwatchValue::WumbusHorn(color)) => {
					menu.head.value.horn_color.value = color;
					true
				}
				(CharacterField::WumbusSpineColor, SwatchValue::WumbusSpine(color)) => {
					menu.head.value.spine_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_lero(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.lero;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::LeroHead, AssetValue::LeroHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::LeroMouth, AssetValue::LeroMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::LeroSkinColor, SwatchValue::LeroSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::LeroEyeColor, SwatchValue::LeroEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::LeroMouthColor, SwatchValue::LeroMouthColor(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::LeroTailColor, SwatchValue::LeroTail(color)) => {
					menu.body.value.tail_color.value = color;
					true
				}
				(CharacterField::LeroSpineColor, SwatchValue::LeroSpine(color)) => {
					menu.body.value.spine_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_spibmom(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.spibmom;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::SpibmomHead, AssetValue::SpibmomHead(value)) => {
					menu.head.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::SpibmomMouth, AssetValue::SpibmomMouth(value)) => {
					menu.head_features.value.snout.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::SpibmomSkinColor, SwatchValue::SpibmomSkin(color)) => {
					menu.head.value.skin.value = color;
					true
				}
				(CharacterField::SpibmomEyeColor, SwatchValue::SpibmomEye(color)) => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				(CharacterField::SpibmomEarColor, SwatchValue::SpibmomEar(color)) => {
					menu.head_features.value.ear_color.value = color;
					true
				}
				(CharacterField::SpibmomMouthColor, SwatchValue::SpibmomMouthColor(color)) => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				(CharacterField::SpibmomCrownColor, SwatchValue::SpibmomCrown(color)) => {
					menu.head.value.crown_color.value = color;
					true
				}
				(CharacterField::SpibmomSpineColor, SwatchValue::SpibmomSpine(color)) => {
					menu.head.value.spine_color.value = color;
					true
				}
				(CharacterField::HairColor, SwatchValue::Item(color)) => {
					menu.hair.value.color.value = color;
					true
				}
				(CharacterField::Clothing(clothing), SwatchValue::Item(color)) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _)
			| MenuEvent::SliderDelta(_, _)
			| MenuEvent::ToggleInventory(_) => false,
		}
	}

	fn apply_grener(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.grener;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(_) | MenuEvent::ToggleInventory(_) => false,
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::GrenerBodyColor, SwatchValue::GrenerBody(color)) => {
					menu.body.value.body.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_thumplus(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.thumplus;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(_) | MenuEvent::ToggleInventory(_) => false,
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::ThumplusBodyColor, SwatchValue::ThumplusBody(color)) => {
					menu.body.value.body.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_mistler(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.mistler;
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::ToggleClothing(_) | MenuEvent::ToggleInventory(_) => false,
			MenuEvent::SetSwatch(field, value) => match (field, value) {
				(CharacterField::MistlerBodyColor, SwatchValue::MistlerBody(color)) => {
					menu.body.value.body.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::Cycle(_, _) | MenuEvent::SliderDelta(_, _) => false,
		}
	}

	fn apply_tuberwaber(&mut self, event: MenuEvent) -> bool {
		let menu = &mut self.tuberwaber;
		if crate::shared::apply_clothing_event(&mut menu.clothing.value, event) {
			return true;
		}
		match event {
			MenuEvent::ToggleSection(_) | MenuEvent::SetSpecies(_) | MenuEvent::Save => false,
			MenuEvent::Cycle(CharacterField::Gender, delta) => {
				menu.presets.value.gender.value =
					cycle_value(menu.presets.value.gender.value, delta);
				true
			}
			MenuEvent::Cycle(CharacterField::Build, delta) => {
				menu.presets.value.build.value = cycle_value(menu.presets.value.build.value, delta);
				true
			}
			MenuEvent::SetAsset(field, value) => match (field, value) {
				(CharacterField::TuberwaberBody, AssetValue::TuberwaberBody(value)) => {
					menu.body.value.body.value = value;
					true
				}
				(CharacterField::TuberwaberHead, AssetValue::TuberwaberHead(value)) => {
					menu.head_features.value.head.value = value;
					true
				}
				(CharacterField::Eye, AssetValue::Eye(value)) => {
					menu.head_features.value.eye.value = value;
					true
				}
				(CharacterField::Nose, AssetValue::Nose(value)) => {
					menu.head_features.value.nose.value = value;
					true
				}
				(CharacterField::Mouth, AssetValue::Mouth(value)) => {
					menu.head_features.value.mouth.value = value;
					true
				}
				(CharacterField::Hair, AssetValue::Hair(value)) => {
					menu.hair.value.style.value = value;
					true
				}
				(CharacterField::Animation, AssetValue::Animation(value)) => {
					menu.animation.value.clip.value = value;
					true
				}
				_ => false,
			},
			MenuEvent::SliderDelta(field, delta) => apply_tuberwaber_slider(menu, field, delta),
			MenuEvent::ToggleClothing(clothing) => {
				menu.clothing.value.layers.toggle(clothing);
				true
			}
			MenuEvent::SetSwatch(field, SwatchValue::Tuberwaber(color)) => match field {
				CharacterField::BodyColor => {
					menu.body.value.color.value = color;
					menu.head_features.value.body_color = color;
					true
				}
				CharacterField::EyeColor => {
					menu.head_features.value.eye_color.value = color;
					true
				}
				CharacterField::MouthColor => {
					menu.head_features.value.mouth_color.value = color;
					true
				}
				CharacterField::HornColor => {
					menu.head_features.value.horn_color.value = color;
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(field, SwatchValue::Item(color)) => match field {
				CharacterField::HairColor => {
					menu.hair.value.color.value = color;
					true
				}
				CharacterField::Clothing(clothing) => {
					menu.set_clothing_color(clothing, color);
					true
				}
				_ => false,
			},
			MenuEvent::SetSwatch(_, _) | MenuEvent::Cycle(_, _) | MenuEvent::ToggleInventory(_) => {
				false
			}
		}
	}
}

impl MenuComponent<MenuEvent> for CharacterMenu {
	fn menu_node(&self) -> MenuNode<MenuEvent> {
		let species_groups: &[(&str, &[ConceptSpecies])] = if self.inventory.is_some() {
			&[("Humanoids", ConceptSpecies::HUMANOIDS)]
		} else {
			&[
				(
					"Humanoids",
					&[
						ConceptSpecies::Braidman,
						ConceptSpecies::Brodler,
						ConceptSpecies::Mygr,
						ConceptSpecies::Dui,
						ConceptSpecies::Wumbus,
						ConceptSpecies::Lero,
						ConceptSpecies::Spibmom,
						ConceptSpecies::Tuberwaber,
					],
				),
				(
					"Quadrupeds",
					&[
						ConceptSpecies::Brenal,
						ConceptSpecies::Caole,
						ConceptSpecies::Epiphant,
						ConceptSpecies::Hars,
						ConceptSpecies::Yilter,
						ConceptSpecies::Sonyak,
						ConceptSpecies::Claber,
						ConceptSpecies::Croconot,
					],
				),
				(
					"Birds",
					&[
						ConceptSpecies::Lidder,
						ConceptSpecies::Chupri,
						ConceptSpecies::Brokker,
						ConceptSpecies::Tipple,
						ConceptSpecies::Topple,
						ConceptSpecies::Kispar,
						ConceptSpecies::Tapp,
						ConceptSpecies::Kaller,
						ConceptSpecies::Kappler,
					],
				),
				(
					"Aquatic",
					&[ConceptSpecies::Grener, ConceptSpecies::Thumplus, ConceptSpecies::Mistler],
				),
			]
		};
		let mut nodes = vec![
			MenuNode::short_text("Name", self.name.clone(), CHARACTER_NAME_MAX_LEN),
			MenuNode::section_select_grouped(
				"Species",
				self.species.value,
				MenuEvent::SetSpecies,
				species_groups,
				self.species_node(),
			),
		];
		if let Some(inventory) = &self.inventory {
			nodes.push(MenuNode::section("Weapons", weapons_catalog(inventory)));
			nodes.push(MenuNode::section("Skill Maps", skills_catalog(inventory)));
			nodes.push(loadout_section(inventory));
		}
		MenuNode::fragment(nodes)
	}
}

fn apply_brenal_slider(menu: &mut BrenalMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_caole_slider(menu: &mut CaoleMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_epiphant_slider(menu: &mut EpiphantMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_hars_slider(menu: &mut HarsMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_ylter_slider(menu: &mut YilterMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_sonyak_slider(menu: &mut SonyakMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_croconot_slider(menu: &mut CroconotMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		CharacterField::SnoutLength => face.snout_length = face.snout_length.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_claber_slider(menu: &mut ClaberMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		CharacterField::SnoutLength => face.snout_length = face.snout_length.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_braidman_slider(menu: &mut BraidmanMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::NoseWidth => face.nose_width = face.nose_width.apply_delta(delta),
		CharacterField::NoseHeight => face.nose_height = face.nose_height.apply_delta(delta),
		CharacterField::MouthWidth => face.mouth_width = face.mouth_width.apply_delta(delta),
		CharacterField::MouthHeight => face.mouth_height = face.mouth_height.apply_delta(delta),
		CharacterField::EarWidth => face.ear_width = face.ear_width.apply_delta(delta),
		CharacterField::EarHeight => face.ear_height = face.ear_height.apply_delta(delta),
		_ => return false,
	}
	true
}

fn apply_tuberwaber_slider(menu: &mut TuberwaberMenu, field: CharacterField, delta: f32) -> bool {
	let body = &mut menu.body.value.sliders;
	let face = &mut menu.head_features.value.feature_sliders;
	match field {
		CharacterField::ShoulderWidth => {
			body.shoulder_width = body.shoulder_width.apply_delta(delta)
		}
		CharacterField::HipWidth => body.hip_width = body.hip_width.apply_delta(delta),
		CharacterField::ChestThickness => {
			body.chest_thickness = body.chest_thickness.apply_delta(delta)
		}
		CharacterField::HipThickness => body.hip_thickness = body.hip_thickness.apply_delta(delta),
		CharacterField::LegThickness => body.leg_thickness = body.leg_thickness.apply_delta(delta),
		CharacterField::ButtocksThickness => {
			body.buttocks_thickness = body.buttocks_thickness.apply_delta(delta)
		}
		CharacterField::WaistThickness => {
			body.waist_thickness = body.waist_thickness.apply_delta(delta)
		}
		CharacterField::LowerTrunkThickness => {
			body.lower_trunk_thickness = body.lower_trunk_thickness.apply_delta(delta)
		}
		CharacterField::ArmLength => body.arm_length = body.arm_length.apply_delta(delta),
		CharacterField::ArmThickness => body.arm_thickness = body.arm_thickness.apply_delta(delta),
		CharacterField::LegLength => body.leg_length = body.leg_length.apply_delta(delta),
		CharacterField::EyeWidth => face.eye_width = face.eye_width.apply_delta(delta),
		CharacterField::EyeHeight => face.eye_height = face.eye_height.apply_delta(delta),
		CharacterField::EyeTilt => face.eye_tilt = face.eye_tilt.apply_delta(delta),
		CharacterField::NoseWidth => face.nose_width = face.nose_width.apply_delta(delta),
		CharacterField::NoseHeight => face.nose_height = face.nose_height.apply_delta(delta),
		CharacterField::MouthWidth => face.mouth_width = face.mouth_width.apply_delta(delta),
		CharacterField::MouthHeight => face.mouth_height = face.mouth_height.apply_delta(delta),
		_ => return false,
	}
	true
}

impl Default for CharacterMenu {
	fn default() -> Self {
		Self::from_braidman(&BraidmanConfig::default_preview(), ConceptAnimation::default())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionOpenState {
	pub presets_open: bool,
	pub head_open: bool,
	pub body_open: bool,
	pub head_features_open: bool,
	pub hair_open: bool,
	pub clothing_open: bool,
	pub weapons_open: bool,
	pub skills_open: bool,
	pub loadout_open: bool,
	pub animation_open: bool,
}

impl SectionOpenState {
	pub fn is_section_open(self, section: SectionId) -> bool {
		match section {
			SectionId::Presets => self.presets_open,
			SectionId::Head => self.head_open,
			SectionId::Body => self.body_open,
			SectionId::HeadFeatures => self.head_features_open,
			SectionId::Hair => self.hair_open,
			SectionId::Clothing => self.clothing_open,
			SectionId::Weapons => self.weapons_open,
			SectionId::SkillMaps => self.skills_open,
			SectionId::Loadout => self.loadout_open,
			SectionId::Animation => self.animation_open,
		}
	}

	pub fn toggle(&mut self, section: SectionId) {
		match section {
			SectionId::Presets => self.presets_open = !self.presets_open,
			SectionId::Head => self.head_open = !self.head_open,
			SectionId::Body => self.body_open = !self.body_open,
			SectionId::HeadFeatures => self.head_features_open = !self.head_features_open,
			SectionId::Hair => self.hair_open = !self.hair_open,
			SectionId::Clothing => self.clothing_open = !self.clothing_open,
			SectionId::Weapons => self.weapons_open = !self.weapons_open,
			SectionId::SkillMaps => self.skills_open = !self.skills_open,
			SectionId::Loadout => self.loadout_open = !self.loadout_open,
			SectionId::Animation => self.animation_open = !self.animation_open,
		}
	}
}

impl Default for SectionOpenState {
	fn default() -> Self {
		Self {
			presets_open: true,
			head_open: true,
			body_open: true,
			head_features_open: false,
			hair_open: false,
			clothing_open: true,
			weapons_open: true,
			skills_open: true,
			loadout_open: true,
			animation_open: false,
		}
	}
}
