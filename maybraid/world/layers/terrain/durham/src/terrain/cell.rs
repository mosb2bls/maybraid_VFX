//! Terrain cell size and origin tiling helpers.

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{IVec2, UVec2, Vec3};
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use lod::LodSceneLevel;

/// Naturescapes cascade `min_size`.
pub const NATURESCAPES_MIN_SIZE: f32 = 20.0;

/// Naturescapes `grid_multiple_2` (chunk size = [`NATURESCAPES_MIN_SIZE`] × 2^this).
pub const NATURESCAPES_GRID_MULTIPLE_2: u8 = 3;

/// Naturescapes `grid_radius` X/Z (inclusive range is `[-r, r]` → `2r + 1` cells).
pub const NATURESCAPES_GRID_RADIUS_XZ: i32 = 12;

/// Default edge length of a procedural terrain origin cell (world units).
///
/// Matches naturescapes grid chunk size: `min_size * 2^grid_multiple_2` = 20 × 8.
pub const TERRAIN_CELL_SIZE: f32 =
	NATURESCAPES_MIN_SIZE * (1_u32 << NATURESCAPES_GRID_MULTIPLE_2) as f32;

/// Macro-cell edge length (`4 ×` terrain cell). Used by jersey guillotine step windows.
pub const MACRO_CELL_SIZE: f32 = TERRAIN_CELL_SIZE * 4.0;

/// Mesh overflow past each Terrain cell face, in voxels at the cell's `res_2`.
///
/// `0` keeps the cascade chunk flush with the cell AABB (no shared skirt).
/// Non-zero values expand XZ by that many sample pitches (and Y by
/// [`TERRAIN_MESH_PAD_Y_SLOPE`] × that pad) so neighbors share a strip.
pub const TERRAIN_MESH_PAD_VOXELS: f32 = 0.0;

/// Extra Y overflow as a multiple of the XZ pad (covers steep ridge crests).
pub const TERRAIN_MESH_PAD_Y_SLOPE: f32 = 4.0;

/// Default cell count along +X / +Z (`2 * grid_radius + 1`).
pub const TERRAIN_CELL_EXTENTS_XZ: u32 = (2 * NATURESCAPES_GRID_RADIUS_XZ + 1) as u32;

/// Default cell-grid origin so the request region is centered like naturescapes at the world origin.
pub const TERRAIN_CELL_ORIGIN: IVec2 =
	IVec2::new(-NATURESCAPES_GRID_RADIUS_XZ, -NATURESCAPES_GRID_RADIUS_XZ);

/// Default vertical half-extent so cells cover naturescapes-scale heightfields
/// (`height_scale=500`, bedrock at `-4 * height_scale`).
pub const TERRAIN_CELL_VERTICAL_HALF_EXTENT: f32 = 2000.0;

/// Vertical half-extent for presentation / producer queries. Generation cells
/// keep [`TERRAIN_CELL_VERTICAL_HALF_EXTENT`]; this only widens keep/cull boxes
/// so tall peaks stay inside the producer volume.
pub const TERRAIN_PRESENT_VERTICAL_HALF_EXTENT: f32 = 8_000.0;

/// Large AABB for universal (`Id::Universal`) generation deps.
pub fn universal_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::splat(-1_000_000.0), Vec3::splat(1_000_000.0))
}

/// Shared scheme for a world singleton at [`Id::Universal`] derived from other
/// universals (or a constant). Root inputs nothing can derive use
/// [`lod::seeded_root`] instead.
macro_rules! derived_universal_scheme {
	($T:ty, |$cx:ident| $native:expr) => {
		impl lod::hcsg::GenerationScheme for $T {
			lod::hcsg_index_scale!($crate::terrain::index::DURHAM_INDEX_SCALE);
			fn original_ids_for(
				_cx: &mut lod::hcsg::GenerationContext,
				_region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				vec![lod::gen::OriginalId::universal()]
			}

			fn build_with_id(
				$cx: &mut lod::hcsg::GenerationContext,
				id: lod::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				if id != lod::gen::Id::Universal {
					return None;
				}
				Some(($native?, $crate::terrain::cell::universal_bounds()))
			}
		}
	};
}

pub(crate) use derived_universal_scheme;

/// A Universal layout that tiles regions into cell ids.
///
/// Grid roots (`PreWatershedTerrain`, `HydroComplexCell`, band controllers)
/// discover their ids with [`Self::original_cell_ids_for`]; everything stacked
/// on a root reuses its ids through the context.
pub trait CellTiling: Sized {
	/// Ids of this layout's cells intersecting `region`.
	fn cell_ids(&self, region: Aabb3d) -> Vec<OriginalId>;

	/// [`Self::cell_ids`] on the context's Universal layout; empty if it cannot be built.
	fn origin_ids_in(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>
	where
		Self: hcsg::GenerationScheme,
	{
		cx.get_or_generate::<Self>(Id::Universal)
			.map(|layout| layout.cell_ids(region))
			.unwrap_or_default()
	}
}

/// Optional coarser origin cells wrapping an inner footprint.
///
/// Each outer cell has edge length [`Self::cell_size`]. [`Self::rows`] Chebyshev
/// rings are placed around the previously covered footprint; cells that overlap
/// that footprint are omitted so grids abut without double-covering.
#[derive(Debug, Clone, PartialEq)]
pub struct OuterCellRing {
	/// Edge length of each outer cell in world units.
	pub cell_size: f32,
	/// Number of Chebyshev rows outside the current covered footprint.
	pub rows: i32,
}

/// One moving terrain-presentation stream on a globally aligned cell grid.
///
/// High is the innermost category around the stream anchor. Near
/// (`high_inner_radius == 0`) draws High. Far / background use High as an
/// empty hole inside `high_inner_radius` and draw Medium on
/// `high_inner_radius..=high_outer_radius`. Playable Far / Background inset
/// that hole by at least one [`Self::anchor_step`] so Medium overlaps the
/// next-finer rim through a snap. Cells stay generated for
/// [`Self::cull_margin`] past both edges. Low is empty retain or cull.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerrainCellRing {
	/// Edge length of cells in this stream.
	pub cell_size: f32,
	/// Cascade mesh resolution for this stream (`2^res_2` samples per axis).
	pub res_2: u8,
	/// World-space lattice used to quantize the shared moving anchor.
	pub anchor_step: f32,
	/// Inner edge of the drawn ring (`0` for the near disk).
	pub high_inner_radius: f32,
	/// Outer edge of the drawn ring (or near disk).
	pub high_outer_radius: f32,
	/// Empty retention band outside both ring edges.
	pub cull_margin: f32,
}

impl TerrainCellRing {
	/// Chebyshev XZ radius of `cell_center` from the aligned stream anchor.
	pub fn radius_from(self, cell_center: Vec3, anchor: Vec3) -> f32 {
		let anchor = self.aligned_anchor(anchor);
		let delta = cell_center - anchor;
		delta.x.abs().max(delta.z.abs())
	}

	/// High is the hole (or near disk). Medium is the far / background ring.
	pub fn level_for(self, cell_center: Vec3, anchor: Vec3) -> LodSceneLevel {
		let radius = self.radius_from(cell_center, anchor);
		if self.draws_high() {
			if radius <= self.high_outer_radius {
				LodSceneLevel::High
			} else if radius <= self.high_outer_radius + self.cull_margin {
				LodSceneLevel::Medium
			} else {
				LodSceneLevel::Low
			}
		} else if radius < self.high_inner_radius {
			LodSceneLevel::High
		} else if radius <= self.high_outer_radius {
			LodSceneLevel::Medium
		} else if radius <= self.high_outer_radius + self.cull_margin {
			LodSceneLevel::Low
		} else {
			LodSceneLevel::Low
		}
	}

	/// True when this stream's visible category is High (no inner hole).
	pub fn draws_high(self) -> bool {
		self.high_inner_radius <= 0.0
	}

	/// Visible band: High on the near disk, Medium on far / background rings.
	pub fn draws_level(self, level: LodSceneLevel) -> bool {
		if self.draws_high() {
			level == LodSceneLevel::High
		} else {
			level == LodSceneLevel::Medium
		}
	}

	/// True when the cell stays in generate / collider keep.
	pub fn retains_cell_center(self, cell_center: Vec3, anchor: Vec3) -> bool {
		let radius = self.radius_from(cell_center, anchor);
		let inner_keep = (self.high_inner_radius - self.cull_margin).max(0.0);
		let outer_keep = self.high_outer_radius + self.cull_margin;
		radius >= inner_keep && radius <= outer_keep
	}

	/// True when this is the near (collision) stream.
	pub fn seeds_collision(self) -> bool {
		self.high_inner_radius <= 0.0
	}

	pub fn aligned_anchor(self, anchor: Vec3) -> Vec3 {
		let step = self.anchor_step.max(1e-3);
		Vec3::new((anchor.x / step).round() * step, 0.0, (anchor.z / step).round() * step)
	}

	/// The retained cells around `anchor` as boxes: one for the near disk,
	/// four strips around a far ring's hole.
	///
	/// Exact when the anchor step and both edges sit on this ring's lattice,
	/// as the playable rings do. Boxes are inset by a sliver so none touches a
	/// cell outside the ring.
	pub fn regions_around(self, anchor: Vec3) -> Vec<Aabb3d> {
		let a = self.aligned_anchor(anchor);
		let outer = self.high_outer_radius + self.cull_margin;
		let inner = (self.high_inner_radius - self.cull_margin).max(0.0);
		let inset = 0.01 * self.cell_size;
		let y = TERRAIN_PRESENT_VERTICAL_HALF_EXTENT;
		let span = |x0: f32, x1: f32, z0: f32, z1: f32| {
			Aabb3d::from_min_max(
				Vec3::new(a.x + x0 + inset, -y, a.z + z0 + inset),
				Vec3::new(a.x + x1 - inset, y, a.z + z1 - inset),
			)
		};
		if inner <= 0.0 {
			return vec![span(-outer, outer, -outer, outer)];
		}
		vec![
			span(-outer, outer, inner, outer),
			span(-outer, outer, -outer, -inner),
			span(-outer, -inner, -inner, inner),
			span(inner, outer, -inner, inner),
		]
	}

	/// Ids of this ring's lattice cells intersecting `region`.
	pub fn cell_ids(self, region: Aabb3d, vertical_half_extent: f32) -> Vec<OriginalId> {
		cell_coords_for_region(region, self.cell_size)
			.map(|(ix, iz)| {
				OriginalId(Id::from_cell(cell_bounds(ix, iz, self.cell_size, vertical_half_extent)))
			})
			.collect()
	}
}

/// Layout for tiling terrain origin cells in the XZ plane.
///
/// Materialized once under [`Id::Universal`] via [`lod::gen::GenerationScheme`].
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct TerrainCellLayout {
	/// Edge length of each origin cell in world units.
	pub cell_size: f32,
	/// Half-extent along Y for cell bounds / SDF sampling volumes.
	pub vertical_half_extent: f32,
	/// Cell-grid coordinates of the min corner (XZ).
	pub origin: IVec2,
	/// Number of cells along +X and +Z from [`Self::origin`].
	pub extents: UVec2,
	/// Optional nested macro rings (increasing cell size), outside the fine grid.
	pub outer_rings: Vec<OuterCellRing>,
	/// Optional moving near / far / background streams.
	///
	/// When non-empty, origin-cell production uses these annuli instead of the
	/// bounded fine footprint and [`Self::outer_rings`].
	pub stream_rings: Vec<TerrainCellRing>,
}

impl Default for TerrainCellLayout {
	fn default() -> Self {
		Self {
			cell_size: TERRAIN_CELL_SIZE,
			vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT,
			origin: TERRAIN_CELL_ORIGIN,
			extents: UVec2::new(TERRAIN_CELL_EXTENTS_XZ, TERRAIN_CELL_EXTENTS_XZ),
			outer_rings: Vec::new(),
			stream_rings: Vec::new(),
		}
	}
}

impl TerrainCellLayout {
	/// Fine-grid request AABB only (ignores [`Self::outer_rings`]).
	pub fn fine_request_region(&self) -> Aabb3d {
		let size = self.cell_size.max(1e-3);
		let vy = self.vertical_half_extent.max(size);
		let min = Vec3::new(self.origin.x as f32 * size, -vy, self.origin.y as f32 * size);
		let max = Vec3::new(
			(self.origin.x + self.extents.x as i32) as f32 * size,
			vy,
			(self.origin.y + self.extents.y as i32) as f32 * size,
		);
		Aabb3d::from_min_max(min, max)
	}

	/// Full request AABB including nested [`Self::outer_rings`] padding.
	pub fn request_region(&self) -> Aabb3d {
		if let Some(radius) = self
			.stream_rings
			.iter()
			.map(|ring| ring.high_outer_radius + ring.cull_margin)
			.max_by(f32::total_cmp)
		{
			let center = self.fine_region_center_xz();
			let vy = self.vertical_half_extent.max(radius);
			return Aabb3d::from_min_max(
				Vec3::new(center.x - radius, -vy, center.z - radius),
				Vec3::new(center.x + radius, vy, center.z + radius),
			);
		}
		let mut region = self.fine_request_region();
		for outer in &self.outer_rings {
			if outer.rows > 0 {
				let pad = outer.rows as f32 * outer.cell_size.max(1e-3);
				region = expand_aabb_xz(region, pad);
			}
		}
		region
	}

	/// [`Self::request_region`] with presentation Y covering ±8 km.
	pub fn presentation_region(&self) -> Aabb3d {
		let region = self.request_region();
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		Aabb3d::from_min_max(
			Vec3::new(min.x, -TERRAIN_PRESENT_VERTICAL_HALF_EXTENT, min.z),
			Vec3::new(max.x, TERRAIN_PRESENT_VERTICAL_HALF_EXTENT, max.z),
		)
	}

	/// World-space center of the fine grid on XZ (Y = 0). Stream annuli use this
	/// as their moving origin; [`Self::request_region`] may be larger.
	pub fn region_center_xz(&self) -> Vec3 {
		self.fine_region_center_xz()
	}

	fn fine_region_center_xz(&self) -> Vec3 {
		let region = self.fine_request_region();
		let min = Vec3::from(region.min);
		let max = Vec3::from(region.max);
		Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5)
	}

	/// Moving stream policy matching a cell edge length.
	pub fn stream_ring_for_cell_size(&self, cell_size: f32) -> Option<TerrainCellRing> {
		self.stream_rings
			.iter()
			.copied()
			.find(|ring| (ring.cell_size - cell_size).abs() < 1e-3)
	}

	/// True when no other stream draws past `ring`'s outer radius.
	pub fn is_outermost_stream_ring(&self, ring: TerrainCellRing) -> bool {
		self.stream_rings
			.iter()
			.all(|other| other.high_outer_radius <= ring.high_outer_radius + 1e-3)
	}

	pub fn is_streamed(&self) -> bool {
		!self.stream_rings.is_empty()
	}

	/// Origin ids the shared runtime discovers in `region`: a streamed
	/// layout's finest ring lattice, else [`CellTiling::cell_ids`].
	///
	/// Coarser rings are discovered by their own streams (see
	/// [`crate::Streamed`]), so no id depends on where a stream is anchored.
	pub fn origin_ids(&self, region: Aabb3d) -> Vec<OriginalId> {
		match self.stream_rings.iter().min_by(|a, b| a.cell_size.total_cmp(&b.cell_size)) {
			Some(finest) => finest.cell_ids(region, self.vertical_half_extent),
			None => self.cell_ids(region),
		}
	}

	/// [`Self::origin_ids`] on the context's Universal layout.
	pub fn origin_ids_in(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.get_or_generate::<Self>(Id::Universal)
			.map(|layout| layout.origin_ids(region))
			.unwrap_or_default()
	}

	/// Origin-cell ids covering `(x, z)` on every lattice this layout tiles
	/// (fine grid, outer rings, and stream rings).
	///
	/// Streamed discovery stays on the finest ring ([`Self::origin_ids`]);
	/// callers that must attach to a coarser presented cell (far / background
	/// bump-outs) use this list so they can `get_or_generate` that size.
	pub fn origin_ids_covering_xz(&self, x: f32, z: f32) -> Vec<OriginalId> {
		std::iter::once(self.cell_size)
			.chain(self.outer_rings.iter().map(|outer| outer.cell_size))
			.chain(self.stream_rings.iter().map(|ring| ring.cell_size))
			.map(|size| {
				let size = size.max(1e-3);
				OriginalId(Id::from_cell(cell_bounds(
					(x / size).floor() as i32,
					(z / size).floor() as i32,
					size,
					self.vertical_half_extent,
				)))
			})
			.collect()
	}

	/// Fine-grid cell containing `xz` (Y ignored).
	pub fn fine_cell_containing_xz(&self, xz: Vec3) -> IVec2 {
		let size = self.cell_size.max(1e-3);
		IVec2::new((xz.x / size).floor() as i32, (xz.z / size).floor() as i32)
	}

	/// Bounds of the fine origin cell containing `(x, z)`.
	pub fn fine_cell_bounds_containing(&self, x: f32, z: f32) -> Aabb3d {
		let cell = self.fine_cell_containing_xz(Vec3::new(x, 0.0, z));
		cell_bounds(cell.x, cell.y, self.cell_size, self.vertical_half_extent)
	}

	/// Fine-grid origin so the window stays centered on the cell that contains `xz`.
	pub fn origin_centered_on_xz(&self, xz: Vec3) -> IVec2 {
		let cell = self.fine_cell_containing_xz(xz);
		let half_x = self.extents.x as i32 / 2;
		let half_z = self.extents.y as i32 / 2;
		IVec2::new(cell.x - half_x, cell.y - half_z)
	}

	/// Recenter the fine-grid origin on `xz`. Returns whether [`Self::origin`] changed.
	pub fn recenter_on_xz(&mut self, xz: Vec3) -> bool {
		let origin = self.origin_centered_on_xz(xz);
		if origin == self.origin {
			false
		} else {
			self.origin = origin;
			true
		}
	}

	/// Chebyshev radius of fine-grid cell `(ix, iz)` from the window's center cell.
	pub fn fine_cell_radius(&self, ix: i32, iz: i32) -> i32 {
		let cx = self.origin.x + self.extents.x as i32 / 2;
		let cz = self.origin.y + self.extents.y as i32 / 2;
		(ix - cx).abs().max((iz - cz).abs())
	}

	/// Macro-cell edge length, preserving the default `MACRO / TERRAIN` ratio.
	pub fn macro_cell_size(&self) -> f32 {
		self.cell_size * (MACRO_CELL_SIZE / TERRAIN_CELL_SIZE)
	}

	/// Macro tiling params derived from this layout.
	pub fn macro_layout(&self) -> MacroCellLayout {
		MacroCellLayout {
			cell_size: self.macro_cell_size(),
			vertical_half_extent: self.vertical_half_extent,
		}
	}
}

// Seeded by the terrain window producer, which recenters it on the viewer.
lod::seeded_root!(TerrainCellLayout);

/// Layout for macro-scale tiling (jersey stamp size defaults).
///
/// Derived from [`TerrainCellLayout::macro_layout`]; not a separate generation type.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroCellLayout {
	pub cell_size: f32,
	pub vertical_half_extent: f32,
}

impl Default for MacroCellLayout {
	fn default() -> Self {
		Self { cell_size: MACRO_CELL_SIZE, vertical_half_extent: TERRAIN_CELL_VERTICAL_HALF_EXTENT }
	}
}

/// Build an origin-cell AABB from integer cell coordinates on the XZ plane.
pub fn cell_bounds(ix: i32, iz: i32, cell_size: f32, vertical_half_extent: f32) -> Aabb3d {
	let size = cell_size.max(1e-3);
	let vy = vertical_half_extent.max(size);
	let min = Vec3::new(ix as f32 * size, -vy, iz as f32 * size);
	let max = Vec3::new((ix + 1) as f32 * size, vy, (iz + 1) as f32 * size);
	Aabb3d::from_min_max(min, max)
}

/// Integer cell coordinates covering a region on XZ (Y ignored for tiling).
///
/// Uses half-open style on the max edge (`ceil(max/size) - 1`), so a query whose
/// max lies exactly on a cell boundary does not include the next cell.
pub fn cell_coords_for_region(region: Aabb3d, cell_size: f32) -> impl Iterator<Item = (i32, i32)> {
	let size = cell_size.max(1e-3);
	let min_x = (region.min.x / size).floor() as i32;
	let max_x = (region.max.x / size).ceil() as i32 - 1;
	let min_z = (region.min.z / size).floor() as i32;
	let max_z = (region.max.z / size).ceil() as i32 - 1;
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}

/// Cell coordinates for closed AABB overlap with half-open tiles `[i·s,(i+1)·s]`.
pub fn cell_coords_for_region_inclusive(
	region: Aabb3d,
	cell_size: f32,
) -> impl Iterator<Item = (i32, i32)> {
	cell_coords_for_region_inclusive_halo(region, cell_size, 0)
}

/// Closed overlap plus a Moore halo of `halo` cells (for softmask / apron reach).
///
/// Closed face rule: `max >= i·s` and `min <= (i+1)·s`, then expand by `halo`.
pub fn cell_coords_for_region_inclusive_halo(
	region: Aabb3d,
	cell_size: f32,
	halo: i32,
) -> impl Iterator<Item = (i32, i32)> {
	let size = cell_size.max(1e-3);
	let halo = halo.max(0);
	let min_x = (region.min.x / size).ceil() as i32 - 1 - halo;
	let max_x = (region.max.x / size).floor() as i32 + halo;
	let min_z = (region.min.z / size).ceil() as i32 - 1 - halo;
	let max_z = (region.max.z / size).floor() as i32 + halo;
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}

/// Expand an AABB on XZ only (Y unchanged).
pub fn expand_aabb_xz(region: Aabb3d, pad: f32) -> Aabb3d {
	expand_aabb_xz_y(region, pad, 0.0)
}

/// Expand an AABB with independent XZ and Y pads.
pub fn expand_aabb_xz_y(region: Aabb3d, pad_xz: f32, pad_y: f32) -> Aabb3d {
	let pad_xz = pad_xz.max(0.0);
	let pad_y = pad_y.max(0.0);
	Aabb3d::from_min_max(
		Vec3::new(region.min.x - pad_xz, region.min.y - pad_y, region.min.z - pad_xz),
		Vec3::new(region.max.x + pad_xz, region.max.y + pad_y, region.max.z + pad_xz),
	)
}

impl CellTiling for TerrainCellLayout {
	/// Origin-cell [`OriginalId`]s covering `region` for a materialized layout.
	///
	/// Fine-grid cells come only from [`TerrainCellLayout::fine_request_region`],
	/// not from the padded request AABB. Each [`TerrainCellLayout::outer_rings`]
	/// entry then tiles only its own expanded frame (not the remaining pad), so
	/// 2× cells do not fill the 4× ring and 160 m cells do not fill either ring.
	///
	/// When [`TerrainCellLayout::stream_rings`] is set, each annulus emits only its
	/// own cell size inside its retain band.
	fn cell_ids(&self, region: Aabb3d) -> Vec<OriginalId> {
		if !self.stream_rings.is_empty() {
			let min = Vec3::from(region.min);
			let max = Vec3::from(region.max);
			let anchor = Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5);
			let mut ids = std::collections::HashSet::new();
			for ring in &self.stream_rings {
				let outer = ring.high_outer_radius + ring.cull_margin;
				let bounds = Aabb3d::from_min_max(
					Vec3::new(anchor.x - outer, region.min.y, anchor.z - outer),
					Vec3::new(anchor.x + outer, region.max.y, anchor.z + outer),
				);
				for (ix, iz) in cell_coords_for_region(bounds, ring.cell_size) {
					let cell = cell_bounds(ix, iz, ring.cell_size, self.vertical_half_extent);
					let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
					if region.intersects(&cell) && ring.retains_cell_center(center, anchor) {
						ids.insert(OriginalId(Id::from_cell(cell)));
					}
				}
			}
			let mut ids: Vec<_> = ids.into_iter().collect();
			ids.sort();
			return ids;
		}

		let fine = self.fine_request_region();
		let mut ids: Vec<OriginalId> = cell_coords_for_region(fine, self.cell_size)
			.filter_map(|(ix, iz)| {
				let bounds = cell_bounds(ix, iz, self.cell_size, self.vertical_half_extent);
				region.intersects(&bounds).then(|| OriginalId(Id::from_cell(bounds)))
			})
			.collect();

		let mut covered = fine;
		for outer in &self.outer_rings {
			if outer.rows <= 0 {
				continue;
			}
			let g = outer.cell_size.max(1e-3);
			let hole: std::collections::HashSet<(i32, i32)> =
				cell_coords_for_region(covered, g).collect();
			let expanded = expand_aabb_xz(covered, outer.rows as f32 * g);
			let outer_ids = cell_coords_for_region(expanded, g).filter_map(|(ix, iz)| {
				if hole.contains(&(ix, iz)) {
					return None;
				}
				let bounds = cell_bounds(ix, iz, g, self.vertical_half_extent);
				if !region.intersects(&bounds) {
					return None;
				}
				Some(OriginalId(Id::from_cell(bounds)))
			});
			ids.extend(outer_ids);
			covered = expanded;
		}

		ids
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn world_like_layout() -> TerrainCellLayout {
		let mut layout = TerrainCellLayout::default();
		layout.origin = IVec2::new(-16, -16);
		layout.extents = UVec2::new(32, 32);
		layout.outer_rings = vec![
			OuterCellRing { cell_size: 2.0 * TERRAIN_CELL_SIZE, rows: 2 },
			OuterCellRing { cell_size: 4.0 * TERRAIN_CELL_SIZE, rows: 1 },
		];
		layout
	}

	fn edge_len(id: &OriginalId) -> f32 {
		let bounds = id.0.origin_cell_bounds().expect("origin cell");
		bounds.max.x - bounds.min.x
	}

	fn count_edge(ids: &[OriginalId], expected: f32) -> usize {
		ids.iter().filter(|id| (edge_len(id) - expected).abs() < 1e-3).count()
	}

	#[test]
	fn padded_request_does_not_paint_fine_cells_on_macro_rings() {
		let layout = world_like_layout();
		let ids = layout.cell_ids(layout.request_region());
		let fine = TERRAIN_CELL_SIZE;
		assert_eq!(count_edge(&ids, fine), 32 * 32);
		assert_eq!(count_edge(&ids, 2.0 * fine), 144);
		assert_eq!(count_edge(&ids, 4.0 * fine), 44);
		assert_eq!(ids.len(), 32 * 32 + 144 + 44);
	}

	#[test]
	fn recenter_slides_fine_origin_without_changing_extent() {
		let mut layout = world_like_layout();
		let size = layout.cell_size;
		assert!(layout.recenter_on_xz(Vec3::new(10.0 * size, 0.0, 0.0)));
		assert_eq!(layout.origin, IVec2::new(-6, -16));
		assert_eq!(layout.extents, UVec2::new(32, 32));
		assert_eq!(layout.fine_cell_radius(10, 0), 0);
		assert_eq!(layout.fine_cell_radius(12, 0), 2);
		assert!(!layout.recenter_on_xz(Vec3::new(10.0 * size, 0.0, 0.0)));
		let ids = layout.cell_ids(layout.request_region());
		assert_eq!(count_edge(&ids, TERRAIN_CELL_SIZE), 32 * 32);
		assert!(ids.len() > 32 * 32);
	}

	fn streamed_layout() -> TerrainCellLayout {
		let mut layout = TerrainCellLayout::default();
		layout.stream_rings = vec![
			TerrainCellRing {
				cell_size: TERRAIN_CELL_SIZE,
				res_2: 5,
				anchor_step: 4.0 * TERRAIN_CELL_SIZE,
				high_inner_radius: 0.0,
				high_outer_radius: 8.0 * TERRAIN_CELL_SIZE,
				cull_margin: 2.0 * TERRAIN_CELL_SIZE,
			},
			TerrainCellRing {
				cell_size: 2.0 * TERRAIN_CELL_SIZE,
				res_2: 4,
				anchor_step: 4.0 * TERRAIN_CELL_SIZE,
				high_inner_radius: 8.0 * TERRAIN_CELL_SIZE,
				high_outer_radius: 16.0 * TERRAIN_CELL_SIZE,
				cull_margin: 2.0 * TERRAIN_CELL_SIZE,
			},
			TerrainCellRing {
				cell_size: 4.0 * TERRAIN_CELL_SIZE,
				res_2: 3,
				anchor_step: 4.0 * TERRAIN_CELL_SIZE,
				high_inner_radius: 16.0 * TERRAIN_CELL_SIZE,
				high_outer_radius: 24.0 * TERRAIN_CELL_SIZE,
				cull_margin: 8.0 * TERRAIN_CELL_SIZE,
			},
		];
		layout
	}

	#[test]
	fn streamed_rings_emit_one_cell_size_per_annulus() {
		let layout = streamed_layout();
		let ids = layout.cell_ids(layout.request_region());
		let fine = TERRAIN_CELL_SIZE;
		assert!(count_edge(&ids, fine) > 0);
		assert!(count_edge(&ids, 2.0 * fine) > 0);
		assert!(count_edge(&ids, 4.0 * fine) > 0);
		assert_eq!(
			count_edge(&ids, fine) + count_edge(&ids, 2.0 * fine) + count_edge(&ids, 4.0 * fine),
			ids.len()
		);
	}

	#[test]
	fn streamed_near_high_is_collision_scale() {
		let layout = streamed_layout();
		assert!(layout.stream_rings[0].seeds_collision());
		assert!(!layout.stream_rings[1].seeds_collision());
		assert!(!layout.stream_rings[2].seeds_collision());
		assert_eq!(layout.stream_rings[0].res_2, 5);
		assert_eq!(layout.stream_rings[1].res_2, 4);
		assert_eq!(layout.stream_rings[2].res_2, 3);
	}

	#[test]
	fn far_cells_inside_near_are_empty_high() {
		let layout = streamed_layout();
		let far = layout.stream_rings[1];
		assert!(!far.draws_high());
		assert!(far.draws_level(LodSceneLevel::Medium));
		assert!(!far.draws_level(LodSceneLevel::High));
		assert_eq!(far.level_for(Vec3::ZERO, Vec3::ZERO), LodSceneLevel::High);
		assert_eq!(
			far.level_for(Vec3::X * 7.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			LodSceneLevel::High
		);
		assert_eq!(
			far.level_for(Vec3::X * 12.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			LodSceneLevel::Medium
		);
	}

	#[test]
	fn covering_xz_includes_every_stream_lattice() {
		let layout = streamed_layout();
		let ids = layout.origin_ids_covering_xz(12.0 * TERRAIN_CELL_SIZE, 0.0);
		assert!(count_edge(&ids, TERRAIN_CELL_SIZE) >= 1);
		assert!(count_edge(&ids, 2.0 * TERRAIN_CELL_SIZE) >= 1);
		assert!(count_edge(&ids, 4.0 * TERRAIN_CELL_SIZE) >= 1);
	}

	#[test]
	fn outer_ring_query_does_not_emit_fine_cells() {
		let layout = world_like_layout();
		let fine_region = layout.fine_request_region();
		let pad = 4.0 * TERRAIN_CELL_SIZE;
		let outer_query = Aabb3d::from_min_max(
			Vec3::new(fine_region.max.x + pad * 0.25, fine_region.min.y, 0.0),
			Vec3::new(fine_region.max.x + pad * 0.75, fine_region.max.y, pad),
		);
		let ids = layout.cell_ids(outer_query);
		assert_eq!(count_edge(&ids, TERRAIN_CELL_SIZE), 0);
		assert!(!ids.is_empty());
	}
}
