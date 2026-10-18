//! Canopy bump-outs on the shared runtime: a [`BumpedOut`] per proxy cell
//! carries its [`BumpOut`] and a reference to the surface mesh it displaces.
//!
//! The reference shares the surface host's mesh key, so it resolves from
//! whichever lands first. Where a bump-out shows is a level, not a bound: a
//! cell inside its proxy's band shows, and the grove-fill hole within it is
//! an empty level. [`BumpOutRing`] re-levels the band as the viewer moves.

use std::marker::PhantomData;
use std::ops::RangeInclusive;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{
	self, GenerationContext, HcsgClass, HcsgNode, PresentationPlugin, ViewerHcsgBounds,
};
use lod::lod_ref::LodRef;
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use lod::{LodSceneRefreshRegionPlugin, LodViewer};
use lod_gimme::GimmeLodSceneRefreshPlugin;
use terrain_chunk_ref::{TerrainChunkRef, TerrainChunkRefPlugin};
use vegetation_bumpout::{BumpOut, BumpOutPlugin};
use vegetation_components::scene_children;

use super::{ForestGround, GROVE_COLUMN_Y};
use crate::bump_out::{
	bump_out_cell_bounds, bump_out_cells_overlapping, bump_out_chebyshev_xz, CanopyBumpOut,
	MediumCanopyBumpOut, BUMP_OUT_CELL_XZ, BUMP_OUT_INNER_RADIUS_M, BUMP_OUT_OUTER_RADIUS_M,
	MEDIUM_BUMP_OUT_ANCHOR_STEP_M, MEDIUM_BUMP_OUT_CELL_XZ, MEDIUM_BUMP_OUT_INNER_RADIUS_M,
	MEDIUM_BUMP_OUT_OUTER_RADIUS_M,
};
use crate::bump_out::{bump_out_from_cell, bump_out_noise};
use crate::material::VegetationOnTerrainMaterialRefPlugin;
use crate::ForestSelection;

/// One canopy proxy grid: [`CanopyBumpOut`] on 160 m cells, or
/// [`MediumCanopyBumpOut`] on the 320 m terrain grid.
pub trait CanopyProxy: Send + Sync + 'static {
	/// Spatial index scale for [`BumpedOut`] cells of this proxy.
	const INDEX_SCALE: DVec3;

	/// Chebyshev distances from the viewer to a cell's center at which the
	/// cell shows.
	const BAND: RangeInclusive<f32>;

	/// Step the viewer snaps to for the proxy's neighborhood.
	const STEP: f32;

	fn cells_overlapping(region: Aabb3d) -> Vec<Aabb3d>;

	fn select(selection: ForestSelection, cell: Aabb3d) -> Option<CanopyBumpOut>;

	/// Whether a surface cell `width` wide carries this proxy.
	fn carried_by(width: f32) -> bool;
}

impl CanopyProxy for CanopyBumpOut {
	const INDEX_SCALE: DVec3 = DVec3::new(BUMP_OUT_CELL_XZ as f64, 1.0, BUMP_OUT_CELL_XZ as f64);
	const BAND: RangeInclusive<f32> = BUMP_OUT_INNER_RADIUS_M..=BUMP_OUT_OUTER_RADIUS_M;
	const STEP: f32 = BUMP_OUT_CELL_XZ;

	fn cells_overlapping(region: Aabb3d) -> Vec<Aabb3d> {
		bump_out_cells_overlapping(region)
			.map(|(ix, iz)| bump_out_cell_bounds(ix, iz))
			.collect()
	}

	fn select(selection: ForestSelection, cell: Aabb3d) -> Option<CanopyBumpOut> {
		CanopyBumpOut::select(selection, cell)
	}

	/// Fine bump-outs take a surface cell of any size.
	fn carried_by(_width: f32) -> bool {
		true
	}
}

impl CanopyProxy for MediumCanopyBumpOut {
	const INDEX_SCALE: DVec3 =
		DVec3::new(MEDIUM_BUMP_OUT_CELL_XZ as f64, 1.0, MEDIUM_BUMP_OUT_CELL_XZ as f64);
	const BAND: RangeInclusive<f32> =
		MEDIUM_BUMP_OUT_INNER_RADIUS_M..=MEDIUM_BUMP_OUT_OUTER_RADIUS_M;
	const STEP: f32 = MEDIUM_BUMP_OUT_ANCHOR_STEP_M;

	fn cells_overlapping(region: Aabb3d) -> Vec<Aabb3d> {
		MediumCanopyBumpOut::cells_overlapping(region)
			.map(|(ix, iz)| MediumCanopyBumpOut::cell_bounds(ix, iz))
			.collect()
	}

	fn select(selection: ForestSelection, cell: Aabb3d) -> Option<CanopyBumpOut> {
		MediumCanopyBumpOut::select(selection, cell).map(|medium| medium.0)
	}

	fn carried_by(width: f32) -> bool {
		(width - MEDIUM_BUMP_OUT_CELL_XZ).abs() <= 1e-2
	}
}

/// Proxy `P`'s bump-out over one cell of ground `G`'s surface.
pub struct BumpedOut<P, G: ForestGround> {
	/// The proxy cell.
	pub cell: Aabb3d,
	pub bump_out: BumpOut,
	pub terrain: TerrainChunkRef<G::Mesh>,
	_proxy: PhantomData<fn() -> P>,
}

impl<P: CanopyProxy, G: ForestGround> BumpedOut<P, G> {
	/// The surface cell carrying `cell` that overlaps it most.
	fn surface(cx: &mut GenerationContext, cell: Aabb3d) -> Option<Arc<G::Surface>> {
		G::best_surface_at(cx, cell, P::carried_by)
	}

	/// Low inside the band, High in the hole within it, UltraLow beyond it.
	fn level_at(&self, viewer: &Transform) -> LodSceneLevel {
		let distance = bump_out_chebyshev_xz(self.cell, viewer.translation);
		if distance < *P::BAND.start() {
			LodSceneLevel::High
		} else if distance <= *P::BAND.end() {
			LodSceneLevel::Low
		} else {
			LodSceneLevel::UltraLow
		}
	}
}

impl<P: CanopyProxy, G: ForestGround> hcsg::GenerationScheme for BumpedOut<P, G> {
	const INDEX_SCALE: DVec3 = P::INDEX_SCALE;
	const RETENTION_MARGIN: DVec3 = P::INDEX_SCALE;

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		P::cells_overlapping(region)
			.into_iter()
			.map(|cell| OriginalId(Id::from_cell(cell)))
			.collect()
	}

	/// `None` off the ground or where nothing is selected.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		let surface = Self::surface(cx, cell)?;
		let selection = cx.get::<ForestSelection>(Id::Universal)?;
		let proxy = P::select(*selection, cell)?;
		let bump_out = bump_out_from_cell(&proxy, bump_out_noise(&selection.noise))?;
		let footprint = G::footprint(&surface);
		let bounds = Aabb3d::from_min_max(
			Vec3::new(cell.min.x, footprint.min.y, cell.min.z),
			Vec3::new(cell.max.x, footprint.max.y, cell.max.z),
		);
		let terrain = G::chunk_ref(&surface);
		Some((Self { cell, bump_out, terrain, _proxy: PhantomData }, bounds))
	}
}

impl<P: CanopyProxy, G: ForestGround> LodScene for BumpedOut<P, G> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_at(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let current = self.level_at(lod_ref.current_transform);
		if self.level_at(lod_ref.previous_transform) == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		if level == LodSceneLevel::Low {
			children.push(Box::new(self.bump_out.clone().scene(self.terrain.clone())));
		}
		scene_children(children)
	}
}

/// Proxy `P`'s band around the [`LodViewer`]: the cells presented within it
/// and kept one step beyond, re-leveled whenever the viewer crosses a step.
#[derive(Resource)]
pub struct BumpOutRing<P>(PhantomData<fn() -> P>);

impl<P> Default for BumpOutRing<P> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<P: CanopyProxy> BumpOutRing<P> {
	fn step(viewer: Vec3) -> (i32, i32) {
		((viewer.x / P::STEP).round() as i32, (viewer.z / P::STEP).round() as i32)
	}

	fn around(viewer: Vec3, radius: f32) -> Aabb3d {
		let (ix, iz) = Self::step(viewer);
		let (x, z) = (ix as f32 * P::STEP, iz as f32 * P::STEP);
		Aabb3d::from_min_max(
			Vec3::new(x - radius, -GROVE_COLUMN_Y, z - radius),
			Vec3::new(x + radius, GROVE_COLUMN_Y, z + radius),
		)
	}
}

impl<P: CanopyProxy> ViewerHcsgBounds for BumpOutRing<P> {
	const CLASS: HcsgClass = HcsgClass::Near;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		vec![Self::around(viewer, *P::BAND.end())]
	}
}

impl<P: CanopyProxy> LodRefreshRegions for BumpOutRing<P> {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		let current = lod_ref.current_transform.translation;
		if Self::step(lod_ref.previous_transform.translation) == Self::step(current) {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(Self::around(current, *P::BAND.end() + P::STEP))
	}
}

/// Presents proxy `P`'s bump-outs over ground `G` within channel `C`'s
/// regions from the shared storage.
pub struct BumpOutPresentationPlugin<C, P, G>(PhantomData<fn() -> (C, P, G)>);

impl<C, P, G> Default for BumpOutPresentationPlugin<C, P, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, P: CanopyProxy, G: ForestGround> Plugin
	for BumpOutPresentationPlugin<C, P, G>
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<BumpOutPlugin>() {
			app.add_plugins(BumpOutPlugin);
		}
		if !app.is_plugin_added::<VegetationOnTerrainMaterialRefPlugin>() {
			app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		}
		if !app.is_plugin_added::<TerrainChunkRefPlugin<G::Mesh>>() {
			app.add_plugins(TerrainChunkRefPlugin::<G::Mesh>::default());
		}
		app.init_resource::<ForestSelection>();
		app.add_plugins(PresentationPlugin::<C, BumpedOut<P, G>>::without_chunk_refresh());
		if !app
			.is_plugin_added::<LodSceneRefreshRegionPlugin<BumpOutRing<P>, With<LodViewer>, BumpOutRing<P>>>(
			) {
			app.add_plugins(LodSceneRefreshRegionPlugin::<
				BumpOutRing<P>,
				With<LodViewer>,
				BumpOutRing<P>,
			>::default());
		}
		app.add_plugins(
			GimmeLodSceneRefreshPlugin::<HcsgNode<BumpedOut<P, G>>, BumpOutRing<P>>::default(),
		);
	}
}
