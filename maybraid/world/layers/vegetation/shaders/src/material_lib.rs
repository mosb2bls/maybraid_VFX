//! [`MaterialLib`] for vegetation shaders: leaf / stick / frond recipes only.

use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};
use vegetation_components::{FROND_MATERIAL, LEAF_MATERIAL, STICK_MATERIAL};

use crate::{FrondMaterial, LeafMaterial, StickMaterial};

/// Cache of resolved [`LeafMaterial`] handles.
pub type LeafMaterialRefCache = MaterialRefCache<LeafMaterial>;

/// Cache of resolved [`StickMaterial`] handles.
pub type StickMaterialRefCache = MaterialRefCache<StickMaterial>;

/// Cache of resolved [`FrondMaterial`] handles.
pub type FrondMaterialRefCache = MaterialRefCache<FrondMaterial>;

/// Inserts leaf / stick / frond material caches. Idempotent.
pub fn init_vegetation_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<LeafMaterialRefCache>()
		.init_resource::<StickMaterialRefCache>()
		.init_resource::<FrondMaterialRefCache>();
}

/// Leaf / stick / frond named recipes. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct VegetationMaterialLib<'w> {
	pub leaf_materials: ResMut<'w, Assets<LeafMaterial>>,
	pub leaf_cache: ResMut<'w, LeafMaterialRefCache>,
	pub stick_materials: ResMut<'w, Assets<StickMaterial>>,
	pub stick_cache: ResMut<'w, StickMaterialRefCache>,
	pub frond_materials: ResMut<'w, Assets<FrondMaterial>>,
	pub frond_cache: ResMut<'w, FrondMaterialRefCache>,
}

impl VegetationMaterialLib<'_> {
	fn resolve_leaf(&mut self, material_ref: &MaterialRef) -> Handle<LeafMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.leaf_cache.get(&key) {
			return handle;
		}
		let handle = self.leaf_materials.add(leaf_from_ref(material_ref));
		self.leaf_cache.insert(key, handle.clone());
		handle
	}

	fn resolve_stick(&mut self, material_ref: &MaterialRef) -> Handle<StickMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.stick_cache.get(&key) {
			return handle;
		}
		let handle = self.stick_materials.add(stick_from_ref(material_ref));
		self.stick_cache.insert(key, handle.clone());
		handle
	}

	fn resolve_frond(&mut self, material_ref: &MaterialRef) -> Handle<FrondMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.frond_cache.get(&key) {
			return handle;
		}
		let handle = self.frond_materials.add(frond_from_ref(material_ref));
		self.frond_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for VegetationMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if name == LEAF_MATERIAL => {
				let handle = self.resolve_leaf(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
				true
			}
			MaterialId::Name(name) if name == STICK_MATERIAL => {
				let handle = self.resolve_stick(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
				true
			}
			MaterialId::Name(name) if name == FROND_MATERIAL => {
				let handle = self.resolve_frond(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
				true
			}
			_ => false,
		}
	}
}

/// Shaders-crate standalone lib: Chico recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct StandaloneVegetationMaterialLib<'w> {
	pub chico: VegetationMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for StandaloneVegetationMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.chico.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

fn leaf_from_ref(material_ref: &MaterialRef) -> LeafMaterial {
	let mut mat = LeafMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

fn stick_from_ref(material_ref: &MaterialRef) -> StickMaterial {
	let mut mat = StickMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

fn frond_from_ref(material_ref: &MaterialRef) -> FrondMaterial {
	let mut mat = FrondMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

/// Registers caches + [`MaterialRefPlugin`] for [`StandaloneVegetationMaterialLib`].
///
/// Vegetation / world apps that compose several domain libs should call
/// [`init_vegetation_material_caches`] and skip this plugin.
pub struct VegetationMaterialRefPlugin;

impl Plugin for VegetationMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_vegetation_material_caches(app);
		if material_ref::material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<StandaloneVegetationMaterialLib<'_>>::default());
	}
}
