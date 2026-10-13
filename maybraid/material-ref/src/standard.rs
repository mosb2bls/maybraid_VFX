//! [`StandardMaterial`] [`MaterialLib`] — reference SystemParam implementor.

use bevy::app::{App, Plugin};
use bevy::ecs::system::SystemParam;
use bevy::prelude::{Assets, Commands, Entity, MeshMaterial3d, ResMut, StandardMaterial};

use crate::fulfill::{material_ref_shared_installed, MaterialRefPlugin};
use crate::key::{MaterialRefCache, MaterialRefKey};
use crate::lib_trait::MaterialLib;
use crate::material_ref::{MaterialId, MaterialRef};
use crate::reference::ReferenceMaterial;

/// Cache of resolved [`StandardMaterial`] handles.
pub type StandardMaterialRefCache = MaterialRefCache<StandardMaterial>;

/// [`SystemParam`] lib that resolves every [`MaterialRef`] to [`StandardMaterial`].
///
/// Named recipes currently share the same constructor as [`MaterialId::Default`];
/// domain libs (e.g. Chico leaf/stick) fork on [`MaterialId`] instead.
#[derive(SystemParam)]
pub struct StandardMaterialLib<'w> {
	pub materials: ResMut<'w, Assets<StandardMaterial>>,
	pub cache: ResMut<'w, StandardMaterialRefCache>,
}

impl StandardMaterialLib<'_> {
	/// Resolve (and memoize) a [`StandardMaterial`] handle for `material_ref`.
	pub fn resolve(
		&mut self,
		material_ref: &MaterialRef,
	) -> bevy::prelude::Handle<StandardMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.cache.get(&key) {
			return handle;
		}
		let mut mat = StandardMaterial::from_material_ref(material_ref);
		// Mild per-name variation so Name keys are distinct assets when palette/noise match.
		if let MaterialId::Name(name) = &material_ref.name {
			let h = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
			mat.perceptual_roughness = 0.5 + ((h % 40) as f32) * 0.01;
		}
		let handle = self.materials.add(mat);
		self.cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for StandardMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		let handle = self.resolve(material_ref);
		commands.entity(entity).try_insert(MeshMaterial3d(handle));
		true
	}
}

/// [`MaterialRefPlugin`] for [`StandardMaterialLib`] plus its cache resource.
pub struct StandardMaterialRefPlugin;

impl Plugin for StandardMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StandardMaterialRefCache>();
		// A composed domain lib already owns fulfill (and should include
		// [`StandardMaterialLib`] as its fallback). A second [`MaterialRefPlugin`]
		// would race the same [`MaterialRefRoot`]s.
		if material_ref_shared_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<StandardMaterialLib<'_>>::default());
	}
}
