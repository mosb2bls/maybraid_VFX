//! Display names for owned items.
//!
//! Each mesh contributes nouns, each look and color adjectives. A stable hash of
//! the triple picks one word from each list so the same item always has the same
//! name (`Celestial Red Tide Joggers`).

use crate::{ClothingMaterial, ClothingMesh, FirearmSpec, GrenadeSpec, ItemColor, SkillMapSpec};

pub(crate) fn mix(seed: u64, label: &str) -> u64 {
	let mut hash = seed ^ 0x9E37_79B9_7F4A_7C15;
	for byte in label.as_bytes() {
		hash = hash.wrapping_mul(0x0100_0000_01B3).wrapping_add(*byte as u64);
	}
	hash ^ hash >> 33
}

fn pick<'a>(words: &'a [&'a str], hash: u64) -> &'a str {
	words[(hash as usize) % words.len()]
}

impl ClothingMesh {
	/// Material adjective, then color adjective, then clothing noun.
	pub fn hashed_name(self, material: ClothingMaterial, color: ItemColor) -> String {
		let hash =
			mix(mix(mix(0xC0FF_EE42_D00D_A5E5, self.label()), material.label()), color.label());
		format!(
			"{} {} {}",
			pick(material.adjectives(), hash),
			pick(color.adjectives(), hash >> 17),
			pick(self.nouns(), hash >> 33),
		)
	}

	/// Nouns a rolled item of this mesh may be called.
	pub const fn nouns(self) -> &'static [&'static str] {
		match self {
			Self::TankTop => {
				&["Shirt", "Vest", "Takt", "Itun", "Underhemd", "Hemlet", "Shortsuit", "Chestsuit"]
			}
			Self::Tunic => &[
				"Stansuit", "Shirt", "Tunic", "Smock", "Tabard", "Shift", "Blouse", "Overkin",
				"Helfslev", "Palu",
			],
			Self::LongDress => &["Dress", "Gown", "Torrean", "Idilist", "Foreme"],
			Self::ShortDress => &["Dress", "Frock", "Sundress", "Foreme", "P'leme"],
			Self::FittedCoat => {
				&["Coat", "Jacket", "Peacoat", "P'cot", "Shell", "Rainshell", "Husk"]
			}
			Self::RobeCoat => &["Robe", "Overrobe", "Clerker", "Mantle", "Greatcoat", "Klark"],
			Self::Robe => {
				&["Robe", "Vestment", "Wrap", "Kilbright", "Cassock", "Raiment", "Shroud", "Shando"]
			}
			Self::Pants => &["Pants", "Trousers", "Slacks", "Botms", "Britches"],
			Self::KneeHighBoots => &["Boots", "Waders", "Buckets", "Cronks"],
			Self::HaremPants => &["Harems", "Pants", "Sarouel", "Wardshale"],
			Self::HaremPantsUpper => &["Harems", "Pants", "Sarouel", "Wardshale"],
			Self::HaremPantsLowerWrap => &["Wrap", "Bindings", "Cuffs"],
		}
	}
}

impl ClothingMaterial {
	/// Adjectives a rolled item of this look may be called.
	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::SpaceSuit => {
				&["Orbital", "Plstar", "Comlet", "Asterbis", "Nambulant", "Latex", "Sunk God's"]
			}
			Self::Tattered => {
				&["Tattered", "Bristling", "Worn", "Holencone", "Distlekempt", "Monk's", "Prester"]
			}
			Self::Hawaiian => &["Isle", "Isthanish", "Hibiscus", "Suburban", "Okomo's"],
			Self::Cloth => &["Plain", "Humble", "Cotton", "Shorn", "Listed", "Standard", "Lentish"],
			Self::Scales => &["Serpent's", "Iridescent", "Draconic", "Plated", "Cymlic"],
			Self::WizardsVeins => {
				&["Celestial", "Bearded", "Gerunt", "Starmage", "Hidren", "Sacrihewn", "Unspoken"]
			}
			Self::Glitter => &["Mistite", "Pogem", "Int", "Celebride", "Sparkletone"],
		}
	}
}

impl FirearmSpec {
	/// Material adjective, color adjective, then firearm noun.
	pub fn hashed_name(self) -> String {
		let hash = mix(0xA11A_4A45_F1A4_0001, &self.identity_label());
		format!(
			"{} {} {}",
			pick(self.looks.body.material.adjectives(), hash),
			pick(self.looks.body.color.adjectives(), hash >> 17),
			pick(self.kit.body.nouns(), hash >> 33),
		)
	}
}

impl GrenadeSpec {
	/// Stable noun from the grenade mesh.
	pub fn hashed_name(self) -> String {
		let hash = mix(0x6E4A_DE00_F1A4_0001, self.mesh.label());
		pick(self.mesh.nouns(), hash).to_string()
	}
}

impl SkillMapSpec {
	/// Adjective plus the authored kind so two seeds of the same map still differ.
	pub fn hashed_name(self) -> String {
		let hash = mix(0x5A11_5A1D_0000_0001, self.kind.label()).wrapping_add(u64::from(self.seed));
		format!("{} {}", pick(self.kind.adjectives(), hash), self.kind.display_name())
	}
}

impl ItemColor {
	/// Adjectives a rolled item of this color may be called.
	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::Natural => &["Undyed", "Naturist", "Red'irt", "Alakash"],
			Self::Warm => &["Honeyed", "Flamelike", "Summergrass", "O'Red Yoeman"],
			Self::Cool => &["Winter", "Blenchit", "Lone", "Alpine"],
			Self::Dark => &["Dark", "Shadow", "Soot", "Umbral", "Wine"],
			Self::Light => &["Pale", "Ivor", "Dawn", "Close"],
			Self::Red => &[
				"Bloody",
				"Red Tide",
				"Scarlet",
				"Burnlead",
				"Passionate",
				"T'dith",
				"Flame",
				"Flemmend",
			],
			Self::Blue => &["Veinish", "Clostrono", "Cool", "Cerulean", "Tidal", "d'Tid"],
			Self::Green => &["Verdant", "Woodmake", "Ciling", "Zealousard", "Estrine"],
			Self::Gold => &["Aureate", "Sunlit", "Beam"],
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{FirearmMaterial, FirearmMesh};

	#[test]
	fn name_is_stable_for_a_triple() {
		let a =
			ClothingMesh::HaremPants.hashed_name(ClothingMaterial::WizardsVeins, ItemColor::Red);
		let b =
			ClothingMesh::HaremPants.hashed_name(ClothingMaterial::WizardsVeins, ItemColor::Red);
		assert_eq!(a, b);
		let material = ClothingMaterial::WizardsVeins.adjectives();
		let color = ItemColor::Red.adjectives();
		let noun = ClothingMesh::HaremPants.nouns();
		assert!(material.iter().any(|word| a.starts_with(*word)));
		assert!(noun.iter().any(|word| a.ends_with(*word)));
		assert!(color.iter().any(|word| a.contains(*word)));
	}

	#[test]
	fn every_catalog_entry_has_words() {
		for mesh in ClothingMesh::VALUES {
			assert!(!mesh.nouns().is_empty(), "{}", mesh.label());
		}
		for material in ClothingMaterial::VALUES {
			assert!(!material.adjectives().is_empty(), "{}", material.label());
		}
		for color in ItemColor::VALUES {
			assert!(!color.adjectives().is_empty(), "{}", color.label());
		}
		for mesh in FirearmMesh::VALUES {
			assert!(!mesh.nouns().is_empty(), "{}", mesh.label());
			let name = FirearmSpec::from_mesh(*mesh).hashed_name();
			assert!(name.contains(' '), "{name}");
		}
		for material in FirearmMaterial::VALUES {
			assert!(!material.adjectives().is_empty(), "{}", material.label());
		}
		for mesh in crate::GrenadeMesh::VALUES {
			assert!(!mesh.nouns().is_empty(), "{}", mesh.label());
			assert!(!crate::GrenadeSpec { mesh: *mesh }.hashed_name().is_empty());
		}
		for kind in crate::SkillMapKind::VALUES {
			assert!(!kind.adjectives().is_empty(), "{}", kind.label());
			let name = crate::SkillMapSpec::new(*kind, 7).hashed_name();
			assert!(name.contains(kind.display_name()), "{name}");
			assert_ne!(kind.preview_srgb(), [1.0, 1.0, 1.0], "{}", kind.label());
		}
	}
}
