//! Noise-dispatched land / water / power tiles and 2D AABB claims.

use bevy::camera::visibility::RenderLayers;
use bevy::math::bounding::{Aabb2d, IntersectsVolume};
use bevy::prelude::*;
use menu_components::HudFonts;
use noise::{Fbm, MultiFractal, NoiseFn, OpenSimplex};

use crate::cursor::SkillMapCursor;
use crate::map::{
	pinned_power_cells, render_layer, AuthoredMap, MapExtents, SkillKind, SkillMapId,
};
use crate::tile_material::SkillMapTileAssets;
use crate::user::{SkillMapHeld, SkillMapMember, SkillMapSession, SkillMapSteerLock, SkillMapUser};
use crate::viewport::{spawn_debraid, SkillMapViewportCamera};
use crate::{SkillMapEnabled, SkillMapEvent};

/// Half-extents for the cheap AABB claim test. Independent of the render mesh.
#[derive(Component, Clone, Copy, Debug)]
pub struct TileBounds {
	pub half: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
	Water,
	Land,
	Power(SkillKind),
}

/// POC thresholds: water below −0.1, power above a rarer high band, else land.
pub fn classify_noise(value: f32, kind: SkillKind) -> TileKind {
	if value < -0.1 {
		TileKind::Water
	} else if value > 0.35 {
		TileKind::Power(kind)
	} else {
		TileKind::Land
	}
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SkillMapTile {
	pub map: SkillMapId,
	pub kind: TileKind,
}

#[derive(Component)]
pub struct IgnoreRightCollisions;

#[derive(Component)]
pub struct RestoreSpentTiles {
	pub session: Entity,
	pub map: SkillMapId,
}

#[derive(Component)]
pub(crate) struct TileOverlap {
	tile: Entity,
}

pub fn spawn_map_tiles(
	commands: &mut Commands,
	spec: AuthoredMap,
	member: SkillMapMember,
	assets: &SkillMapTileAssets,
) {
	spawn_map_tiles_at(commands, spec, Some(member), assets, Vec2::ZERO, None);
}

pub fn spawn_map_tiles_at(
	commands: &mut Commands,
	spec: AuthoredMap,
	member: Option<SkillMapMember>,
	assets: &SkillMapTileAssets,
	origin: Vec2,
	parent: Option<Entity>,
) {
	let extents = MapExtents::default();
	let size = extents.tile_size();
	let layer = render_layer(spec.id);
	let noise = Fbm::<OpenSimplex>::new(spec.seed).set_frequency(spec.frequency);
	let pins = pinned_power_cells(extents.steps);
	let mid = extents.steps / 2;

	for x in 0..extents.steps {
		for y in 0..extents.steps {
			let local = extents.tile_center(x, y);
			let raw = noise.get([local.x as f64, local.y as f64]) as f32;
			let center = local + origin;
			let kind = if x == mid && y == mid {
				TileKind::Land
			} else if pins.contains(&(x, y)) {
				TileKind::Power(spec.kind)
			} else {
				classify_noise(raw, spec.kind)
			};
			spawn_tile(
				commands,
				spec.id,
				spec.kind,
				kind,
				center,
				size,
				layer.clone(),
				member,
				parent,
				assets,
			);
		}
	}
}

fn spawn_tile(
	commands: &mut Commands,
	map: SkillMapId,
	map_kind: SkillKind,
	kind: TileKind,
	center: Vec2,
	size: Vec2,
	layer: RenderLayers,
	member: Option<SkillMapMember>,
	parent: Option<Entity>,
	assets: &SkillMapTileAssets,
) {
	// Water and marks sit on land so a blob or a claim does not punch a hole.
	if matches!(kind, TileKind::Power(_) | TileKind::Water) {
		spawn_tile(
			commands,
			map,
			map_kind,
			TileKind::Land,
			center,
			size,
			layer.clone(),
			member,
			parent,
			assets,
		);
	}
	let z = match kind {
		TileKind::Power(_) => 0.2,
		TileKind::Water => 0.1,
		TileKind::Land => 0.0,
	};
	let mut entity = commands.spawn((
		Name::new("skill-map-tile"),
		SkillMapTile { map, kind },
		TileBounds { half: size * 0.5 },
		Mesh2d(assets.mesh.clone()),
		MeshMaterial2d(assets.material(kind, map_kind)),
		Transform::from_xyz(center.x, center.y, z),
		layer,
	));
	if let Some(member) = member {
		entity.insert(member);
	}
	let id = entity.id();
	if let Some(parent) = parent {
		commands.entity(parent).add_child(id);
	}
}

type CursorRow<'a> =
	(Entity, &'a SkillMapId, &'a SkillMapMember, &'a Transform, Option<&'a TileOverlap>);

type MapCameras<'w, 's> = Query<
	'w,
	's,
	(&'static SkillMapId, &'static SkillMapMember, &'static mut Transform),
	(With<SkillMapViewportCamera>, Without<SkillMapCursor>, Without<SkillMapTile>),
>;

#[allow(clippy::too_many_arguments)]
pub fn collide_tiles(
	mut commands: Commands,
	asset_server: Option<Res<AssetServer>>,
	enabled: Res<SkillMapEnabled>,
	mut users: Query<(Entity, &SkillMapUser, &SkillMapHeld, &mut SkillMapSteerLock)>,
	mut sessions: Query<&mut SkillMapSession>,
	cursors: Query<
		CursorRow<'_>,
		(With<SkillMapCursor>, Without<SkillMapViewportCamera>, Without<SkillMapTile>),
	>,
	tiles: Query<
		(Entity, &SkillMapTile, &SkillMapMember, &Transform, &TileBounds),
		(Without<IgnoreRightCollisions>, Without<SkillMapViewportCamera>, Without<SkillMapCursor>),
	>,
	mut events: MessageWriter<SkillMapEvent>,
	mut cameras: MapCameras,
) {
	if !enabled.0 {
		return;
	}

	for (cursor_entity, cursor_map, cursor_member, cursor_transform, overlap) in &cursors {
		let Some((user_entity, settings)) = users.iter().find_map(|(entity, user, held, lock)| {
			(entity == cursor_member.user && held.0 && !lock.locked())
				.then_some((entity, user.settings))
		}) else {
			continue;
		};

		let cursor_bounds = Aabb2d::new(cursor_transform.translation.xy(), Vec2::splat(5.0));
		let mut hit: Option<(Entity, SkillMapTile)> = None;
		for (tile_entity, tile, tile_member, tile_transform, bounds) in &tiles {
			if tile.map != *cursor_map || tile_member.session != cursor_member.session {
				continue;
			}
			if matches!(tile.kind, TileKind::Land) {
				continue;
			}
			let tile_bounds = Aabb2d::new(tile_transform.translation.xy(), bounds.half);
			if cursor_bounds.intersects(&tile_bounds) {
				hit = Some((tile_entity, *tile));
				break;
			}
		}

		match hit {
			None => {
				if overlap.is_some() {
					commands.entity(cursor_entity).remove::<TileOverlap>();
				}
			}
			Some((tile_entity, tile)) => {
				if overlap.is_some_and(|current| current.tile == tile_entity) {
					continue;
				}
				commands.entity(cursor_entity).insert(TileOverlap { tile: tile_entity });
				match tile.kind {
					TileKind::Water => {
						if let Ok((_, _, _, mut lock)) = users.get_mut(user_entity) {
							lock.arm(settings.water_lock_secs);
						}
						for (map, member, mut camera) in &mut cameras {
							if *map == tile.map && member.session == cursor_member.session {
								camera.translation.x = 0.0;
								camera.translation.y = 0.0;
							}
						}
						commands.spawn(RestoreSpentTiles {
							session: cursor_member.session,
							map: tile.map,
						});
						if let Ok(session) = sessions.get_mut(cursor_member.session) {
							let fonts = asset_server.as_ref().map(|server| HudFonts::load(server));
							spawn_debraid(
								&mut commands,
								fonts.as_ref(),
								&session,
								tile.map,
								settings.water_lock_secs,
							);
						}
						events.write(SkillMapEvent::Fail { user: user_entity, map: tile.map });
					}
					TileKind::Power(kind) => {
						commands
							.entity(tile_entity)
							.insert((Visibility::Hidden, IgnoreRightCollisions));
						events.write(SkillMapEvent::Claim { user: user_entity, kind });
					}
					TileKind::Land => {}
				}
			}
		}
	}
}

pub fn restore_spent_tiles(
	mut commands: Commands,
	requests: Query<(Entity, &RestoreSpentTiles), Added<RestoreSpentTiles>>,
	spent: Query<(Entity, &SkillMapTile, &SkillMapMember), With<IgnoreRightCollisions>>,
) {
	for (request, restore) in &requests {
		for (entity, tile, member) in &spent {
			if tile.map == restore.map && member.session == restore.session {
				commands
					.entity(entity)
					.insert(Visibility::Visible)
					.remove::<IgnoreRightCollisions>();
			}
		}
		commands.entity(request).despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn noise_bands_match_the_poc_water_cut() {
		assert_eq!(classify_noise(-0.2, SkillKind::Fireball), TileKind::Water);
		assert_eq!(classify_noise(0.0, SkillKind::Fireball), TileKind::Land);
		assert_eq!(classify_noise(0.4, SkillKind::Dumbwave), TileKind::Power(SkillKind::Dumbwave));
	}

	#[test]
	fn collide_queries_are_disjoint() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<SkillMapEnabled>()
			.add_message::<SkillMapEvent>()
			.add_systems(Update, collide_tiles);
		app.update();
	}
}
