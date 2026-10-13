//! [`LodSceneRefreshPlugin`] / region cull wired to [`GimmeLodSceneHostIndex`].

use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use lod::scene::SemanticLodScene;
use lod::{LodSceneRefreshPlugin, LodSceneRegionCullPlugin, LodViewer, PatchSceneBounds};

use crate::{GimmeLodHostMarshaller, GimmeLodHostPlugin, GimmeLodSceneHostIndex};

fn ensure_gimme_host_index<T: Component + SemanticLodScene + 'static>(app: &mut App) {
	if !app.is_plugin_added::<GimmeLodHostPlugin>() {
		app.add_plugins(GimmeLodHostPlugin);
	}
	if !app.is_plugin_added::<PatchSceneBounds<T, GimmeLodHostMarshaller>>() {
		app.add_plugins(PatchSceneBounds::<T, GimmeLodHostMarshaller>::default());
	}
}

/// [`LodSceneRefreshPlugin`] with [`GimmeLodSceneHostIndex`].
///
/// Produce and cull fill are once per host index (every [`lod::LodNode`], not `F`);
/// emit is once per `T`. `F` still selects region / sync drivers. Channel `M` is
/// accepted so existing dual bullseye/spotlight plugin adds stay valid.
///
/// Use [`Self::without_full_scan_cull`] with [`GimmeLodSceneCullPlugin`] for
/// OpenLattice (or other) region-scoped cull enqueue.
pub struct GimmeLodSceneRefreshPlugin<T, M, F = With<LodViewer>>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	full_scan_cull: bool,
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for GimmeLodSceneRefreshPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { full_scan_cull: true, _marker: PhantomData }
	}
}

impl<T, M, F> GimmeLodSceneRefreshPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	pub fn without_full_scan_cull() -> Self {
		Self { full_scan_cull: false, _marker: PhantomData }
	}
}

impl<T, M, F> Plugin for GimmeLodSceneRefreshPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_gimme_host_index::<T>(app);
		if self.full_scan_cull {
			app.add_plugins(
				LodSceneRefreshPlugin::<T, M, GimmeLodSceneHostIndex<'_>, F>::default(),
			);
		} else {
			app.add_plugins(LodSceneRefreshPlugin::<
				T,
				M,
				GimmeLodSceneHostIndex<'_>,
				F,
			>::without_full_scan_cull());
		}
	}
}

/// Region-scoped cull enqueue for host `T` on cull channel `M` (Gimme index).
///
/// Fill is once per host index (every [`lod::LodNode`]); `F` is accepted so existing
/// Camera / LodViewer plugin adds stay valid.
pub struct GimmeLodSceneCullPlugin<T, M, F = With<LodViewer>>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for GimmeLodSceneCullPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, M, F> Plugin for GimmeLodSceneCullPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_gimme_host_index::<T>(app);
		app.add_plugins(LodSceneRegionCullPlugin::<GimmeLodSceneHostIndex<'_>, M, T, F>::default());
	}
}
