//! Fulfill [`MaterialRefRoot`] via a generic [`MaterialLib`] [`SystemParam`].

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::{
	Added, App, Changed, ChildOf, Children, Commands, Entity, IntoScheduleConfigs, Mesh3d, Plugin,
	Query, Resource, SystemSet, Update, With, Without,
};

use crate::lib_trait::MaterialLib;
use crate::material_ref::{
	MaterialRef, MaterialRefApplied, MaterialRefRoot, PropagateToDescendants,
};

/// For each [`MaterialRefRoot`] without [`MaterialRefApplied`], ask `L` to insert a material
/// (or register a propagating root).
pub fn fulfill_material_ref_roots<L>(
	mut commands: Commands,
	query: Query<(Entity, &MaterialRefRoot), Without<MaterialRefApplied>>,
	propagate: Query<(), With<PropagateToDescendants>>,
	meshes: Query<(), With<Mesh3d>>,
	lib: StaticSystemParam<L>,
) where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	let mut lib = lib.into_inner();
	for (entity, root) in &query {
		let propagate = propagate.contains(entity);
		let has_mesh = meshes.contains(entity);
		if propagate && !has_mesh {
			// Meshes arrive later under WorldAsset; only mark the root seen.
			commands.entity(entity).try_insert(MaterialRefApplied);
			continue;
		}
		lib.fulfill(entity, &root.0, &mut commands);
		commands.entity(entity).try_insert(MaterialRefApplied);
	}
}

/// Drop [`MaterialRefApplied`] on a changed root and its descendants so fulfill restamps.
pub fn invalidate_changed_material_ref_roots(
	mut commands: Commands,
	changed: Query<Entity, (Changed<MaterialRefRoot>, With<MaterialRefApplied>)>,
	children: Query<&Children>,
	applied: Query<(), With<MaterialRefApplied>>,
) {
	for root in &changed {
		// LOD / restamp can despawn the walked tree before deferred commands apply.
		commands.entity(root).try_remove::<MaterialRefApplied>();
		let mut stack: Vec<Entity> =
			children.get(root).map(|c| c.iter().copied().collect()).unwrap_or_default();
		while let Some(child) = stack.pop() {
			if applied.contains(child) {
				commands.entity(child).try_remove::<MaterialRefApplied>();
			}
			if let Ok(kids) = children.get(child) {
				stack.extend(kids.iter().copied());
			}
		}
	}
}

/// When [`PropagateToDescendants`] is set, fulfill newly added `Mesh3d` entities under the root.
pub fn fulfill_material_ref_descendants<L>(
	mut commands: Commands,
	added_meshes: Query<Entity, (Added<Mesh3d>, Without<MaterialRefApplied>)>,
	parents: Query<&ChildOf>,
	roots: Query<&MaterialRefRoot, With<PropagateToDescendants>>,
	lib: StaticSystemParam<L>,
) where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	let mut lib = lib.into_inner();
	for entity in &added_meshes {
		let Some(material_ref) = propagating_material_ref(entity, &parents, &roots) else {
			continue;
		};
		lib.fulfill(entity, &material_ref, &mut commands);
		commands.entity(entity).try_insert(MaterialRefApplied);
	}
}

/// Restamp existing `Mesh3d` descendants when a propagating root's identity changes.
pub fn restamp_material_ref_descendants_of_changed<L>(
	mut commands: Commands,
	changed: Query<
		(Entity, &MaterialRefRoot),
		(Changed<MaterialRefRoot>, With<PropagateToDescendants>),
	>,
	children: Query<&Children>,
	meshes: Query<(), With<Mesh3d>>,
	lib: StaticSystemParam<L>,
) where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	let mut lib = lib.into_inner();
	for (root, material) in &changed {
		let mut stack: Vec<Entity> =
			children.get(root).map(|c| c.iter().copied().collect()).unwrap_or_default();
		while let Some(child) = stack.pop() {
			if meshes.contains(child) {
				lib.fulfill(child, &material.0, &mut commands);
				commands.entity(child).try_insert(MaterialRefApplied);
			}
			if let Ok(kids) = children.get(child) {
				stack.extend(kids.iter().copied());
			}
		}
	}
}

fn propagating_material_ref(
	mut entity: Entity,
	parents: &Query<&ChildOf>,
	roots: &Query<&MaterialRefRoot, With<PropagateToDescendants>>,
) -> Option<MaterialRef> {
	loop {
		if let Ok(root) = roots.get(entity) {
			return Some(root.0.clone());
		}
		entity = parents.get(entity).ok()?.parent();
	}
}

/// Shared invalidate / fulfill schedule labels. [`invalidate_changed_material_ref_roots`]
/// is not generic and is installed once, even when several [`MaterialRefPlugin`]`<L>`
/// instances share the app (Chico + characters).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialRefSystems {
	Invalidate,
	Fulfill,
}

#[derive(Resource, Default)]
pub(crate) struct MaterialRefShared;

/// True after the first [`MaterialRefPlugin`] has installed the shared invalidate system.
///
/// App crates that compose domain libs should install one fulfill plugin. Later
/// plugins (including [`crate::StandardMaterialRefPlugin`]) skip when this is set.
pub fn material_ref_plugin_installed(app: &App) -> bool {
	app.world().contains_resource::<MaterialRefShared>()
}

pub(crate) fn material_ref_shared_installed(app: &App) -> bool {
	material_ref_plugin_installed(app)
}

/// Installs fulfill for material lib `L` (a [`SystemParam`] whose item implements [`MaterialLib`]).
///
/// Parallel to [`lod::LodSceneRefreshLevelsPlugin`]’s index parameter: pass the concrete
/// `#[derive(SystemParam)]` type (e.g. [`crate::StandardMaterialLib`]).
///
/// Resources the lib needs (caches, etc.) must be initialized separately — see
/// [`crate::StandardMaterialRefPlugin`].
pub struct MaterialRefPlugin<L>
where
	L: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> L>,
}

impl<L> Default for MaterialRefPlugin<L>
where
	L: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<L> Plugin for MaterialRefPlugin<L>
where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	fn build(&self, app: &mut App) {
		if !material_ref_shared_installed(app) {
			app.init_resource::<MaterialRefShared>().configure_sets(
				Update,
				MaterialRefSystems::Fulfill.after(MaterialRefSystems::Invalidate),
			);
			app.add_systems(
				Update,
				invalidate_changed_material_ref_roots.in_set(MaterialRefSystems::Invalidate),
			);
		}
		app.add_systems(
			Update,
			(
				fulfill_material_ref_roots::<L>,
				fulfill_material_ref_descendants::<L>.after(fulfill_material_ref_roots::<L>),
				restamp_material_ref_descendants_of_changed::<L>
					.after(fulfill_material_ref_roots::<L>),
			)
				.in_set(MaterialRefSystems::Fulfill),
		);
	}
}
