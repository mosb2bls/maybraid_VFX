//! Terrestrial character species and type-erased model dispatch.

use bevy::prelude::*;
use character_items::{Inventory, InventoryItem};
use characters::{
	species::{
		braidman::BraidmanConfig, brenal::BrenalConfig, brodler::BrodlerConfig,
		brokker::BrokkerConfig, caole::CaoleConfig, chupri::ChupriConfig, claber::ClaberConfig,
		croconot::CroconotConfig, dui::DuiConfig, epiphant::EpiphantConfig, hars::HarsConfig,
		kaller::KallerConfig, kappler::KapplerConfig, kispar::KisparConfig, lero::LeroConfig,
		lidder::LidderConfig, mygr::MygrConfig, sonyak::SonyakConfig, spibmom::SpibmomConfig,
		tapp::TappConfig, thumplus::ThumplusConfig, tipple::TippleConfig, topple::ToppleConfig,
		tuberwaber::TuberwaberConfig, wumbus::WumbusConfig, ylter::YilterConfig,
	},
	CharacterRecipe, LocomotionCapsule,
};
use maybraid_audio::{FlinchProfile, GruntStyle};
use player::spawn_npc_visual;

use crate::number::{index, FromMobNumber};
use crate::CharacterBuild;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CharacterSpecies {
	#[default]
	Braidman,
	Brenal,
	Brodler,
	Brokker,
	Caole,
	Chupri,
	Claber,
	Croconot,
	Dui,
	Epiphant,
	Hars,
	Kaller,
	Kappler,
	Kispar,
	Lero,
	Lidder,
	Mygr,
	Sonyak,
	Spibmom,
	Tapp,
	Thumplus,
	Tipple,
	Topple,
	Tuberwaber,
	Wumbus,
	Ylter,
}

impl CharacterSpecies {
	pub const BIPEDS: [Self; 17] = [
		Self::Braidman,
		Self::Brodler,
		Self::Brokker,
		Self::Chupri,
		Self::Dui,
		Self::Kaller,
		Self::Kappler,
		Self::Kispar,
		Self::Lero,
		Self::Lidder,
		Self::Mygr,
		Self::Spibmom,
		Self::Tapp,
		Self::Tipple,
		Self::Topple,
		Self::Tuberwaber,
		Self::Wumbus,
	];

	/// Bipeds that read at the player's scale: 1.4–2.1 m locomotion capsule,
	/// at most 0.5 m radius. Close-quarters modes draw fighters from these.
	pub const PLAYER_SCALE_BIPEDS: [Self; 6] =
		[Self::Braidman, Self::Brokker, Self::Lero, Self::Mygr, Self::Tuberwaber, Self::Wumbus];

	pub const QUADRUPEDS: [Self; 8] = [
		Self::Brenal,
		Self::Caole,
		Self::Claber,
		Self::Croconot,
		Self::Epiphant,
		Self::Hars,
		Self::Sonyak,
		Self::Ylter,
	];

	/// Every non-fish species. Grener and Mistler remain excluded.
	pub const VALUES: [Self; 26] = [
		Self::Braidman,
		Self::Brenal,
		Self::Brodler,
		Self::Brokker,
		Self::Caole,
		Self::Chupri,
		Self::Claber,
		Self::Croconot,
		Self::Dui,
		Self::Epiphant,
		Self::Hars,
		Self::Kaller,
		Self::Kappler,
		Self::Kispar,
		Self::Lero,
		Self::Lidder,
		Self::Mygr,
		Self::Sonyak,
		Self::Spibmom,
		Self::Tapp,
		Self::Thumplus,
		Self::Tipple,
		Self::Topple,
		Self::Tuberwaber,
		Self::Wumbus,
		Self::Ylter,
	];

	pub const fn is_biped(self) -> bool {
		matches!(
			self,
			Self::Braidman
				| Self::Brodler
				| Self::Brokker
				| Self::Chupri
				| Self::Dui | Self::Kaller
				| Self::Kappler
				| Self::Kispar
				| Self::Lero | Self::Lidder
				| Self::Mygr | Self::Spibmom
				| Self::Tapp | Self::Tipple
				| Self::Topple
				| Self::Tuberwaber
				| Self::Wumbus
		)
	}

	pub const fn is_quadruped(self) -> bool {
		matches!(
			self,
			Self::Brenal
				| Self::Caole
				| Self::Claber
				| Self::Croconot
				| Self::Epiphant
				| Self::Hars | Self::Sonyak
				| Self::Ylter
		)
	}

	pub const fn supports_inventory(self) -> bool {
		self.is_biped()
	}

	/// Vocal flinch used when this species takes damage.
	pub const fn flinch_profile(self) -> FlinchProfile {
		let _ = self;
		FlinchProfile::new(GruntStyle::Man)
	}

	pub(crate) fn model(self, build: CharacterBuild, inventory: &Inventory) -> CharacterModel {
		macro_rules! clothed {
			($variant:ident, $config:ty) => {{
				let mut config = <$config>::default_preview();
				for item in inventory.worn_items() {
					let InventoryItem::Clothing { mesh, material, .. } = item else {
						continue;
					};
					if !config.clothing.contains(mesh) {
						config.clothing.push(*mesh);
					}
					config.colors.set_clothing_color(*mesh, material.color);
					config.colors.set_clothing_material(*mesh, material.id);
				}
				CharacterModel::$variant(config)
			}};
		}
		macro_rules! clothed_build {
			($variant:ident, $config:ty) => {{
				let mut config = <$config>::default_preview().with_build(build.visual_preset());
				for item in inventory.worn_items() {
					let InventoryItem::Clothing { mesh, material, .. } = item else {
						continue;
					};
					if !config.clothing.contains(mesh) {
						config.clothing.push(*mesh);
					}
					config.colors.set_clothing_color(*mesh, material.color);
					config.colors.set_clothing_material(*mesh, material.id);
				}
				CharacterModel::$variant(config)
			}};
		}
		macro_rules! bare {
			($variant:ident, $config:ty) => {
				CharacterModel::$variant(
					<$config>::default_preview().with_build(build.visual_preset()),
				)
			};
		}
		match self {
			Self::Braidman => clothed_build!(Braidman, BraidmanConfig),
			Self::Brenal => bare!(Brenal, BrenalConfig),
			Self::Brodler => clothed!(Brodler, BrodlerConfig),
			Self::Brokker => clothed!(Brokker, BrokkerConfig),
			Self::Caole => bare!(Caole, CaoleConfig),
			Self::Chupri => clothed!(Chupri, ChupriConfig),
			Self::Claber => bare!(Claber, ClaberConfig),
			Self::Croconot => bare!(Croconot, CroconotConfig),
			Self::Dui => clothed!(Dui, DuiConfig),
			Self::Epiphant => bare!(Epiphant, EpiphantConfig),
			Self::Hars => bare!(Hars, HarsConfig),
			Self::Kaller => clothed!(Kaller, KallerConfig),
			Self::Kappler => clothed!(Kappler, KapplerConfig),
			Self::Kispar => clothed!(Kispar, KisparConfig),
			Self::Lero => clothed!(Lero, LeroConfig),
			Self::Lidder => clothed!(Lidder, LidderConfig),
			Self::Mygr => clothed!(Mygr, MygrConfig),
			Self::Sonyak => bare!(Sonyak, SonyakConfig),
			Self::Spibmom => clothed!(Spibmom, SpibmomConfig),
			Self::Tapp => clothed!(Tapp, TappConfig),
			Self::Thumplus => CharacterModel::Thumplus(ThumplusConfig::default_preview()),
			Self::Tipple => clothed!(Tipple, TippleConfig),
			Self::Topple => clothed!(Topple, ToppleConfig),
			Self::Tuberwaber => clothed_build!(Tuberwaber, TuberwaberConfig),
			Self::Wumbus => clothed!(Wumbus, WumbusConfig),
			Self::Ylter => bare!(Ylter, YilterConfig),
		}
	}
}

impl FromMobNumber for CharacterSpecies {
	fn from_num(num: f32) -> Self {
		Self::VALUES[index(num, 0x05EE_C1E5, Self::VALUES.len())]
	}
}

macro_rules! character_models {
	($($variant:ident($config:ty)),+ $(,)?) => {
		#[derive(Clone, Debug)]
		pub(crate) enum CharacterModel {
			$($variant($config)),+
		}

		impl CharacterModel {
			pub(crate) fn hull(&self) -> LocomotionCapsule {
				match self {
					$(Self::$variant(config) => config.locomotion_capsule()),+
				}
			}

			pub(crate) fn spawn_visual(
				self,
				commands: &mut Commands,
				body: Entity,
				facing: Quat,
			) -> Entity {
				match self {
					$(Self::$variant(config) => {
						spawn_npc_visual(commands, body, config.clothed(), facing)
					}),+
				}
			}
		}
	};
}

character_models!(
	Braidman(BraidmanConfig),
	Brenal(BrenalConfig),
	Brodler(BrodlerConfig),
	Brokker(BrokkerConfig),
	Caole(CaoleConfig),
	Chupri(ChupriConfig),
	Claber(ClaberConfig),
	Croconot(CroconotConfig),
	Dui(DuiConfig),
	Epiphant(EpiphantConfig),
	Hars(HarsConfig),
	Kaller(KallerConfig),
	Kappler(KapplerConfig),
	Kispar(KisparConfig),
	Lero(LeroConfig),
	Lidder(LidderConfig),
	Mygr(MygrConfig),
	Sonyak(SonyakConfig),
	Spibmom(SpibmomConfig),
	Tapp(TappConfig),
	Thumplus(ThumplusConfig),
	Tipple(TippleConfig),
	Topple(ToppleConfig),
	Tuberwaber(TuberwaberConfig),
	Wumbus(WumbusConfig),
	Ylter(YilterConfig),
);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_species_has_a_man_flinch_until_more_styles_exist() {
		for species in CharacterSpecies::VALUES {
			assert_eq!(species.flinch_profile().grunt, GruntStyle::Man);
		}
	}

	#[test]
	fn generated_species_exclude_fish() {
		assert_eq!(CharacterSpecies::VALUES.len(), 26);
		assert_eq!(
			CharacterSpecies::VALUES.iter().filter(|species| species.is_biped()).count(),
			17
		);
		assert_eq!(
			CharacterSpecies::VALUES.iter().filter(|species| species.is_quadruped()).count(),
			8
		);
		assert_eq!(
			CharacterSpecies::VALUES
				.iter()
				.filter(|species| !species.supports_inventory())
				.count(),
			9
		);
	}
}
