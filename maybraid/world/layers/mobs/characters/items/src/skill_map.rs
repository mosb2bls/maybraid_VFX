//! Discover skill-map items in a character bag.
//!
//! Kind plus seed is the whole identity: the same kind with a different seed
//! is a different map. The live session presents one equipped spec at a time.

use serde::{Deserialize, Serialize};

/// Authored Discover map a character can carry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkillMapKind {
	#[default]
	Fireball,
	Dumbwave,
	Rockadder,
	Cosimo,
}

impl SkillMapKind {
	pub const VALUES: &'static [Self] =
		&[Self::Fireball, Self::Dumbwave, Self::Rockadder, Self::Cosimo];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Fireball => "fireball",
			Self::Dumbwave => "dumbwave",
			Self::Rockadder => "rockadder",
			Self::Cosimo => "cosimo",
		}
	}

	pub const fn display_name(self) -> &'static str {
		match self {
			Self::Fireball => "Fireball",
			Self::Dumbwave => "Dumbwave",
			Self::Rockadder => "Rockadder",
			Self::Cosimo => "Cosimo",
		}
	}

	pub const fn adjectives(self) -> &'static [&'static str] {
		match self {
			Self::Fireball => &["Searing", "Ember", "Cinder", "Solar", "Ashen"],
			Self::Dumbwave => &["Dull", "Hollow", "Mute", "Blank", "Haze"],
			Self::Rockadder => &["Inlaid", "Tessellated", "Faceted", "Mosaic", "Geometric"],
			Self::Cosimo => &["Celestial", "Astral", "Violet", "Sidereal", "Midnight"],
		}
	}

	/// Catalog plate. Matches the live viewport clear for that kind.
	pub const fn preview_srgb(self) -> [f32; 3] {
		match self {
			Self::Fireball => [0.82, 0.22, 0.08],
			Self::Dumbwave => [0.12, 0.62, 0.72],
			Self::Rockadder => [0.78, 0.58, 0.18],
			Self::Cosimo => [0.48, 0.18, 0.78],
		}
	}
}

/// One owned skill map: which effect, and which noise field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillMapSpec {
	pub kind: SkillMapKind,
	pub seed: u32,
}

impl SkillMapSpec {
	pub const fn new(kind: SkillMapKind, seed: u32) -> Self {
		Self { kind, seed }
	}

	/// Stable catalog thumbnail key: kind in the high word, seed in the low.
	pub const fn catalog_key(self) -> u64 {
		let kind = match self.kind {
			SkillMapKind::Fireball => 0,
			SkillMapKind::Dumbwave => 1,
			SkillMapKind::Rockadder => 2,
			SkillMapKind::Cosimo => 3,
		};
		(kind << 32) | self.seed as u64
	}
}
