//! Fine-phase mesh / feature [`lod::LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Transform, Vec3, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use character_shaders::{eye_palette, mouth_palette, RECIPE_FACE_EYE, RECIPE_FACE_MOUTH};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use scene_ref::{MirrorAxis, SceneRef};

use crate::assembly::CharacterPartSlot;
use crate::assets::AssetNormalization;
use crate::rig::CharacterPart;
use crate::scene_children::maybe_component;
use crate::socket::{RigId, SkinRef, SkinRefRoot, SocketRef, SocketRefRoot};
use rigs::AssemblyHost;

/// Authoring IR for a character mesh or feature — also the fine-phase host component.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct PartNode {
	pub slot: CharacterPartSlot,
	pub label: &'static str,
	pub scene: SceneRef,
	pub normalization: AssetNormalization,
	/// Extra local TRS composed with [`Self::normalization`] (feature sliders).
	pub feature: Transform,
	pub socket: Option<SocketRef>,
	pub skin: Option<SkinRef>,
	/// Deferred material (palette[0] is the preview / PBR base color).
	pub material: MaterialRef,
}

impl Default for PartNode {
	fn default() -> Self {
		Self {
			slot: CharacterPartSlot::BodyMesh,
			label: "",
			scene: SceneRef::default(),
			normalization: AssetNormalization::IDENTITY,
			feature: Transform::IDENTITY,
			socket: None,
			skin: None,
			material: MaterialRef::default(),
		}
	}
}

impl PartNode {
	pub fn glb(
		slot: CharacterPartSlot,
		label: &'static str,
		path: impl Into<String>,
		normalization: AssetNormalization,
	) -> Self {
		Self {
			slot,
			label,
			scene: SceneRef::glb(path),
			normalization,
			feature: Transform::IDENTITY,
			socket: None,
			skin: None,
			material: MaterialRef::default(),
		}
	}

	pub fn socketed(mut self, socket: SocketRef) -> Self {
		self.socket = Some(socket);
		self
	}

	pub fn skinned(mut self, skin: SkinRef) -> Self {
		self.skin = Some(skin);
		self
	}

	/// Socket onto a head-rig bone and skin to that same rig.
	pub fn on_head(self, bone: &'static str, local: Transform) -> Self {
		self.socketed(SocketRef::on(RigId::Head, bone).with_local(local))
			.skinned(SkinRef::to(RigId::Head))
	}

	/// Socket onto a neck-rig bone and skin to that same rig.
	pub fn on_neck(self, bone: &'static str, local: Transform) -> Self {
		self.socketed(SocketRef::on(RigId::Neck, bone).with_local(local))
			.skinned(SkinRef::to(RigId::Neck))
	}

	/// Skin to the body rig with no socket (typical body mesh).
	pub fn on_body(self) -> Self {
		self.skinned(SkinRef::to(RigId::Body))
	}

	/// Socket onto a body-rig bone and skin to that same rig (tails, spines).
	pub fn on_body_bone(self, bone: &'static str, local: Transform) -> Self {
		self.socketed(SocketRef::on(RigId::Body, bone).with_local(local))
			.skinned(SkinRef::to(RigId::Body))
	}

	pub fn with_normalization(mut self, normalization: AssetNormalization) -> Self {
		self.normalization = normalization;
		self
	}

	pub fn with_feature(mut self, feature: Transform) -> Self {
		self.feature = feature;
		self
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = material;
		self
	}

	/// Tint via [`MaterialRef`] palette[0]. Eye and mouth slots claim the face
	/// recipes so bipeds and quadrupeds share the same shader after stamping.
	///
	/// Existing scalars (pupil shape) are kept. Other parts keep their recipe.
	pub fn with_base_color(self, color: bevy::prelude::Color) -> Self {
		let material = match self.slot {
			CharacterPartSlot::EyeLeft | CharacterPartSlot::EyeRight => {
				let scalars: Vec<f32> = self.material.scalar_values().to_vec();
				let mut material =
					MaterialRef::named(RECIPE_FACE_EYE).with_palette(eye_palette(color));
				if !scalars.is_empty() {
					material = material.with_scalars(scalars);
				}
				material
			}
			CharacterPartSlot::Mouth => {
				let scalars: Vec<f32> = self.material.scalar_values().to_vec();
				let mut material =
					MaterialRef::named(RECIPE_FACE_MOUTH).with_palette(mouth_palette(color));
				if !scalars.is_empty() {
					material = material.with_scalars(scalars);
				}
				material
			}
			_ => self.material.clone().with_palette([color]),
		};
		self.with_material(material)
	}

	/// `0` round, `1` slit. Stored in `face_eye` scalars[0].
	pub fn with_pupil_shape(self, shape: f32) -> Self {
		let material = self.material.clone().with_scalars([shape]);
		self.with_material(material)
	}

	/// Open-rate multiplier for `face_mouth` scalars[0]. Higher parts more often.
	pub fn with_mouth_open_rate(self, rate: f32) -> Self {
		let material = self.material.clone().with_scalars([rate]);
		self.with_material(material)
	}

	pub fn mirrored(mut self, axis: MirrorAxis) -> Self {
		self.scene = self.scene.mirrored(axis);
		self
	}

	pub fn reflected(mut self, axis: MirrorAxis) -> Self {
		self.scene = self.scene.reflected(axis);
		self
	}

	/// Normalization × feature, applied on the host before socket fulfill.
	pub fn authored_transform(&self) -> Transform {
		self.normalization.transform().mul_transform(self.feature)
	}

	/// Typed part member without [`lod::LodSceneHost`] scaffolding.
	pub fn assembly_contents(&self) -> impl Scene + 'static {
		let node = self.clone();
		let transform = node.authored_transform();
		let part = CharacterPart { slot: node.slot };
		let material = MaterialRefRoot(node.material.clone());
		let socket = node.socket.map(SocketRefRoot);
		let skin = node.skin.map(SkinRefRoot);
		(
			bsn! {
				template_value(node)
				template_value(transform)
				template_value(part)
				template_value(material)
				PropagateToDescendants
				AssemblyHost
			},
			maybe_component(socket),
			maybe_component(skin),
		)
	}

	/// [`Self::assembly_contents`] plus the GLB [`SceneRef`] on the same entity.
	pub fn assembly_scene(&self) -> impl Scene + 'static {
		(self.assembly_contents(), self.scene.clone().scene(), bsn! { Visibility::Hidden })
	}
}

impl LodScene for PartNode {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		self.scene.clone().scene()
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5))
	}

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let _ = lod_ref;
		self.assembly_contents()
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Color;
	use character_shaders::EYE_PALETTE_PUPIL;
	use material_ref::MaterialId;

	use super::*;

	#[test]
	fn with_base_color_keeps_the_recipe_name() {
		let iris = Color::srgb(0.1, 0.2, 0.3);
		let part = PartNode::glb(
			CharacterPartSlot::EyeLeft,
			"eye",
			"characters/eyes/humanoid_eye_left.glb",
			AssetNormalization::IDENTITY,
		)
		.with_material(MaterialRef::named("face_eye"))
		.with_base_color(iris);
		assert_eq!(part.material.name, MaterialId::named("face_eye"));
		assert_eq!(part.material.palette.len(), 6);
		assert_eq!(part.material.palette[0], iris);
		assert_ne!(part.material.palette[EYE_PALETTE_PUPIL], iris);
	}

	#[test]
	fn with_base_color_claims_face_eye_from_slot() {
		use character_shaders::PUPIL_SHAPE_SLIT;
		let iris = Color::srgb(0.1, 0.2, 0.3);
		let part = PartNode::glb(
			CharacterPartSlot::EyeLeft,
			"eye",
			"characters/eyes/humanoid_eye_left.glb",
			AssetNormalization::IDENTITY,
		)
		.with_pupil_shape(PUPIL_SHAPE_SLIT)
		.with_base_color(iris);
		assert_eq!(part.material.name, MaterialId::named("face_eye"));
		assert_eq!(part.material.scalar_values(), &[PUPIL_SHAPE_SLIT]);
	}

	#[test]
	fn with_base_color_packs_mouth_detail() {
		use character_shaders::MOUTH_PALETTE_CREASE;
		let lip = Color::srgb(0.7, 0.3, 0.28);
		let part = PartNode::glb(
			CharacterPartSlot::Mouth,
			"mouth",
			"characters/mouths/common_mouth.glb",
			AssetNormalization::IDENTITY,
		)
		.with_material(MaterialRef::named("face_mouth"))
		.with_mouth_open_rate(3.0)
		.with_base_color(lip);
		assert_eq!(part.material.name, MaterialId::named("face_mouth"));
		assert_eq!(part.material.palette.len(), 5);
		assert_eq!(part.material.palette[0], lip);
		assert_ne!(part.material.palette[MOUTH_PALETTE_CREASE], lip);
		assert_eq!(part.material.scalar_values(), &[3.0]);
	}
}
