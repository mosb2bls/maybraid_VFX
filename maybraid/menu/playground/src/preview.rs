//! Live character preview driven by [`CharacterMenuState`].

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use bevy::window::PrimaryWindow;
use character_creation_menus::{CharacterMenu, ConceptSpecies, MenuEvent};
#[cfg(test)]
use character_items::InventoryItem;
use character_items::{ClothingHost, ClothingMesh, FirearmSpec, ItemColor};
use character_persist::SaveRoot;
use character_ui_menu::{CameraFocus, FocusRig};
use characters::{
	ActiveRigPose, AnimRef, AnimRefRoot, ApplyTerrainPitch, BoneMap, CharacterComponents,
	CharacterHostSystems, CharacterMembers, CharacterRecipe, CharacterRig, CharacterRigRole,
	ClothingLayer, ComponentsOnly, Layers, PartNode, ResolvedPoseApplied, RigBindScales,
	SocketRefApplied, SocketRefRoot, add_character_components_host, character_bounds,
};
use characters_playground::CameraController;
use firearm_user::GeneratedFirearm;
use firearms_components::{
	FirearmComponentsPlugin, FirearmRoot, add_firearm_components_host, firearm_bounds,
	firearm_preview_camera, spawn_firearm_components,
};
use grenades::{GrenadeMaterial, GrenadeMaterialPlugin, grenade_material, grenade_mesh};
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::scene::LodSceneLevel;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use maybraid_input::produce::gamepad::GamepadAxes;
use maybraid_input::{Deadzone, VirtualPadConfig};
use menu_components::DESCRIPTION_PANE_LEFT_PERCENT;
use menu_screens::{
	GalleryScreen, HomeScreen, SpinRevealCurrent, SpinRevealScreen, SpinRevealSystems,
};

use crate::character::{CharacterMenuState, CharacterScreen};
use crate::session::{ActiveCharacter, CharacterEditorReturn};
use crate::weapon_gallery::{RequestShowWeapons, WeaponGalleryScreen};

#[derive(Component)]
pub struct CharacterPreviewRoot;

/// Menu inspect orb for a starter grenade. Realized after spawn so [`Assets`]
/// can own the mesh / material.
#[derive(Component)]
struct GrenadePreview;

/// Menu-playground key / fill lights. Composed applications can put these on
/// the same render layer as [`CharacterPreviewRoot`].
#[derive(Component)]
pub struct CharacterPreviewLight;

#[derive(Resource, Default)]
struct PreviewSyncState {
	key: String,
	species: Option<ConceptSpecies>,
	anim: Option<AnimRef>,
}

/// Menu preview vertical FOV; gameplay leaves its own hip / ADS FOV on the
/// shared world camera.
const PREVIEW_FOV: f32 = CameraFocus::BODY_PREVIEW_FOV;
/// Right-justified character HUD panel; keep the body in the leftover preview.
const CHARACTER_PANEL_WIDTH_PX: f32 = 480.0;
const ORBIT_YAW_SPEED: f32 = 2.4;
const ORBIT_PITCH_SPEED: f32 = 1.6;
/// Elevation cap so the orbit never reaches the `looking_at` up-vector pole.
const ORBIT_ELEVATION_LIMIT: f32 = 1.3;

#[derive(Resource, Default)]
struct PendingCameraFocus {
	focus: Option<CameraFocus>,
	frame: Option<FocusFrame>,
	/// Garment reveals have no rig; offsets are world space around the origin.
	world_space: bool,
	orbit: PreviewOrbit,
	waiting: Option<FocusWait>,
}

impl PendingCameraFocus {
	/// Re-frame and snap the orbit home only when the framed part changes.
	fn set_focus(&mut self, focus: CameraFocus) {
		if self.focus == Some(focus) {
			return;
		}
		self.focus = Some(focus);
		self.frame = None;
		self.orbit = PreviewOrbit::default();
		// Hull-fit body offsets are already in preview-root space.
		self.world_space = focus.rig == FocusRig::Body && focus.socket == "root";
	}

	fn clear(&mut self) {
		*self = Self::default();
	}
}

/// Right-stick swing around the framed look-at point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PreviewOrbit {
	yaw: f32,
	pitch: f32,
}

impl PreviewOrbit {
	fn steer(&mut self, stick: Vec2, dt: f32) {
		self.yaw = (self.yaw - stick.x * ORBIT_YAW_SPEED * dt).rem_euclid(TAU);
		self.pitch -= stick.y * ORBIT_PITCH_SPEED * dt;
	}

	/// Rotate the look-at → camera offset. Pitch is clamped against the framed
	/// elevation so the swing stops short of straight up or down.
	fn swing(&mut self, offset: Vec3) -> Vec3 {
		let radius = offset.length();
		if radius < 1e-4 {
			return offset;
		}
		let azimuth = offset.x.atan2(offset.z);
		let elevation = (offset.y / radius).clamp(-1.0, 1.0).asin();
		let low = (-ORBIT_ELEVATION_LIMIT - elevation).min(0.0);
		let high = (ORBIT_ELEVATION_LIMIT - elevation).max(0.0);
		self.pitch = self.pitch.clamp(low, high);
		let azimuth = azimuth + self.yaw;
		let elevation = elevation + self.pitch;
		Vec3::new(elevation.cos() * azimuth.sin(), elevation.sin(), elevation.cos() * azimuth.cos())
			* radius
	}
}

/// Framing captured once the focus rig is ready. The anchor's live position
/// is tracked; the offsets keep the rest-pose orientation so the camera does
/// not swing with the idle clip.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FocusFrame {
	anchor: Option<Entity>,
	look_delta: Vec3,
	camera_delta: Vec3,
}

impl FocusFrame {
	fn world(camera: Vec3, look_at: Vec3) -> Self {
		Self { anchor: None, look_delta: look_at, camera_delta: camera - look_at }
	}

	fn socketed(anchor: Entity, rotation: Quat, focus: CameraFocus) -> Self {
		let look_delta = rotation * focus.look_at_offset;
		Self {
			anchor: Some(anchor),
			look_delta,
			camera_delta: rotation * focus.camera_offset - look_delta,
		}
	}

	fn place(
		&self,
		orbit: &mut PreviewOrbit,
		transforms: &Query<&GlobalTransform>,
	) -> Option<(Transform, Vec3)> {
		let origin = match self.anchor {
			Some(anchor) => transforms.get(anchor).ok()?.translation(),
			None => Vec3::ZERO,
		};
		let look_at = origin + self.look_delta;
		let camera = look_at + orbit.swing(self.camera_delta);
		Some((Transform::from_translation(camera).looking_at(look_at, Vec3::Y), look_at))
	}
}

/// Why a rig focus has not framed yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusWait {
	/// No preview rig with the focus role.
	Rig,
	/// The socket bone has not been mapped on this rig yet.
	Socket(Entity),
	/// This rig's proportional pose has not been written yet.
	Pose(Entity),
	/// This rig is not parented onto its socket yet.
	Attach(Entity),
}

type FocusRigs<'w, 's> = Query<
	'w,
	's,
	(
		&'static CharacterRig,
		&'static BoneMap,
		&'static RigBindScales,
		&'static ActiveRigPose,
		Has<ResolvedPoseApplied>,
		Has<SocketRefRoot>,
		Has<SocketRefApplied>,
	),
>;

/// Preview rigs plus the hierarchy needed to resolve a socket's rest pose.
#[derive(SystemParam)]
struct FocusRigScene<'w, 's> {
	roots: Query<'w, 's, &'static CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: FocusRigs<'w, 's>,
	parents: Query<'w, 's, &'static ChildOf>,
	locals: Query<'w, 's, &'static Transform, Without<Camera3d>>,
	transforms: Query<'w, 's, &'static GlobalTransform>,
}

impl FocusRigScene<'_, '_> {
	/// Waits until the focus rig and every rig above it are posed and
	/// socketed, so a head focus never frames a head still parked at the
	/// character origin.
	fn resolve(&self, focus: CameraFocus) -> Result<FocusFrame, FocusWait> {
		let role = match focus.rig {
			FocusRig::Body => CharacterRigRole::Body,
			FocusRig::Head => CharacterRigRole::Head,
		};
		for members in &self.roots {
			let Some((rig, bones, posed)) = members.iter().find_map(|member| {
				let (rig, bones, _, _, posed, ..) = self.rigs.get(member).ok()?;
				(rig.role == role).then_some((member, bones, posed))
			}) else {
				continue;
			};
			let anchor = match (focus.socket, bones.by_name.get(focus.socket)) {
				("root", _) => rig,
				(_, Some(bone)) => *bone,
				(_, None) if posed => rig,
				(_, None) => return Err(FocusWait::Socket(rig)),
			};
			let rest = self.rest_rotations(members);
			let rotation = self.rest_rotation(anchor, &rest)?;
			return Ok(FocusFrame::socketed(anchor, rotation, focus));
		}
		Err(FocusWait::Rig)
	}

	/// Head armatures carry none of the humanoid landmark bones, so the rigs
	/// pose pass never marks them; they have no proportional pose to wait on.
	fn awaits_pose(role: CharacterRigRole, posed: bool) -> bool {
		!posed && role != CharacterRigRole::Head
	}

	/// Bind rotation with the proportional pose offset, keyed by bone entity.
	fn rest_rotations(&self, members: &CharacterMembers) -> HashMap<Entity, Quat> {
		let mut rest = HashMap::new();
		for member in members.iter() {
			let Ok((_, bones, bind, pose, ..)) = self.rigs.get(member) else {
				continue;
			};
			for (name, bone) in &bones.by_name {
				if let Some(bind_rotation) = bind.rotations.get(name) {
					rest.insert(*bone, pose.pose.rotation_for_bone(name) * *bind_rotation);
				}
			}
		}
		rest
	}

	fn rest_rotation(
		&self,
		entity: Entity,
		rest: &HashMap<Entity, Quat>,
	) -> Result<Quat, FocusWait> {
		let mut rotation = Quat::IDENTITY;
		let mut current = Some(entity);
		while let Some(entity) = current {
			if let Ok((rig, _, _, _, posed, socketed, fulfilled)) = self.rigs.get(entity) {
				if Self::awaits_pose(rig.role, posed) {
					return Err(FocusWait::Pose(entity));
				}
				if socketed && !fulfilled {
					return Err(FocusWait::Attach(entity));
				}
			}
			let local = match rest.get(&entity) {
				Some(local) => *local,
				None => self.locals.get(entity).map_or(Quat::IDENTITY, |local| local.rotation),
			};
			rotation = local * rotation;
			current = self.parents.get(entity).ok().map(ChildOf::parent);
		}
		Ok(rotation.normalize())
	}
}

pub struct CharacterPreviewPlugin;

impl Plugin for CharacterPreviewPlugin {
	fn build(&self, app: &mut App) {
		add_character_components_host::<ClothingPreview>(app);
		if !app.is_plugin_added::<FirearmComponentsPlugin>() {
			app.add_plugins(FirearmComponentsPlugin);
		}
		if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
			app.add_plugins(GrenadeMaterialPlugin);
		}
		add_firearm_components_host::<GeneratedFirearm>(app);
		app.init_resource::<PreviewSyncState>()
			.init_resource::<PendingCameraFocus>()
			.insert_resource(GlobalAmbientLight {
				color: Color::WHITE,
				brightness: 200.0,
				..default()
			})
			.add_systems(Startup, setup_lighting)
			.add_systems(
				Update,
				(
					sync_preview.after(SpinRevealSystems::Apply),
					realize_preview_grenades.after(sync_preview),
					stamp_preview_animation
						.after(sync_preview)
						.after(CharacterHostSystems::Membership)
						.before(characters::CharacterMotionSystems::Anim),
					queue_preview_camera_focus,
					steer_preview_orbit.after(queue_preview_camera_focus),
				),
			)
			.add_systems(PostUpdate, apply_preview_camera_focus.after(TransformSystems::Propagate));
	}
}

fn setup_lighting(mut commands: Commands) {
	commands.spawn((
		CharacterPreviewLight,
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		CharacterPreviewLight,
		DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}

fn sync_preview(
	mut commands: Commands,
	screens: Query<Entity, With<CharacterScreen>>,
	spin_screens: Query<Entity, With<SpinRevealScreen>>,
	home: Query<Entity, With<HomeScreen>>,
	gallery: Query<Entity, With<GalleryScreen>>,
	weapons: Query<Entity, With<WeaponGalleryScreen>>,
	weapon_requests: Query<Entity, With<RequestShowWeapons>>,
	menu_state: Res<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	spin: Option<Res<SpinRevealCurrent>>,
	active: Option<Res<ActiveCharacter>>,
	save_root: Option<Res<SaveRoot>>,
	mut sync: ResMut<PreviewSyncState>,
	mut pending: ResMut<PendingCameraFocus>,
	roots: Query<Entity, With<CharacterPreviewRoot>>,
) {
	if !weapons.is_empty() || !weapon_requests.is_empty() {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	}

	if return_to.is_some_and(|return_to| return_to.uses_live_world_player()) {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	}

	if !screens.is_empty() {
		respawn_from_menu(
			&mut commands,
			&mut sync,
			&mut pending,
			&roots,
			&menu_state.0,
			format!("{:?}", menu_state.0),
			false,
		);
		return;
	}

	if !spin_screens.is_empty() {
		// Starter gallery paints the granted bag on the HUD. A live inspect
		// mesh plus camera would stack on those tiles.
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		if let Some(spin) = spin {
			sync.key = format!("spin-gallery:{}", spin.items.len());
		}
		return;
	}

	if home.is_empty() && gallery.is_empty() {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	}

	let Some(active) = active else {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	};
	let Some(save_root) = save_root else {
		return;
	};
	let Some(menu) = menu_for_saved(&save_root, active.id) else {
		clear_preview(&mut commands, &mut sync, &mut pending, &roots);
		return;
	};
	respawn_from_menu(
		&mut commands,
		&mut sync,
		&mut pending,
		&roots,
		&menu,
		format!("active:{}", active.id.to_hex()),
		true,
	);
}

fn respawn_from_menu(
	commands: &mut Commands,
	sync: &mut PreviewSyncState,
	pending: &mut PendingCameraFocus,
	roots: &Query<Entity, With<CharacterPreviewRoot>>,
	menu: &CharacterMenu,
	key: String,
	force_focus: bool,
) {
	if sync.key == key && !roots.is_empty() {
		return;
	}
	let species = menu.species.value;
	let reframe = force_focus || pending.focus.is_none() || sync.species != Some(species);
	sync.key = key;
	sync.species = Some(species);
	sync.anim = Some(AnimRef::from(menu.animation()));
	for entity in roots {
		commands.entity(entity).try_despawn();
	}
	spawn_from_menu(commands, menu);
	pending.frame = None;
	if reframe {
		pending.set_focus(menu.body_preview_focus());
	}
}

fn clear_preview(
	commands: &mut Commands,
	sync: &mut PreviewSyncState,
	pending: &mut PendingCameraFocus,
	roots: &Query<Entity, With<CharacterPreviewRoot>>,
) {
	for entity in roots {
		commands.entity(entity).try_despawn();
	}
	sync.key.clear();
	sync.species = None;
	sync.anim = None;
	pending.clear();
}

fn menu_for_saved(root: &SaveRoot, id: character_persist::CharacterId) -> Option<CharacterMenu> {
	let model = character_model_user::load(root, id).ok()?;
	let inventory = character_inventory_user::load(root, id).ok()?;
	Some(CharacterMenu::for_saved(model.name, &model.appearance, inventory))
}

pub(crate) fn spawn_firearm(
	commands: &mut Commands,
	spec: FirearmSpec,
	transform: Transform,
) -> Vec<Entity> {
	let preview = GeneratedFirearm::from_spec(spec);
	spawn_firearm_components(commands, &preview, transform, firearm_bounds(&preview))
}

/// Preview orb diameter. Held gameplay scale is 0.2; inspect needs to read
/// in the right-hand pane without using the firearm camera distance.
#[cfg(test)]
const GRENADE_PREVIEW_SCALE: f32 = 0.7;

#[cfg(test)]
fn spawn_from_item(commands: &mut Commands, item: &InventoryItem) {
	if let Some(spec) = item.firearm_spec() {
		for entity in spawn_firearm(commands, spec, Transform::IDENTITY) {
			commands.entity(entity).insert(CharacterPreviewRoot);
		}
		return;
	}
	if item.grenade_spec().is_some() {
		spawn_grenade_preview(commands);
		return;
	}
	let Some(mesh) = item.mesh() else {
		return;
	};
	let Some(material) = item.material() else {
		return;
	};
	spawn_clothed(
		commands,
		&ClothingPreview {
			layer: ClothingLayer::new(mesh, material.color, ClothingHost::HUMANOID)
				.with_material(material.id),
		},
	);
}

#[cfg(test)]
fn spawn_grenade_preview(commands: &mut Commands) {
	commands.spawn((
		Name::new("preview-grenade"),
		CharacterPreviewRoot,
		GrenadePreview,
		Transform::from_scale(Vec3::splat(GRENADE_PREVIEW_SCALE)),
		Visibility::default(),
	));
}

fn realize_preview_grenades(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<GrenadeMaterial>>,
	pending: Query<Entity, (With<GrenadePreview>, Without<Mesh3d>)>,
) {
	for entity in &pending {
		commands.entity(entity).insert((
			Mesh3d(meshes.add(grenade_mesh())),
			MeshMaterial3d(materials.add(grenade_material())),
		));
	}
}

/// Unskinned garment in bind pose. Camera framing is per clothing kind.
#[derive(Clone, PartialEq)]
struct ClothingPreview {
	layer: ClothingLayer,
}

impl Default for ClothingPreview {
	fn default() -> Self {
		Self {
			layer: ClothingLayer::new(
				ClothingMesh::TankTop,
				ItemColor::Natural,
				ClothingHost::HUMANOID,
			),
		}
	}
}

impl CharacterComponents for ClothingPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("clothing", vec![self.layer.preview_part_node()])
	}
}

fn spawn_from_menu(commands: &mut Commands, menu: &CharacterMenu) {
	match menu.species.value {
		ConceptSpecies::Braidman => spawn_clothed(commands, &menu.braidman_config().clothed()),
		ConceptSpecies::Brenal => spawn_clothed(commands, &menu.brenal_config().clothed()),
		ConceptSpecies::Caole => spawn_clothed(commands, &menu.caole_config().clothed()),
		ConceptSpecies::Epiphant => spawn_clothed(commands, &menu.epiphant_config().clothed()),
		ConceptSpecies::Hars => spawn_clothed(commands, &menu.hars_config().clothed()),
		ConceptSpecies::Yilter => spawn_clothed(commands, &menu.ylter_config().clothed()),
		ConceptSpecies::Sonyak => spawn_clothed(commands, &menu.sonyak_config().clothed()),
		ConceptSpecies::Claber => spawn_clothed(commands, &menu.claber_config().clothed()),
		ConceptSpecies::Croconot => spawn_clothed(commands, &menu.croconot_config().clothed()),
		ConceptSpecies::Brodler => spawn_clothed(commands, &menu.brodler_config().clothed()),
		ConceptSpecies::Mygr => spawn_clothed(commands, &menu.mygr_config().clothed()),
		ConceptSpecies::Dui => spawn_clothed(commands, &menu.dui_config().clothed()),
		ConceptSpecies::Lidder => spawn_clothed(commands, &menu.lidder_config().clothed()),
		ConceptSpecies::Chupri => spawn_clothed(commands, &menu.chupri_config().clothed()),
		ConceptSpecies::Brokker => spawn_clothed(commands, &menu.brokker_config().clothed()),
		ConceptSpecies::Tipple => spawn_clothed(commands, &menu.tipple_config().clothed()),
		ConceptSpecies::Topple => spawn_clothed(commands, &menu.topple_config().clothed()),
		ConceptSpecies::Kispar => spawn_clothed(commands, &menu.kispar_config().clothed()),
		ConceptSpecies::Tapp => spawn_clothed(commands, &menu.tapp_config().clothed()),
		ConceptSpecies::Kaller => spawn_clothed(commands, &menu.kaller_config().clothed()),
		ConceptSpecies::Kappler => spawn_clothed(commands, &menu.kappler_config().clothed()),
		ConceptSpecies::Wumbus => spawn_clothed(commands, &menu.wumbus_config().clothed()),
		ConceptSpecies::Lero => spawn_clothed(commands, &menu.lero_config().clothed()),
		ConceptSpecies::Spibmom => spawn_clothed(commands, &menu.spibmom_config().clothed()),
		ConceptSpecies::Grener => spawn_clothed(commands, &menu.grener_config().clothed()),
		ConceptSpecies::Thumplus => spawn_clothed(commands, &menu.thumplus_config().clothed()),
		ConceptSpecies::Mistler => spawn_clothed(commands, &menu.mistler_config().clothed()),
		ConceptSpecies::Tuberwaber => spawn_clothed(commands, &menu.tuberwaber_config().clothed()),
	}
}

fn spawn_clothed<T>(commands: &mut Commands, character: &T)
where
	T: CharacterComponents + Clone + Default + Unpin + Send + Sync + 'static,
{
	let bounds = character_bounds(character);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(character.clone());
	let entity = commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				Transform::IDENTITY
			},
		))
		.id();
	commands
		.entity(entity)
		.insert(CharacterPreviewRoot)
		.remove::<ApplyTerrainPitch>();
}

fn stamp_preview_animation(
	mut commands: Commands,
	sync: Res<PreviewSyncState>,
	menu_state: Res<CharacterMenuState>,
	character_screens: Query<Entity, With<CharacterScreen>>,
	spin_screens: Query<Entity, With<SpinRevealScreen>>,
	roots: Query<&CharacterMembers, With<CharacterPreviewRoot>>,
	rigs: Query<&CharacterRig>,
	anims: Query<&AnimRefRoot>,
) {
	if !spin_screens.is_empty() {
		return;
	}
	let desired = if character_screens.is_empty() {
		let Some(anim) = sync.anim else {
			return;
		};
		anim
	} else {
		AnimRef::from(menu_state.0.animation())
	};
	for members in &roots {
		for member in members.iter() {
			if !rigs.get(member).is_ok_and(|rig| rig.role == CharacterRigRole::Body) {
				continue;
			}
			let needs_clip = match anims.get(member) {
				Ok(root) => root.0 != desired,
				Err(_) => true,
			};
			if needs_clip {
				commands.entity(member).try_insert(AnimRefRoot(desired));
			}
		}
	}
}

fn queue_preview_camera_focus(
	return_to: Option<Res<CharacterEditorReturn>>,
	mut events: MessageReader<CharacterMenuEvent<MenuEvent>>,
	mut pending: ResMut<PendingCameraFocus>,
) {
	if return_to.is_some_and(|return_to| return_to.uses_live_world_player()) {
		return;
	}
	for event in events.read() {
		if let CharacterMenuEvent::CameraFocus(focus) = event {
			pending.set_focus(*focus);
		}
	}
}

fn steer_preview_orbit(
	time: Res<Time>,
	return_to: Option<Res<CharacterEditorReturn>>,
	config: Option<Res<VirtualPadConfig>>,
	gamepads: Query<&Gamepad>,
	mut pending: ResMut<PendingCameraFocus>,
) {
	if pending.focus.is_none()
		|| return_to.is_some_and(|return_to| return_to.uses_live_world_player())
	{
		return;
	}
	let deadzone = config.map_or(Deadzone(0.15), |config| config.stick_deadzone);
	let stick = gamepads
		.iter()
		.map(|gamepad| GamepadAxes::look_stick(gamepad, deadzone))
		.sum::<Vec2>()
		.clamp_length_max(1.0);
	if stick != Vec2::ZERO {
		pending.orbit.steer(stick, time.delta_secs());
	}
}

fn apply_preview_camera_focus(
	return_to: Option<Res<CharacterEditorReturn>>,
	mut pending: ResMut<PendingCameraFocus>,
	mut cameras: Query<
		(&mut Transform, &mut CameraController, &mut Camera, &mut Projection),
		With<Camera3d>,
	>,
	windows: Query<&Window, With<PrimaryWindow>>,
	home: Query<(), With<HomeScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
	character: Query<(), With<CharacterScreen>>,
	weapons: Query<(), With<WeaponGalleryScreen>>,
	firearm_previews: Query<(), (With<CharacterPreviewRoot>, With<FirearmRoot>)>,
	scene: FocusRigScene,
) {
	if !weapons.is_empty() || return_to.is_some_and(|return_to| return_to.uses_live_world_player())
	{
		return;
	}
	let pending = &mut *pending;
	let Some(focus) = pending.focus else {
		return;
	};
	let Ok((mut transform, mut controller, mut camera, mut projection)) = cameras.single_mut()
	else {
		return;
	};
	camera.viewport = None;
	if matches!(&*projection, Projection::Perspective(p) if p.fov != PREVIEW_FOV) {
		if let Projection::Perspective(perspective) = projection.as_mut() {
			perspective.fov = PREVIEW_FOV;
		}
	}
	let frame = if !firearm_previews.is_empty() {
		let (camera, look_at) = firearm_preview_camera(PREVIEW_FOV);
		FocusFrame::world(camera, look_at)
	} else if let Some(frame) = pending.frame {
		frame
	} else if pending.world_space {
		FocusFrame::world(focus.camera_offset, focus.look_at_offset)
	} else {
		match scene.resolve(focus) {
			Ok(frame) => frame,
			Err(wait) => {
				if pending.waiting != Some(wait) {
					debug!("preview focus {:?}/{} waiting: {wait:?}", focus.rig, focus.socket);
					pending.waiting = Some(wait);
				}
				return;
			}
		}
	};
	pending.waiting = None;
	let Some((target, look_at)) = frame.place(&mut pending.orbit, &scene.transforms) else {
		pending.frame = None;
		return;
	};
	pending.frame = Some(frame);
	*transform = target;
	if !home.is_empty() || !gallery.is_empty() {
		let aspect = windows
			.single()
			.ok()
			.map(|window| window.width() / window.height().max(1.0))
			.unwrap_or(16.0 / 9.0);
		offset_camera_into_display_pane(&mut transform, &projection, look_at, aspect);
	} else if !character.is_empty() {
		let (aspect, width) = windows.single().ok().map_or((16.0 / 9.0, 1920.0), |window| {
			(window.width() / window.height().max(1.0), window.width().max(1.0))
		});
		offset_camera_for_character_panel(&mut transform, &projection, look_at, aspect, width);
	}
	sync_controller_from_transform(&mut controller, &transform);
}

fn sync_controller_from_transform(controller: &mut CameraController, transform: &Transform) {
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	controller.yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	controller.pitch = sin_pitch.clamp(-1.0, 1.0).asin();
}

/// Shift the framed body into the right-hand display pane so the left menu
/// does not sit on top of it. NDC x = 0 is screen center.
fn offset_camera_into_display_pane(
	transform: &mut Transform,
	projection: &Projection,
	look_at: Vec3,
	aspect: f32,
) {
	let Projection::Perspective(perspective) = projection else {
		return;
	};
	let depth = transform.translation.distance(look_at).max(0.1);
	let half_height = (perspective.fov * 0.5).tan() * depth;
	let half_width = half_height * aspect.max(0.1);
	let pane_center = (DESCRIPTION_PANE_LEFT_PERCENT + 100.0) * 0.005;
	let ndc_x = pane_center * 2.0 - 1.0;
	transform.translation -= *transform.right() * ndc_x * half_width;
	let ndc_y = 0.16;
	transform.translation -= *transform.up() * ndc_y * half_height;
}

/// Shift the framed body into the open preview left of the 480px HUD panel.
fn offset_camera_for_character_panel(
	transform: &mut Transform,
	projection: &Projection,
	look_at: Vec3,
	aspect: f32,
	window_width: f32,
) {
	let Projection::Perspective(perspective) = projection else {
		return;
	};
	let depth = transform.translation.distance(look_at).max(0.1);
	let half_height = (perspective.fov * 0.5).tan() * depth;
	let half_width = half_height * aspect.max(0.1);
	let visible = (window_width - CHARACTER_PANEL_WIDTH_PX).max(1.0);
	let pane_center = (visible * 0.5) / window_width.max(1.0);
	let ndc_x = pane_center * 2.0 - 1.0;
	transform.translation -= *transform.right() * ndc_x * half_width;
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_4;

	use character_creation_menus::BODY_FOCUS;
	use character_creation_menus::focus::{
		EYE_FOCUS, HEAD_ROOT_FOCUS, spin_reveal_firearm_focus, spin_reveal_grenade_focus,
	};

	const EPS: f32 = 1e-4;

	#[test]
	fn a_resting_orbit_keeps_the_framed_offset() {
		let offset = Vec3::new(-1.0, 1.0, 4.0);
		assert!(PreviewOrbit::default().swing(offset).abs_diff_eq(offset, EPS));
	}

	#[test]
	fn orbit_swings_around_the_look_at_without_changing_distance() {
		let offset = Vec3::new(0.0, 0.0, 2.0);
		let mut orbit = PreviewOrbit::default();
		orbit.steer(Vec2::X, 0.5);
		let swung = orbit.swing(offset);
		assert!((swung.length() - offset.length()).abs() < EPS);
		assert!(swung.x < 0.0, "right stick swings the camera toward -X: {swung}");
		assert!(swung.y.abs() < EPS);
	}

	#[test]
	fn orbit_pitch_stops_short_of_the_pole() {
		let offset = Vec3::new(0.0, 0.0, 2.0);
		let mut orbit = PreviewOrbit::default();
		orbit.steer(Vec2::NEG_Y, 10.0);
		let swung = orbit.swing(offset);
		let elevation = (swung.y / swung.length()).asin();
		assert!((elevation - ORBIT_ELEVATION_LIMIT).abs() < EPS);
		assert!((orbit.pitch - ORBIT_ELEVATION_LIMIT).abs() < EPS);
	}

	#[test]
	fn orbit_resets_only_when_the_framed_part_changes() {
		let mut pending = PendingCameraFocus::default();
		pending.set_focus(HEAD_ROOT_FOCUS);
		pending.orbit.steer(Vec2::ONE, 0.25);
		pending.frame = Some(FocusFrame::world(Vec3::Z, Vec3::ZERO));
		let steered = pending.orbit;

		pending.set_focus(HEAD_ROOT_FOCUS);
		assert_eq!(pending.orbit, steered);
		assert!(pending.frame.is_some());

		pending.set_focus(EYE_FOCUS);
		assert_eq!(pending.orbit, PreviewOrbit::default());
		assert!(pending.frame.is_none());
	}

	fn resolve_eye(scene: FocusRigScene) -> Result<FocusFrame, FocusWait> {
		scene.resolve(EYE_FOCUS)
	}

	/// `posed` mirrors the rigs pose pass: bind rotations and the marker land
	/// together, and head armatures never get either.
	fn rig(role: CharacterRigRole, bone: &str, entity: Entity, posed: bool) -> impl Bundle {
		let mut bind = RigBindScales::default();
		if posed {
			bind.rotations.insert(bone.to_string(), Quat::IDENTITY);
		}
		(
			CharacterRig { role, ..default() },
			BoneMap { by_name: [(bone.to_string(), entity)].into_iter().collect() },
			bind,
			ActiveRigPose::default(),
		)
	}

	struct Fixture {
		world: World,
		body: Entity,
		neck: Entity,
		head: Entity,
		eye: Entity,
	}

	impl Fixture {
		fn new() -> Self {
			use characters::MemberOf;

			let mut world = World::new();
			let root = world.spawn((CharacterPreviewRoot, Transform::IDENTITY)).id();
			let neck = world.spawn(Transform::from_xyz(0.0, 1.5, 0.0)).id();
			let body = world
				.spawn((rig(CharacterRigRole::Body, "neck", neck, true), MemberOf(root)))
				.id();
			world.entity_mut(body).insert(ChildOf(root));
			world.entity_mut(neck).insert(ChildOf(body));
			let eye = world.spawn(Transform::from_xyz(0.03, 0.1, 0.1)).id();
			let head = world
				.spawn((
					rig(CharacterRigRole::Head, "eye_socket.L", eye, false),
					MemberOf(root),
					SocketRefRoot::default(),
					ChildOf(root),
				))
				.id();
			world.entity_mut(eye).insert(ChildOf(head));
			Self { world, body, neck, head, eye }
		}

		fn resolve(&mut self) -> Result<Result<FocusFrame, FocusWait>, String> {
			use bevy::ecs::system::RunSystemOnce;
			self.world.run_system_once(resolve_eye).map_err(|error| format!("{error:?}"))
		}
	}

	#[test]
	fn head_focus_waits_for_the_socket_and_ignores_the_clip()
	-> Result<(), Box<dyn std::error::Error>> {
		let mut fixture = Fixture::new();
		fixture.world.entity_mut(fixture.body).insert(ResolvedPoseApplied);
		assert_eq!(fixture.resolve()?, Err(FocusWait::Attach(fixture.head)));

		fixture
			.world
			.entity_mut(fixture.head)
			.insert((ChildOf(fixture.neck), SocketRefApplied));
		let clip = Transform::from_xyz(0.0, 1.5, 0.0).with_rotation(Quat::from_rotation_y(1.0));
		fixture.world.entity_mut(fixture.neck).insert(clip);
		let frame = fixture.resolve()?.map_err(|wait| format!("{wait:?}"))?;
		assert_eq!(frame.anchor, Some(fixture.eye));
		assert!(frame.camera_delta.abs_diff_eq(EYE_FOCUS.camera_offset, EPS));
		Ok(())
	}

	#[test]
	fn head_focus_waits_for_the_body_pose() -> Result<(), Box<dyn std::error::Error>> {
		let mut fixture = Fixture::new();
		fixture
			.world
			.entity_mut(fixture.head)
			.insert((ChildOf(fixture.neck), SocketRefApplied));
		assert_eq!(fixture.resolve()?, Err(FocusWait::Pose(fixture.body)));
		Ok(())
	}

	#[test]
	fn socketed_frame_orients_offsets_by_the_rest_rotation() {
		let rotation = Quat::from_rotation_y(FRAC_PI_4 * 2.0);
		let frame = FocusFrame::socketed(Entity::PLACEHOLDER, rotation, EYE_FOCUS);
		let camera = frame.look_delta + frame.camera_delta;
		assert!(camera.abs_diff_eq(rotation * EYE_FOCUS.camera_offset, EPS));
		assert!(frame.look_delta.abs_diff_eq(rotation * EYE_FOCUS.look_at_offset, EPS));
	}

	#[test]
	fn grenade_preview_spawns_an_inspect_orb() {
		let mut world = World::new();
		spawn_from_item(&mut world.commands(), &InventoryItem::standard_grenade());
		world.flush();
		let count = world.query::<(&GrenadePreview, &CharacterPreviewRoot)>().iter(&world).count();
		assert_eq!(count, 1);
	}

	#[test]
	fn spin_reveal_does_not_keep_a_live_item_mesh() {
		use bevy::ecs::system::RunSystemOnce;
		use character_items::{ClothingMaterial, ClothingMesh};
		use menu_screens::SpinRevealCurrent;

		let mut world = World::new();
		world.insert_resource(CharacterMenuState::default());
		world.insert_resource(PreviewSyncState::default());
		world.insert_resource(PendingCameraFocus {
			focus: Some(BODY_FOCUS),
			world_space: true,
			..default()
		});
		world.spawn(SpinRevealScreen);
		world.insert_resource(SpinRevealCurrent {
			items: vec![InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Red,
			)],
			revealed: true,
		});
		let leftover = world.spawn(CharacterPreviewRoot).id();
		world.run_system_once(sync_preview).expect("sync");
		world.flush();
		assert!(world.get_entity(leftover).is_err(), "leftover inspect mesh must despawn");
		assert_eq!(world.query::<&CharacterPreviewRoot>().iter(&world).count(), 0);
		assert!(world.resource::<PendingCameraFocus>().focus.is_none());
	}

	#[test]
	fn grenade_inspect_is_closer_than_the_firearm_kit() {
		let gun = spin_reveal_firearm_focus();
		let charge = spin_reveal_grenade_focus();
		assert!(charge.camera_offset.length() < gun.camera_offset.length());
		assert!(GRENADE_PREVIEW_SCALE > 0.2);
	}

	#[test]
	fn body_root_focus_is_applied_in_preview_root_space() {
		let mut pending = PendingCameraFocus::default();
		pending.set_focus(BODY_FOCUS);
		assert!(pending.world_space);
		pending.set_focus(HEAD_ROOT_FOCUS);
		assert!(!pending.world_space);
	}

	#[test]
	fn species_change_replaces_stale_body_focus() {
		let braidman = CharacterMenu::default();
		let mut spibmom = CharacterMenu::default();
		assert!(spibmom.apply(MenuEvent::SetSpecies(ConceptSpecies::Spibmom)));
		let mut pending = PendingCameraFocus::default();
		pending.set_focus(braidman.body_preview_focus());
		pending.set_focus(spibmom.body_preview_focus());
		assert_eq!(pending.focus, Some(spibmom.body_preview_focus()));
		assert_ne!(pending.focus, Some(braidman.body_preview_focus()));
		assert!(pending.world_space);
	}
}
