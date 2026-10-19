//! Concept-screen species identity, metadata, and typed config dispatch.
//!
//! [`ConceptSpecies`] is every species the concept screen can preview. Persisted
//! [`crate::CharacterAppearance`] remains the eight-humanoid subset and keeps its
//! serialized form — preview-only species are not forced into appearance files.

use character_ui_menu::CameraFocus;

use crate::{
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
	CharacterRecipe, ConceptAnimation, LocomotionCapsule, RigSkeletonKind,
};

/// Locomotion / silhouette family. Not serialized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConceptSpeciesFamily {
	Humanoid,
	Quadruped,
	Bird,
	Aquatic,
}

/// One catalog: identity, typed config, family metadata, and clothed-recipe dispatch.
///
/// Family groups are the picker sections. `VALUES` is those groups in menu order.
/// `$dollar` is a `$` token so the exported clothed-recipe matcher can be generated
/// without unstable `$$` metavar expressions.
macro_rules! concept_species_catalog {
	(
		$dollar:tt
		humanoids: [$($h_variant:ident { config: $h_config:ty, label: $h_label:literal $(,)? }),+ $(,)?],
		quadrupeds: [$($q_variant:ident { config: $q_config:ty, label: $q_label:literal $(,)? }),+ $(,)?],
		birds: [$($b_variant:ident { config: $b_config:ty, label: $b_label:literal $(,)? }),+ $(,)?],
		aquatic: [$($a_variant:ident { config: $a_config:ty, label: $a_label:literal $(,)? }),+ $(,)?] $(,)?
	) => {
		/// Every species the concept / creation screen can preview.
		#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
		pub enum ConceptSpecies {
			#[default]
			$($h_variant,)+
			$($q_variant,)+
			$($b_variant,)+
			$($a_variant,)+
		}

		/// Typed species payload for preview and menus. Not a persist format.
		#[derive(Debug, Clone, PartialEq)]
		pub enum ConceptSpeciesConfig {
			$($h_variant($h_config),)+
			$($q_variant($q_config),)+
			$($b_variant($b_config),)+
			$($a_variant($a_config),)+
		}

		impl ConceptSpecies {
			pub const HUMANOIDS: &'static [Self] = &[$(Self::$h_variant,)+];
			pub const QUADRUPEDS: &'static [Self] = &[$(Self::$q_variant,)+];
			pub const BIRDS: &'static [Self] = &[$(Self::$b_variant,)+];
			pub const AQUATIC: &'static [Self] = &[$(Self::$a_variant,)+];

			/// Species picker order: humanoids, quadrupeds, birds, aquatic.
			pub const VALUES: &'static [Self] = &[
				$(Self::$h_variant,)+
				$(Self::$q_variant,)+
				$(Self::$b_variant,)+
				$(Self::$a_variant,)+
			];

			/// Same set and order as [`Self::VALUES`].
			pub const ALL: &'static [Self] = Self::VALUES;

			pub const fn label(self) -> &'static str {
				match self {
					$(Self::$h_variant => $h_label,)+
					$(Self::$q_variant => $q_label,)+
					$(Self::$b_variant => $b_label,)+
					$(Self::$a_variant => $a_label,)+
				}
			}

			pub const fn title(self) -> &'static str {
				match self {
					$(Self::$h_variant => stringify!($h_variant),)+
					$(Self::$q_variant => stringify!($q_variant),)+
					$(Self::$b_variant => stringify!($b_variant),)+
					$(Self::$a_variant => stringify!($a_variant),)+
				}
			}

			pub const fn family(self) -> ConceptSpeciesFamily {
				match self {
					$(Self::$h_variant => ConceptSpeciesFamily::Humanoid,)+
					$(Self::$q_variant => ConceptSpeciesFamily::Quadruped,)+
					$(Self::$b_variant => ConceptSpeciesFamily::Bird,)+
					$(Self::$a_variant => ConceptSpeciesFamily::Aquatic,)+
				}
			}
		}

		impl ConceptSpeciesConfig {
			pub fn species(&self) -> ConceptSpecies {
				match self {
					$(Self::$h_variant(_) => ConceptSpecies::$h_variant,)+
					$(Self::$q_variant(_) => ConceptSpecies::$q_variant,)+
					$(Self::$b_variant(_) => ConceptSpecies::$b_variant,)+
					$(Self::$a_variant(_) => ConceptSpecies::$a_variant,)+
				}
			}

			pub fn default_for(species: ConceptSpecies) -> Self {
				match species {
					$(ConceptSpecies::$h_variant => Self::$h_variant(<$h_config>::default_preview()),)+
					$(ConceptSpecies::$q_variant => Self::$q_variant(<$q_config>::default_preview()),)+
					$(ConceptSpecies::$b_variant => Self::$b_variant(<$b_config>::default_preview()),)+
					$(ConceptSpecies::$a_variant => Self::$a_variant(<$a_config>::default_preview()),)+
				}
			}

			pub fn status_label(&self) -> String {
				match self {
					$(Self::$h_variant(config) => config.status_label(),)+
					$(Self::$q_variant(config) => config.status_label(),)+
					$(Self::$b_variant(config) => config.status_label(),)+
					$(Self::$a_variant(config) => config.status_label(),)+
				}
			}

			pub fn sync_key(&self) -> String {
				match self {
					$(Self::$h_variant(config) => config.sync_key(),)+
					$(Self::$q_variant(config) => config.sync_key(),)+
					$(Self::$b_variant(config) => config.sync_key(),)+
					$(Self::$a_variant(config) => config.sync_key(),)+
				}
			}

			pub fn locomotion_capsule(&self) -> LocomotionCapsule {
				match self {
					$(Self::$h_variant(config) => CharacterRecipe::locomotion_capsule(config),)+
					$(Self::$q_variant(config) => CharacterRecipe::locomotion_capsule(config),)+
					$(Self::$b_variant(config) => CharacterRecipe::locomotion_capsule(config),)+
					$(Self::$a_variant(config) => CharacterRecipe::locomotion_capsule(config),)+
				}
			}
		}

		/// Monomorphize `$body` with `$recipe` bound to the species' `clothed()` recipe.
		#[macro_export]
		macro_rules! with_clothed_concept_recipe {
			($dollar config:expr, $dollar recipe:ident => $dollar body:expr) => {
				match $dollar config {
					$($crate::ConceptSpeciesConfig::$h_variant(config) => {
						let $dollar recipe = $crate::CharacterRecipe::clothed(config);
						$dollar body
					})+
					$($crate::ConceptSpeciesConfig::$q_variant(config) => {
						let $dollar recipe = $crate::CharacterRecipe::clothed(config);
						$dollar body
					})+
					$($crate::ConceptSpeciesConfig::$b_variant(config) => {
						let $dollar recipe = $crate::CharacterRecipe::clothed(config);
						$dollar body
					})+
					$($crate::ConceptSpeciesConfig::$a_variant(config) => {
						let $dollar recipe = $crate::CharacterRecipe::clothed(config);
						$dollar body
					})+
				}
			};
		}
	};
}

concept_species_catalog! {
	$
	humanoids: [
		Braidman { config: BraidmanConfig, label: "braidman" },
		Brodler { config: BrodlerConfig, label: "brodler" },
		Mygr { config: MygrConfig, label: "mygr" },
		Dui { config: DuiConfig, label: "dui" },
		Wumbus { config: WumbusConfig, label: "wumbus" },
		Lero { config: LeroConfig, label: "lero" },
		Spibmom { config: SpibmomConfig, label: "spibmom" },
		Tuberwaber { config: TuberwaberConfig, label: "tuberwaber" },
	],
	quadrupeds: [
		Brenal { config: BrenalConfig, label: "brenal" },
		Caole { config: CaoleConfig, label: "caole" },
		Epiphant { config: EpiphantConfig, label: "epiphant" },
		Hars { config: HarsConfig, label: "hars" },
		Yilter { config: YilterConfig, label: "ylter" },
		Sonyak { config: SonyakConfig, label: "sonyak" },
		Claber { config: ClaberConfig, label: "claber" },
		Croconot { config: CroconotConfig, label: "croconot" },
	],
	birds: [
		Lidder { config: LidderConfig, label: "lidder" },
		Chupri { config: ChupriConfig, label: "chupri" },
		Brokker { config: BrokkerConfig, label: "brokker" },
		Tipple { config: TippleConfig, label: "tipple" },
		Topple { config: ToppleConfig, label: "topple" },
		Kispar { config: KisparConfig, label: "kispar" },
		Tapp { config: TappConfig, label: "tapp" },
		Kaller { config: KallerConfig, label: "kaller" },
		Kappler { config: KapplerConfig, label: "kappler" },
	],
	aquatic: [
		Grener { config: GrenerConfig, label: "grener" },
		Thumplus { config: ThumplusConfig, label: "thumplus" },
		Mistler { config: MistlerConfig, label: "mistler" },
	],
}

/// Quadruped clip picker: still / run / gallop. Other families use the full catalog.
pub const QUADRUPED_CONCEPT_ANIMATIONS: &[ConceptAnimation] =
	&[ConceptAnimation::Still, ConceptAnimation::Run, ConceptAnimation::Gallop];

impl ConceptSpecies {
	pub const fn is_humanoid(self) -> bool {
		matches!(self.family(), ConceptSpeciesFamily::Humanoid)
	}

	/// Whether this species may be written as [`crate::CharacterAppearance`].
	pub const fn persists_appearance(self) -> bool {
		self.is_humanoid()
	}

	/// Body-rig family used to expand and aim whole-body preview framing.
	pub const fn skeleton_kind(self) -> RigSkeletonKind {
		match self.family() {
			ConceptSpeciesFamily::Quadruped => RigSkeletonKind::Quadruped,
			ConceptSpeciesFamily::Aquatic => RigSkeletonKind::Forelimbed,
			ConceptSpeciesFamily::Humanoid | ConceptSpeciesFamily::Bird => {
				RigSkeletonKind::Humanoid
			}
		}
	}

	/// Quadruped live-preview keys historically omit animation; keep that split.
	pub const fn preview_sync_includes_animation(self) -> bool {
		!matches!(self.family(), ConceptSpeciesFamily::Quadruped)
	}

	pub const fn supported_animations(self) -> &'static [ConceptAnimation] {
		match self.family() {
			ConceptSpeciesFamily::Quadruped => QUADRUPED_CONCEPT_ANIMATIONS,
			_ => ConceptAnimation::VALUES,
		}
	}

	/// Whole-body preview framing from a rest-pose hull (no per-species offsets).
	pub fn body_preview_focus(self, hull: LocomotionCapsule) -> CameraFocus {
		let kind = self.skeleton_kind();
		CameraFocus::fit_preview_bounds(
			hull.preview_aabb_for(kind),
			CameraFocus::BODY_PREVIEW_FOV,
			kind.preview_view_direction(),
		)
	}
}

impl Default for ConceptSpeciesConfig {
	fn default() -> Self {
		Self::default_for(ConceptSpecies::Braidman)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{CharacterAppearance, CharacterComponents};

	#[test]
	fn picker_order_covers_the_full_catalog_once() {
		assert_eq!(ConceptSpecies::ALL.len(), 28);
		assert_eq!(ConceptSpecies::VALUES.len(), 28);
		assert_eq!(ConceptSpecies::ALL, ConceptSpecies::VALUES);
		assert_eq!(
			ConceptSpecies::HUMANOIDS.len()
				+ ConceptSpecies::QUADRUPEDS.len()
				+ ConceptSpecies::BIRDS.len()
				+ ConceptSpecies::AQUATIC.len(),
			ConceptSpecies::ALL.len()
		);
		let mut concatenated = Vec::from(ConceptSpecies::HUMANOIDS);
		concatenated.extend_from_slice(ConceptSpecies::QUADRUPEDS);
		concatenated.extend_from_slice(ConceptSpecies::BIRDS);
		concatenated.extend_from_slice(ConceptSpecies::AQUATIC);
		assert_eq!(concatenated, ConceptSpecies::VALUES);
		for species in ConceptSpecies::ALL {
			let group = match species.family() {
				ConceptSpeciesFamily::Humanoid => ConceptSpecies::HUMANOIDS,
				ConceptSpeciesFamily::Quadruped => ConceptSpecies::QUADRUPEDS,
				ConceptSpeciesFamily::Bird => ConceptSpecies::BIRDS,
				ConceptSpeciesFamily::Aquatic => ConceptSpecies::AQUATIC,
			};
			assert!(group.contains(species), "{species:?} family/group mismatch");
		}
	}

	#[test]
	fn persistable_species_match_character_appearance() {
		assert_eq!(ConceptSpecies::HUMANOIDS.len(), 8);
		for species in ConceptSpecies::ALL {
			assert_eq!(species.persists_appearance(), species.is_humanoid());
			assert_eq!(
				species.persists_appearance(),
				matches!(species.family(), ConceptSpeciesFamily::Humanoid)
			);
		}
		let appearance_species = [
			CharacterAppearance::default().species(),
			CharacterAppearance::Brodler(crate::species::brodler::BrodlerConfig::default_preview())
				.species(),
			CharacterAppearance::Mygr(crate::species::mygr::MygrConfig::default_preview())
				.species(),
			CharacterAppearance::Dui(crate::species::dui::DuiConfig::default_preview()).species(),
			CharacterAppearance::Wumbus(crate::species::wumbus::WumbusConfig::default_preview())
				.species(),
			CharacterAppearance::Lero(crate::species::lero::LeroConfig::default_preview())
				.species(),
			CharacterAppearance::Spibmom(crate::species::spibmom::SpibmomConfig::default_preview())
				.species(),
			CharacterAppearance::Tuberwaber(
				crate::species::tuberwaber::TuberwaberConfig::default_preview(),
			)
			.species(),
		];
		assert_eq!(appearance_species, ConceptSpecies::HUMANOIDS);
	}

	#[test]
	fn preview_covers_more_species_than_persisted_appearance() {
		assert!(ConceptSpecies::ALL.len() > ConceptSpecies::HUMANOIDS.len());
		assert!(ConceptSpeciesConfig::default_for(ConceptSpecies::Brenal).appearance().is_none());
		assert!(ConceptSpeciesConfig::default_for(ConceptSpecies::Braidman)
			.appearance()
			.is_some());
	}

	#[test]
	fn skeleton_kind_follows_family() {
		assert_eq!(ConceptSpecies::Braidman.skeleton_kind(), RigSkeletonKind::Humanoid);
		assert_eq!(ConceptSpecies::Lidder.skeleton_kind(), RigSkeletonKind::Humanoid);
		assert_eq!(ConceptSpecies::Brenal.skeleton_kind(), RigSkeletonKind::Quadruped);
		assert_eq!(ConceptSpecies::Grener.skeleton_kind(), RigSkeletonKind::Forelimbed);
		assert!(!ConceptSpecies::Brenal.preview_sync_includes_animation());
		assert!(ConceptSpecies::Braidman.preview_sync_includes_animation());
		assert_eq!(ConceptSpecies::Brenal.supported_animations(), QUADRUPED_CONCEPT_ANIMATIONS);
		assert_eq!(ConceptSpecies::Braidman.supported_animations(), ConceptAnimation::VALUES);
	}

	#[test]
	fn default_config_round_trips_species_identity() {
		for &species in ConceptSpecies::ALL {
			let config = ConceptSpeciesConfig::default_for(species);
			assert_eq!(config.species(), species);
			assert!(!config.status_label().is_empty());
			assert!(!config.sync_key().is_empty());
		}
	}

	#[test]
	fn clothed_recipe_macro_dispatches_every_catalog_row() {
		for &species in ConceptSpecies::ALL {
			let config = ConceptSpeciesConfig::default_for(species);
			let capsule = with_clothed_concept_recipe!(&config, recipe => {
				recipe.locomotion_capsule()
			});
			assert_eq!(capsule, config.locomotion_capsule());
		}
	}
}
