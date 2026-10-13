//! Seeded finish picks: quiet carcass wood and a small chest rotation.
//!
//! Frames stay stained wood. Chests pick among the looks that already read
//! well in the world — Rockadder mosaics, Cosimo nebula, foliage, and
//! Durham earth — instead of lava / ornate / scale carnival skins.

use bevy::prelude::Color;
use furniture_shaders::{
	RECIPE_FURNITURE_CLOTH, RECIPE_FURNITURE_COSMOS, RECIPE_FURNITURE_LACQUER,
	RECIPE_FURNITURE_LAVA, RECIPE_FURNITURE_MARBLE, RECIPE_FURNITURE_METAL,
	RECIPE_FURNITURE_ORNATE, RECIPE_FURNITURE_ROCKADDER, RECIPE_FURNITURE_SCALES,
	RECIPE_FURNITURE_SOFT, RECIPE_FURNITURE_WOOD,
};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

const CHEST_SALT: u64 = 0xC7E5_7B0D;

/// Frond recipe: foliage lighting and grove greens, no leaf-cheese holes.
pub const RECIPE_FOLIAGE: &str = "CHICO_FROND_MATERIAL";

/// Structural look for frames, legs, backs, and cabinet bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarcassKind {
	Wood,
}

/// Chest field look. Trunk and lid share one kind per seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChestKind {
	Rockadder,
	Cosimo,
	Foliage,
	Terrain,
}

/// Honey / cherry lead; olive, bark, and greige are the quieter rows.
pub const WOOD: [[f32; 3]; 6] = [
	[0.58, 0.40, 0.24],
	[0.50, 0.28, 0.20],
	[0.72, 0.62, 0.46],
	[0.18, 0.12, 0.10],
	[0.42, 0.52, 0.28],
	[0.46, 0.38, 0.34],
];
pub const WOOD_ACCENT: [[f32; 3]; 6] = [
	[0.62, 0.36, 0.16],
	[0.32, 0.36, 0.22],
	[0.58, 0.48, 0.28],
	[0.16, 0.22, 0.18],
	[0.32, 0.28, 0.16],
	[0.32, 0.40, 0.24],
];
pub const WOOD_SHADE: [[f32; 3]; 6] = [
	[0.36, 0.24, 0.18],
	[0.28, 0.14, 0.12],
	[0.48, 0.40, 0.28],
	[0.10, 0.08, 0.08],
	[0.22, 0.28, 0.16],
	[0.28, 0.22, 0.20],
];

pub const LACQUER: [[f32; 3]; 4] =
	[[0.86, 0.16, 0.14], [0.18, 0.22, 0.62], [0.12, 0.52, 0.38], [0.92, 0.84, 0.68]];
pub const LACQUER_ACCENT: [[f32; 3]; 4] =
	[[1.00, 0.38, 0.22], [0.38, 0.42, 0.88], [0.28, 0.72, 0.52], [1.00, 0.94, 0.80]];

pub const METAL: [[f32; 3]; 4] =
	[[0.78, 0.62, 0.28], [0.72, 0.38, 0.22], [0.58, 0.60, 0.62], [0.22, 0.20, 0.22]];
pub const METAL_ACCENT: [[f32; 3]; 4] =
	[[0.96, 0.82, 0.42], [0.92, 0.52, 0.30], [0.82, 0.84, 0.86], [0.40, 0.38, 0.40]];

/// Grove foliage + linen. No neon yellow / magenta.
pub const CLOTH: [[f32; 3]; 4] =
	[[0.42, 0.52, 0.28], [0.25, 0.62, 0.32], [0.12, 0.35, 0.18], [0.86, 0.78, 0.62]];
pub const CLOTH_ACCENT: [[f32; 3]; 4] =
	[[0.32, 0.40, 0.22], [0.38, 0.48, 0.28], [0.22, 0.42, 0.28], [0.72, 0.62, 0.46]];

pub const MATTRESS: [[f32; 3]; 4] =
	[[0.92, 0.86, 0.74], [0.88, 0.84, 0.76], [0.86, 0.78, 0.62], [0.80, 0.74, 0.64]];

/// Durham macro swatches: greige, chalk, shale, baked clay.
pub const MARBLE: [[f32; 3]; 4] =
	[[0.46, 0.38, 0.34], [0.82, 0.80, 0.70], [0.18, 0.20, 0.22], [0.48, 0.20, 0.12]];
pub const MARBLE_VEIN: [[f32; 3]; 4] =
	[[0.34, 0.28, 0.24], [0.62, 0.58, 0.48], [0.10, 0.12, 0.13], [0.32, 0.14, 0.10]];
pub const MARBLE_SPARK: [[f32; 3]; 4] =
	[[0.62, 0.53, 0.49], [0.92, 0.88, 0.78], [0.28, 0.30, 0.32], [0.68, 0.34, 0.20]];

pub const GOLD: [[f32; 3]; 4] =
	[[0.78, 0.58, 0.22], [0.72, 0.54, 0.20], [0.68, 0.50, 0.18], [0.74, 0.56, 0.24]];
pub const GEM: [[f32; 3]; 4] =
	[[0.18, 0.46, 0.62], [0.62, 0.18, 0.28], [0.16, 0.52, 0.38], [0.42, 0.22, 0.58]];

pub const LAVA_COAL: [[f32; 3]; 4] =
	[[0.10, 0.05, 0.04], [0.14, 0.06, 0.04], [0.08, 0.04, 0.06], [0.16, 0.08, 0.05]];
pub const LAVA_GLOW: [[f32; 3]; 4] =
	[[1.00, 0.32, 0.05], [0.95, 0.22, 0.04], [1.00, 0.42, 0.10], [0.90, 0.18, 0.08]];
pub const LAVA_HOT: [[f32; 3]; 4] =
	[[1.00, 0.88, 0.42], [1.00, 0.78, 0.28], [1.00, 0.94, 0.62], [0.98, 0.70, 0.22]];

/// Skill-map Cosimo: void / nebula / bloom. Bloom stays violet, not neon.
pub const COSMOS_VOID: [[f32; 3]; 4] =
	[[0.04, 0.02, 0.09], [0.05, 0.03, 0.11], [0.03, 0.04, 0.10], [0.06, 0.02, 0.08]];
pub const COSMOS_NEBULA: [[f32; 3]; 4] =
	[[0.22, 0.06, 0.38], [0.18, 0.08, 0.36], [0.26, 0.08, 0.34], [0.16, 0.10, 0.40]];
pub const COSMOS_BLOOM: [[f32; 3]; 4] =
	[[0.42, 0.16, 0.62], [0.38, 0.14, 0.56], [0.46, 0.20, 0.58], [0.36, 0.18, 0.52]];

pub const SCALE_BELLY: [[f32; 3]; 4] =
	[[0.22, 0.48, 0.38], [0.48, 0.22, 0.28], [0.18, 0.32, 0.52], [0.62, 0.48, 0.18]];
pub const SCALE_EDGE: [[f32; 3]; 4] =
	[[0.08, 0.16, 0.12], [0.18, 0.08, 0.10], [0.06, 0.10, 0.20], [0.22, 0.16, 0.06]];
pub const SCALE_IRID: [[f32; 3]; 4] =
	[[0.32, 0.88, 0.70], [0.88, 0.42, 0.62], [0.42, 0.72, 0.95], [0.95, 0.78, 0.32]];

/// Skill-map mosaic: terracotta, teal, cream, muted gold.
pub const ROCK_TILE: [[f32; 3]; 4] =
	[[0.62, 0.28, 0.16], [0.86, 0.78, 0.62], [0.52, 0.22, 0.14], [0.12, 0.38, 0.36]];
pub const ROCK_TEAL: [[f32; 3]; 4] =
	[[0.12, 0.38, 0.36], [0.08, 0.06, 0.05], [0.10, 0.32, 0.40], [0.86, 0.78, 0.62]];
pub const ROCK_GOLD: [[f32; 3]; 4] =
	[[0.78, 0.58, 0.22], [0.78, 0.58, 0.22], [0.72, 0.58, 0.40], [0.62, 0.28, 0.16]];

/// Whole square-lattice rows so tile / grout / inlay stay a set.
pub const ROCK_SQUARES: [[[f32; 3]; 3]; 4] = [
	[[0.62, 0.28, 0.16], [0.12, 0.38, 0.36], [0.78, 0.58, 0.22]],
	[[0.86, 0.78, 0.62], [0.08, 0.06, 0.05], [0.78, 0.58, 0.22]],
	[[0.12, 0.38, 0.36], [0.86, 0.78, 0.62], [0.62, 0.28, 0.16]],
	[[0.52, 0.22, 0.14], [0.10, 0.28, 0.32], [0.72, 0.58, 0.40]],
];

/// Grove canopy mix: olive, fresh, deep, sage.
pub const FOLIAGE: [[f32; 3]; 4] =
	[[0.42, 0.52, 0.28], [0.25, 0.62, 0.32], [0.12, 0.35, 0.18], [0.42, 0.48, 0.32]];

/// Produce skins: apple, orange, pear, banana.
pub const PRODUCE: [[f32; 3]; 4] =
	[[0.78, 0.16, 0.14], [0.92, 0.48, 0.12], [0.42, 0.58, 0.18], [0.92, 0.78, 0.22]];
pub const PRODUCE_SHADE: [[f32; 3]; 4] =
	[[0.48, 0.10, 0.10], [0.62, 0.28, 0.08], [0.22, 0.36, 0.12], [0.62, 0.50, 0.12]];

/// Bread crust / crumb.
pub const CRUST: [[f32; 3]; 4] =
	[[0.72, 0.48, 0.22], [0.86, 0.70, 0.42], [0.52, 0.32, 0.16], [0.90, 0.78, 0.52]];

/// Durham earth: weathered greige, baked clay, sage alluvium, shale.
pub const TERRAIN: [[f32; 3]; 4] =
	[[0.46, 0.38, 0.34], [0.48, 0.20, 0.12], [0.46, 0.50, 0.36], [0.18, 0.20, 0.22]];
pub const TERRAIN_VEIN: [[f32; 3]; 4] =
	[[0.34, 0.28, 0.24], [0.32, 0.14, 0.10], [0.32, 0.36, 0.24], [0.10, 0.12, 0.13]];
pub const TERRAIN_SPARK: [[f32; 3]; 4] =
	[[0.62, 0.53, 0.49], [0.68, 0.34, 0.20], [0.66, 0.64, 0.42], [0.28, 0.30, 0.32]];

pub fn mix_seed(seed: u64, salt: u64) -> u64 {
	let mut value = seed ^ salt;
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

fn rgb(table: &[[f32; 3]], seed: u64, salt: u64) -> Color {
	let [r, g, b] = table[(mix_seed(seed, salt) as usize) % table.len()];
	Color::srgb(r, g, b)
}

fn look_noise(seed: u64, frequency: f32) -> NoiseParams {
	NoiseParams { seed: seed as i32, frequency, ..NoiseParams::default() }
}

fn recipe(
	name: &'static str,
	seed: u64,
	frequency: f32,
	palette: impl IntoIterator<Item = Color>,
) -> MaterialRef {
	MaterialRef::named(name)
		.with_palette(palette)
		.with_noise(look_noise(seed, frequency))
}

/// Carcass recipe from the assembly seed (same for every woody part).
pub fn carcass_kind(_seed: u64) -> CarcassKind {
	CarcassKind::Wood
}

/// Chest field recipe from the assembly seed (trunk and lid share it).
///
/// Rockadder is the common mosaic; Cosimo, foliage, and terrain share the rest.
pub fn chest_kind(seed: u64) -> ChestKind {
	match mix_seed(seed, CHEST_SALT) % 5 {
		0 | 1 => ChestKind::Rockadder,
		2 => ChestKind::Cosimo,
		3 => ChestKind::Foliage,
		_ => ChestKind::Terrain,
	}
}

/// First seed in `0..limit` that yields `want`, if any.
pub fn first_seed_for_chest(want: ChestKind, limit: u64) -> Option<u64> {
	(0..limit).find(|&seed| chest_kind(seed) == want)
}

fn wood_row(seed: u64, salt: u64) -> usize {
	match mix_seed(seed, salt) % 12 {
		0..=4 => 0,
		5..=8 => 1,
		9 => 3,
		10 => 4,
		_ => {
			if mix_seed(seed, salt.wrapping_add(1)) % 2 == 0 {
				2
			} else {
				5
			}
		}
	}
}

pub fn wood(seed: u64, salt: u64) -> MaterialRef {
	let row = wood_row(seed, salt);
	recipe(
		RECIPE_FURNITURE_WOOD,
		seed ^ salt,
		2.4,
		[
			Color::srgb(WOOD[row][0], WOOD[row][1], WOOD[row][2]),
			Color::srgb(WOOD_ACCENT[row][0], WOOD_ACCENT[row][1], WOOD_ACCENT[row][2]),
			Color::srgb(WOOD_SHADE[row][0], WOOD_SHADE[row][1], WOOD_SHADE[row][2]),
		],
	)
}

pub fn lacquer(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_LACQUER,
		seed ^ salt,
		2.2,
		[rgb(&LACQUER, seed, salt), rgb(&LACQUER_ACCENT, seed, salt.wrapping_add(7))],
	)
}

pub fn metal(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_METAL,
		seed ^ salt,
		2.8,
		[rgb(&METAL, seed, salt), rgb(&METAL_ACCENT, seed, salt.wrapping_add(13))],
	)
}

/// Pewter / iron latch hardware — no carnival brass.
pub fn hardware(seed: u64, salt: u64) -> MaterialRef {
	let row = 2 + (mix_seed(seed, salt) as usize % 2);
	recipe(
		RECIPE_FURNITURE_METAL,
		seed ^ salt,
		2.8,
		[
			Color::srgb(METAL[row][0], METAL[row][1], METAL[row][2]),
			Color::srgb(METAL_ACCENT[row][0], METAL_ACCENT[row][1], METAL_ACCENT[row][2]),
		],
	)
}

/// Frame / leg / back / cabinet body. Stained wood only.
pub fn carcass(seed: u64, salt: u64) -> MaterialRef {
	wood(seed, salt)
}

pub fn cloth(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_CLOTH,
		seed ^ salt,
		3.4,
		[rgb(&CLOTH, seed, salt), rgb(&CLOTH_ACCENT, seed, salt.wrapping_add(5))],
	)
}

pub fn mattress(seed: u64, salt: u64) -> MaterialRef {
	recipe(RECIPE_FURNITURE_SOFT, seed ^ salt, 2.2, [rgb(&MATTRESS, seed, salt)])
}

pub fn marble(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_MARBLE,
		seed ^ salt,
		1.8,
		[
			rgb(&MARBLE, seed, salt),
			rgb(&MARBLE_VEIN, seed, salt.wrapping_add(3)),
			rgb(&MARBLE_SPARK, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn ornate(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_ORNATE,
		seed ^ salt,
		2.4,
		[
			rgb(&WOOD, seed, salt),
			rgb(&GOLD, seed, salt.wrapping_add(5)),
			rgb(&GEM, seed, salt.wrapping_add(9)),
		],
	)
}

pub fn lava(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_LAVA,
		seed ^ salt,
		1.9,
		[
			rgb(&LAVA_COAL, seed, salt),
			rgb(&LAVA_GLOW, seed, salt.wrapping_add(3)),
			rgb(&LAVA_HOT, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn cosmos(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_COSMOS,
		seed ^ salt,
		1.6,
		[
			rgb(&COSMOS_VOID, seed, salt),
			rgb(&COSMOS_NEBULA, seed, salt.wrapping_add(3)),
			rgb(&COSMOS_BLOOM, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn rockadder(seed: u64, salt: u64) -> MaterialRef {
	let set = (mix_seed(seed, salt) as usize) % ROCK_SQUARES.len();
	let [tile, teal, gold] = ROCK_SQUARES[set];
	recipe(
		RECIPE_FURNITURE_ROCKADDER,
		seed ^ salt,
		2.0,
		[
			Color::srgb(tile[0], tile[1], tile[2]),
			Color::srgb(teal[0], teal[1], teal[2]),
			Color::srgb(gold[0], gold[1], gold[2]),
		],
	)
}

pub fn scales(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_SCALES,
		seed ^ salt,
		2.1,
		[
			rgb(&SCALE_BELLY, seed, salt),
			rgb(&SCALE_EDGE, seed, salt.wrapping_add(3)),
			rgb(&SCALE_IRID, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn foliage(seed: u64, salt: u64) -> MaterialRef {
	recipe(RECIPE_FOLIAGE, seed ^ salt, 1.8, [rgb(&FOLIAGE, seed, salt)])
}

/// Shiny fruit skin.
pub fn produce(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_LACQUER,
		seed ^ salt,
		2.0,
		[rgb(&PRODUCE, seed, salt), rgb(&PRODUCE_SHADE, seed, salt.wrapping_add(5))],
	)
}

/// Baked crust.
pub fn crust(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_MARBLE,
		seed ^ salt,
		1.7,
		[rgb(&CRUST, seed, salt), rgb(&CRUST, seed, salt.wrapping_add(3))],
	)
}

/// Enamel appliance body (fridge / basin).
pub fn enamel(seed: u64, salt: u64) -> MaterialRef {
	marble(seed, salt)
}

/// Durham earth swatches through the marble look (stone / dirt coffer).
pub fn terrain(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_MARBLE,
		seed ^ salt,
		1.6,
		[
			rgb(&TERRAIN, seed, salt),
			rgb(&TERRAIN_VEIN, seed, salt.wrapping_add(3)),
			rgb(&TERRAIN_SPARK, seed, salt.wrapping_add(7)),
		],
	)
}

/// Chest field. Recipe from `seed`; trunk/lid salts only change the row.
pub fn chest(seed: u64, salt: u64) -> MaterialRef {
	match chest_kind(seed) {
		ChestKind::Rockadder => rockadder(seed, salt),
		ChestKind::Cosimo => cosmos(seed, salt),
		ChestKind::Foliage => foliage(seed, salt),
		ChestKind::Terrain => terrain(seed, salt),
	}
}

#[cfg(test)]
fn recipe_name(material: &MaterialRef) -> Option<&str> {
	match &material.name {
		material_ref::MaterialId::Name(name) => Some(name.as_str()),
		material_ref::MaterialId::Default => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mattress_paint_is_not_cover_paint() -> anyhow::Result<()> {
		for seed in [1_u64, 7, 99] {
			if mattress(seed, 2) == cloth(seed, 3) {
				return Err(anyhow::anyhow!("seed {seed}: mattress matched covers"));
			}
		}
		Ok(())
	}

	#[test]
	fn wood_table_spans_more_than_honey() -> anyhow::Result<()> {
		let warm = WOOD.iter().any(|row| row[0] > row[2] + 0.25);
		let pale_or_cool = WOOD.iter().any(|row| row[2] + 0.05 >= row[0] || row[1] > row[0]);
		if !warm || !pale_or_cool {
			return Err(anyhow::anyhow!("wood table should mix warm and pale/cool rows"));
		}
		Ok(())
	}

	#[test]
	fn wood_accent_is_a_different_hue() -> anyhow::Result<()> {
		for (i, (base, accent)) in WOOD.iter().zip(WOOD_ACCENT.iter()).enumerate() {
			let scale = base[0] / accent[0].max(1e-4);
			let aligned = (base[1] - accent[1] * scale).abs() + (base[2] - accent[2] * scale).abs();
			if aligned < 0.12 {
				return Err(anyhow::anyhow!(
					"wood row {i} accent is a scale of the base, not a hue shift"
				));
			}
		}
		Ok(())
	}

	#[test]
	fn carcass_is_wood() -> anyhow::Result<()> {
		if (0..40).any(|seed| carcass_kind(seed) != CarcassKind::Wood) {
			return Err(anyhow::anyhow!("carcass should stay wood"));
		}
		if (0..16).any(|seed| recipe_name(&carcass(seed, 1)) != Some(RECIPE_FURNITURE_WOOD)) {
			return Err(anyhow::anyhow!("carcass recipe should be furniture_wood"));
		}
		Ok(())
	}

	#[test]
	fn chest_skins_cover_the_set() -> anyhow::Result<()> {
		for want in
			[ChestKind::Rockadder, ChestKind::Cosimo, ChestKind::Foliage, ChestKind::Terrain]
		{
			if first_seed_for_chest(want, 80).is_none() {
				return Err(anyhow::anyhow!("no seed in 0..80 for {want:?}"));
			}
		}
		Ok(())
	}

	#[test]
	fn recipes_match_the_part_roles() -> anyhow::Result<()> {
		if recipe_name(&marble(3, 1)) != Some(RECIPE_FURNITURE_MARBLE) {
			return Err(anyhow::anyhow!("marble recipe drifted"));
		}
		if recipe_name(&rockadder(3, 1)) != Some(RECIPE_FURNITURE_ROCKADDER) {
			return Err(anyhow::anyhow!("rockadder recipe drifted"));
		}
		if recipe_name(&cosmos(3, 1)) != Some(RECIPE_FURNITURE_COSMOS) {
			return Err(anyhow::anyhow!("cosimo recipe drifted"));
		}
		if recipe_name(&foliage(3, 1)) != Some(RECIPE_FOLIAGE) {
			return Err(anyhow::anyhow!("foliage recipe drifted"));
		}
		if recipe_name(&terrain(3, 1)) != Some(RECIPE_FURNITURE_MARBLE) {
			return Err(anyhow::anyhow!("terrain recipe drifted"));
		}
		Ok(())
	}
}
