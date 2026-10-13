use bevy::prelude::*;
use lod_cascade::Chunk;
use procedural_common::NoiseParams;
use render_item::mesh::{IdentifiedMesh, MeshId};
use render_item::sdf::cpu_shot::CpuShotBuilder;
use render_item::NormalizeChunk;
use sdf::Sdf;
use terrain_chunk_ref::TerrainChunkRef;
use vegetation_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};

use crate::{PresenterLayer, TileCoordinate};

pub(crate) type PlaygroundTerrainBuilder = CpuShotBuilder<PlaygroundTerrain>;

pub(crate) const TILE_RADIUS: i32 = 2;
const TILE_SIZE: f32 = 52.0;
const TERRAIN_MIN_Y: f32 = -18.0;
const TERRAIN_MAX_Y: f32 = 42.0;
const TILE_RES_2: u8 = 5;

#[derive(Clone, Debug)]
pub(crate) struct PlaygroundTerrain;

impl PlaygroundTerrain {
	fn height_at(x: f32, z: f32) -> f32 {
		3.8 * (x * 0.035).sin() * (z * 0.028).cos()
			+ 1.7 * ((x + z) * 0.075).sin()
			+ 0.8 * (z * 0.11).cos()
	}
}

impl Sdf for PlaygroundTerrain {
	fn distance(&self, p: Vec3) -> f32 {
		p.y - Self::height_at(p.x, p.z)
	}
}

impl NormalizeChunk for PlaygroundTerrain {}

impl IdentifiedMesh for PlaygroundTerrain {
	fn id(&self) -> MeshId {
		MeshId::new("chico-bumpout-playground-terrain-v1".into())
	}
}

#[derive(Debug, Clone, Copy)]
struct NeighborhoodSample {
	density: f32,
	bite_size: f32,
	bite_size_deviation: f32,
	average_height: f32,
	height_deviation: f32,
}

impl PresenterLayer {
	fn sample(self, coordinate: IVec2) -> NeighborhoodSample {
		let x = coordinate.x as f32;
		let z = coordinate.y as f32;
		match self {
			Self::GroundCover => {
				let density = (0.62
					+ 0.20 * (x * 0.93 + z * 0.37).sin()
					+ 0.15 * (z * 1.11 - x * 0.29).cos())
				.clamp(0.08, 0.98);
				NeighborhoodSample {
					density,
					bite_size: 4.5 + 3.5 * (0.5 + 0.5 * (x * 0.71 + z * 0.53).sin()),
					bite_size_deviation: 0.2 + 0.65 * (0.5 + 0.5 * (x * 0.43 - z * 0.97).cos()),
					average_height: (1.8
						+ 1.65 * (x * 1.17 - z * 0.63).sin()
						+ 0.75 * (z * 1.31 + x * 0.41).cos())
					.clamp(0.1, 4.2),
					height_deviation: 0.15 + 0.8 * (0.5 + 0.5 * (x * 0.83 + z * 0.59).cos()),
				}
			}
			Self::CanopyProxy => {
				let density = (0.50
					+ 0.32 * (x * 0.78 + z * 0.44).sin()
					+ 0.20 * (z * 0.91 - x * 0.32).cos())
				.clamp(0.03, 0.98);
				NeighborhoodSample {
					density,
					bite_size: 9.0 + 15.0 * (0.5 + 0.5 * (x * 0.57 - z * 0.81).sin()),
					bite_size_deviation: 0.25 + 1.0 * (0.5 + 0.5 * (x * 0.37 + z * 0.73).cos()),
					average_height: (25.0
						+ 14.0 * (x * 1.17 - z * 0.63).sin()
						+ 10.0 * (z * 1.03 + x * 0.41).cos())
					.clamp(5.0, 48.0),
					height_deviation: 2.0 + 7.0 * (0.5 + 0.5 * (x * 0.69 + z * 0.47).cos()),
				}
			}
			Self::Terrain => NeighborhoodSample {
				density: 0.0,
				bite_size: 1.0,
				bite_size_deviation: 0.0,
				average_height: 0.0,
				height_deviation: 0.0,
			},
		}
	}

	fn neighborhood(self, center: IVec2) -> BumpOutNeighborhood {
		let mut densities = [0.0; 9];
		let mut bite_sizes = [0.0; 9];
		let mut bite_size_deviations = [0.0; 9];
		let mut average_heights = [0.0; 9];
		let mut height_deviations = [0.0; 9];
		for row in 0..3 {
			for column in 0..3 {
				let coordinate = center + IVec2::new(column as i32 - 1, row as i32 - 1);
				let index = row * 3 + column;
				let sample = self.sample(coordinate);
				densities[index] = sample.density;
				bite_sizes[index] = sample.bite_size;
				bite_size_deviations[index] = sample.bite_size_deviation;
				average_heights[index] = sample.average_height;
				height_deviations[index] = sample.height_deviation;
			}
		}
		BumpOutNeighborhood::new(
			densities,
			bite_sizes,
			bite_size_deviations,
			average_heights,
			height_deviations,
		)
	}

	fn bump_out(self, tile: IVec2) -> BumpOut {
		match self {
			Self::GroundCover => BumpOut::from_neighborhood(
				self.neighborhood(tile),
				[
					Color::srgb(0.12, 0.28, 0.08),
					Color::srgb(0.22, 0.48, 0.12),
					Color::srgb(0.52, 0.66, 0.18),
				],
				NoiseParams::from_scalar(101.0, 0.085, 0.0, 2),
			)
			.with_style(
				BumpOutStyle::new(0.055, 0.96, 0.42)
					.with_cheese(0.72, 1.35)
					.with_fragment_height(5.0, 0.14),
			),
			Self::CanopyProxy => BumpOut::from_neighborhood(
				self.neighborhood(tile),
				[
					Color::srgb(0.035, 0.16, 0.055),
					Color::srgb(0.08, 0.31, 0.10),
					Color::srgb(0.23, 0.48, 0.13),
				],
				NoiseParams::from_scalar(307.0, 0.045, 0.0, 3),
			)
			.with_style(
				BumpOutStyle::new(0.065, 0.88, 0.18)
					.with_cheese(0.88, 1.0)
					.with_fragment_height(4.5, 0.85),
			),
			Self::Terrain => unreachable!("terrain does not use a bump-out material"),
		}
	}
}

pub(crate) fn setup_tiles(
	mut commands: Commands,
	mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
	let terrain_material = standard_materials.add(StandardMaterial {
		base_color: Color::srgb(0.33, 0.30, 0.20),
		perceptual_roughness: 0.98,
		..default()
	});
	for z in -TILE_RADIUS..=TILE_RADIUS {
		for x in -TILE_RADIUS..=TILE_RADIUS {
			let tile = IVec2::new(x, z);
			let horizontal_min =
				Vec2::new((x as f32 - 0.5) * TILE_SIZE, (z as f32 - 0.5) * TILE_SIZE);
			let chunk = Chunk::from_min_max(
				Vec3::new(horizontal_min.x, TERRAIN_MIN_Y, horizontal_min.y),
				Vec3::new(
					horizontal_min.x + TILE_SIZE,
					TERRAIN_MAX_Y,
					horizontal_min.y + TILE_SIZE,
				),
				None,
			);
			let terrain_ref =
				TerrainChunkRef::new(CpuShotBuilder::new(PlaygroundTerrain), chunk, TILE_RES_2);
			let source_transform = terrain_ref.transform();

			commands.spawn((
				PresenterLayer::Terrain,
				TileCoordinate(tile),
				terrain_ref.clone(),
				MeshMaterial3d(terrain_material.clone()),
				source_transform,
				Visibility::default(),
			));

			for layer in [PresenterLayer::GroundCover, PresenterLayer::CanopyProxy] {
				let entity = layer.bump_out(tile).spawn(&mut commands, terrain_ref.clone());
				commands.entity(entity).insert((layer, TileCoordinate(tile)));
			}
		}
	}
}
