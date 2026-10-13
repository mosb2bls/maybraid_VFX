//! Preview configuration and spawning.
//!
//! Commands update [`ConceptPreviewConfig`]. This module spawns nested LodScene
//! hosts via [`ComponentsOnly`] / [`lod::LodScene::host`] ([`CharacterRecipe::clothed`]). Live color
//! inserts [`MaterialRefRoot`] on part hosts; [`PreviewAssetTarget`] stays UI mapping.

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use character_items::ClothingMesh;
use characters::{
	assembly::CharacterPartSlot,
	character_bounds,
	species::{
		braidman::BraidmanConfig,
		brenal::BrenalConfig,
		brodler::{BrodlerConfig, BrodlerHeadMesh, HornMesh},
		brokker::{BrokkerConfig, BrokkerSnoutMesh},
		caole::CaoleConfig,
		chupri::{ChupriBeakMesh, ChupriConfig},
		claber::ClaberConfig,
		common::{BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh},
		croconot::CroconotConfig,
		dui::{DuiConfig, DuiNoseMesh},
		epiphant::EpiphantConfig,
		grener::GrenerConfig,
		hars::HarsConfig,
		kaller::{KallerConfig, KallerSnoutMesh},
		kappler::{KapplerBeakMesh, KapplerConfig},
		kispar::{KisparBeakMesh, KisparConfig},
		lero::{LeroConfig, LeroMouthMesh},
		lidder::{LidderBeakMesh, LidderConfig},
		mistler::MistlerConfig,
		mygr::MygrConfig,
		sonyak::SonyakConfig,
		spibmom::SpibmomConfig,
		tapp::{TappBeakMesh, TappConfig},
		thumplus::ThumplusConfig,
		tipple::{TippleBeakMesh, TippleConfig},
		topple::{ToppleBeakMesh, ToppleConfig},
		tuberwaber::{TuberwaberBodyMesh, TuberwaberConfig, TuberwaberHeadMesh},
		wumbus::{WumbusConfig, WumbusHornMesh},
		ylter::YilterConfig,
	},
	AnimRef, AnimRefRoot, CharacterComponents, CharacterMembers, CharacterRecipe, ComponentsOnly,
	MaterialRefRoot, PartNode, RigId, RigNode, SkinRefApplied, SkinRefRoot, SocketRefApplied,
	SocketRefRoot,
};
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::LodSceneLevel;

use crate::animation::ConceptAnimation;
use crate::skinning::{
	bind_scales_ready, bone_map_ready, missing_landmark_bones, preview_debug_enabled,
	ActiveRigPose, BoneMap, CharacterPart, CharacterRig, CharacterRigRole,
	NeedsDuplicateScenePrune, NeedsSkinRemap, NoMatchingArmature, RigBindScales,
};

/// Run `$body` with `$recipe` bound to `config.clothed()` (monomorphized per species).
macro_rules! with_clothed_recipe {
	($config:expr, $recipe:ident => $body:expr) => {
		match $config {
			ConceptPreviewConfig::Braidman { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Brenal { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Caole { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Epiphant { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Hars { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Yilter { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Sonyak { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Claber { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Croconot { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Brodler { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Mygr { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Dui { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Lidder { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Chupri { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Brokker { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Tipple { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Topple { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Kispar { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Tapp { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Kaller { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Kappler { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Wumbus { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Lero { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Spibmom { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Grener { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Thumplus { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Mistler { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
			ConceptPreviewConfig::Tuberwaber { config, .. } => {
				let $recipe = config.clothed();
				$body
			}
		}
	};
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConceptSpecies {
	#[default]
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

#[derive(Resource, Debug, Clone, PartialEq)]
pub enum ConceptPreviewConfig {
	Braidman { config: BraidmanConfig, animation: ConceptAnimation },
	Brenal { config: BrenalConfig, animation: ConceptAnimation },
	Caole { config: CaoleConfig, animation: ConceptAnimation },
	Epiphant { config: EpiphantConfig, animation: ConceptAnimation },
	Hars { config: HarsConfig, animation: ConceptAnimation },
	Yilter { config: YilterConfig, animation: ConceptAnimation },
	Sonyak { config: SonyakConfig, animation: ConceptAnimation },
	Claber { config: ClaberConfig, animation: ConceptAnimation },
	Croconot { config: CroconotConfig, animation: ConceptAnimation },
	Brodler { config: BrodlerConfig, animation: ConceptAnimation },
	Mygr { config: MygrConfig, animation: ConceptAnimation },
	Dui { config: DuiConfig, animation: ConceptAnimation },
	Lidder { config: LidderConfig, animation: ConceptAnimation },
	Chupri { config: ChupriConfig, animation: ConceptAnimation },
	Brokker { config: BrokkerConfig, animation: ConceptAnimation },
	Tipple { config: TippleConfig, animation: ConceptAnimation },
	Topple { config: ToppleConfig, animation: ConceptAnimation },
	Kispar { config: KisparConfig, animation: ConceptAnimation },
	Tapp { config: TappConfig, animation: ConceptAnimation },
	Kaller { config: KallerConfig, animation: ConceptAnimation },
	Kappler { config: KapplerConfig, animation: ConceptAnimation },
	Wumbus { config: WumbusConfig, animation: ConceptAnimation },
	Lero { config: LeroConfig, animation: ConceptAnimation },
	Spibmom { config: SpibmomConfig, animation: ConceptAnimation },
	Grener { config: GrenerConfig, animation: ConceptAnimation },
	Thumplus { config: ThumplusConfig, animation: ConceptAnimation },
	Mistler { config: MistlerConfig, animation: ConceptAnimation },
	Tuberwaber { config: TuberwaberConfig, animation: ConceptAnimation },
}

impl Default for ConceptPreviewConfig {
	fn default() -> Self {
		Self::default_for(ConceptSpecies::Braidman)
	}
}

impl ConceptPreviewConfig {
	pub fn default_for(species: ConceptSpecies) -> Self {
		match species {
			ConceptSpecies::Braidman => Self::braidman(BraidmanConfig::default_preview()),
			ConceptSpecies::Brenal => Self::brenal(BrenalConfig::default_preview()),
			ConceptSpecies::Caole => Self::caole(CaoleConfig::default_preview()),
			ConceptSpecies::Epiphant => Self::epiphant(EpiphantConfig::default_preview()),
			ConceptSpecies::Hars => Self::hars(HarsConfig::default_preview()),
			ConceptSpecies::Yilter => Self::ylter(YilterConfig::default_preview()),
			ConceptSpecies::Sonyak => Self::sonyak(SonyakConfig::default_preview()),
			ConceptSpecies::Claber => Self::claber(ClaberConfig::default_preview()),
			ConceptSpecies::Croconot => Self::croconot(CroconotConfig::default_preview()),
			ConceptSpecies::Brodler => Self::brodler(BrodlerConfig::default_preview()),
			ConceptSpecies::Mygr => Self::mygr(MygrConfig::default_preview()),
			ConceptSpecies::Dui => Self::dui(DuiConfig::default_preview()),
			ConceptSpecies::Lidder => Self::lidder(LidderConfig::default_preview()),
			ConceptSpecies::Chupri => Self::chupri(ChupriConfig::default_preview()),
			ConceptSpecies::Brokker => Self::brokker(BrokkerConfig::default_preview()),
			ConceptSpecies::Tipple => Self::tipple(TippleConfig::default_preview()),
			ConceptSpecies::Topple => Self::topple(ToppleConfig::default_preview()),
			ConceptSpecies::Kispar => Self::kispar(KisparConfig::default_preview()),
			ConceptSpecies::Tapp => Self::tapp(TappConfig::default_preview()),
			ConceptSpecies::Kaller => Self::kaller(KallerConfig::default_preview()),
			ConceptSpecies::Kappler => Self::kappler(KapplerConfig::default_preview()),
			ConceptSpecies::Wumbus => Self::wumbus(WumbusConfig::default_preview()),
			ConceptSpecies::Lero => Self::lero(LeroConfig::default_preview()),
			ConceptSpecies::Spibmom => Self::spibmom(SpibmomConfig::default_preview()),
			ConceptSpecies::Grener => Self::grener(GrenerConfig::default_preview()),
			ConceptSpecies::Thumplus => Self::thumplus(ThumplusConfig::default_preview()),
			ConceptSpecies::Mistler => Self::mistler(MistlerConfig::default_preview()),
			ConceptSpecies::Tuberwaber => Self::tuberwaber(TuberwaberConfig::default_preview()),
		}
	}

	pub fn species(&self) -> ConceptSpecies {
		match self {
			Self::Braidman { .. } => ConceptSpecies::Braidman,
			Self::Brenal { .. } => ConceptSpecies::Brenal,
			Self::Caole { .. } => ConceptSpecies::Caole,
			Self::Epiphant { .. } => ConceptSpecies::Epiphant,
			Self::Hars { .. } => ConceptSpecies::Hars,
			Self::Yilter { .. } => ConceptSpecies::Yilter,
			Self::Sonyak { .. } => ConceptSpecies::Sonyak,
			Self::Claber { .. } => ConceptSpecies::Claber,
			Self::Croconot { .. } => ConceptSpecies::Croconot,
			Self::Brodler { .. } => ConceptSpecies::Brodler,
			Self::Mygr { .. } => ConceptSpecies::Mygr,
			Self::Dui { .. } => ConceptSpecies::Dui,
			Self::Lidder { .. } => ConceptSpecies::Lidder,
			Self::Chupri { .. } => ConceptSpecies::Chupri,
			Self::Brokker { .. } => ConceptSpecies::Brokker,
			Self::Tipple { .. } => ConceptSpecies::Tipple,
			Self::Topple { .. } => ConceptSpecies::Topple,
			Self::Kispar { .. } => ConceptSpecies::Kispar,
			Self::Tapp { .. } => ConceptSpecies::Tapp,
			Self::Kaller { .. } => ConceptSpecies::Kaller,
			Self::Kappler { .. } => ConceptSpecies::Kappler,
			Self::Wumbus { .. } => ConceptSpecies::Wumbus,
			Self::Lero { .. } => ConceptSpecies::Lero,
			Self::Spibmom { .. } => ConceptSpecies::Spibmom,
			Self::Grener { .. } => ConceptSpecies::Grener,
			Self::Thumplus { .. } => ConceptSpecies::Thumplus,
			Self::Mistler { .. } => ConceptSpecies::Mistler,
			Self::Tuberwaber { .. } => ConceptSpecies::Tuberwaber,
		}
	}

	pub fn braidman(config: BraidmanConfig) -> Self {
		Self::Braidman { config, animation: ConceptAnimation::default() }
	}

	pub fn braidman_with_animation(config: BraidmanConfig, animation: ConceptAnimation) -> Self {
		Self::Braidman { config, animation }
	}

	pub fn brenal(config: BrenalConfig) -> Self {
		Self::Brenal { config, animation: ConceptAnimation::default() }
	}

	pub fn brenal_with_animation(config: BrenalConfig, animation: ConceptAnimation) -> Self {
		Self::Brenal { config, animation }
	}

	pub fn caole(config: CaoleConfig) -> Self {
		Self::Caole { config, animation: ConceptAnimation::default() }
	}

	pub fn caole_with_animation(config: CaoleConfig, animation: ConceptAnimation) -> Self {
		Self::Caole { config, animation }
	}

	pub fn epiphant(config: EpiphantConfig) -> Self {
		Self::Epiphant { config, animation: ConceptAnimation::default() }
	}

	pub fn epiphant_with_animation(config: EpiphantConfig, animation: ConceptAnimation) -> Self {
		Self::Epiphant { config, animation }
	}

	pub fn hars(config: HarsConfig) -> Self {
		Self::Hars { config, animation: ConceptAnimation::default() }
	}

	pub fn hars_with_animation(config: HarsConfig, animation: ConceptAnimation) -> Self {
		Self::Hars { config, animation }
	}

	pub fn ylter(config: YilterConfig) -> Self {
		Self::Yilter { config, animation: ConceptAnimation::default() }
	}

	pub fn ylter_with_animation(config: YilterConfig, animation: ConceptAnimation) -> Self {
		Self::Yilter { config, animation }
	}

	pub fn sonyak(config: SonyakConfig) -> Self {
		Self::Sonyak { config, animation: ConceptAnimation::default() }
	}

	pub fn sonyak_with_animation(config: SonyakConfig, animation: ConceptAnimation) -> Self {
		Self::Sonyak { config, animation }
	}

	pub fn croconot(config: CroconotConfig) -> Self {
		Self::Croconot { config, animation: ConceptAnimation::default() }
	}

	pub fn croconot_with_animation(config: CroconotConfig, animation: ConceptAnimation) -> Self {
		Self::Croconot { config, animation }
	}

	pub fn claber(config: ClaberConfig) -> Self {
		Self::Claber { config, animation: ConceptAnimation::default() }
	}

	pub fn claber_with_animation(config: ClaberConfig, animation: ConceptAnimation) -> Self {
		Self::Claber { config, animation }
	}

	pub fn brodler(config: BrodlerConfig) -> Self {
		Self::Brodler { config, animation: ConceptAnimation::default() }
	}

	pub fn brodler_with_animation(config: BrodlerConfig, animation: ConceptAnimation) -> Self {
		Self::Brodler { config, animation }
	}

	pub fn mygr(config: MygrConfig) -> Self {
		Self::Mygr { config, animation: ConceptAnimation::default() }
	}

	pub fn mygr_with_animation(config: MygrConfig, animation: ConceptAnimation) -> Self {
		Self::Mygr { config, animation }
	}

	pub fn dui(config: DuiConfig) -> Self {
		Self::Dui { config, animation: ConceptAnimation::default() }
	}

	pub fn dui_with_animation(config: DuiConfig, animation: ConceptAnimation) -> Self {
		Self::Dui { config, animation }
	}

	pub fn lidder(config: LidderConfig) -> Self {
		Self::Lidder { config, animation: ConceptAnimation::default() }
	}

	pub fn lidder_with_animation(config: LidderConfig, animation: ConceptAnimation) -> Self {
		Self::Lidder { config, animation }
	}

	pub fn chupri(config: ChupriConfig) -> Self {
		Self::Chupri { config, animation: ConceptAnimation::default() }
	}

	pub fn chupri_with_animation(config: ChupriConfig, animation: ConceptAnimation) -> Self {
		Self::Chupri { config, animation }
	}

	pub fn brokker(config: BrokkerConfig) -> Self {
		Self::Brokker { config, animation: ConceptAnimation::default() }
	}

	pub fn brokker_with_animation(config: BrokkerConfig, animation: ConceptAnimation) -> Self {
		Self::Brokker { config, animation }
	}

	pub fn tipple(config: TippleConfig) -> Self {
		Self::Tipple { config, animation: ConceptAnimation::default() }
	}

	pub fn tipple_with_animation(config: TippleConfig, animation: ConceptAnimation) -> Self {
		Self::Tipple { config, animation }
	}

	pub fn topple(config: ToppleConfig) -> Self {
		Self::Topple { config, animation: ConceptAnimation::default() }
	}

	pub fn topple_with_animation(config: ToppleConfig, animation: ConceptAnimation) -> Self {
		Self::Topple { config, animation }
	}

	pub fn kispar(config: KisparConfig) -> Self {
		Self::Kispar { config, animation: ConceptAnimation::default() }
	}

	pub fn kispar_with_animation(config: KisparConfig, animation: ConceptAnimation) -> Self {
		Self::Kispar { config, animation }
	}

	pub fn tapp(config: TappConfig) -> Self {
		Self::Tapp { config, animation: ConceptAnimation::default() }
	}

	pub fn tapp_with_animation(config: TappConfig, animation: ConceptAnimation) -> Self {
		Self::Tapp { config, animation }
	}

	pub fn kaller(config: KallerConfig) -> Self {
		Self::Kaller { config, animation: ConceptAnimation::default() }
	}

	pub fn kaller_with_animation(config: KallerConfig, animation: ConceptAnimation) -> Self {
		Self::Kaller { config, animation }
	}

	pub fn kappler(config: KapplerConfig) -> Self {
		Self::Kappler { config, animation: ConceptAnimation::default() }
	}

	pub fn kappler_with_animation(config: KapplerConfig, animation: ConceptAnimation) -> Self {
		Self::Kappler { config, animation }
	}

	pub fn wumbus(config: WumbusConfig) -> Self {
		Self::Wumbus { config, animation: ConceptAnimation::default() }
	}

	pub fn wumbus_with_animation(config: WumbusConfig, animation: ConceptAnimation) -> Self {
		Self::Wumbus { config, animation }
	}

	pub fn lero(config: LeroConfig) -> Self {
		Self::Lero { config, animation: ConceptAnimation::default() }
	}

	pub fn lero_with_animation(config: LeroConfig, animation: ConceptAnimation) -> Self {
		Self::Lero { config, animation }
	}

	pub fn spibmom(config: SpibmomConfig) -> Self {
		Self::Spibmom { config, animation: ConceptAnimation::default() }
	}

	pub fn spibmom_with_animation(config: SpibmomConfig, animation: ConceptAnimation) -> Self {
		Self::Spibmom { config, animation }
	}

	pub fn grener(config: GrenerConfig) -> Self {
		Self::Grener { config, animation: ConceptAnimation::default() }
	}

	pub fn grener_with_animation(config: GrenerConfig, animation: ConceptAnimation) -> Self {
		Self::Grener { config, animation }
	}

	pub fn thumplus(config: ThumplusConfig) -> Self {
		Self::Thumplus { config, animation: ConceptAnimation::default() }
	}

	pub fn thumplus_with_animation(config: ThumplusConfig, animation: ConceptAnimation) -> Self {
		Self::Thumplus { config, animation }
	}

	pub fn mistler(config: MistlerConfig) -> Self {
		Self::Mistler { config, animation: ConceptAnimation::default() }
	}

	pub fn mistler_with_animation(config: MistlerConfig, animation: ConceptAnimation) -> Self {
		Self::Mistler { config, animation }
	}

	pub fn tuberwaber(config: TuberwaberConfig) -> Self {
		Self::Tuberwaber { config, animation: ConceptAnimation::default() }
	}

	pub fn tuberwaber_with_animation(
		config: TuberwaberConfig,
		animation: ConceptAnimation,
	) -> Self {
		Self::Tuberwaber { config, animation }
	}

	/// High-band armature nodes from the LodScene recipe (body / neck / head).
	pub fn lod_rig_nodes(&self) -> Vec<RigNode> {
		with_clothed_recipe!(self, recipe => {
			recipe.rig_nodes_for_level(LodSceneLevel::High).flatten()
		})
	}

	/// High-band part nodes from the LodScene recipe (meshes + clothing).
	pub fn lod_part_nodes(&self) -> Vec<PartNode> {
		with_clothed_recipe!(self, recipe => {
			recipe.part_nodes_for_level(LodSceneLevel::High).flatten()
		})
	}

	pub fn status_label(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Brenal { config, .. } => config.status_label(),
			Self::Caole { config, .. } => config.status_label(),
			Self::Epiphant { config, .. } => config.status_label(),
			Self::Hars { config, .. } => config.status_label(),
			Self::Yilter { config, .. } => config.status_label(),
			Self::Sonyak { config, .. } => config.status_label(),
			Self::Claber { config, .. } => config.status_label(),
			Self::Croconot { config, .. } => config.status_label(),
			Self::Brodler { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Mygr { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Dui { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Lidder { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Chupri { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Brokker { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Tipple { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Topple { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Kispar { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Tapp { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Kaller { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Kappler { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Wumbus { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Lero { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Spibmom { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Grener { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Thumplus { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Mistler { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
			Self::Tuberwaber { config, animation } => {
				format!("{} animation={}", config.status_label(), animation.label())
			}
		}
	}

	pub fn sync_key(&self) -> String {
		match self {
			Self::Braidman { config, animation } => {
				format!("species=braidman {} animation={animation:?}", config.sync_key())
			}
			Self::Brenal { config, .. } => format!("species=brenal {}", config.sync_key()),
			Self::Caole { config, .. } => format!("species=caole {}", config.sync_key()),
			Self::Epiphant { config, .. } => format!("species=epiphant {}", config.sync_key()),
			Self::Hars { config, .. } => format!("species=hars {}", config.sync_key()),
			Self::Yilter { config, .. } => format!("species=ylter {}", config.sync_key()),
			Self::Sonyak { config, .. } => format!("species=sonyak {}", config.sync_key()),
			Self::Claber { config, .. } => format!("species=claber {}", config.sync_key()),
			Self::Croconot { config, .. } => format!("species=croconot {}", config.sync_key()),
			Self::Brodler { config, animation } => {
				format!("species=brodler {} animation={animation:?}", config.sync_key())
			}
			Self::Mygr { config, animation } => {
				format!("species=mygr {} animation={animation:?}", config.sync_key())
			}
			Self::Dui { config, animation } => {
				format!("species=dui {} animation={animation:?}", config.sync_key())
			}
			Self::Lidder { config, animation } => {
				format!("species=lidder {} animation={animation:?}", config.sync_key())
			}
			Self::Chupri { config, animation } => {
				format!("species=chupri {} animation={animation:?}", config.sync_key())
			}
			Self::Brokker { config, animation } => {
				format!("species=brokker {} animation={animation:?}", config.sync_key())
			}
			Self::Tipple { config, animation } => {
				format!("species=tipple {} animation={animation:?}", config.sync_key())
			}
			Self::Topple { config, animation } => {
				format!("species=topple {} animation={animation:?}", config.sync_key())
			}
			Self::Kispar { config, animation } => {
				format!("species=kispar {} animation={animation:?}", config.sync_key())
			}
			Self::Tapp { config, animation } => {
				format!("species=tapp {} animation={animation:?}", config.sync_key())
			}
			Self::Kaller { config, animation } => {
				format!("species=kaller {} animation={animation:?}", config.sync_key())
			}
			Self::Kappler { config, animation } => {
				format!("species=kappler {} animation={animation:?}", config.sync_key())
			}
			Self::Wumbus { config, animation } => {
				format!("species=wumbus {} animation={animation:?}", config.sync_key())
			}
			Self::Lero { config, animation } => {
				format!("species=lero {} animation={animation:?}", config.sync_key())
			}
			Self::Spibmom { config, animation } => {
				format!("species=spibmom {} animation={animation:?}", config.sync_key())
			}
			Self::Grener { config, animation } => {
				format!("species=grener {} animation={animation:?}", config.sync_key())
			}
			Self::Thumplus { config, animation } => {
				format!("species=thumplus {} animation={animation:?}", config.sync_key())
			}
			Self::Mistler { config, animation } => {
				format!("species=mistler {} animation={animation:?}", config.sync_key())
			}
			Self::Tuberwaber { config, animation } => {
				format!("species=tuberwaber {} animation={animation:?}", config.sync_key())
			}
		}
	}

	pub fn spawn_key(&self) -> String {
		match self {
			Self::Braidman { config, .. } => format!(
				"species=braidman body={:?} head={:?} eye={:?} nose={:?} mouth={:?} ear={:?} hair={:?} clothing={:?}",
				config.body,
				config.head,
				config.eye,
				config.nose,
				config.mouth,
				config.ear,
				config.hair,
				config.clothing,
			),
			Self::Brenal { config, .. } => format!(
				"species=brenal horns={:?} eye={:?}",
				config.horns,
				config.eye,
			),
			Self::Caole { config, .. } => format!(
				"species=caole body={:?} mouth={:?} eye={:?}",
				config.body,
				config.mouth,
				config.eye,
			),
			Self::Epiphant { config, .. } => format!(
				"species=epiphant body={:?} nose={:?} eye={:?}",
				config.body,
				config.nose,
				config.eye,
			),
			Self::Hars { config, .. } => format!(
				"species=hars mouth={:?} eye={:?}",
				config.mouth,
				config.eye,
			),
			Self::Yilter { config, .. } => format!(
				"species=ylter mouth={:?}",
				config.mouth,
			),
			Self::Sonyak { config, .. } => format!(
				"species=sonyak mouth={:?}",
				config.mouth,
			),
			Self::Claber { config, .. } => format!(
				"species=claber horns={:?} eye={:?}",
				config.horns,
				config.eye,
			),
			Self::Croconot { config, .. } => format!(
				"species=croconot horns={:?} eye={:?}",
				config.horns,
				config.eye,
			),
			Self::Brodler { config, .. } => format!(
				"species=brodler head={:?} horns={:?} eye={:?} nose={:?} mouth={:?} ear={:?} hair={:?} clothing={:?}",
				config.head,
				config.horns,
				config.eye,
				config.nose,
				config.mouth,
				config.ear,
				config.hair,
				config.clothing,
			),
			Self::Mygr { config, .. } => format!(
				"species=mygr eye={:?} hair={:?} clothing={:?}",
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Dui { config, .. } => format!(
				"species=dui nose={:?} hair={:?} clothing={:?}",
				config.nose,
				config.hair,
				config.clothing,
			),
			Self::Lidder { config, .. } => format!(
				"species=lidder beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Chupri { config, .. } => format!(
				"species=chupri beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Brokker { config, .. } => format!(
				"species=brokker eye={:?} hair={:?} clothing={:?}",
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Tipple { config, .. } => format!(
				"species=tipple beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Topple { config, .. } => format!(
				"species=topple beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Kispar { config, .. } => format!(
				"species=kispar beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Tapp { config, .. } => format!(
				"species=tapp beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Kaller { config, .. } => format!(
				"species=kaller eye={:?} hair={:?} clothing={:?}",
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Kappler { config, .. } => format!(
				"species=kappler beak={:?} eye={:?} hair={:?} clothing={:?}",
				config.beak,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Wumbus { config, .. } => format!(
				"species=wumbus horns={:?} eye={:?} hair={:?} clothing={:?}",
				config.horns,
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Lero { config, .. } => format!(
				"species=lero mouth={:?} hair={:?} clothing={:?}",
				config.mouth,
				config.hair,
				config.clothing,
			),
			Self::Spibmom { config, .. } => format!(
				"species=spibmom eye={:?} hair={:?} clothing={:?}",
				config.eye,
				config.hair,
				config.clothing,
			),
			Self::Grener { config, .. } => format!(
				"species=grener body={:?}",
				config.colors.body,
			),
			Self::Thumplus { config, .. } => format!(
				"species=thumplus body={:?}",
				config.colors.body,
			),
			Self::Mistler { config, .. } => format!(
				"species=mistler body={:?}",
				config.colors.body,
			),
			Self::Tuberwaber { config, .. } => format!(
				"species=tuberwaber body={:?} head={:?} eye={:?} nose={:?} mouth={:?} hair={:?} clothing={:?}",
				config.body,
				config.head,
				config.eye,
				config.nose,
				config.mouth,
				config.hair,
				config.clothing,
			),
		}
	}

	pub const fn animation(&self) -> ConceptAnimation {
		match self {
			Self::Braidman { animation, .. }
			| Self::Brenal { animation, .. }
			| Self::Caole { animation, .. }
			| Self::Epiphant { animation, .. }
			| Self::Hars { animation, .. }
			| Self::Yilter { animation, .. }
			| Self::Sonyak { animation, .. }
			| Self::Claber { animation, .. }
			| Self::Croconot { animation, .. }
			| Self::Brodler { animation, .. }
			| Self::Mygr { animation, .. }
			| Self::Dui { animation, .. }
			| Self::Lidder { animation, .. }
			| Self::Chupri { animation, .. }
			| Self::Brokker { animation, .. }
			| Self::Tipple { animation, .. }
			| Self::Topple { animation, .. }
			| Self::Kispar { animation, .. }
			| Self::Tapp { animation, .. }
			| Self::Kaller { animation, .. }
			| Self::Kappler { animation, .. }
			| Self::Wumbus { animation, .. }
			| Self::Lero { animation, .. }
			| Self::Spibmom { animation, .. }
			| Self::Grener { animation, .. }
			| Self::Thumplus { animation, .. }
			| Self::Mistler { animation, .. }
			| Self::Tuberwaber { animation, .. } => *animation,
		}
	}
}

#[derive(Resource, Default)]
pub struct ConceptPreviewSyncState {
	live_key: String,
	spawn_key: String,
}

impl ConceptPreviewSyncState {
	/// Drop cached pose/config so the next sync can live-update or respawn parts.
	pub(crate) fn invalidate_live(&mut self) {
		self.live_key.clear();
	}

	/// Force a full preview respawn (species switch).
	pub(crate) fn invalidate(&mut self) {
		self.live_key.clear();
		self.spawn_key.clear();
	}
}

/// Skips part attachment/remap for one frame after a GLTF respawn so queued
/// despawn commands are not racing inserts on the outgoing entities.
#[derive(Resource, Default)]
pub struct PreviewRespawnCooldown {
	pub frames_remaining: u8,
}

pub fn tick_preview_respawn_cooldown(mut cooldown: ResMut<PreviewRespawnCooldown>) {
	if cooldown.frames_remaining > 0 {
		cooldown.frames_remaining -= 1;
	}
}

pub fn preview_pass_ready(cooldown: Res<PreviewRespawnCooldown>) -> bool {
	cooldown.frames_remaining == 0
}

#[derive(Component)]
pub struct ConceptPreviewRoot;

/// Spawned hidden until the body rig bone map and bind scales are ready.
#[derive(Component)]
pub struct PreviewAwaitingReveal;

#[derive(Component, Clone, Copy)]
pub struct PreviewPartBaseTransform {
	normalization: Transform,
	socket: Option<Transform>,
}

#[derive(Component, Clone, Copy)]
pub struct PreviewAssetTarget {
	pub target: PreviewTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviewTarget {
	BraidmanBody(BodyMesh),
	BraidmanHead(HeadMesh),
	BraidmanEye(EyeMesh),
	BraidmanNose(NoseMesh),
	BraidmanMouth(MouthMesh),
	BraidmanEar(EarMesh),
	BraidmanHair(HairMesh),
	BraidmanClothing(ClothingMesh),
	BrenalBody,
	BrenalHead,
	BrenalHorns(characters::species::brenal::BrenalHornMesh),
	BrenalEye(EyeMesh),
	BrenalEar,
	BrenalMouth,
	BrenalTail,
	CaoleBody,
	CaoleHead,
	CaoleEye(EyeMesh),
	CaoleEar,
	CaoleMouth,
	CaoleTail,
	EpiphantBody,
	EpiphantHead,
	EpiphantEye(EyeMesh),
	EpiphantEar,
	EpiphantNose,
	EpiphantTail,
	HarsBody,
	HarsHead,
	HarsEye(EyeMesh),
	HarsEar,
	HarsMouth,
	HarsTail,
	YilterBody,
	YilterHead,
	YilterEye,
	YilterNeck,
	YilterMouth,
	YilterTail,
	SonyakBody,
	SonyakHead,
	SonyakEye,
	SonyakHair,
	SonyakMouth,
	SonyakTail,
	ClaberBody,
	ClaberHead,
	ClaberHorns(characters::species::claber::ClaberHornMesh),
	ClaberEye(EyeMesh),
	ClaberEar,
	ClaberMouth,
	ClaberTail,
	CroconotBody,
	CroconotHead,
	CroconotHorns(characters::species::croconot::CroconotHornMesh),
	CroconotEye(EyeMesh),
	CroconotEar,
	CroconotMouth,
	CroconotTail,
	BrodlerBody,
	BrodlerHead(BrodlerHeadMesh),
	BrodlerHorns(HornMesh),
	BrodlerEye(EyeMesh),
	BrodlerNose(NoseMesh),
	BrodlerMouth(MouthMesh),
	BrodlerEar(EarMesh),
	BrodlerHair(HairMesh),
	BrodlerClothing(ClothingMesh),
	MygrBody,
	MygrHead,
	MygrEye(EyeMesh),
	MygrMouth,
	MygrEar,
	MygrTail,
	MygrHair(HairMesh),
	MygrClothing(ClothingMesh),
	DuiBody,
	DuiHead,
	DuiEye,
	DuiNose(DuiNoseMesh),
	DuiMouth,
	DuiHair(HairMesh),
	DuiClothing(ClothingMesh),
	LidderBody,
	LidderHead,
	LidderEye,
	LidderBeak(LidderBeakMesh),
	LidderHair(HairMesh),
	LidderClothing(ClothingMesh),
	ChupriBody,
	ChupriHead,
	ChupriEye,
	ChupriBeak(ChupriBeakMesh),
	ChupriHair(HairMesh),
	ChupriClothing(ClothingMesh),
	BrokkerBody,
	BrokkerHead,
	BrokkerEye,
	BrokkerSnout(BrokkerSnoutMesh),
	BrokkerHair(HairMesh),
	BrokkerClothing(ClothingMesh),
	TippleBody,
	TippleHead,
	TippleEye,
	TippleBeak(TippleBeakMesh),
	TippleHair(HairMesh),
	TippleClothing(ClothingMesh),
	ToppleBody,
	ToppleHead,
	ToppleEye,
	ToppleBeak(ToppleBeakMesh),
	ToppleHair(HairMesh),
	ToppleClothing(ClothingMesh),
	KisparBody,
	KisparHead,
	KisparEye,
	KisparBeak(KisparBeakMesh),
	KisparHair(HairMesh),
	KisparClothing(ClothingMesh),
	TappBody,
	TappHead,
	TappEye,
	TappBeak(TappBeakMesh),
	TappHair(HairMesh),
	TappClothing(ClothingMesh),
	KallerBody,
	KallerHead,
	KallerEye,
	KallerSnout(KallerSnoutMesh),
	KallerCrown,
	KallerHair(HairMesh),
	KallerClothing(ClothingMesh),
	KapplerBody,
	KapplerHead,
	KapplerEye,
	KapplerBeak(KapplerBeakMesh),
	KapplerHair(HairMesh),
	KapplerClothing(ClothingMesh),
	WumbusBody,
	WumbusHead,
	WumbusHorns(WumbusHornMesh),
	WumbusSpine,
	WumbusEye(EyeMesh),
	WumbusMouth,
	WumbusEar,
	WumbusHair(HairMesh),
	WumbusClothing(ClothingMesh),
	LeroBody,
	LeroHead,
	LeroEye,
	LeroMouth(LeroMouthMesh),
	LeroTail,
	LeroSpine,
	LeroHair(HairMesh),
	LeroClothing(ClothingMesh),
	SpibmomBody,
	SpibmomHead,
	SpibmomHorns,
	SpibmomSpine,
	SpibmomEye(EyeMesh),
	SpibmomMouth,
	SpibmomEar,
	SpibmomHair(HairMesh),
	SpibmomClothing(ClothingMesh),
	GrenerBody,
	ThumplusBody,
	MistlerBody,
	TuberwaberBody(TuberwaberBodyMesh),
	TuberwaberHead(TuberwaberHeadMesh),
	TuberwaberEye(EyeMesh),
	TuberwaberNose(NoseMesh),
	TuberwaberMouth(MouthMesh),
	TuberwaberHorns,
	TuberwaberHair(HairMesh),
	TuberwaberClothing(ClothingMesh),
}

pub fn sync_preview(
	mut commands: Commands,
	config: Res<ConceptPreviewConfig>,
	mut sync_state: ResMut<ConceptPreviewSyncState>,
	mut respawn_cooldown: ResMut<PreviewRespawnCooldown>,
	roots: Query<(Entity, Option<&CharacterMembers>), With<ConceptPreviewRoot>>,
	mut poses: Query<(&mut ActiveRigPose, &CharacterRig)>,
	anims: Query<(Entity, &AnimRefRoot)>,
	mut parts: Query<(
		Entity,
		&PartNode,
		&CharacterPart,
		Option<&MaterialRefRoot>,
		Option<&PreviewPartBaseTransform>,
		Option<&mut Transform>,
	)>,
) {
	let live_key = config.sync_key();
	let spawn_key = config.spawn_key();
	if sync_state.live_key == live_key {
		return;
	}

	if sync_state.spawn_key == spawn_key {
		sync_state.live_key = live_key;
		sync_live_preview(&mut commands, &config, &roots, &mut poses, &anims, &mut parts);
		return;
	}

	sync_state.live_key = live_key;
	sync_state.spawn_key.clone_from(&spawn_key);
	respawn_cooldown.frames_remaining = 1;

	for (entity, _) in &roots {
		commands.entity(entity).try_despawn();
	}

	spawn_lod_character_preview(&mut commands, &config);
}

fn spawn_lod_character_preview(commands: &mut Commands, config: &ConceptPreviewConfig) {
	with_clothed_recipe!(config, recipe => {
		spawn_clothed_character(commands, &recipe);
	});
}

fn spawn_clothed_character<T>(commands: &mut Commands, character: &T)
where
	T: CharacterComponents + Clone + Default + Unpin + Send + Sync + 'static,
{
	let bounds = character_bounds(character);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(character.clone());
	let entity = commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				Transform::IDENTITY
			},
		))
		.id();
	commands
		.entity(entity)
		.insert((ConceptPreviewRoot, PreviewAwaitingReveal, Visibility::Hidden));
}

/// Stamp preview-only UI markers onto nested hosts after membership, and write
/// the session clip onto the body member (`AnimRefRoot` defaults to still on host).
pub fn stamp_lod_character_preview(
	mut commands: Commands,
	config: Res<ConceptPreviewConfig>,
	roots: Query<&CharacterMembers, With<ConceptPreviewRoot>>,
	parts: Query<&PartNode, Without<PreviewAssetTarget>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	let desired_anim = AnimRef::from(config.animation());
	for members in &roots {
		for member in members.iter() {
			if let Ok(node) = parts.get(member) {
				commands.entity(member).insert((
					preview_asset_target(&config, node.slot, node.label),
					PreviewPartBaseTransform {
						normalization: node.normalization.transform(),
						socket: node.socket.map(|socket| socket.local),
					},
				));
			}
			if rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				let needs_clip = match anims.get(member) {
					Ok(root) => root.0 != desired_anim,
					Err(_) => true,
				};
				if needs_clip {
					commands.entity(member).insert(AnimRefRoot(desired_anim));
				}
			}
		}
	}
}

fn sync_live_preview(
	commands: &mut Commands,
	config: &ConceptPreviewConfig,
	roots: &Query<(Entity, Option<&CharacterMembers>), With<ConceptPreviewRoot>>,
	poses: &mut Query<(&mut ActiveRigPose, &CharacterRig)>,
	anims: &Query<(Entity, &AnimRefRoot)>,
	parts: &mut Query<(
		Entity,
		&PartNode,
		&CharacterPart,
		Option<&MaterialRefRoot>,
		Option<&PreviewPartBaseTransform>,
		Option<&mut Transform>,
	)>,
) {
	let rig_nodes = config.lod_rig_nodes();
	let body_pose = rig_nodes
		.iter()
		.find(|node| node.id == RigId::Body)
		.map(|node| node.pose.clone());
	let neck_pose = rig_nodes
		.iter()
		.find(|node| node.id == RigId::Neck)
		.map(|node| node.pose.clone());
	let desired_anim = AnimRef::from(config.animation());
	let recipe_parts = config.lod_part_nodes();

	for (_, members) in roots {
		let Some(members) = members else {
			continue;
		};
		for member in members.iter() {
			if let Ok((mut pose, rig)) = poses.get_mut(member) {
				match rig.role {
					CharacterRigRole::Body => {
						if let Some(body_pose) = &body_pose {
							pose.pose = body_pose.clone();
						}
					}
					CharacterRigRole::Neck => {
						if let Some(neck_pose) = &neck_pose {
							pose.pose = neck_pose.clone();
						}
					}
					CharacterRigRole::Head => {}
				}
			}
			if let Ok((_, root)) = anims.get(member) {
				if root.0 != desired_anim {
					commands.entity(member).insert(AnimRefRoot(desired_anim));
				}
			}
			if let Ok((entity, node, part, current_material, base, transform)) =
				parts.get_mut(member)
			{
				if let Some(recipe) = recipe_part(&recipe_parts, node) {
					let needs_paint = current_material.is_none_or(|root| root.0 != recipe.material);
					if needs_paint {
						commands.entity(entity).insert(MaterialRefRoot(recipe.material.clone()));
					}
				}
				if !has_feature_transform(part.slot) {
					continue;
				}
				let Some(recipe) = recipe_part(&recipe_parts, node) else {
					continue;
				};
				let Some(base) = base else {
					continue;
				};
				let Some(mut transform) = transform else {
					continue;
				};
				let authored = base.normalization.mul_transform(recipe.feature);
				match base.socket {
					Some(socket) => {
						*transform = socket;
						transform.scale *= authored.scale;
						transform.rotation *= authored.rotation;
					}
					None => *transform = authored,
				}
			}
		}
	}
}

fn recipe_part<'a>(parts: &'a [PartNode], node: &PartNode) -> Option<&'a PartNode> {
	parts
		.iter()
		.find(|part| part.slot == node.slot && part.label == node.label)
		.or_else(|| parts.iter().find(|part| part.slot == node.slot))
}

#[derive(Resource, Default)]
pub struct PreviewRevealDebugState {
	spawn_key: String,
	logged_block: bool,
}

/// Reveal a respawned preview only after the body pose, socket attach, and skin
/// remap passes have settled.
pub fn reveal_ready_preview(
	mut commands: Commands,
	mut debug: ResMut<PreviewRevealDebugState>,
	config: Res<ConceptPreviewConfig>,
	pending: Query<Entity, With<PreviewAwaitingReveal>>,
	body_rigs: Query<(&BoneMap, &RigBindScales, &CharacterRig), With<AnimRefRoot>>,
	awaiting_lod_socket: Query<
		(),
		(Or<(With<RigNode>, With<PartNode>)>, With<SocketRefRoot>, Without<SocketRefApplied>),
	>,
	awaiting_lod_skin: Query<(), (With<SkinRefRoot>, Without<SkinRefApplied>)>,
	awaiting_remap: Query<
		(),
		(With<NeedsSkinRemap>, With<CharacterPart>, Without<NoMatchingArmature>),
	>,
	awaiting_prune: Query<(), (With<NeedsDuplicateScenePrune>, With<CharacterPart>)>,
) {
	let spawn_key = config.spawn_key();
	if debug.spawn_key != spawn_key {
		debug.spawn_key.clone_from(&spawn_key);
		debug.logged_block = false;
		if preview_debug_enabled() {
			info!(
				"[preview] awaiting reveal for species={:?} spawn_key={spawn_key}",
				config.species()
			);
		}
	}

	let Ok((bone_map, bind_scales, rig)) = body_rigs.single() else {
		if !debug.logged_block {
			debug.logged_block = true;
			warn!("[preview] reveal blocked: no animated body rig entity");
		}
		return;
	};
	if !bone_map_ready(bone_map, rig.skeleton)
		|| !bind_scales_ready(bind_scales, bone_map, rig.skeleton)
	{
		if bone_map.by_name.is_empty() {
			// GLTF scene bones are not wired yet; wait without treating it as an error.
			return;
		}
		if !debug.logged_block {
			debug.logged_block = true;
			let missing = missing_landmark_bones(bone_map, rig.skeleton);
			warn!(
				"[preview] reveal blocked: rig not ready skeleton={:?} missing_landmarks=[{}] mapped_bones={}",
				rig.skeleton,
				missing.join(", "),
				bone_map.by_name.len()
			);
		}
		return;
	}
	if !awaiting_lod_socket.is_empty()
		|| !awaiting_lod_skin.is_empty()
		|| !awaiting_remap.is_empty()
		|| !awaiting_prune.is_empty()
	{
		if !debug.logged_block {
			debug.logged_block = true;
			warn!(
				"[preview] reveal blocked: lod_socket={} lod_skin={} awaiting_remap={} awaiting_prune={}",
				awaiting_lod_socket.iter().count(),
				awaiting_lod_skin.iter().count(),
				awaiting_remap.iter().count(),
				awaiting_prune.iter().count()
			);
		}
		return;
	}
	if pending.is_empty() {
		return;
	}
	if preview_debug_enabled() {
		info!(
			"[preview] revealing {} preview entities for species={:?}",
			pending.iter().count(),
			config.species()
		);
	}
	for entity in &pending {
		commands.entity(entity).try_insert(Visibility::Inherited);
		commands.entity(entity).try_remove::<PreviewAwaitingReveal>();
	}
}

fn has_feature_transform(slot: CharacterPartSlot) -> bool {
	matches!(
		slot,
		CharacterPartSlot::EyeLeft
			| CharacterPartSlot::EyeRight
			| CharacterPartSlot::Nose
			| CharacterPartSlot::Mouth
			| CharacterPartSlot::EarLeft
			| CharacterPartSlot::EarRight
	)
}

pub fn preview_asset_target(
	config: &ConceptPreviewConfig,
	slot: CharacterPartSlot,
	label: &str,
) -> PreviewAssetTarget {
	match config {
		ConceptPreviewConfig::Braidman { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::BraidmanBody(config.body),
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::BraidmanBody(config.body)
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::BraidmanHead(config.head)
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::BraidmanEye(config.eye)
				}
				CharacterPartSlot::Nose => PreviewTarget::BraidmanNose(config.nose),
				CharacterPartSlot::Mouth => PreviewTarget::BraidmanMouth(config.mouth),
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::BraidmanEar(config.ear)
				}
				CharacterPartSlot::Hair => PreviewTarget::BraidmanHair(config.hair),
				CharacterPartSlot::Horns => PreviewTarget::BraidmanHead(config.head),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::BraidmanClothing)
					.unwrap_or(PreviewTarget::BraidmanHead(config.head)),
				CharacterPartSlot::Tail => PreviewTarget::BraidmanBody(config.body),
				CharacterPartSlot::Spine => PreviewTarget::BraidmanBody(config.body),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Brenal { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::BrenalBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::BrenalBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::BrenalHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::BrenalEye(config.eye)
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::BrenalEar
				}
				CharacterPartSlot::Horns => PreviewTarget::BrenalHorns(config.horns),
				CharacterPartSlot::Mouth => PreviewTarget::BrenalMouth,
				CharacterPartSlot::Tail => PreviewTarget::BrenalTail,
				CharacterPartSlot::Nose
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine => PreviewTarget::BrenalHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Caole { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::CaoleBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::CaoleBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::CaoleHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::CaoleEye(config.eye)
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => PreviewTarget::CaoleEar,
				CharacterPartSlot::Mouth => PreviewTarget::CaoleMouth,
				CharacterPartSlot::Tail => PreviewTarget::CaoleTail,
				CharacterPartSlot::Nose
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine
				| CharacterPartSlot::Horns => PreviewTarget::CaoleHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Epiphant { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::EpiphantBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::EpiphantBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::EpiphantHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::EpiphantEye(config.eye)
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::EpiphantEar
				}
				CharacterPartSlot::Nose => PreviewTarget::EpiphantNose,
				CharacterPartSlot::Tail => PreviewTarget::EpiphantTail,
				CharacterPartSlot::Mouth
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine
				| CharacterPartSlot::Horns => PreviewTarget::EpiphantHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Hars { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::HarsBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => PreviewTarget::HarsBody,
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::HarsHead,
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::HarsEye(config.eye)
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => PreviewTarget::HarsEar,
				CharacterPartSlot::Mouth => PreviewTarget::HarsMouth,
				CharacterPartSlot::Tail => PreviewTarget::HarsTail,
				CharacterPartSlot::Nose
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine
				| CharacterPartSlot::Horns => PreviewTarget::HarsHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Yilter { .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::YilterBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::YilterNeck
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::YilterHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::YilterEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::YilterMouth,
				CharacterPartSlot::Tail => PreviewTarget::YilterTail,
				CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Nose
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine
				| CharacterPartSlot::Horns => PreviewTarget::YilterHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Sonyak { .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::SonyakBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::SonyakBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::SonyakHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::SonyakEye
				}
				CharacterPartSlot::Hair => PreviewTarget::SonyakHair,
				CharacterPartSlot::Mouth => PreviewTarget::SonyakMouth,
				CharacterPartSlot::Tail => PreviewTarget::SonyakTail,
				CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Nose
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine
				| CharacterPartSlot::Horns => PreviewTarget::SonyakHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Claber { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::ClaberBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::ClaberBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::ClaberHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::ClaberEye(config.eye)
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::ClaberEar
				}
				CharacterPartSlot::Horns => PreviewTarget::ClaberHorns(config.horns),
				CharacterPartSlot::Mouth => PreviewTarget::ClaberMouth,
				CharacterPartSlot::Tail => PreviewTarget::ClaberTail,
				CharacterPartSlot::Nose
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine => PreviewTarget::ClaberHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Croconot { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::CroconotBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::CroconotBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::CroconotHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::CroconotEye(config.eye)
				}
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::CroconotEar
				}
				CharacterPartSlot::Horns => PreviewTarget::CroconotHorns(config.horns),
				CharacterPartSlot::Mouth => PreviewTarget::CroconotMouth,
				CharacterPartSlot::Tail => PreviewTarget::CroconotTail,
				CharacterPartSlot::Nose
				| CharacterPartSlot::Hair
				| CharacterPartSlot::Clothing
				| CharacterPartSlot::Spine => PreviewTarget::CroconotHead,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Brodler { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::BrodlerBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::BrodlerBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::BrodlerHead(config.head)
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::BrodlerEye(config.eye)
				}
				CharacterPartSlot::Nose => PreviewTarget::BrodlerNose(config.nose),
				CharacterPartSlot::Mouth => PreviewTarget::BrodlerMouth(config.mouth),
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::BrodlerEar(config.ear)
				}
				CharacterPartSlot::Horns => PreviewTarget::BrodlerHorns(config.horns),
				CharacterPartSlot::Hair => PreviewTarget::BrodlerHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::BrodlerClothing)
					.unwrap_or(PreviewTarget::BrodlerHead(config.head)),
				CharacterPartSlot::Tail => PreviewTarget::BrodlerBody,
				CharacterPartSlot::Spine => PreviewTarget::BrodlerBody,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Mygr { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::MygrBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => PreviewTarget::MygrBody,
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::MygrHead,
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::MygrEye(config.eye)
				}
				CharacterPartSlot::Mouth => PreviewTarget::MygrMouth,
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => PreviewTarget::MygrEar,
				CharacterPartSlot::Tail => PreviewTarget::MygrTail,
				CharacterPartSlot::Nose | CharacterPartSlot::Horns => PreviewTarget::MygrHead,
				CharacterPartSlot::Hair => PreviewTarget::MygrHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::MygrClothing)
					.unwrap_or(PreviewTarget::MygrHead),
				CharacterPartSlot::Spine => PreviewTarget::MygrBody,
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Dui { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::DuiBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => PreviewTarget::DuiBody,
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::DuiHead,
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => PreviewTarget::DuiEye,
				CharacterPartSlot::Nose => PreviewTarget::DuiNose(config.nose),
				CharacterPartSlot::Mouth => PreviewTarget::DuiMouth,
				CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::DuiHead,
				CharacterPartSlot::Hair => PreviewTarget::DuiHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::DuiClothing)
					.unwrap_or(PreviewTarget::DuiHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Lidder { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::LidderBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::LidderBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::LidderHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::LidderEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::LidderBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::LidderHead,
				CharacterPartSlot::Hair => PreviewTarget::LidderHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::LidderClothing)
					.unwrap_or(PreviewTarget::LidderHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Chupri { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::ChupriBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::ChupriBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::ChupriHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::ChupriEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::ChupriBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::ChupriHead,
				CharacterPartSlot::Hair => PreviewTarget::ChupriHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::ChupriClothing)
					.unwrap_or(PreviewTarget::ChupriHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Brokker { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::BrokkerBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::BrokkerBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::BrokkerHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::BrokkerEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::BrokkerSnout(BrokkerSnoutMesh::Igny),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::BrokkerHead,
				CharacterPartSlot::Hair => PreviewTarget::BrokkerHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::BrokkerClothing)
					.unwrap_or(PreviewTarget::BrokkerHead),
			};
			PreviewAssetTarget { target }
		}

		ConceptPreviewConfig::Tipple { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::TippleBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::TippleBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::TippleHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::TippleEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::TippleBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::TippleHead,
				CharacterPartSlot::Hair => PreviewTarget::TippleHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::TippleClothing)
					.unwrap_or(PreviewTarget::TippleHead),
			};
			PreviewAssetTarget { target }
		}

		ConceptPreviewConfig::Topple { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::ToppleBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::ToppleBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::ToppleHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::ToppleEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::ToppleBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::ToppleHead,
				CharacterPartSlot::Hair => PreviewTarget::ToppleHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::ToppleClothing)
					.unwrap_or(PreviewTarget::ToppleHead),
			};
			PreviewAssetTarget { target }
		}

		ConceptPreviewConfig::Kispar { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::KisparBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::KisparBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::KisparHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::KisparEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::KisparBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::KisparHead,
				CharacterPartSlot::Hair => PreviewTarget::KisparHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::KisparClothing)
					.unwrap_or(PreviewTarget::KisparHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Tapp { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::TappBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => PreviewTarget::TappBody,
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::TappHead,
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => PreviewTarget::TappEye,
				CharacterPartSlot::Mouth => PreviewTarget::TappBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::TappHead,
				CharacterPartSlot::Hair => PreviewTarget::TappHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::TappClothing)
					.unwrap_or(PreviewTarget::TappHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Kaller { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::KallerBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::KallerBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::KallerHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::KallerEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::KallerSnout(KallerSnoutMesh::Robrek),
				CharacterPartSlot::Horns => PreviewTarget::KallerCrown,
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::KallerHead,
				CharacterPartSlot::Hair => PreviewTarget::KallerHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::KallerClothing)
					.unwrap_or(PreviewTarget::KallerHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Kappler { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::KapplerBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::KapplerBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::KapplerHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::KapplerEye
				}
				CharacterPartSlot::Mouth => PreviewTarget::KapplerBeak(config.beak),
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::KapplerHead,
				CharacterPartSlot::Hair => PreviewTarget::KapplerHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::KapplerClothing)
					.unwrap_or(PreviewTarget::KapplerHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Wumbus { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::WumbusBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::WumbusBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::WumbusHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::WumbusEye(config.eye)
				}
				CharacterPartSlot::Mouth => PreviewTarget::WumbusMouth,
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::WumbusEar
				}
				CharacterPartSlot::Horns => PreviewTarget::WumbusHorns(config.horns),
				CharacterPartSlot::Spine => PreviewTarget::WumbusSpine,
				CharacterPartSlot::Nose | CharacterPartSlot::Tail => PreviewTarget::WumbusHead,
				CharacterPartSlot::Hair => PreviewTarget::WumbusHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::WumbusClothing)
					.unwrap_or(PreviewTarget::WumbusHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Lero { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::LeroBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => PreviewTarget::LeroBody,
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => PreviewTarget::LeroHead,
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => PreviewTarget::LeroEye,
				CharacterPartSlot::Mouth => PreviewTarget::LeroMouth(config.mouth),
				CharacterPartSlot::Tail => PreviewTarget::LeroTail,
				CharacterPartSlot::Spine => PreviewTarget::LeroSpine,
				CharacterPartSlot::Nose
				| CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Horns => PreviewTarget::LeroHead,
				CharacterPartSlot::Hair => PreviewTarget::LeroHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::LeroClothing)
					.unwrap_or(PreviewTarget::LeroHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Spibmom { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::SpibmomBody,
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::SpibmomBody
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::SpibmomHead
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::SpibmomEye(config.eye)
				}
				CharacterPartSlot::Nose => PreviewTarget::SpibmomMouth,
				CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
					PreviewTarget::SpibmomEar
				}
				CharacterPartSlot::Horns => PreviewTarget::SpibmomHorns,
				CharacterPartSlot::Spine => PreviewTarget::SpibmomSpine,
				CharacterPartSlot::Mouth | CharacterPartSlot::Tail => PreviewTarget::SpibmomHead,
				CharacterPartSlot::Hair => PreviewTarget::SpibmomHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::SpibmomClothing)
					.unwrap_or(PreviewTarget::SpibmomHead),
			};
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Grener { .. } => {
			let target = PreviewTarget::GrenerBody;
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Thumplus { .. } => {
			let target = PreviewTarget::ThumplusBody;
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Mistler { .. } => {
			let target = PreviewTarget::MistlerBody;
			PreviewAssetTarget { target }
		}
		ConceptPreviewConfig::Tuberwaber { config, .. } => {
			let target = match slot {
				CharacterPartSlot::BodyMesh => PreviewTarget::TuberwaberBody(config.body),
				CharacterPartSlot::NeckRig | CharacterPartSlot::NeckMesh => {
					PreviewTarget::TuberwaberBody(config.body)
				}
				CharacterPartSlot::HeadRig | CharacterPartSlot::HeadMesh => {
					PreviewTarget::TuberwaberHead(config.head)
				}
				CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
					PreviewTarget::TuberwaberEye(config.eye)
				}
				CharacterPartSlot::Nose => PreviewTarget::TuberwaberNose(config.nose),
				CharacterPartSlot::Mouth => PreviewTarget::TuberwaberMouth(config.mouth),
				CharacterPartSlot::Horns => PreviewTarget::TuberwaberHorns,
				CharacterPartSlot::Hair => PreviewTarget::TuberwaberHair(config.hair),
				CharacterPartSlot::Clothing => config
					.clothing
					.iter()
					.copied()
					.find(|clothing| clothing.label() == label)
					.map(PreviewTarget::TuberwaberClothing)
					.unwrap_or(PreviewTarget::TuberwaberHead(config.head)),
				CharacterPartSlot::EarLeft
				| CharacterPartSlot::EarRight
				| CharacterPartSlot::Tail
				| CharacterPartSlot::Spine => PreviewTarget::TuberwaberBody(config.body),
			};
			PreviewAssetTarget { target }
		}
	}
}
