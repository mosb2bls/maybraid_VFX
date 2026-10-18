//! Fine-phase armature [`lod::LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Vec3, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use character_rigs::ResolvedRigPose;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;
use scene_ref::SceneRef;

use crate::assets::AssetNormalization;
use crate::rig::{
	ActiveRigPose, BoneMap, CharacterRig, LodCharacterRig, RigBindScales, RigSkeletonKind,
};
use crate::scene_children::maybe_component;
use crate::socket::{RigId, SocketRef, SocketRefRoot};
use character_motion::{motion_policy, AnimRefRoot};
use rigs::{AssemblyHost, RigRoot};

/// Authoring IR for a character armature — also the fine-phase host component.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct RigNode {
	pub id: RigId,
	pub label: &'static str,
	pub scene: SceneRef,
	pub normalization: AssetNormalization,
	pub socket: Option<SocketRef>,
	pub pose: ResolvedRigPose,
	pub skeleton: RigSkeletonKind,
}

impl Default for RigNode {
	fn default() -> Self {
		Self {
			id: RigId::Body,
			label: "",
			scene: SceneRef::default(),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::Humanoid,
		}
	}
}

impl RigNode {
	pub fn body(label: &'static str, path: impl Into<String>) -> Self {
		Self {
			id: RigId::Body,
			label,
			scene: SceneRef::glb(path),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::from_body_rig_label(label),
		}
	}

	pub fn head(label: &'static str, path: impl Into<String>) -> Self {
		Self {
			id: RigId::Head,
			label,
			scene: SceneRef::glb(path),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::Humanoid,
		}
	}

	pub fn neck(label: &'static str, path: impl Into<String>) -> Self {
		Self {
			id: RigId::Neck,
			label,
			scene: SceneRef::glb(path),
			normalization: AssetNormalization::IDENTITY,
			socket: None,
			pose: ResolvedRigPose::new(),
			skeleton: RigSkeletonKind::Neck,
		}
	}

	pub fn with_normalization(mut self, normalization: AssetNormalization) -> Self {
		self.normalization = normalization;
		self
	}

	pub fn with_pose(mut self, pose: ResolvedRigPose) -> Self {
		self.pose = pose;
		self
	}

	pub fn with_skeleton(mut self, skeleton: RigSkeletonKind) -> Self {
		self.skeleton = skeleton;
		self
	}

	pub fn socketed(mut self, socket: SocketRef) -> Self {
		self.socket = Some(socket);
		self
	}

	fn content_for_level(&self, _level: LodSceneLevel) -> impl Scene + 'static {
		// Motion markers live on the body **host** and are synced from the shown
		// band — level content is only the GLB / mesh scene.
		self.scene.clone().scene()
	}

	/// Typed rig member without [`lod::LodSceneHost`] scaffolding.
	///
	/// Runtime world/mob visuals spawn this. Playground [`LodScene::host`] still
	/// wraps [`Self::host_contents`] in a pending host + level root.
	pub fn assembly_contents(&self) -> impl Scene + 'static {
		self.member_contents(motion_policy(LodSceneLevel::High), false)
	}

	/// [`Self::assembly_contents`] plus the GLB [`SceneRef`] on the same entity.
	pub fn assembly_scene(&self) -> impl Scene + 'static {
		(self.assembly_contents(), self.scene.clone().scene(), bsn! { Visibility::Hidden })
	}

	fn member_contents(
		&self,
		policy: character_motion::MotionPolicy,
		lod_host: bool,
	) -> impl Scene + 'static {
		let node = self.clone();
		let transform = node.normalization.transform();
		let rig = CharacterRig { role: node.id.role(), skeleton: node.skeleton };
		let pose = ActiveRigPose { pose: node.pose.clone() };
		let socket = node.socket.map(SocketRefRoot);
		let body = node.id == RigId::Body;
		let anim = body.then_some(AnimRefRoot::default());
		let bones = body.then_some(()).and(policy.animate_bones());
		let effects = body.then_some(()).and(policy.animate_effects());
		let lod_rig = lod_host.then_some(LodCharacterRig);
		let rig_root = RigRoot::new(node.id.into()).with_landmarks(node.skeleton.landmark_bones());
		(
			bsn! {
				template_value(node)
				template_value(transform)
				template_value(rig)
				AssemblyHost
				template_value(rig_root)
				template_value(BoneMap::default())
				template_value(pose)
				template_value(RigBindScales::default())
			},
			maybe_component(lod_rig),
			maybe_component(socket),
			maybe_component(anim),
			maybe_component(bones),
			maybe_component(effects),
		)
	}
}

impl LodScene for RigNode {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, -1.0), Vec3::new(1.0, 2.5, 1.0))
	}

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		self.member_contents(motion_policy(self.scene_lod_level(lod_ref)), true)
	}
}
