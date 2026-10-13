//! Asset references and authored-scale metadata shared by character species.
//!
//! The character concept spec separates two ideas that are easy to conflate:
//! authored asset normalization and character proportion scaling. The types in
//! this module describe the former. Species proportions, presets, and sliders
//! should multiply a normalized baseline rather than reusing these values as
//! body-shape controls.

use bevy::prelude::*;

/// Runtime asset path relative to the `maybraid/assets` root.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetPath(&'static str);

impl AssetPath {
	pub const fn new(path: &'static str) -> Self {
		Self(path)
	}

	pub const fn as_str(self) -> &'static str {
		self.0
	}

	/// GLTF scene label for [`WorldAssetRoot`] in BSN (`path#Scene0`).
	pub fn gltf_scene_0(self) -> String {
		format!("{}#Scene0", self.0)
	}

	/// Shared [`scene_ref::SceneRef`] for this GLB (scene 0).
	pub fn scene_ref(self) -> scene_ref::SceneRef {
		scene_ref::SceneRef::glb(self.0)
	}
}

impl std::fmt::Display for AssetPath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

/// Where an imported asset's authored origin sits relative to its visible mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredAnchor {
	/// The asset is centered closely enough for local scale to be symmetric.
	Centroid,
	/// The asset is anchored at its lower/base extent in Bevy's +Y up axis.
	BaseY,
}

/// The authored facing direction for socket placement metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetFacing {
	Forward,
	PositiveX,
}

/// One-time asset-local normalization applied before sliders and species scales.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetNormalization {
	pub scale: f32,
	pub anchor: AuthoredAnchor,
	pub facing: AssetFacing,
}

impl AssetNormalization {
	pub const IDENTITY: Self =
		Self { scale: 1.0, anchor: AuthoredAnchor::Centroid, facing: AssetFacing::Forward };

	pub const fn centroid(scale: f32) -> Self {
		Self { scale, anchor: AuthoredAnchor::Centroid, facing: AssetFacing::Forward }
	}

	pub const fn base_y(scale: f32) -> Self {
		Self { scale, anchor: AuthoredAnchor::BaseY, facing: AssetFacing::Forward }
	}

	pub const fn facing_positive_x(mut self) -> Self {
		self.facing = AssetFacing::PositiveX;
		self
	}

	pub fn transform(self) -> Transform {
		// Applied at spawn time only; species/preset/slider scales are separate.
		Transform::from_scale(Vec3::splat(self.scale))
	}
}
