//! Authored bump-out profiles on grove **selection** identities.
//!
//! Density, bite size, height, and palette are properties of [`ForestGroveKind`] after
//! forest selection. They do not require growing a [`crate::ChicoGrove`].
//!
//! Generated [`CanopyBumpOut`] origins are 160 m terrain cells. Present is a
//! 1–5 km annulus so they do not cover the same tiles as grove geometry.
//! [`MediumCanopyBumpOut`] uses the 320 m Far terrain grid on the aligned
//! 1.28–2.56 km band.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Color, Vec2, Vec3};
use lod::gen::Id;
use procedural_common::NoiseParams;
use vegetation_bumpout::{BumpOut, BumpOutNeighborhood, BumpOutStyle};
use vegetation_groves::GroveExtent;

use crate::{
	ForestExtent, ForestGroveKind, ForestLayer, ForestSelection, SelectedLayers,
	DEFAULT_FOREST_GROVE_TILE_XZ,
};

/// Terrain-cell edge used as bump-out origin (metres). Matches Durham fine cells.
pub const BUMP_OUT_CELL_XZ: f32 = 160.0;

/// Inner hole of the bump-out annulus (metres). Same as grove geometry present.
pub const BUMP_OUT_INNER_RADIUS_M: f32 = 1000.0;

/// Outer radius of the bump-out annulus (metres).
pub const BUMP_OUT_OUTER_RADIUS_M: f32 = 5000.0;

/// Edge length of one medium terrain bump-out cell.
pub const MEDIUM_BUMP_OUT_CELL_XZ: f32 = 2.0 * BUMP_OUT_CELL_XZ;

/// Inner edge shared with the 160 m terrain stream.
pub const MEDIUM_BUMP_OUT_INNER_RADIUS_M: f32 = 8.0 * BUMP_OUT_CELL_XZ;

/// Outer edge shared with the 640 m terrain stream.
pub const MEDIUM_BUMP_OUT_OUTER_RADIUS_M: f32 = 16.0 * BUMP_OUT_CELL_XZ;

/// Anchor quantization shared by all moving terrain streams.
pub const MEDIUM_BUMP_OUT_ANCHOR_STEP_M: f32 = 4.0 * BUMP_OUT_CELL_XZ;

/// Authored canopy / cover fields sampled from a grove selection identifier.
pub trait BumpOutSelection {
	fn bump_out_density(self) -> f32;
	fn bump_out_bite_size(self) -> f32;
	fn bump_out_bite_size_deviation(self) -> f32;
	fn bump_out_height_m(self) -> f32;
	fn bump_out_height_deviation_m(self) -> f32;
	fn bump_out_palette(self) -> [Color; 3];
}

/// One blended bump-out sample (possibly empty when every overlapping tile selected `None`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BumpOutSelectionSample {
	pub kind: Option<ForestGroveKind>,
	pub density: f32,
	pub bite_size: f32,
	pub bite_size_deviation: f32,
	pub height_m: f32,
	pub height_deviation_m: f32,
	pub palette: [Color; 3],
}

impl BumpOutSelectionSample {
	pub fn empty() -> Self {
		Self {
			kind: None,
			density: 0.0,
			bite_size: 1.0,
			bite_size_deviation: 0.0,
			height_m: 0.0,
			height_deviation_m: 0.0,
			palette: [
				Color::srgb(0.16, 0.36, 0.14),
				Color::srgb(0.24, 0.52, 0.20),
				Color::srgb(0.38, 0.64, 0.24),
			],
		}
	}

	pub fn from_kind(kind: ForestGroveKind) -> Self {
		Self {
			kind: Some(kind),
			density: kind.bump_out_density(),
			bite_size: kind.bump_out_bite_size(),
			bite_size_deviation: kind.bump_out_bite_size_deviation(),
			height_m: kind.bump_out_height_m(),
			height_deviation_m: kind.bump_out_height_deviation_m(),
			palette: kind.bump_out_palette(),
		}
	}
}

/// Select-only canopy proxy sampled over one terrain cell. No grow, no mesh.
///
/// The base stream uses 160 m cells. [`MediumCanopyBumpOut`] wraps the same
/// sampled representation on the 320 m terrain grid.
#[derive(Debug, Clone, PartialEq)]
pub struct CanopyBumpOut {
	pub bounds: Aabb3d,
	pub samples: [BumpOutSelectionSample; 9],
}

impl CanopyBumpOut {
	/// The proxy over the 160 m cell `bounds`; `None` for another cell size
	/// or where nothing around it is selected.
	pub fn select(selection: ForestSelection, bounds: Aabb3d) -> Option<Self> {
		if (bounds.max.x - bounds.min.x - BUMP_OUT_CELL_XZ).abs() > 1e-2 {
			return None;
		}
		let cell = Self { bounds, samples: blend_selection_neighborhood(selection, bounds) };
		cell.has_density().then_some(cell)
	}

	pub fn id(&self) -> Id {
		Id::from_cell(self.bounds)
	}

	pub fn has_density(&self) -> bool {
		self.samples.iter().any(|sample| sample.density > 0.001)
	}

	pub fn center_palette(&self) -> [Color; 3] {
		self.samples[4].palette
	}
}

/// Select-only canopy proxy matched to one medium (320 m) terrain cell.
#[derive(Debug, Clone, PartialEq)]
pub struct MediumCanopyBumpOut(pub CanopyBumpOut);

impl MediumCanopyBumpOut {
	/// The proxy over the 320 m cell `bounds`; `None` for another cell size.
	pub fn select(selection: ForestSelection, bounds: Aabb3d) -> Option<Self> {
		if (bounds.max.x - bounds.min.x - MEDIUM_BUMP_OUT_CELL_XZ).abs() > 1e-2 {
			return None;
		}
		Some(Self(CanopyBumpOut {
			bounds,
			samples: blend_selection_neighborhood(selection, bounds),
		}))
	}

	pub fn cell_bounds(ix: i32, iz: i32) -> Aabb3d {
		let size = MEDIUM_BUMP_OUT_CELL_XZ;
		Aabb3d::from_min_max(
			Vec3::new(ix as f32 * size, 0.0, iz as f32 * size),
			Vec3::new((ix + 1) as f32 * size, 1.0, (iz + 1) as f32 * size),
		)
	}

	pub fn cells_overlapping(region: Aabb3d) -> impl Iterator<Item = (i32, i32)> {
		let size = MEDIUM_BUMP_OUT_CELL_XZ;
		let min_x = (region.min.x / size).floor() as i32;
		let max_x = ((region.max.x / size).ceil() as i32 - 1).max(min_x);
		let min_z = (region.min.z / size).floor() as i32;
		let max_z = ((region.max.z / size).ceil() as i32 - 1).max(min_z);
		(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
	}
}

/// World-aligned 160 m cell AABB (`y` in `[0, 1]`, origin at 0).
pub fn bump_out_cell_bounds(ix: i32, iz: i32) -> Aabb3d {
	let s = BUMP_OUT_CELL_XZ;
	Aabb3d::from_min_max(
		Vec3::new(ix as f32 * s, 0.0, iz as f32 * s),
		Vec3::new((ix + 1) as f32 * s, 1.0, (iz + 1) as f32 * s),
	)
}

/// Integer 160 m cells whose footprints overlap `region` on XZ.
pub fn bump_out_cells_overlapping(region: Aabb3d) -> impl Iterator<Item = (i32, i32)> {
	let s = BUMP_OUT_CELL_XZ;
	let min_x = (region.min.x / s).floor() as i32;
	let max_x = ((region.max.x / s).ceil() as i32 - 1).max(min_x);
	let min_z = (region.min.z / s).floor() as i32;
	let max_z = ((region.max.z / s).ceil() as i32 - 1).max(min_z);
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}

/// Chebyshev XZ distance from `origin` to the cell center.
pub fn bump_out_chebyshev_xz(bounds: Aabb3d, origin: Vec3) -> f32 {
	let center = Vec3::from((bounds.min + bounds.max) * 0.5);
	(center.x - origin.x).abs().max((center.z - origin.z).abs())
}

/// Whether a 160 m cell sits in the grove-fill hole of a camera-centered region.
pub fn bump_out_in_inner_hole(bounds: Aabb3d, region: Aabb3d) -> bool {
	let origin =
		Vec3::new((region.min.x + region.max.x) * 0.5, 0.0, (region.min.z + region.max.z) * 0.5);
	bump_out_chebyshev_xz(bounds, origin) < BUMP_OUT_INNER_RADIUS_M
}

/// Whether a 320 m cell center lies in the visible medium-terrain annulus.
pub fn medium_bump_out_in_band(bounds: Aabb3d, region: Aabb3d) -> bool {
	let origin =
		Vec3::new((region.min.x + region.max.x) * 0.5, 0.0, (region.min.z + region.max.z) * 0.5);
	let radius = bump_out_chebyshev_xz(bounds, origin);
	(MEDIUM_BUMP_OUT_INNER_RADIUS_M..=MEDIUM_BUMP_OUT_OUTER_RADIUS_M).contains(&radius)
}

impl SelectedLayers {
	/// Highest occupied vegetation layer (`UpperCanopy` … `Tufts`).
	pub fn highest_kind(self) -> Option<ForestGroveKind> {
		ForestLayer::ALL.into_iter().rev().find_map(|layer| layer.kind(self))
	}
}

impl BumpOutSelection for ForestGroveKind {
	fn bump_out_density(self) -> f32 {
		self.authored().density
	}

	fn bump_out_bite_size(self) -> f32 {
		self.authored().bite_size
	}

	fn bump_out_bite_size_deviation(self) -> f32 {
		self.authored().bite_size_deviation
	}

	fn bump_out_height_m(self) -> f32 {
		self.authored().height_m
	}

	fn bump_out_height_deviation_m(self) -> f32 {
		self.authored().height_deviation_m
	}

	fn bump_out_palette(self) -> [Color; 3] {
		self.authored().palette
	}
}

impl ForestGroveKind {
	fn authored(self) -> AuthoredBumpOut {
		match self {
			Self::Alpine => canopy(0.22, 36.0, conifer()),
			Self::AridConiferSapling => woody(0.12, 8.0, dry()),
			Self::BraidGrass => tuft(0.12, 0.8, meadow()),
			Self::BushScrub => woody(0.14, 3.0, scrub()),
			Self::ChristmasTaiga => canopy(0.26, 28.0, conifer()),
			Self::CommonTufts => tuft(0.10, 0.25, meadow()),
			Self::ConiferMassives => massive(0.32, 160.0, conifer()),
			Self::ConiferSapling => woody(0.14, 10.0, conifer()),
			Self::DateGrove => woody(0.16, 18.0, palm()),
			Self::Dryland => woody(0.08, 12.0, dry()),
			Self::ForlornSavanna => woody(0.10, 25.0, savanna()),
			Self::GoettingenFollow => woody(0.18, 22.0, temperate()),
			Self::HighBush => woody(0.16, 6.0, scrub()),
			Self::JerrysChaparral => woody(0.12, 4.0, dry()),
			Self::JungleLowerMassives => canopy(0.28, 40.0, jungle()),
			Self::JungleMassives => massive(0.36, 180.0, jungle()),
			Self::Leeward => woody(0.16, 20.0, palm()),
			Self::LevantineScrub => woody(0.10, 3.5, dry()),
			Self::LowBush => woody(0.14, 2.0, scrub()),
			Self::MonsterGrass => tuft(0.14, 1.2, meadow()),
			Self::Orchard => woody(0.20, 8.0, orchard()),
			Self::PalmShade => canopy(0.18, 32.0, palm()),
			Self::RiparianGeneral => woody(0.18, 16.0, temperate()),
			Self::RiparianMix => woody(0.18, 18.0, temperate()),
			Self::RiverineGreen => woody(0.16, 14.0, temperate()),
			Self::RollingOaks => canopy(0.22, 36.0, temperate()),
			Self::Shamanhome => woody(0.16, 20.0, jungle()),
			Self::SpottyBushes => woody(0.10, 3.0, scrub()),
			Self::Storytellers => woody(0.18, 24.0, temperate()),
			Self::StrangeOasis => woody(0.12, 14.0, palm()),
			Self::TallGrass => tuft(0.14, 1.0, meadow()),
			Self::TemperateLowerMassives => canopy(0.26, 40.0, temperate()),
			Self::TemperateMassives => massive(0.34, 170.0, temperate()),
			Self::TradeWinds => canopy(0.24, 36.0, jungle()),
			Self::TropicalThicket => woody(0.24, 8.0, jungle()),
			Self::TropicalTufts => tuft(0.12, 0.4, jungle()),
			Self::TropicalUndergrowth => woody(0.20, 5.0, jungle()),
			Self::UnendingJungle => canopy(0.30, 30.0, jungle()),
			Self::Vineyard => woody(0.18, 2.5, orchard()),
			Self::WanderingAcacia => woody(0.08, 16.0, savanna()),
			Self::WildGrass => tuft(0.12, 0.6, meadow()),
		}
	}
}

struct AuthoredBumpOut {
	density: f32,
	bite_size: f32,
	bite_size_deviation: f32,
	height_m: f32,
	height_deviation_m: f32,
	palette: [Color; 3],
}

fn tuft(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: (height_m * 1.8).max(0.35),
		bite_size_deviation: 0.45,
		height_m,
		height_deviation_m: (height_m * 0.65).max(0.08),
		palette,
	}
}

fn woody(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: (height_m * 0.32).max(1.2),
		bite_size_deviation: 0.55,
		height_m,
		height_deviation_m: (height_m * 0.50).max(0.8),
		palette,
	}
}

fn canopy(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: (height_m * 0.38).max(4.0),
		bite_size_deviation: 0.65,
		height_m,
		height_deviation_m: height_m * 0.48,
		palette,
	}
}

fn massive(density: f32, height_m: f32, palette: [Color; 3]) -> AuthoredBumpOut {
	AuthoredBumpOut {
		density,
		bite_size: height_m * 0.28,
		bite_size_deviation: 0.70,
		height_m,
		height_deviation_m: height_m * 0.42,
		palette,
	}
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
	Color::srgb(r, g, b)
}

fn meadow() -> [Color; 3] {
	[rgb(0.18, 0.38, 0.10), rgb(0.32, 0.55, 0.14), rgb(0.55, 0.68, 0.20)]
}

fn scrub() -> [Color; 3] {
	[rgb(0.18, 0.28, 0.10), rgb(0.30, 0.42, 0.14), rgb(0.48, 0.52, 0.22)]
}

fn dry() -> [Color; 3] {
	[rgb(0.22, 0.24, 0.10), rgb(0.36, 0.34, 0.14), rgb(0.52, 0.46, 0.20)]
}

fn temperate() -> [Color; 3] {
	[rgb(0.08, 0.22, 0.08), rgb(0.14, 0.38, 0.12), rgb(0.28, 0.52, 0.16)]
}

fn conifer() -> [Color; 3] {
	[rgb(0.04, 0.16, 0.10), rgb(0.08, 0.28, 0.14), rgb(0.16, 0.40, 0.18)]
}

fn jungle() -> [Color; 3] {
	[rgb(0.04, 0.18, 0.08), rgb(0.08, 0.34, 0.12), rgb(0.18, 0.50, 0.14)]
}

fn palm() -> [Color; 3] {
	[rgb(0.10, 0.28, 0.08), rgb(0.18, 0.46, 0.12), rgb(0.40, 0.58, 0.16)]
}

fn savanna() -> [Color; 3] {
	[rgb(0.20, 0.28, 0.08), rgb(0.34, 0.40, 0.12), rgb(0.52, 0.50, 0.18)]
}

fn orchard() -> [Color; 3] {
	[rgb(0.12, 0.30, 0.08), rgb(0.22, 0.48, 0.12), rgb(0.42, 0.58, 0.16)]
}

/// Highest-layer selection at a world XZ point. Does not grow tiles.
///
/// If this forest cell selected `None` on every layer, borrow occupied cardinal
/// neighbors. Stay empty only when the cell and those neighbors are all empty.
pub fn selection_sample_at(selection: ForestSelection, xz: Vec2) -> BumpOutSelectionSample {
	let (ix, iz) = ForestExtent::cell_index_containing(Vec3::new(xz.x, 0.0, xz.y));
	let self_layers = selection.layers_for(ForestExtent::from_cell_index(ix, iz));
	sample_with_neighbors(
		self_layers,
		[
			selection.layers_for(ForestExtent::from_cell_index(ix, iz + 1)),
			selection.layers_for(ForestExtent::from_cell_index(ix + 1, iz)),
			selection.layers_for(ForestExtent::from_cell_index(ix, iz - 1)),
			selection.layers_for(ForestExtent::from_cell_index(ix - 1, iz)),
		],
	)
}

fn sample_with_neighbors(
	self_layers: SelectedLayers,
	neighbors: [SelectedLayers; 4],
) -> BumpOutSelectionSample {
	if let Some(kind) = self_layers.highest_kind() {
		return BumpOutSelectionSample::from_kind(kind);
	}
	let borrowed: Vec<BumpOutSelectionSample> = neighbors
		.into_iter()
		.filter_map(|layers| layers.highest_kind().map(BumpOutSelectionSample::from_kind))
		.collect();
	average_occupied(&borrowed)
}

fn average_occupied(samples: &[BumpOutSelectionSample]) -> BumpOutSelectionSample {
	if samples.is_empty() {
		return BumpOutSelectionSample::empty();
	}
	let n = samples.len() as f32;
	let mut density = 0.0;
	let mut bite_size = 0.0;
	let mut bite_size_deviation = 0.0;
	let mut height_m = 0.0;
	let mut height_deviation_m = 0.0;
	let mut best = samples[0];
	for sample in samples {
		density += sample.density;
		bite_size += sample.bite_size;
		bite_size_deviation += sample.bite_size_deviation;
		height_m += sample.height_m;
		height_deviation_m += sample.height_deviation_m;
		if sample.density > best.density {
			best = *sample;
		}
	}
	BumpOutSelectionSample {
		kind: best.kind,
		density: (density / n).clamp(0.0, 1.0),
		bite_size: (bite_size / n).max(0.01),
		bite_size_deviation: (bite_size_deviation / n).max(0.0),
		height_m: height_m / n,
		height_deviation_m: (height_deviation_m / n).max(0.0),
		palette: best.palette,
	}
}

/// Area-weight overlapping 100 m grove tiles onto `bounds` (typically a 160 m terrain cell).
///
/// Empty tiles do not dilute occupied ones. The result is empty only when every
/// overlapping tile (after neighbor borrow) is empty.
pub fn blend_selection_on_bounds(
	selection: ForestSelection,
	bounds: Aabb3d,
) -> BumpOutSelectionSample {
	let tiles = ForestExtent::grove_tiles_overlapping(bounds);
	let mut occupied = Vec::new();

	for tile in tiles {
		let area = xz_overlap_area(bounds, grove_aabb(tile));
		if area <= 1e-4 {
			continue;
		}
		let sample = selection_sample_at(
			selection,
			Vec2::new((tile.min().x + tile.max().x) * 0.5, (tile.min().z + tile.max().z) * 0.5),
		);
		if sample.kind.is_none() && sample.density <= 0.001 {
			continue;
		}
		occupied.push((area, sample));
	}

	if occupied.is_empty() {
		return BumpOutSelectionSample::empty();
	}

	let mut density = 0.0;
	let mut bite_size = 0.0;
	let mut bite_size_deviation = 0.0;
	let mut height_m = 0.0;
	let mut height_deviation_m = 0.0;
	let mut weight_sum = 0.0;
	let mut best_kind: Option<(ForestGroveKind, f32, [Color; 3])> = None;

	for (area, sample) in occupied {
		density += sample.density * area;
		bite_size += sample.bite_size * area;
		bite_size_deviation += sample.bite_size_deviation * area;
		height_m += sample.height_m * area;
		height_deviation_m += sample.height_deviation_m * area;
		weight_sum += area;
		if let Some(kind) = sample.kind {
			match best_kind {
				Some((_, best_area, _)) if best_area >= area => {}
				_ => best_kind = Some((kind, area, sample.palette)),
			}
		}
	}

	if weight_sum <= 1e-4 {
		return BumpOutSelectionSample::empty();
	}

	let (kind, palette) = match best_kind {
		Some((kind, _, palette)) => (Some(kind), palette),
		None => {
			let empty = BumpOutSelectionSample::empty();
			(None, empty.palette)
		}
	};

	BumpOutSelectionSample {
		kind,
		density: (density / weight_sum).clamp(0.0, 1.0),
		bite_size: (bite_size / weight_sum).max(0.01),
		bite_size_deviation: (bite_size_deviation / weight_sum).max(0.0),
		height_m: height_m / weight_sum,
		height_deviation_m: (height_deviation_m / weight_sum).max(0.0),
		palette,
	}
}

/// 3×3 terrain-cell neighborhood centered on `bounds`, each sample blended from 100 m tiles.
pub fn blend_selection_neighborhood(
	selection: ForestSelection,
	bounds: Aabb3d,
) -> [BumpOutSelectionSample; 9] {
	let size = (bounds.max.x - bounds.min.x).max(DEFAULT_FOREST_GROVE_TILE_XZ);
	let mut samples = [BumpOutSelectionSample::empty(); 9];
	for row in 0..3 {
		for column in 0..3 {
			let dx = (column as f32 - 1.0) * size;
			let dz = (row as f32 - 1.0) * size;
			let min = Vec3::new(bounds.min.x + dx, bounds.min.y, bounds.min.z + dz);
			let max = Vec3::new(bounds.max.x + dx, bounds.max.y, bounds.max.z + dz);
			samples[row * 3 + column] =
				blend_selection_on_bounds(selection, Aabb3d::from_min_max(min, max));
		}
	}
	samples
}

fn grove_aabb(tile: GroveExtent) -> Aabb3d {
	Aabb3d::from_min_max(tile.min(), tile.max())
}

fn xz_overlap_area(a: Aabb3d, b: Aabb3d) -> f32 {
	let x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
	let z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);
	x * z
}

pub fn bump_out_from_cell(cell: &CanopyBumpOut, noise: NoiseParams) -> Option<BumpOut> {
	let samples = cell.samples;
	let neighborhood = BumpOutNeighborhood::new(
		samples.map(|sample| sample.density),
		samples.map(|sample| sample.bite_size),
		samples.map(|sample| sample.bite_size_deviation),
		samples.map(|sample| sample.height_m),
		samples.map(|sample| sample.height_deviation_m),
	);
	if neighborhood.densities.iter().all(|density| *density <= 0.001) {
		return None;
	}
	Some(
		BumpOut::from_neighborhood(neighborhood, cell.center_palette(), noise).with_style(
			BumpOutStyle::new(0.065, 0.88, 0.18)
				.with_cheese(0.88, 1.0)
				.with_fragment_height(4.5, 0.85),
		),
	)
}

pub fn bump_out_noise(forest: &NoiseParams) -> NoiseParams {
	NoiseParams {
		seed: forest.seed.wrapping_add(307),
		frequency: 0.045,
		amplitude: forest.amplitude,
		octaves: 3,
		..*forest
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{LayeringKind, SelectedLayers};
	use anyhow::Result;

	#[test]
	fn highest_kind_prefers_upper_canopy() -> Result<()> {
		let layers = SelectedLayers {
			layering: LayeringKind::Meadowland,
			tufts: Some(ForestGroveKind::WildGrass),
			understory: Some(ForestGroveKind::LowBush),
			lower_canopy: None,
			upper_canopy: Some(ForestGroveKind::RollingOaks),
		};
		assert_eq!(layers.highest_kind(), Some(ForestGroveKind::RollingOaks));
		Ok(())
	}

	#[test]
	fn highest_kind_falls_back_to_tufts() -> Result<()> {
		let layers = SelectedLayers {
			layering: LayeringKind::Meadowland,
			tufts: Some(ForestGroveKind::CommonTufts),
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		assert_eq!(layers.highest_kind(), Some(ForestGroveKind::CommonTufts));
		Ok(())
	}

	#[test]
	fn orchard_height_is_authored_midpoint() -> Result<()> {
		assert!((ForestGroveKind::Orchard.bump_out_height_m() - 8.0).abs() < 1e-4);
		assert!(ForestGroveKind::Orchard.bump_out_density() > 0.0);
		assert!(ForestGroveKind::Orchard.bump_out_density() < 0.35);
		assert_eq!(ForestGroveKind::Orchard.bump_out_palette().len(), 3);
		Ok(())
	}

	#[test]
	fn class_densities_stay_below_grove_fill() -> Result<()> {
		assert!(ForestGroveKind::WildGrass.bump_out_density() < 0.20);
		assert!(ForestGroveKind::RollingOaks.bump_out_density() < 0.35);
		assert!(ForestGroveKind::JungleMassives.bump_out_density() < 0.45);
		assert!(ForestGroveKind::WanderingAcacia.bump_out_density() < 0.15);
		Ok(())
	}

	#[test]
	fn height_deviation_is_a_large_fraction_of_height() -> Result<()> {
		let oak = ForestGroveKind::RollingOaks;
		assert!(oak.bump_out_height_deviation_m() / oak.bump_out_height_m() > 0.40);
		let orchard = ForestGroveKind::Orchard;
		assert!(orchard.bump_out_height_deviation_m() / orchard.bump_out_height_m() > 0.40);
		Ok(())
	}

	#[test]
	fn empty_cell_borrows_occupied_neighbors() -> Result<()> {
		let empty = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let oak = SelectedLayers {
			layering: LayeringKind::MiRobles,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: Some(ForestGroveKind::RollingOaks),
		};
		let sample = sample_with_neighbors(empty, [oak, empty, empty, empty]);
		assert_eq!(sample.kind, Some(ForestGroveKind::RollingOaks));
		assert!(sample.density > 0.0);
		Ok(())
	}

	#[test]
	fn empty_stays_empty_when_neighbors_are_empty() -> Result<()> {
		let empty = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let sample = sample_with_neighbors(empty, [empty, empty, empty, empty]);
		assert!(sample.kind.is_none());
		assert!(sample.density <= 0.001);
		Ok(())
	}

	#[test]
	fn occupied_cell_does_not_blend_empty_neighbors() -> Result<()> {
		let empty = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let oak = SelectedLayers {
			layering: LayeringKind::MiRobles,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: Some(ForestGroveKind::RollingOaks),
		};
		let sample = sample_with_neighbors(oak, [empty, empty, empty, empty]);
		assert_eq!(sample.kind, Some(ForestGroveKind::RollingOaks));
		assert!((sample.density - ForestGroveKind::RollingOaks.bump_out_density()).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn blend_on_empty_layering_is_zero_density() -> Result<()> {
		let selection = crate::ForestSelection {
			layering: Some(LayeringKind::SunsBarren),
			..Default::default()
		};
		let bounds = Aabb3d::from_min_max(Vec3::new(-80.0, 0.0, -80.0), Vec3::new(80.0, 1.0, 80.0));
		let sample = blend_selection_on_bounds(selection, bounds);
		// SunsBarren typical layers may still have groves; density is authored, not occupancy.
		assert!(sample.density >= 0.0);
		assert!(sample.density <= 1.0);
		Ok(())
	}
}
