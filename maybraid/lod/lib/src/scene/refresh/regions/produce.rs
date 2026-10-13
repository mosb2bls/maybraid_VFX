//! Produce [`LodSceneRefreshRegion`] messages from [`LodNode`] drivers.

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use thiserror::Error;

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose, LodRef,
};

use super::super::levels::{LodRefreshDomain, LodSceneRefreshAabb};
use super::super::{ensure_refresh_core, LodRefreshSystems};

/// Impulse: refresh hosts overlapping `region` (type `M` scopes the channel).
#[derive(Message, Debug, Clone)]
pub struct LodSceneRefreshRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodSceneRefreshRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self { region, _marker: PhantomData }
	}
}

/// Result of mapping driver [`LodRef`]s to a refresh region.
#[derive(Debug, Clone)]
pub enum LodRefreshRegionsStatus {
	/// Driver did not cross a production threshold; emit nothing.
	Unchanged,
	/// Emit a max-extent AABB.
	Changed(Aabb3d),
}

/// Errors from [`LodRefreshRegions::lod_refresh_regions_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LodRefreshRegionsError {
	/// No driver [`LodRef`]s were provided.
	#[error("lod_refresh_regions_for called with no LodRefs")]
	Empty,
}

/// How to produce a max-extent refresh AABB from driver [`LodRef`]s.
///
/// Implementors are typically registered as a [`Resource`] and driven by
/// [`LodSceneRefreshRegionPlugin`].
pub trait LodRefreshRegions: Send + Sync + 'static {
	/// Region for a single driver. Must be cheap — no scene build.
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus;

	/// Region this driver covers now, whether or not it just crossed a threshold.
	///
	/// Generation producers union this over every active driver, so a resting
	/// driver keeps its coverage while another one moves. The default evaluates
	/// [`Self::lod_refresh_regions`] as if the driver had arrived from nowhere;
	/// override it when that is not the driver's current region.
	fn lod_coverage(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		let nowhere = Transform::from_translation(Vec3::INFINITY);
		let arrived = LodRef { previous_transform: &nowhere, ..*lod_ref };
		match self.lod_refresh_regions(&arrived) {
			LodRefreshRegionsStatus::Changed(region) => Some(region),
			LodRefreshRegionsStatus::Unchanged => None,
		}
	}

	/// Fold many drivers into one max-extent AABB (or [`Unchanged`] / [`Empty`]).
	fn lod_refresh_regions_for(
		&self,
		lod_refs: &[LodRef],
	) -> Result<LodRefreshRegionsStatus, LodRefreshRegionsError> {
		if lod_refs.is_empty() {
			return Err(LodRefreshRegionsError::Empty);
		}

		let mut union: Option<Aabb3d> = None;
		for lod_ref in lod_refs {
			match self.lod_refresh_regions(lod_ref) {
				LodRefreshRegionsStatus::Unchanged => {}
				LodRefreshRegionsStatus::Changed(aabb) => {
					union = Some(match union {
						None => aabb,
						Some(prev) => union_aabb(prev, aabb),
					});
				}
			}
		}

		Ok(match union {
			Some(region) => LodRefreshRegionsStatus::Changed(region),
			None => LodRefreshRegionsStatus::Unchanged,
		})
	}
}

fn union_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max))
}

/// Read `F`-filtered [`LodNode`]s whose pose changed, compute a region via `P`,
/// write [`LodSceneRefreshRegion<M>`].
///
/// Only drivers with [`Changed<LodNodePose>`] (after [`crate::track_lod_nodes`])
/// participate. Strategies may still return [`LodRefreshRegionsStatus::Unchanged`]
/// for sub-threshold motion (e.g. same lattice cell).
pub fn produce_lod_refresh_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<
		(Entity, &LodNodePose, Option<&LodNodeBounds>),
		(With<LodNode>, Changed<LodNodePose>, F),
	>,
	mut writer: MessageWriter<LodSceneRefreshRegion<M>>,
	mut bus: MessageWriter<LodSceneRefreshAabb>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	if nodes.is_empty() {
		return;
	}
	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);

	let Ok(LodRefreshRegionsStatus::Changed(region)) = producer.lod_refresh_regions_for(&refs)
	else {
		return;
	};

	writer.write(LodSceneRefreshRegion::<M>::new(region));
	bus.write(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<M>() });
}

/// Produce [`LodSceneRefreshRegion<M>`] from `F`-filtered [`LodNode`]s via strategy `P`.
///
/// `P` is a [`Resource`] implementing [`LodRefreshRegions`] (`init_resource` on add).
/// `M` is a channel marker (`Send + Sync`), not an ECS component.
pub struct LodSceneRefreshRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodSceneRefreshRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<P, F, M> Plugin for LodSceneRefreshRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<P>()
			.add_message::<LodSceneRefreshRegion<M>>()
			.add_message::<LodSceneRefreshAabb>()
			.add_systems(
				Update,
				produce_lod_refresh_regions::<P, F, M>.in_set(LodRefreshSystems::ProduceRegions),
			);
	}
}
