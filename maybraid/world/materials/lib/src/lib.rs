//! Composed Maybraid World [`material_ref::MaterialLib`].
//!
//! Domain shaders stay in their crates. This crate is the global recipe-name
//! table: furniture, character, urban, muzzle flame, energy, then vegetation
//! and Standard.

mod material_lib;

pub use material_lib::{
	WorldMaterialLib, WorldMaterialRefPlugin, WorldMaterialsPlugin, WorldMaterialsShadersPlugin,
};
