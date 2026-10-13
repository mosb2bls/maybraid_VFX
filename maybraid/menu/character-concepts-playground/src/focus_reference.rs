//! Hidden "shadow" rigs that hold the character's proportional pose for
//! camera-focus socket resolution.
//!
//! # Why a shadow rig?
//!
//! The visible preview rig is animated: its bones move every frame, so camera
//! framing derived from it would chase the walk/run/gallop cycle. The shadow
//! rigs load the same body and head-rig armatures and apply the same resolved
//! proportion pose ([`ActiveRigPose`]), but are never animated and never
//! rendered ([`Visibility::Hidden`]). Socket lookups against them therefore
//! reflect the character's proportions without any animation transforms.
//!
//! # Head-rig scale must match the preview
//!
//! Orthograde / pronograde head armatures are authored large and brought down
//! with [`AssetNormalization`](characters::assets::AssetNormalization)
//! at preview spawn (e.g. Brodler / Braidman `base_y(0.26)`). The shadow head
//! must use that same transform: socket fulfill parents the head to the bone
//! and **preserves** the entity's authored scale into the final local
//! transform. Spawning the shadow head at [`Transform::IDENTITY`] leaves
//! sockets at full authored size while the neck attachment stays posed
//! correctly, so nose / eye / crown world Y values come out far too large
//! (roughly `1 / normalization.scale` relative to the neck).
//!
//! # Lifecycle
//!
//! Readiness is signalled by imperative state changes, never approximated from
//! bone transforms:
//!
//! 1. [`sync_focus_reference`] spawns a shadow [`CharacterRoot`] (body, plus
//!    neck / head when the LodScene recipe has those [`RigNode`]s) whenever
//!    the spawn key — body, neck, and head-rig asset paths — changes. Old
//!    shadow roots are despawned. The head is spawned with
//!    `node.normalization.transform()`, matching preview. Neck/head carry
//!    [`SocketRefRoot`] + [`MemberOf`]; fulfill parents them.
//! 2. `build_rig_bone_map` fills [`BoneMap`] as the GLTF scenes spawn bones.
//! 3. `maintain_resolved_pose` applies the proportional pose and inserts
//!    [`ResolvedPoseApplied`](crate::skinning::ResolvedPoseApplied) on the
//!    body rig once the pose is fully written. Camera focus gates on that
//!    marker.
//! 4. Socket fulfill parents the head/neck to the named bone and inserts
//!    [`SocketRefApplied`] — the readiness signal for head-socket focuses.
//! 5. Config tweaks that keep the same armatures (sliders, colors) only update
//!    [`ActiveRigPose`] in place; pose maintenance re-applies it every frame,
//!    so no respawn and no readiness reset is needed.

use bevy::prelude::*;
use characters::{
	AssemblyHost, AssemblyRoot, BoneMap, CharacterRig, CharacterRoot, MemberOf, RigId, RigNode,
	RigRoot, SocketRefRoot,
};

use crate::preview::ConceptPreviewConfig;
use crate::skinning::{ActiveRigPose, CharacterRigRole, RigBindScales};

#[derive(Component)]
pub struct FocusReferenceRoot;

/// Marks a hidden rig used only for [`crate::camera_focus`] socket resolution.
#[derive(Component)]
pub struct FocusReferenceRig;

#[derive(Resource, Default)]
pub struct FocusReferenceSyncState {
	live_key: String,
	spawn_key: String,
}

impl FocusReferenceSyncState {
	/// Drop cached pose/config so the next sync re-runs without despawning rigs.
	pub(crate) fn invalidate_live(&mut self) {
		self.live_key.clear();
	}

	/// Force a full hidden-rig respawn (species switch).
	pub(crate) fn invalidate(&mut self) {
		self.live_key.clear();
		self.spawn_key.clear();
	}
}

/// Any config change that can affect proportions refreshes the applied pose.
fn focus_live_key(config: &ConceptPreviewConfig) -> String {
	match config {
		ConceptPreviewConfig::Braidman { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Brenal { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Caole { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Epiphant { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Hars { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Yilter { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Sonyak { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Claber { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Croconot { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Brodler { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Mygr { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Dui { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Lidder { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Chupri { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Brokker { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Tipple { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Topple { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Kispar { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Tapp { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Kaller { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Kappler { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Wumbus { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Lero { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Spibmom { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Grener { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Thumplus { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Mistler { config, .. } => config.sync_key(),
		ConceptPreviewConfig::Tuberwaber { config, .. } => config.sync_key(),
	}
}

/// Shadow rigs must respawn when the underlying armature assets change: a new
/// body, neck, or head rig carries different socket bones. Cosmetic part swaps on
/// the same armatures only refresh the live pose.
fn focus_spawn_key(nodes: &[RigNode]) -> String {
	let path = |id: RigId| {
		nodes
			.iter()
			.find(|node| node.id == id)
			.map(|node| node.scene.path.as_str())
			.unwrap_or("")
	};
	format!("body={} neck={} head={}", path(RigId::Body), path(RigId::Neck), path(RigId::Head))
}

pub fn sync_focus_reference(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	config: Res<ConceptPreviewConfig>,
	mut sync_state: ResMut<FocusReferenceSyncState>,
	mut poses: Query<(&mut ActiveRigPose, &CharacterRig), With<FocusReferenceRig>>,
	roots: Query<Entity, With<FocusReferenceRoot>>,
) {
	let live_key = focus_live_key(&config);
	if sync_state.live_key == live_key {
		return;
	}

	let nodes = config.lod_rig_nodes();
	let spawn_key = focus_spawn_key(&nodes);
	if sync_state.spawn_key == spawn_key {
		// Same armatures: update poses in place. `maintain_resolved_pose`
		// re-applies them every frame, so nothing else needs to be invalidated.
		sync_state.live_key = live_key;
		let body_pose =
			nodes.iter().find(|node| node.id == RigId::Body).map(|node| node.pose.clone());
		let neck_pose =
			nodes.iter().find(|node| node.id == RigId::Neck).map(|node| node.pose.clone());
		for (mut pose, rig) in &mut poses {
			match rig.role {
				CharacterRigRole::Body => {
					if let Some(body_pose) = &body_pose {
						pose.pose = body_pose.clone();
					}
				}
				CharacterRigRole::Neck => {
					if let Some(neck_pose) = &neck_pose {
						pose.pose = neck_pose.clone();
					}
				}
				CharacterRigRole::Head => {}
			}
		}
		return;
	}

	sync_state.live_key = live_key;
	sync_state.spawn_key = spawn_key;

	// Respawning from scratch resets all readiness markers: the fresh rigs
	// carry no `ResolvedPoseApplied` and socketed members start without
	// `SocketRefApplied`, so camera focus waits for the new pose.
	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	spawn_focus_reference(&mut commands, &asset_server, &nodes);
}

fn spawn_focus_reference(commands: &mut Commands, asset_server: &AssetServer, nodes: &[RigNode]) {
	let Some(body) = nodes.iter().find(|node| node.id == RigId::Body) else {
		return;
	};
	let root = commands
		.spawn((
			AssemblyRoot,
			CharacterRoot,
			FocusReferenceRoot,
			Visibility::Hidden,
			Name::new("focus_reference"),
		))
		.id();

	let transform = body.normalization.transform();
	commands.spawn((
		WorldAssetRoot(
			asset_server.load(GltfAssetLabel::Scene(0).from_asset(body.scene.path.clone())),
		),
		CharacterRig { role: CharacterRigRole::Body, skeleton: body.skeleton },
		FocusReferenceRig,
		AssemblyHost,
		RigRoot::new(RigId::Body.into()).with_landmarks(body.skeleton.landmark_bones()),
		BoneMap::default(),
		ActiveRigPose { pose: body.pose.clone() },
		RigBindScales::default(),
		MemberOf(root),
		ChildOf(root),
		Visibility::Hidden,
		transform,
		Name::new(format!("focus_{}_body_rig", body.label)),
	));

	if let Some(neck) = nodes.iter().find(|node| node.id == RigId::Neck) {
		let mut entity = commands.spawn((
			WorldAssetRoot(
				asset_server.load(GltfAssetLabel::Scene(0).from_asset(neck.scene.path.clone())),
			),
			CharacterRig { role: CharacterRigRole::Neck, skeleton: neck.skeleton },
			FocusReferenceRig,
			AssemblyHost,
			RigRoot::new(RigId::Neck.into()).with_landmarks(neck.skeleton.landmark_bones()),
			BoneMap::default(),
			ActiveRigPose { pose: neck.pose.clone() },
			RigBindScales::default(),
			MemberOf(root),
			ChildOf(root),
			Visibility::Hidden,
			Transform::IDENTITY,
			Name::new("focus_neck_rig"),
		));
		if let Some(socket) = neck.socket {
			entity.insert(SocketRefRoot(socket));
		}
	}

	let Some(head) = nodes.iter().find(|node| node.id == RigId::Head) else {
		return;
	};

	// CRITICAL: same authored scale as the LodScene head [`RigNode`].
	//
	// Head GLTFs are normalized at spawn (often `AssetNormalization::base_y(0.26)`).
	// Socket fulfill does `local_transform.scale *= authored_scale`, so this
	// value must already be on the entity when the head is parented to the
	// body socket. Identity scale here is what made camera-focus nose/crown Y
	// resolve several times too high after shadow-only focus resolution landed.
	let head_transform = head.normalization.transform();
	let mut entity = commands.spawn((
		WorldAssetRoot(
			asset_server.load(GltfAssetLabel::Scene(0).from_asset(head.scene.path.clone())),
		),
		CharacterRig { role: CharacterRigRole::Head, skeleton: head.skeleton },
		FocusReferenceRig,
		AssemblyHost,
		RigRoot::new(RigId::Head.into()).with_landmarks(head.skeleton.landmark_bones()),
		BoneMap::default(),
		MemberOf(root),
		ChildOf(root),
		Visibility::Hidden,
		head_transform,
		Name::new("focus_head_rig"),
	));
	if let Some(socket) = head.socket {
		entity.insert(SocketRefRoot(socket));
	}
}
