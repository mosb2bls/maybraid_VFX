//! Playground-only gallery of generated firearm kits on Braidman operators.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use camera_controls::look::CameraLookEnabled;
use character_items::{FirearmSpec, ItemRng};
use characters::species::braidman::BraidmanConfig;
use characters::{character_bounds, CharacterMotionSystems, CharacterRecipe, ComponentsOnly};
use characters_playground::CameraController;
use firearm_user::{
	pose_held_firearm, stamp_holding_arms, sync_hands_to_firearm, FirearmUser, HeldFirearm,
};
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use maybraid_menu_controller::MenuController;
use menu_components::{screen_back_scene, BrandModeLine};
use menu_screens::{
	add_menu_input, add_menu_show_systems, MenuScreen, MenuScreens, MenuShowRequest,
};
use player::PlayerLook;

use crate::preview::spawn_firearm;

const GALLERY_COUNT: usize = 20;
const GALLERY_COLS: usize = 5;
const SPACING_X: f32 = 2.2;
const SPACING_Z: f32 = 1.8;

/// Queue a generated-weapon gallery spawn.
#[derive(Component, Debug, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowWeapons;

/// Marker on the spawned weapon-gallery HUD root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WeaponGalleryScreen;

/// Tag on spawned operator / kit hosts so preview sync does not treat them as
/// the character preview (and so they can be cleaned up with the screen).
#[derive(Component, Debug, Clone, Copy)]
struct WeaponGalleryItem;

pub fn request_show_weapons(commands: &mut Commands) {
	commands.spawn(RequestShowWeapons);
}

pub struct WeaponGalleryPlugin;

impl Plugin for WeaponGalleryPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		add_menu_show_systems(app, apply_show_weapons);
		app.add_systems(
			Update,
			(
				despawn_weapon_gallery_items,
				sync_weapon_gallery_look,
				stamp_holding_arms,
				pose_held_firearm,
				sync_hands_to_firearm.after(CharacterMotionSystems::Anim),
			),
		);
	}
}

fn apply_show_weapons(
	mut commands: Commands,
	requests: Query<(), With<RequestShowWeapons>>,
	screens: MenuScreens,
	items: Query<Entity, With<WeaponGalleryItem>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if screens.take(&mut commands, &requests).is_none() {
		return;
	}
	for entity in &items {
		commands.entity(entity).try_despawn();
	}
	commands.spawn_scene(weapon_gallery_scene());
	spawn_gallery_operators(&mut commands);
	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		frame_weapon_gallery(&mut transform, &mut controller);
	}
}

fn spawn_gallery_operators(commands: &mut Commands) {
	let mut rng = ItemRng::from_entropy();
	let items = rng.random_gallery_firearms(GALLERY_COUNT);
	let rows = GALLERY_COUNT.div_ceil(GALLERY_COLS);
	for (index, item) in items.into_iter().enumerate() {
		let Some(spec) = item.firearm_spec() else {
			continue;
		};
		spawn_operator(commands, spec, cell_translation(index, rows));
	}
}

fn spawn_operator(commands: &mut Commands, spec: FirearmSpec, translation: Vec3) {
	let mut gun = Entity::PLACEHOLDER;
	for entity in spawn_firearm(commands, spec, Transform::IDENTITY) {
		commands.entity(entity).insert((HeldFirearm { scale: 1.0 }, WeaponGalleryItem));
		gun = entity;
	}
	let operator = commands
		.spawn((
			WeaponGalleryItem,
			FirearmUser::holding(gun),
			PlayerLook { yaw: FRAC_PI_2, ..default() },
			Transform::from_translation(translation),
			Visibility::default(),
		))
		.id();
	let visual = spawn_operator_visual(commands);
	commands.entity(visual).insert(ChildOf(operator));
}

fn spawn_operator_visual(commands: &mut Commands) -> Entity {
	let clothed = BraidmanConfig::default_preview().clothed();
	let bounds = character_bounds(&clothed);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(clothed);
	commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				template_value(Transform::from_rotation(Quat::from_rotation_y(FRAC_PI_2)))
			},
		))
		.id()
}

fn cell_translation(index: usize, rows: usize) -> Vec3 {
	let col = (index % GALLERY_COLS) as f32;
	let row = (index / GALLERY_COLS) as f32;
	let cols = GALLERY_COLS as f32;
	let rows = rows.max(1) as f32;
	Vec3::new((col - (cols - 1.0) * 0.5) * SPACING_X, 0.0, (row - (rows - 1.0) * 0.5) * SPACING_Z)
}

fn frame_weapon_gallery(transform: &mut Transform, controller: &mut CameraController) {
	let look_at = Vec3::new(0.0, 1.0, 0.0);
	*transform = Transform::from_xyz(0.0, 2.2, 12.0).looking_at(look_at, Vec3::Y);
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	controller.yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	controller.pitch = sin_pitch.clamp(-1.0, 1.0).asin();
}

fn despawn_weapon_gallery_items(
	mut commands: Commands,
	screens: Query<(), With<WeaponGalleryScreen>>,
	pending: Query<(), With<RequestShowWeapons>>,
	items: Query<Entity, With<WeaponGalleryItem>>,
) {
	if !screens.is_empty() || !pending.is_empty() {
		return;
	}
	for entity in &items {
		commands.entity(entity).try_despawn();
	}
}

fn sync_weapon_gallery_look(
	screens: Query<(), With<WeaponGalleryScreen>>,
	mut look: ResMut<CameraLookEnabled>,
) {
	look.0 = !screens.is_empty();
}

fn weapon_gallery_scene() -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> =
		vec![Box::new(BrandModeLine::new("Weapons").scene()), Box::new(screen_back_scene())];
	bsn! {
		WeaponGalleryScreen
		MenuScreen
		MenuController
		BackgroundColor(Color::NONE)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		Children [ {children} ]
	}
}

#[cfg(test)]
mod tests {
	use super::{cell_translation, GALLERY_COLS, GALLERY_COUNT};

	#[test]
	fn grid_is_centered_on_the_origin() {
		let rows = GALLERY_COUNT.div_ceil(GALLERY_COLS);
		let first = cell_translation(0, rows);
		let last_col = cell_translation(GALLERY_COLS - 1, rows);
		assert!((first.x + last_col.x).abs() < 1e-5);
		assert!(first.y.abs() < 1e-5);
	}

	#[test]
	fn teardown_ignores_items_that_are_already_gone() -> Result<(), Box<dyn std::error::Error>> {
		use super::{despawn_weapon_gallery_items, WeaponGalleryItem};
		use bevy::ecs::system::RunSystemOnce;
		use bevy::prelude::*;

		let mut world = World::new();
		let stale = world.spawn(WeaponGalleryItem).id();
		world.despawn(stale);
		world.spawn(WeaponGalleryItem);
		world
			.run_system_once(despawn_weapon_gallery_items)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		let left = world.query::<&WeaponGalleryItem>().iter(&world).count();
		if left != 0 {
			return Err(format!("gallery items should despawn, left {left}").into());
		}
		Ok(())
	}
}
