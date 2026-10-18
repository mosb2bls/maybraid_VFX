//! Additive energy looks: hex cells, pulsing rings, and a long tail.
//!
//! One [`EnergyMaterial`] plus a kind uniform, authored as [`MaterialRef`]
//! recipes (`laser_hex`, `laser_pulse`, `laser_tail`). [`EnergyMaterialLib`]
//! claims those names only so a composed world lib can fall through.

use bevy::asset::embedded_asset;
use bevy::ecs::system::SystemParam;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialScalars,
	ReferenceMaterial,
};
use procedural_common::NoiseParams;

/// Recipe claimed by the hex-cell energy look.
pub const LASER_HEX_RECIPE: &str = "laser_hex";
/// Recipe claimed by the pulsing-ring energy look.
pub const LASER_PULSE_RECIPE: &str = "laser_pulse";
/// Recipe claimed by the long-tail energy look.
pub const LASER_TAIL_RECIPE: &str = "laser_tail";

/// [`EnergyMaterial::extras`].y for hex cells.
pub const KIND_HEX: f32 = 0.0;
/// [`EnergyMaterial::extras`].y for pulsing rings.
pub const KIND_PULSE: f32 = 1.0;
/// [`EnergyMaterial::extras`].y for the vertex tail.
pub const KIND_TAIL: f32 = 2.0;

const HEX_CORE: Color = Color::srgb(0.45, 1.0, 0.82);
const HEX_LIMB: Color = Color::srgb(0.08, 0.85, 0.72);
const HEX_GROUT: Color = Color::srgb(0.02, 0.12, 0.14);
const HEX_GAIN: f32 = 2.4;
const HEX_SCALE: f32 = 6.0;
const HEX_RATE: f32 = 1.2;
const HEX_DEPTH: f32 = 0.18;

const PULSE_CORE: Color = Color::srgb(1.0, 0.95, 0.85);
const PULSE_LIMB: Color = Color::srgb(1.0, 0.22, 0.55);
const PULSE_GROUT: Color = Color::srgb(0.15, 0.02, 0.08);
const PULSE_GAIN: f32 = 3.2;
const PULSE_SCALE: f32 = 4.0;
const PULSE_RATE: f32 = 1.8;
const PULSE_DEPTH: f32 = 0.0;

const TAIL_CORE: Color = Color::srgb(0.85, 0.97, 1.0);
const TAIL_LIMB: Color = Color::srgb(0.18, 0.55, 1.0);
const TAIL_GROUT: Color = Color::srgb(0.02, 0.06, 0.16);
const TAIL_GAIN: f32 = 2.8;
const TAIL_SCALE: f32 = 3.5;
const TAIL_RATE: f32 = 2.4;
const TAIL_DEPTH: f32 = 0.75;

/// Cache of resolved [`EnergyMaterial`] handles.
pub type EnergyMaterialRefCache = MaterialRefCache<EnergyMaterial>;

/// Inserts the energy-material cache. Idempotent.
pub fn init_energy_material_caches(app: &mut App) {
	app.init_resource::<EnergyMaterialRefCache>();
}

/// True when `name` is one of the energy recipes.
pub fn is_energy_recipe(name: &str) -> bool {
	matches!(name, LASER_HEX_RECIPE | LASER_PULSE_RECIPE | LASER_TAIL_RECIPE)
}

/// Authored hex-cell recipe.
pub fn laser_hex_ref() -> MaterialRef {
	MaterialRef::named(LASER_HEX_RECIPE)
		.with_palette([HEX_CORE, HEX_LIMB, HEX_GROUT])
		.with_scalars([HEX_GAIN, HEX_SCALE, HEX_RATE, HEX_DEPTH])
}

/// Authored pulsing-ring recipe.
pub fn laser_pulse_ref() -> MaterialRef {
	MaterialRef::named(LASER_PULSE_RECIPE)
		.with_palette([PULSE_CORE, PULSE_LIMB, PULSE_GROUT])
		.with_scalars([PULSE_GAIN, PULSE_SCALE, PULSE_RATE, PULSE_DEPTH])
}

/// Authored long-tail recipe.
pub fn laser_tail_ref() -> MaterialRef {
	MaterialRef::named(LASER_TAIL_RECIPE)
		.with_palette([TAIL_CORE, TAIL_LIMB, TAIL_GROUT])
		.with_scalars([TAIL_GAIN, TAIL_SCALE, TAIL_RATE, TAIL_DEPTH])
}

/// Kind packed into [`EnergyMaterial::extras`].y.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnergyKind {
	Hex,
	Pulse,
	Tail,
}

impl EnergyKind {
	pub const fn as_f32(self) -> f32 {
		match self {
			Self::Hex => KIND_HEX,
			Self::Pulse => KIND_PULSE,
			Self::Tail => KIND_TAIL,
		}
	}

	pub fn from_recipe_name(name: &str) -> Self {
		match name {
			LASER_PULSE_RECIPE => Self::Pulse,
			LASER_TAIL_RECIPE => Self::Tail,
			_ => Self::Hex,
		}
	}

	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		match &material_ref.name {
			MaterialId::Name(name) => Self::from_recipe_name(name),
			MaterialId::Default => Self::Hex,
		}
	}
}

/// Unlit additive energy field. `core.w` is gain, `limb.w` is pattern scale,
/// `grout.w` is pulse / scroll rate, `extras.x` is relief / tail depth, and
/// `extras.y` is [`EnergyKind`].
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct EnergyMaterial {
	#[uniform(0)]
	pub core: Vec4,
	#[uniform(1)]
	pub limb: Vec4,
	#[uniform(2)]
	pub grout: Vec4,
	#[uniform(3)]
	pub extras: Vec4,
}

impl EnergyMaterial {
	fn from_parts(
		core: Color,
		limb: Color,
		grout: Color,
		gain: f32,
		scale: f32,
		rate: f32,
		depth: f32,
		kind: f32,
	) -> Self {
		let core = LinearRgba::from(core);
		let limb = LinearRgba::from(limb);
		let grout = LinearRgba::from(grout);
		Self {
			core: Vec4::new(core.red, core.green, core.blue, gain),
			limb: Vec4::new(limb.red, limb.green, limb.blue, scale),
			grout: Vec4::new(grout.red, grout.green, grout.blue, rate),
			extras: Vec4::new(depth, kind, 0.0, 0.0),
		}
	}

	fn from_kind(kind: EnergyKind) -> Self {
		match kind {
			EnergyKind::Hex => Self::from_parts(
				HEX_CORE, HEX_LIMB, HEX_GROUT, HEX_GAIN, HEX_SCALE, HEX_RATE, HEX_DEPTH, KIND_HEX,
			),
			EnergyKind::Pulse => Self::from_parts(
				PULSE_CORE,
				PULSE_LIMB,
				PULSE_GROUT,
				PULSE_GAIN,
				PULSE_SCALE,
				PULSE_RATE,
				PULSE_DEPTH,
				KIND_PULSE,
			),
			EnergyKind::Tail => Self::from_parts(
				TAIL_CORE, TAIL_LIMB, TAIL_GROUT, TAIL_GAIN, TAIL_SCALE, TAIL_RATE, TAIL_DEPTH,
				KIND_TAIL,
			),
		}
	}
}

impl Default for EnergyMaterial {
	fn default() -> Self {
		Self::from_kind(EnergyKind::Hex)
	}
}

impl ReferenceMaterial for EnergyMaterial {
	fn with_palette(mut self, palette: &[Color]) -> Self {
		if let Some(core) = palette.first() {
			let core = LinearRgba::from(*core);
			self.core = Vec4::new(core.red, core.green, core.blue, self.core.w);
		}
		if let Some(limb) = palette.get(1) {
			let limb = LinearRgba::from(*limb);
			self.limb = Vec4::new(limb.red, limb.green, limb.blue, self.limb.w);
		}
		if let Some(grout) = palette.get(2) {
			let grout = LinearRgba::from(*grout);
			self.grout = Vec4::new(grout.red, grout.green, grout.blue, self.grout.w);
		}
		self
	}

	fn with_noise_params(self, _noise: &NoiseParams) -> Self {
		self
	}

	fn with_scalars(mut self, scalars: &MaterialScalars) -> Self {
		if let Some(gain) = scalars.get(0) {
			self.core.w = gain;
		}
		if let Some(scale) = scalars.get(1) {
			self.limb.w = scale;
		}
		if let Some(rate) = scalars.get(2) {
			self.grout.w = rate;
		}
		if let Some(depth) = scalars.get(3) {
			self.extras.x = depth;
		}
		self
	}

	fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self::from_kind(EnergyKind::from_material_ref(material_ref))
			.with_palette(&material_ref.palette)
			.with_noise_params(&material_ref.noise)
			.with_scalars(&material_ref.scalars)
	}
}

impl Material for EnergyMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "energy.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "energy.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Add
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}

/// Embedded shader plus [`MaterialPlugin`].
pub struct EnergyMaterialPlugin;

impl Plugin for EnergyMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "energy.wgsl");
		if !app.is_plugin_added::<MaterialPlugin<EnergyMaterial>>() {
			app.add_plugins(MaterialPlugin::<EnergyMaterial>::default());
		}
	}
}

pub(crate) fn resolve_energy(
	materials: &mut Assets<EnergyMaterial>,
	cache: &mut EnergyMaterialRefCache,
	material_ref: &MaterialRef,
) -> Handle<EnergyMaterial> {
	let key = MaterialRefKey::from(material_ref);
	if let Some(handle) = cache.get(&key) {
		return handle;
	}
	let handle = materials.add(EnergyMaterial::from_material_ref(material_ref));
	cache.insert(key, handle.clone());
	handle
}

/// Claims `laser_hex` / `laser_pulse` / `laser_tail` only.
#[derive(SystemParam)]
pub struct EnergyMaterialLib<'w> {
	pub materials: ResMut<'w, Assets<EnergyMaterial>>,
	pub cache: ResMut<'w, EnergyMaterialRefCache>,
}

impl MaterialLib for EnergyMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if is_energy_recipe(name) => {
				let handle = resolve_energy(&mut self.materials, &mut self.cache, material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hex_ref_packs_teal_cells() {
		let material = EnergyMaterial::from_material_ref(&laser_hex_ref());
		assert_eq!(laser_hex_ref().name, MaterialId::named(LASER_HEX_RECIPE));
		assert!((material.extras.y - KIND_HEX).abs() < 1e-4);
		assert!(material.core.y > material.core.x, "hex core is green-cyan, {:?}", material.core);
		assert!((material.core.w - HEX_GAIN).abs() < 1e-4);
		assert!((material.limb.w - HEX_SCALE).abs() < 1e-4);
		assert!((material.extras.x - HEX_DEPTH).abs() < 1e-4);
	}

	#[test]
	fn pulse_ref_packs_hot_rings() {
		let material = EnergyMaterial::from_material_ref(&laser_pulse_ref());
		assert_eq!(laser_pulse_ref().name, MaterialId::named(LASER_PULSE_RECIPE));
		assert!((material.extras.y - KIND_PULSE).abs() < 1e-4);
		assert!(material.limb.x > material.limb.z, "pulse limb is magenta, {:?}", material.limb);
		assert!((material.grout.w - PULSE_RATE).abs() < 1e-4);
	}

	#[test]
	fn tail_ref_packs_axial_depth() {
		let material = EnergyMaterial::from_material_ref(&laser_tail_ref());
		assert_eq!(laser_tail_ref().name, MaterialId::named(LASER_TAIL_RECIPE));
		assert!((material.extras.y - KIND_TAIL).abs() < 1e-4);
		assert!(material.limb.z > material.limb.x, "tail limb is blue, {:?}", material.limb);
		assert!((material.extras.x - TAIL_DEPTH).abs() < 1e-4);
		assert!(is_energy_recipe(LASER_TAIL_RECIPE));
		assert!(!is_energy_recipe("muzzle_flame"));
	}
}
