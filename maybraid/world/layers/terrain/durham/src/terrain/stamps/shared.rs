//! Shared helpers for per-family jersey guillotine stacks.

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{Vec2, Vec3};
use comproc::guillotine::{Bounds2, Guillotine, GuillotineCuts};
use comproc::noise::config::NoiseConfig;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use noise::Perlin;
use procedural_common::Bounds2 as ProcBounds2;
use terrain_stamps::StampModulation;

use crate::terrain::cell::{cell_coords_for_region, TERRAIN_CELL_VERTICAL_HALF_EXTENT};
use crate::terrain::stamps::configs::FamilyGuillotineConfig;

/// Uniform controller grid with optional XZ origin offset.
#[derive(Debug, Clone, PartialEq)]
pub struct OffsetControllerGrid {
	pub cell_size: f32,
	pub vertical_half_extent: f32,
	pub origin_offset: Vec2,
}

impl OffsetControllerGrid {
	pub fn new(cell_size: f32, origin_offset: Vec2) -> Self {
		Self { cell_size, vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT, origin_offset }
	}

	pub fn cell_bounds(&self, ix: i32, iz: i32) -> Aabb3d {
		let size = self.cell_size.max(1e-3);
		let vy = self.vertical_half_extent.max(size);
		let ox = self.origin_offset.x;
		let oz = self.origin_offset.y;
		Aabb3d::from_min_max(
			Vec3::new(ix as f32 * size + ox, -vy, iz as f32 * size + oz),
			Vec3::new((ix + 1) as f32 * size + ox, vy, (iz + 1) as f32 * size + oz),
		)
	}

	pub fn region_in_grid_space(&self, region: Aabb3d) -> Aabb3d {
		let ox = self.origin_offset.x;
		let oz = self.origin_offset.y;
		Aabb3d::from_min_max(
			Vec3::new(region.min.x - ox, region.min.y, region.min.z - oz),
			Vec3::new(region.max.x - ox, region.max.y, region.max.z - oz),
		)
	}

	/// Controller-cell ids intersecting `region`.
	pub fn cell_ids(&self, region: Aabb3d) -> Vec<OriginalId> {
		cell_coords_for_region(self.region_in_grid_space(region), self.cell_size)
			.map(|(ix, iz)| OriginalId(Id::from_cell(self.cell_bounds(ix, iz))))
			.filter(|OriginalId(id)| id.origin_cell_bounds().is_some_and(|b| region.intersects(&b)))
			.collect()
	}
}

pub fn cell_salt(cell: Aabb3d) -> u32 {
	let ix = cell.min.x.to_bits();
	let iz = cell.min.z.to_bits();
	ix.wrapping_mul(73856093) ^ iz.wrapping_mul(19349663)
}

pub fn bounds2(cell: Aabb3d) -> ProcBounds2 {
	ProcBounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

pub fn family_seed(base_seed: u32, cell: Aabb3d, family_salt: u32) -> u32 {
	base_seed.wrapping_add(cell_salt(cell)).wrapping_add(family_salt)
}

/// Spatially correlated leaf occupancy via bilinear **value noise** at the leaf center.
///
/// Lattice corner hashes are ~uniform on `[0, 1]`, so `likelihood` approximately
/// matches the fraction of leaves accepted. `spatial_correlation` is the lattice
/// spacing (world units). `occupancy_seed` must be **band-stable** (no per-cell
/// salt). `likelihood >= 1` always accepts; `<= 0` always rejects.
pub fn leaf_selected(
	cell: Aabb3d,
	occupancy_seed: u32,
	likelihood: f32,
	spatial_correlation: f32,
) -> bool {
	let p = likelihood.clamp(0.0, 1.0);
	if p >= 1.0 {
		return true;
	}
	if p <= 0.0 {
		return false;
	}
	let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
	occupancy_unit(center, occupancy_seed, spatial_correlation) < p
}

/// Smooth value noise in `[0, 1]` with lattice spacing `spatial_correlation`.
fn occupancy_unit(p: Vec2, seed: u32, spatial_correlation: f32) -> f32 {
	let spacing = spatial_correlation.max(1.0);
	let fx = p.x / spacing;
	let fz = p.y / spacing;
	let x0 = fx.floor() as i32;
	let z0 = fz.floor() as i32;
	let tx = fx - x0 as f32;
	let tz = fz - z0 as f32;
	let sx = tx * tx * (3.0 - 2.0 * tx);
	let sz = tz * tz * (3.0 - 2.0 * tz);
	let n00 = lattice_unit(seed, x0, z0);
	let n10 = lattice_unit(seed, x0 + 1, z0);
	let n01 = lattice_unit(seed, x0, z0 + 1);
	let n11 = lattice_unit(seed, x0 + 1, z0 + 1);
	let nx0 = n00 + (n10 - n00) * sx;
	let nx1 = n01 + (n11 - n01) * sx;
	nx0 + (nx1 - nx0) * sz
}

fn lattice_unit(seed: u32, ix: i32, iz: i32) -> f32 {
	let mut n = seed
		.wrapping_add((ix as u32).wrapping_mul(73856093))
		.wrapping_add((iz as u32).wrapping_mul(19349663));
	n = n.wrapping_mul(0x9E37_79B9) ^ (n >> 16);
	(n >> 8) as f32 / ((u32::MAX >> 8) as f32)
}

/// Band-stable seed for occupancy noise (world seed ⊕ family cut seed ⊕ salt).
pub fn occupancy_seed(base_seed: u32, family_cut_seed: u32, family_salt: u32) -> u32 {
	base_seed
		.wrapping_add(family_cut_seed)
		.wrapping_add(family_salt)
		.wrapping_add(0x0CC_5E1D)
}

/// Sample stamp strength in `[min, max]` from a leaf-stable seed mix.
pub fn sample_strength(seed: u32, min: f32, max: f32) -> f32 {
	let lo = min.min(max);
	let hi = min.max(max);
	if (hi - lo).abs() < 1e-6 {
		return lo.max(0.0);
	}
	let mut n = seed.wrapping_mul(0x9E37_79B9) ^ seed.wrapping_add(0x51A3_E1B5);
	n = n.wrapping_mul(0x85EB_CA6B) ^ (n >> 13);
	let u = (n >> 8) as f32 / ((u32::MAX >> 8) as f32);
	(lo + (hi - lo) * u).max(0.0)
}

fn root_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_vec2(Vec2::new(cell.min.x, cell.min.z), Vec2::new(cell.max.x, cell.max.z))
}

/// Run guillotine cuts for a controller cell from family knobs.
pub fn guillotine_cuts<P>(cell: Aabb3d, config: &FamilyGuillotineConfig<P>) -> GuillotineCuts<2> {
	let seed = config.seed.wrapping_add(cell_salt(cell));
	let noise = NoiseConfig::new(Perlin::default())
		.with_seed(seed)
		.with_frequency(config.noise_frequency)
		.with_amplitude(1.0)
		.with_octaves(1);
	let cutter = Guillotine::new(noise, config.guillotine, config.depth);
	cutter.cut(root_bounds2(cell))
}

pub fn leaf_aabbs(cell: Aabb3d, cuts: &GuillotineCuts<2>) -> Vec<Aabb3d> {
	let vy_min = cell.min.y;
	let vy_max = cell.max.y;
	cuts.regions()
		.map(|leaf| {
			Aabb3d::from_min_max(
				Vec3::new(leaf.min[0], vy_min, leaf.min[1]),
				Vec3::new(leaf.max[0], vy_max, leaf.max[1]),
			)
		})
		.collect()
}

/// A controller that partitions its cell into leaf cells.
pub trait LeafAabbs: Sized {
	fn leaf_aabbs(&self) -> Vec<Aabb3d>;

	/// Sorted, deduplicated leaf ids intersecting `region`, from the
	/// controllers in `region`. Only the controller level materializes.
	fn leaf_ids_in(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>
	where
		Self: hcsg::GenerationScheme,
	{
		let mut ids: Vec<OriginalId> = cx
			.get_or_generate_in::<Self>(region)
			.iter()
			.flat_map(|controller| controller.leaf_aabbs())
			.filter(|leaf| region.intersects(leaf))
			.map(|leaf| OriginalId(Id::from_cell(leaf)))
			.collect();
		ids.sort();
		ids.dedup();
		ids
	}
}

/// Leaf output of any jersey family band, so composers stay band-agnostic.
pub trait StampLeaf {
	fn cell(&self) -> Aabb3d;
	fn modulations(&self) -> &[StampModulation];
}
