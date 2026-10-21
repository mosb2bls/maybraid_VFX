//! Authored skill maps and the render-layer range from the POC.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use character_items::{SkillMapKind, SkillMapSpec};

/// User-facing map id. Live play presents one equipped spec at layer `0`.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkillMapId(pub u32);

/// World effect a power tile claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillKind {
	Fireball,
	Dumbwave,
	Rockadder,
	Cosimo,
}

impl SkillKind {
	/// Stable order for catalog tables and parallel asset setup.
	pub const ALL: [Self; 4] = [Self::Fireball, Self::Dumbwave, Self::Rockadder, Self::Cosimo];

	pub fn label(self) -> &'static str {
		match self {
			Self::Fireball => "Fireball",
			Self::Dumbwave => "Dumbwave",
			Self::Rockadder => "Rockadder",
			Self::Cosimo => "Cosimo",
		}
	}

	pub fn from_item(kind: SkillMapKind) -> Self {
		match kind {
			SkillMapKind::Fireball => Self::Fireball,
			SkillMapKind::Dumbwave => Self::Dumbwave,
			SkillMapKind::Rockadder => Self::Rockadder,
			SkillMapKind::Cosimo => Self::Cosimo,
		}
	}

	pub fn tile_color(self) -> Color {
		match self {
			Self::Fireball => Color::srgb(0.82, 0.18, 0.78),
			Self::Dumbwave => Color::srgb(0.22, 0.78, 0.86),
			Self::Rockadder => Color::srgb(0.72, 0.38, 0.16),
			Self::Cosimo => Color::srgb(0.42, 0.16, 0.72),
		}
	}

	pub fn viewport_clear(self) -> Color {
		match self {
			Self::Fireball => Color::srgb(0.11, 0.09, 0.07),
			Self::Dumbwave => Color::srgb(0.04, 0.07, 0.10),
			Self::Rockadder => Color::srgb(0.10, 0.07, 0.05),
			Self::Cosimo => Color::srgb(0.06, 0.03, 0.10),
		}
	}

	fn catalog_seed(self) -> u32 {
		match self {
			Self::Fireball => 0,
			Self::Dumbwave => 7,
			Self::Rockadder => 13,
			Self::Cosimo => 19,
		}
	}
}

/// First Bevy render layer reserved for skill maps (POC used 24..34).
pub const SKILL_MAP_LAYER_BASE: usize = 24;

pub fn render_layer(id: SkillMapId) -> RenderLayers {
	RenderLayers::layer(SKILL_MAP_LAYER_BASE + id.0 as usize)
}

#[derive(Clone, Copy, Debug)]
pub struct AuthoredMap {
	pub id: SkillMapId,
	pub kind: SkillKind,
	pub seed: u32,
	pub frequency: f64,
	pub label: &'static str,
}

pub fn authored_map(kind: SkillKind, seed: u32) -> AuthoredMap {
	AuthoredMap {
		id: SkillMapId(0),
		kind,
		seed,
		frequency: match kind {
			SkillKind::Fireball => 0.08,
			SkillKind::Dumbwave => 0.11,
			SkillKind::Rockadder => 0.09,
			SkillKind::Cosimo => 0.07,
		},
		label: kind.label(),
	}
}

pub fn authored_map_from_spec(spec: SkillMapSpec) -> AuthoredMap {
	authored_map(SkillKind::from_item(spec.kind), spec.seed)
}

/// Catalog of kinds. Live play presents one equipped spec, not this list.
pub fn authored_maps() -> [AuthoredMap; 4] {
	SkillKind::ALL.map(|kind| authored_map(kind, kind.catalog_seed()))
}

#[derive(Clone, Copy, Debug)]
pub struct MapExtents {
	pub min: Vec2,
	pub max: Vec2,
	pub steps: u32,
}

impl Default for MapExtents {
	fn default() -> Self {
		Self { min: Vec2::splat(-128.0), max: Vec2::splat(128.0), steps: 16 }
	}
}

impl MapExtents {
	pub fn tile_size(self) -> Vec2 {
		Vec2::new(
			(self.max.x - self.min.x) / self.steps as f32,
			(self.max.y - self.min.y) / self.steps as f32,
		)
	}

	pub fn tile_center(self, x: u32, y: u32) -> Vec2 {
		let size = self.tile_size();
		self.min + size * Vec2::new(x as f32 + 0.5, y as f32 + 0.5)
	}
}

/// Grid cells that always spawn a power tile so the opening walk finds one.
pub fn pinned_power_cells(steps: u32) -> [(u32, u32); 3] {
	let mid = steps / 2;
	[(mid + 2, mid), (mid.saturating_sub(2), mid + 2), (mid + 1, mid.saturating_sub(3))]
}
