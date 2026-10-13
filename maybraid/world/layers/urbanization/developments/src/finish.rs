//! Per-cell urban finish: wall / roof [`MaterialRef`] recipes and palettes.

use bevy::prelude::Color;
use building_shaders::{
	RECIPE_HAY, RECIPE_IRON, RECIPE_STONE, RECIPE_STUCCO, RECIPE_TERRACOTTA, RECIPE_WOOD,
};
use buildings::ConnectingStairwell;
use material_ref::{MaterialId, MaterialRef};
use procedural_common::{NoiseParams, SeededHash};

/// Architectural role used to select a stable family of wall and roof finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DevelopmentFinishRole {
	DefaultUrban,
	SuburbanHome,
	OldCityMarket,
	Highrise,
	Temple,
	WizardsTower,
	Connector,
}

impl DevelopmentFinishRole {
	pub const ALL: [Self; 7] = [
		Self::DefaultUrban,
		Self::SuburbanHome,
		Self::OldCityMarket,
		Self::Highrise,
		Self::Temple,
		Self::WizardsTower,
		Self::Connector,
	];
}

/// Development-wide color family shared by a suburban neighborhood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuburbanPaletteBias {
	Warm,
	Pastoral,
	Cool,
}

impl SuburbanPaletteBias {
	pub(crate) fn select(hash: SeededHash) -> Self {
		match (hash.unit(97) * 3.0).floor() as u32 {
			0 => Self::Warm,
			1 => Self::Pastoral,
			_ => Self::Cool,
		}
	}
}

/// Stairwell surfaces in `wall`, treads in a contrasting material.
pub(crate) fn paint_stairwell(
	stair: ConnectingStairwell,
	wall: MaterialRef,
) -> ConnectingStairwell {
	stair
		.with_surface_material(wall.clone())
		.with_stair_material(contrasting_stair_material(&wall))
}

/// Wood treads against masonry, marble treads against timber.
fn contrasting_stair_material(surround: &MaterialRef) -> MaterialRef {
	match &surround.name {
		MaterialId::Name(name) if name == "wood" => MaterialRef::named("furniture_marble"),
		_ => MaterialRef::named("wood"),
	}
}

/// One wall look and one roof look for a filled development cell.
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopmentFinish {
	pub wall: MaterialRef,
	pub roof: MaterialRef,
}

impl DevelopmentFinish {
	/// Select the original general-purpose urban finish.
	pub fn pick(hash: SeededHash) -> Self {
		Self::pick_for_role(hash, DevelopmentFinishRole::DefaultUrban, false)
	}

	/// Select a deterministic finish for an architectural role.
	///
	/// `wooden` communicates whether a fitted house or hut uses timber wall construction.
	/// Roles that do not fit Shepherds buildings ignore it.
	pub fn pick_for_role(hash: SeededHash, role: DevelopmentFinishRole, wooden: bool) -> Self {
		match role {
			DevelopmentFinishRole::DefaultUrban => Self::pick_default_urban(hash),
			DevelopmentFinishRole::SuburbanHome => {
				Self::pick_suburban_home(hash, SuburbanPaletteBias::select(hash), wooden)
			}
			DevelopmentFinishRole::OldCityMarket => Self::pick_old_city_market(hash, wooden),
			DevelopmentFinishRole::Highrise => Self::pick_highrise(hash),
			DevelopmentFinishRole::Temple => Self::pick_temple(hash),
			DevelopmentFinishRole::WizardsTower => Self::pick_wizards_tower(hash),
			DevelopmentFinishRole::Connector => Self::pick_connector(hash),
		}
	}

	pub(crate) fn pick_suburban_home(
		hash: SeededHash,
		bias: SuburbanPaletteBias,
		wooden: bool,
	) -> Self {
		let (wood_palette, wood_accent, stucco_palette, stucco_accent, terracotta_chance) =
			match bias {
				SuburbanPaletteBias::Warm => (
					[(0.66, 0.45, 0.24), (0.84, 0.68, 0.42)],
					[(0.40, 0.24, 0.12), (0.58, 0.36, 0.18)],
					[(0.76, 0.72, 0.62), (0.92, 0.86, 0.72)],
					[(0.46, 0.48, 0.40), (0.70, 0.60, 0.44)],
					0.78,
				),
				SuburbanPaletteBias::Pastoral => (
					[(0.48, 0.38, 0.20), (0.70, 0.58, 0.32)],
					[(0.28, 0.22, 0.12), (0.48, 0.40, 0.18)],
					[(0.68, 0.72, 0.58), (0.84, 0.82, 0.64)],
					[(0.38, 0.46, 0.32), (0.62, 0.58, 0.36)],
					0.38,
				),
				SuburbanPaletteBias::Cool => (
					[(0.44, 0.38, 0.34), (0.66, 0.58, 0.48)],
					[(0.25, 0.24, 0.23), (0.46, 0.40, 0.34)],
					[(0.65, 0.70, 0.70), (0.84, 0.86, 0.82)],
					[(0.36, 0.42, 0.44), (0.58, 0.58, 0.54)],
					0.58,
				),
			};
		let wall = if wooden {
			custom_recipe_material(
				RECIPE_WOOD,
				hash,
				101,
				wood_palette,
				wood_accent,
				[0.0, 0.58, 0.16],
				0.52,
			)
		} else {
			custom_recipe_material(
				RECIPE_STUCCO,
				hash,
				101,
				stucco_palette,
				stucco_accent,
				[0.0, 0.30, 0.18],
				0.32,
			)
		};
		let roof_recipe =
			if hash.unit(107) < terracotta_chance { RECIPE_TERRACOTTA } else { RECIPE_HAY };
		let roof = role_roof_material(roof_recipe, hash, 109, DevelopmentFinishRole::SuburbanHome);
		Self { wall, roof }
	}

	/// Brighter timber or darker masonry beneath a hay/thatch roof.
	pub fn pick_shepherds(hash: SeededHash, wooden: bool) -> Self {
		let wall = if wooden {
			custom_recipe_material(
				RECIPE_WOOD,
				hash,
				41,
				[(0.58, 0.39, 0.20), (0.76, 0.58, 0.32)],
				[(0.34, 0.20, 0.10), (0.50, 0.31, 0.15)],
				[0.0, 0.68, 0.22],
				0.62,
			)
		} else {
			custom_recipe_material(
				RECIPE_STUCCO,
				hash,
				41,
				[(0.22, 0.24, 0.22), (0.38, 0.36, 0.30)],
				[(0.14, 0.16, 0.15), (0.48, 0.44, 0.34)],
				[0.0, 0.46, 0.38],
				0.34,
			)
		};
		let roof = custom_recipe_material(
			RECIPE_HAY,
			hash,
			47,
			[(0.82, 0.68, 0.32), (0.50, 0.38, 0.16)],
			[(0.92, 0.80, 0.44), (0.34, 0.24, 0.10)],
			[0.0, 0.78, 0.24],
			0.76,
		);
		Self { wall, roof }
	}

	/// Weathered grey-buff ashlar for curtain walls and ramparts.
	pub fn rampart_stone(hash: SeededHash) -> MaterialRef {
		custom_recipe_material(
			RECIPE_STONE,
			hash,
			151,
			[(0.46, 0.44, 0.40), (0.60, 0.56, 0.48)],
			[(0.30, 0.29, 0.27), (0.52, 0.47, 0.38)],
			[0.0, 0.55, 0.45],
			0.35,
		)
	}

	fn pick_default_urban(hash: SeededHash) -> Self {
		let wall_recipe = if hash.unit(19) < 0.5 { RECIPE_STUCCO } else { RECIPE_WOOD };
		let roof_recipe = match (hash.unit(23) * 3.0).floor() as u32 {
			0 => RECIPE_IRON,
			1 => RECIPE_TERRACOTTA,
			_ => RECIPE_HAY,
		};
		Self {
			wall: recipe_material(wall_recipe, hash, 29),
			roof: recipe_material(roof_recipe, hash, 31),
		}
	}

	fn pick_old_city_market(hash: SeededHash, wooden: bool) -> Self {
		let wall = if wooden {
			custom_recipe_material(
				RECIPE_WOOD,
				hash,
				127,
				[(0.34, 0.20, 0.10), (0.58, 0.34, 0.15)],
				[(0.18, 0.10, 0.07), (0.72, 0.46, 0.20)],
				[0.0, 0.76, 0.40],
				0.70,
			)
		} else {
			custom_recipe_material(
				RECIPE_STUCCO,
				hash,
				127,
				[(0.46, 0.34, 0.24), (0.72, 0.56, 0.34)],
				[(0.24, 0.17, 0.13), (0.54, 0.30, 0.18)],
				[0.0, 0.60, 0.46],
				0.58,
			)
		};
		let roof_recipe = match (hash.unit(131) * 3.0).floor() as u32 {
			0 => RECIPE_TERRACOTTA,
			1 => RECIPE_HAY,
			_ => RECIPE_IRON,
		};
		let roof = role_roof_material(roof_recipe, hash, 137, DevelopmentFinishRole::OldCityMarket);
		Self { wall, roof }
	}

	fn pick_highrise(hash: SeededHash) -> Self {
		let wall = custom_recipe_material(
			RECIPE_STUCCO,
			hash,
			151,
			[(0.44, 0.48, 0.50), (0.72, 0.74, 0.70)],
			[(0.20, 0.24, 0.28), (0.52, 0.42, 0.32)],
			[0.0, 0.28, 0.32],
			0.28,
		);
		let roof_recipe = if hash.unit(157) < 0.78 { RECIPE_IRON } else { RECIPE_TERRACOTTA };
		let roof = role_roof_material(roof_recipe, hash, 163, DevelopmentFinishRole::Highrise);
		Self { wall, roof }
	}

	fn pick_temple(hash: SeededHash) -> Self {
		let wall = custom_recipe_material(
			RECIPE_STUCCO,
			hash,
			181,
			[(0.82, 0.73, 0.56), (0.96, 0.90, 0.72)],
			[(0.54, 0.25, 0.18), (0.74, 0.48, 0.22)],
			[0.0, 0.24, 0.16],
			0.26,
		);
		let roof_recipe = if hash.unit(191) < 0.82 { RECIPE_TERRACOTTA } else { RECIPE_HAY };
		let roof = role_roof_material(roof_recipe, hash, 193, DevelopmentFinishRole::Temple);
		Self { wall, roof }
	}

	fn pick_wizards_tower(hash: SeededHash) -> Self {
		let wall_recipe = if hash.unit(211) < 0.76 { RECIPE_STUCCO } else { RECIPE_WOOD };
		let wall = custom_recipe_material(
			wall_recipe,
			hash,
			223,
			[(0.27, 0.30, 0.44), (0.48, 0.42, 0.58)],
			[(0.14, 0.18, 0.30), (0.64, 0.48, 0.28)],
			[0.0, 0.42, 0.30],
			0.40,
		);
		let roof_recipe = if hash.unit(227) < 0.64 { RECIPE_IRON } else { RECIPE_TERRACOTTA };
		let roof = role_roof_material(roof_recipe, hash, 229, DevelopmentFinishRole::WizardsTower);
		Self { wall, roof }
	}

	fn pick_connector(hash: SeededHash) -> Self {
		let wall_recipe = if hash.unit(241) < 0.72 { RECIPE_IRON } else { RECIPE_STUCCO };
		let wall = custom_recipe_material(
			wall_recipe,
			hash,
			251,
			[(0.30, 0.34, 0.36), (0.50, 0.48, 0.42)],
			[(0.16, 0.20, 0.22), (0.68, 0.34, 0.18)],
			[0.0, 0.34, 0.48],
			0.34,
		);
		let roof_recipe = if hash.unit(257) < 0.84 { RECIPE_IRON } else { RECIPE_TERRACOTTA };
		let roof = role_roof_material(roof_recipe, hash, 263, DevelopmentFinishRole::Connector);
		Self { wall, roof }
	}
}

struct RecipeLook {
	palette: [(f32, f32, f32); 2],
	accent: [(f32, f32, f32); 2],
	/// roughness, grain/mottle scale, wear
	scalars: [f32; 3],
	frequency: f32,
}

fn recipe_look(name: &str) -> RecipeLook {
	match name {
		RECIPE_WOOD => RecipeLook {
			palette: [(0.42, 0.28, 0.16), (0.52, 0.36, 0.20)],
			accent: [(0.28, 0.16, 0.10), (0.34, 0.22, 0.12)],
			scalars: [0.0, 0.62, 0.28],
			frequency: 0.55,
		},
		RECIPE_TERRACOTTA => RecipeLook {
			palette: [(0.72, 0.32, 0.18), (0.62, 0.24, 0.14)],
			accent: [(0.48, 0.18, 0.10), (0.82, 0.42, 0.24)],
			scalars: [0.0, 0.48, 0.22],
			frequency: 0.42,
		},
		RECIPE_HAY => RecipeLook {
			palette: [(0.72, 0.58, 0.28), (0.62, 0.50, 0.24)],
			accent: [(0.48, 0.36, 0.16), (0.80, 0.68, 0.38)],
			scalars: [0.0, 0.72, 0.18],
			frequency: 0.70,
		},
		RECIPE_IRON => RecipeLook {
			palette: [(0.38, 0.40, 0.42), (0.28, 0.30, 0.32)],
			accent: [(0.55, 0.22, 0.12), (0.42, 0.18, 0.10)],
			scalars: [0.0, 0.40, 0.45],
			frequency: 0.38,
		},
		_ => RecipeLook {
			palette: [(0.78, 0.72, 0.62), (0.70, 0.64, 0.54)],
			accent: [(0.62, 0.54, 0.42), (0.84, 0.78, 0.68)],
			scalars: [0.0, 0.35, 0.30],
			frequency: 0.48,
		},
	}
}

fn role_roof_material(
	name: &'static str,
	hash: SeededHash,
	salt: u32,
	role: DevelopmentFinishRole,
) -> MaterialRef {
	let (palette, accent, scalars, frequency) = match role {
		DevelopmentFinishRole::SuburbanHome => (
			[(0.70, 0.31, 0.17), (0.88, 0.52, 0.26)],
			[(0.46, 0.20, 0.11), (0.78, 0.66, 0.34)],
			[0.0, 0.50, 0.18],
			0.48,
		),
		DevelopmentFinishRole::OldCityMarket => (
			[(0.42, 0.22, 0.13), (0.68, 0.40, 0.18)],
			[(0.20, 0.16, 0.13), (0.78, 0.52, 0.24)],
			[0.0, 0.68, 0.44],
			0.66,
		),
		DevelopmentFinishRole::Highrise => (
			[(0.24, 0.28, 0.31), (0.52, 0.50, 0.44)],
			[(0.12, 0.16, 0.18), (0.64, 0.30, 0.18)],
			[0.0, 0.30, 0.36],
			0.30,
		),
		DevelopmentFinishRole::Temple => (
			[(0.70, 0.30, 0.16), (0.90, 0.56, 0.24)],
			[(0.48, 0.17, 0.12), (0.88, 0.72, 0.36)],
			[0.0, 0.38, 0.12],
			0.36,
		),
		DevelopmentFinishRole::WizardsTower => (
			[(0.24, 0.28, 0.38), (0.56, 0.38, 0.32)],
			[(0.12, 0.16, 0.24), (0.68, 0.48, 0.26)],
			[0.0, 0.46, 0.34],
			0.44,
		),
		DevelopmentFinishRole::Connector => (
			[(0.28, 0.32, 0.34), (0.46, 0.42, 0.36)],
			[(0.14, 0.18, 0.20), (0.62, 0.30, 0.16)],
			[0.0, 0.34, 0.50],
			0.32,
		),
		DevelopmentFinishRole::DefaultUrban => {
			let look = recipe_look(name);
			(look.palette, look.accent, look.scalars, look.frequency)
		}
	};
	custom_recipe_material(name, hash, salt, palette, accent, scalars, frequency)
}

fn recipe_material(name: &'static str, hash: SeededHash, salt: u32) -> MaterialRef {
	let look = recipe_look(name);
	custom_recipe_material(
		name,
		hash,
		salt,
		look.palette,
		look.accent,
		look.scalars,
		look.frequency,
	)
}

fn custom_recipe_material(
	name: &'static str,
	hash: SeededHash,
	salt: u32,
	palette: [(f32, f32, f32); 2],
	accent: [(f32, f32, f32); 2],
	scalars: [f32; 3],
	frequency: f32,
) -> MaterialRef {
	let pick = hash.unit(salt);
	let mix = hash.unit(salt.wrapping_add(3));
	let base = lerp_rgb(palette[0], palette[1], pick);
	let accent = lerp_rgb(accent[0], accent[1], mix);
	let seed = (hash.unit(salt.wrapping_add(7)) * 10_000.0) as i32;
	MaterialRef::named(name)
		.with_palette([srgb(base), srgb(accent)])
		.with_scalars(scalars)
		.with_noise(NoiseParams {
			seed,
			frequency,
			amplitude: 1.0,
			octaves: 3,
			..NoiseParams::default()
		})
}

fn lerp_rgb(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
	(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

fn srgb(rgb: (f32, f32, f32)) -> Color {
	Color::srgb(rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeSet;

	use building_shaders::is_urban_surface_recipe;
	use material_ref::{MaterialId, MaterialRef};

	use super::*;

	fn recipe_name(material: &MaterialRef) -> anyhow::Result<&str> {
		match &material.name {
			MaterialId::Name(name) => Ok(name),
			MaterialId::Default => Err(anyhow::anyhow!("finish recipe should be named")),
		}
	}

	#[test]
	fn pick_uses_wall_and_roof_recipes() -> anyhow::Result<()> {
		let finish = DevelopmentFinish::pick(SeededHash::new(42));
		let wall = recipe_name(&finish.wall)?;
		let roof = recipe_name(&finish.roof)?;
		assert!(matches!(wall, RECIPE_STUCCO | RECIPE_WOOD));
		assert!(matches!(roof, RECIPE_IRON | RECIPE_TERRACOTTA | RECIPE_HAY));
		assert!(is_urban_surface_recipe(wall));
		assert!(is_urban_surface_recipe(roof));
		assert_eq!(finish.wall.palette.len(), 2);
		assert_eq!(finish.roof.palette.len(), 2);
		Ok(())
	}

	#[test]
	fn role_picks_are_stable_for_the_same_hash() {
		for role in DevelopmentFinishRole::ALL {
			let a = DevelopmentFinish::pick_for_role(SeededHash::new(7), role, true);
			let b = DevelopmentFinish::pick_for_role(SeededHash::new(7), role, true);
			assert_eq!(a, b, "{role:?}");
		}
	}

	#[test]
	fn roles_produce_distinct_palettes_and_roof_recipes() -> anyhow::Result<()> {
		let finishes: Vec<_> = DevelopmentFinishRole::ALL
			.into_iter()
			.map(|role| DevelopmentFinish::pick_for_role(SeededHash::new(91), role, true))
			.collect();
		let distinct_palettes = finishes
			.iter()
			.enumerate()
			.filter(|(index, finish)| {
				finishes[..*index].iter().all(|earlier| {
					earlier.wall.palette != finish.wall.palette
						|| earlier.roof.palette != finish.roof.palette
				})
			})
			.count();
		assert!(distinct_palettes >= 6);

		for role in DevelopmentFinishRole::ALL {
			let mut roof_recipes = BTreeSet::new();
			for seed in 0..128 {
				let finish = DevelopmentFinish::pick_for_role(SeededHash::new(seed), role, true);
				roof_recipes.insert(recipe_name(&finish.roof)?.to_owned());
			}
			assert!(roof_recipes.len() >= 2, "{role:?} should vary roof recipes");
		}
		Ok(())
	}

	#[test]
	fn shepherds_finish_is_wood_or_dark_masonry_with_hay() {
		let wood = DevelopmentFinish::pick_shepherds(SeededHash::new(11), true);
		let stone = DevelopmentFinish::pick_shepherds(SeededHash::new(11), false);
		assert!(matches!(&wood.wall.name, MaterialId::Name(n) if n == RECIPE_WOOD));
		assert!(matches!(&stone.wall.name, MaterialId::Name(n) if n == RECIPE_STUCCO));
		assert!(matches!(&wood.roof.name, MaterialId::Name(n) if n == RECIPE_HAY));
		assert_ne!(wood.wall.palette, stone.wall.palette);
	}
}
