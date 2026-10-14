//! Downed world-player retirement and POI-based replacement.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_ragdoll::CharacterRagdollSystems;
use damage::{DamageSystems, DespawnAfter, Downed};
use durham::{DurhamSurface, TerrainCellLayout, WorldBaseTerrain};
use firearm_user::FirearmUser;
use firearms::WeaponTrigger;
use grenade_user::{GrenadeUser, GrenadeUserSystems};
use lubikans::LubikanOwner;
use maybraid_character_controller::CharacterIntent;
use maybraid_input::{PadButton, VirtualPad};
use mob_characters::{LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use player::{CameraFollow, Player as MaybraidPlayer, PlayerUse};
use player_camera::{CameraController, FollowCamera};
use poi_intelligence::{
	place_nearby, NearbyFallback, NearbyQuery, PoiId, PoiInterest, PoiInterests, PoiKind,
	PoiRecord, PoiRegistry, PoiSystems,
};
use spotting_intelligence::SpotSubject;
use threat_intelligence::{Affiliations, ThreatSubject};
use world_player::{
	holding_elevation, player_position_above_surface, player_spawn_point_at, spawn_player_body,
	surface_or_hold_at, AwaitingTerrainSurface, CharacterLocomotion, CharacterSpecies,
	ModePlayerPolicies, MoveWish, OffTerrainAnchor, Player as VegetationPlayer, PlayerLifeEnded,
	PlayerLifeSet, PlayerSpawnXz, RequestSetCharacter, RequestSetCharacterAppearance,
	RespawnOrigin,
};

use layer_stack::ActiveGenerationMode;

use crate::control::strip_world_player_motor;
use crate::map_view::WorldMapView;
use crate::weapon::WorldPlayerAppearanceRequested;
use crate::{
	FirstWave, HcsgDemand, LodJobCounter, PlayerPositionWaypoints, WorldGameplayEnabled,
	WorldPlayerLoadout,
};

const MAP_OPEN_SECS: f32 = 0.18;
const STICK_REST: f32 = 0.28;
const STICK_FLICK: f32 = 0.45;
const STICK_ALIGN: f32 = 0.2;

/// World-player downed duration, nearby POI scan, and replacement interests.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerRespawnConfig {
	pub delay_secs: f32,
	pub poi_radius: f32,
	pub fallback: NearbyFallback,
	pub interests: PoiInterests,
}

impl Default for WorldPlayerRespawnConfig {
	fn default() -> Self {
		Self {
			delay_secs: 4.0,
			poi_radius: 320.0,
			fallback: NearbyFallback::new(60.0, 100.0),
			interests: default_player_respawn_interests(),
		}
	}
}

impl WorldPlayerRespawnConfig {
	/// Nearest POI outside the fallback ring's inner radius.
	pub fn nearby_query(&self) -> NearbyQuery {
		NearbyQuery::nearest_beyond(self.poi_radius, self.fallback.min_radius)
	}

	/// Cover the overhead view so labeled groves at the edge stay selectable.
	pub fn picker_query(&self, map_height: f32) -> NearbyQuery {
		let radius = (map_height * 1.2).max(self.poi_radius);
		NearbyQuery::nearest_beyond(radius, self.fallback.min_radius)
	}
}

#[derive(Debug)]
pub(crate) struct PendingPlayerRespawn {
	pub timer: Timer,
	pub death_at: Vec3,
	pub origin: RespawnOrigin,
	pub candidates: Vec<PoiId>,
	pub highlighted: Option<PoiId>,
	/// World XZ of [`Self::highlighted`]. Flick heading is measured from here.
	pub highlighted_at: Option<Vec2>,
	pub map_opened: bool,
	/// Stick is home; the next throw is one flick.
	pub stick_resting: bool,
	/// First Discovery life: no death glaze or bones.
	pub first_life: bool,
	/// Ring placement used when the registry has nothing selectable.
	pub fallback_at: Vec3,
	pub registry_revision: u64,
	/// Reused by [`refresh_picker_candidates`] while the respawn map is open.
	picker_nearby_scratch: Vec<PoiId>,
}

#[derive(Resource, Default)]
pub struct WorldPlayerRespawnState {
	pub(crate) pending: Option<PendingPlayerRespawn>,
	last_poi: Option<PoiId>,
	/// This world load already queued or skipped the first-life picker.
	first_spawn_offered: bool,
}

/// Clear per-session spawn latches so the next world load (or character) can pick again.
pub fn reset_first_spawn_offer(mut state: ResMut<WorldPlayerRespawnState>) {
	state.first_spawn_offered = false;
	state.last_poi = None;
	state.pending = None;
}

/// Discovery map picker confirmed a POI, or `None` for the fallback ring.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerChoseRespawnPoi {
	pub poi: Option<PoiId>,
}

#[derive(Component)]
struct PlayerDeathGlaze;

type DownedWorldPlayer<'a> = (
	Entity,
	&'a Transform,
	&'a mut LinearVelocity,
	Option<&'a FirearmUser>,
	Option<&'a GrenadeUser>,
	Option<&'a InventoryUser>,
);

pub struct WorldPlayerLifecyclePlugin;

impl Plugin for WorldPlayerLifecyclePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldPlayerRespawnConfig>()
			.init_resource::<WorldPlayerRespawnState>()
			.init_resource::<WorldMapView>()
			.add_message::<PlayerLifeEnded>()
			.add_message::<PlayerChoseRespawnPoi>()
			.add_message::<CharacterIntent>()
			.configure_sets(
				Update,
				PlayerLifeSet::Resolve.before(player_camera::PlayerCameraSystems::Look),
			)
			.add_systems(Startup, spawn_player_death_glaze)
			.add_systems(
				PostUpdate,
				queue_downed_world_player
					.after(DamageSystems::Down)
					.after(CharacterRagdollSystems::Handoff)
					.after(GrenadeUserSystems::Interrupt),
			)
			.add_systems(
				Update,
				(
					crate::first_wave::refresh_first_wave,
					queue_first_spawn_picker.after(crate::first_wave::refresh_first_wave),
					drive_respawn_picker,
					respawn_world_player
						.after(queue_first_spawn_picker)
						.after(drive_respawn_picker)
						.run_if(world_player_respawn_pending),
					close_orphaned_locked_respawn_map
						.after(respawn_world_player)
						.run_if(orphaned_locked_respawn_map),
					apply_camera_begin_life
						.after(respawn_world_player)
						.after(close_orphaned_locked_respawn_map),
				)
					.after(PoiSystems::Index)
					.in_set(PlayerLifeSet::Resolve),
			)
			.add_systems(Update, sync_player_death_glaze.after(respawn_world_player));
	}
}

fn spawn_player_death_glaze(mut commands: Commands) {
	commands.spawn((
		Name::new("player-death-glaze"),
		PlayerDeathGlaze,
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			..default()
		},
		BackgroundColor(death_glaze_color(0.0)),
		GlobalZIndex(i32::MAX - 4),
		Visibility::Hidden,
		Pickable::IGNORE,
	));
}

fn sync_player_death_glaze(
	gameplay: Res<WorldGameplayEnabled>,
	state: Res<WorldPlayerRespawnState>,
	map: Option<Res<WorldMapView>>,
	mut glaze: Query<(&mut BackgroundColor, &mut Visibility), With<PlayerDeathGlaze>>,
) {
	let Ok((mut color, mut visibility)) = glaze.single_mut() else {
		return;
	};
	let Some(pending) = state.pending.as_ref() else {
		color.0 = death_glaze_color(0.0);
		*visibility = Visibility::Hidden;
		return;
	};
	if pending.first_life || !gameplay.0 {
		color.0 = death_glaze_color(0.0);
		*visibility = Visibility::Hidden;
		return;
	}
	let map_open = map.is_some_and(|map| map.open);
	color.0 = death_glaze_color(death_glaze_alpha(&pending.timer, map_open));
	*visibility = Visibility::Visible;
}

fn queue_downed_world_player(
	config: Res<WorldPlayerRespawnConfig>,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	policies: Option<Res<ModePlayerPolicies>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	mut players: Query<DownedWorldPlayer<'_>, (With<VegetationPlayer>, Added<Downed>)>,
	mut triggers: Query<&mut WeaponTrigger>,
	bags: Query<&character_items::Inventory>,
) {
	for (player, transform, mut velocity, firearm, grenade, inventory) in &mut players {
		let now = mode.as_deref().and_then(|mode| mode.get().mode_id());
		let ends_life = policies.as_deref().is_some_and(|policies| policies.respawn_ends_life(now));
		state.pending = Some(PendingPlayerRespawn {
			timer: Timer::from_seconds(config.delay_secs.max(0.0), TimerMode::Once),
			death_at: transform.translation,
			origin: RespawnOrigin::began(now, ends_life),
			candidates: Vec::new(),
			highlighted: None,
			highlighted_at: None,
			map_opened: false,
			stick_resting: true,
			first_life: false,
			fallback_at: transform.translation,
			registry_revision: 0,
			picker_nearby_scratch: Vec::new(),
		});
		velocity.0 = Vec3::ZERO;
		if let Some(firearm) = firearm {
			if let Ok(mut trigger) = triggers.get_mut(firearm.held) {
				trigger.0 = false;
			}
			commands.entity(firearm.held).try_insert(DespawnAfter::seconds(0.0));
		}
		if let Some(grenade) = grenade {
			commands.entity(grenade.held).try_despawn();
		}
		if let Some(inventory) = inventory {
			if let (Some(loadout), Ok(bag)) = (loadout.as_deref_mut(), bags.get(inventory.bag)) {
				loadout.retarget_inventory(bag.clone());
			}
			commands.entity(inventory.bag).try_despawn();
		}
		strip_world_player_motor(&mut commands, player);
		commands.entity(player).remove::<(
			VegetationPlayer,
			MaybraidPlayer,
			CameraFollow,
			PlayerUse,
			FirearmUser,
			GrenadeUser,
			grenade_user::GrenadeThrow,
			InventoryUser,
			MoveWish,
			SpotSubject,
			ThreatSubject,
			Affiliations,
			LubikanOwner,
		)>();
	}
}

fn queue_first_spawn_picker(
	gameplay: Res<WorldGameplayEnabled>,
	spawn: Res<PlayerSpawnXz>,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	policies: Option<Res<ModePlayerPolicies>>,
	wave: Option<Res<FirstWave>>,
	demand: Option<Res<HcsgDemand>>,
	jobs: Option<Res<LodJobCounter>>,
	waypoints: Option<Res<PlayerPositionWaypoints>>,
	layout: Option<Res<TerrainCellLayout>>,
	base: Option<Res<WorldBaseTerrain>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut commands: Commands,
	players: Query<
		(Entity, &Transform, Option<&FirearmUser>, Option<&InventoryUser>),
		With<VegetationPlayer>,
	>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	if !gameplay.0 || state.first_spawn_offered || state.pending.is_some() {
		return;
	}
	// Latch before first-wave. Resume clears `resume_pending` once the body
	// is on the trail; waiting for HCSG would re-offer the picker ~30 s later.
	if spawn.0.is_some()
		|| waypoints.as_deref().is_some_and(|waypoints| waypoints.resume_pending)
		|| character_has_resume_trail(waypoints.as_deref(), layout.as_deref(), base.as_deref())
	{
		state.first_spawn_offered = true;
		return;
	}
	let ready = wave.as_deref().map(FirstWave::ready).unwrap_or_else(|| {
		FirstWave::sample(demand.as_deref(), jobs.map(|jobs| jobs.active()).unwrap_or(0)).ready()
	});
	if !ready {
		return;
	}
	let now = mode.as_deref().and_then(|mode| mode.get().mode_id());
	if !policies.as_deref().is_some_and(|policies| policies.pick_first_spawn(now)) {
		return;
	}
	let Ok((player, transform, firearm, inventory)) = players.single() else {
		return;
	};
	state.first_spawn_offered = true;
	state.pending = Some(PendingPlayerRespawn {
		timer: Timer::from_seconds(0.0, TimerMode::Once),
		death_at: transform.translation,
		origin: RespawnOrigin::began(now, false),
		candidates: Vec::new(),
		highlighted: None,
		highlighted_at: None,
		map_opened: false,
		stick_resting: true,
		first_life: true,
		fallback_at: transform.translation,
		registry_revision: 0,
		picker_nearby_scratch: Vec::new(),
	});
	retire_startup_player(&mut commands, player, firearm, inventory, &mut triggers);
}

fn character_has_resume_trail(
	waypoints: Option<&PlayerPositionWaypoints>,
	layout: Option<&TerrainCellLayout>,
	base: Option<&WorldBaseTerrain>,
) -> bool {
	let (Some(waypoints), Some(layout), Some(base)) = (waypoints, layout, base) else {
		return false;
	};
	let center = layout.region_center_xz().xz();
	let default_spawn =
		player_spawn_point_at(center, holding_elevation(&base.0, center.x, center.y));
	waypoints.has_resume_trail(default_spawn)
}

fn retire_startup_player(
	commands: &mut Commands,
	player: Entity,
	firearm: Option<&FirearmUser>,
	inventory: Option<&InventoryUser>,
	triggers: &mut Query<&mut WeaponTrigger>,
) {
	if let Some(firearm) = firearm {
		if let Ok(mut trigger) = triggers.get_mut(firearm.held) {
			trigger.0 = false;
		}
		commands.entity(firearm.held).try_insert(DespawnAfter::seconds(0.0));
	}
	if let Some(inventory) = inventory {
		commands.entity(inventory.bag).try_despawn();
	}
	commands.entity(player).try_despawn();
}

fn drive_respawn_picker(
	pad: Option<Res<VirtualPad>>,
	map: Res<WorldMapView>,
	registry: Res<PoiRegistry>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut chosen: MessageWriter<PlayerChoseRespawnPoi>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !map.open || !map.close_locked {
		return;
	}
	let Some(pending) = state.pending.as_mut() else {
		return;
	};
	if !pending.candidates.is_empty() {
		let stick = pad.as_deref().map(|pad| pad.move_stick).unwrap_or(Vec2::ZERO);
		let mut step = 0i32;
		if let Some(pad) = pad.as_deref() {
			if pad.just_pressed(PadButton::DpadUp) {
				step -= 1;
			}
			if pad.just_pressed(PadButton::DpadDown) {
				step += 1;
			}
		}
		if let Some(dir) = consume_stick_flick(&mut pending.stick_resting, stick) {
			let dir = map_flick_dir(dir);
			if let Some(from) = current_highlight_xz(pending, &registry) {
				if let Some(next) = next_candidate_in_direction(
					&pending.candidates,
					&registry,
					from,
					pending.highlighted,
					dir,
				) {
					set_highlighted(pending, next, &registry);
				}
			}
		} else if step != 0 {
			let len = pending.candidates.len() as i32;
			let current = pending
				.highlighted
				.and_then(|id| pending.candidates.iter().position(|candidate| *candidate == id))
				.unwrap_or(0) as i32;
			let next = (current + step).rem_euclid(len) as usize;
			if let Some(id) = pending.candidates.get(next).copied() {
				set_highlighted(pending, id, &registry);
			}
		} else if pending.highlighted.is_none() {
			if let Some(nearest) =
				nearest_candidate(&pending.candidates, &registry, pending.death_at.xz())
			{
				set_highlighted(pending, nearest, &registry);
			}
		}
	}
	if intents
		.read()
		.any(|intent| matches!(intent, CharacterIntent::Jump | CharacterIntent::StartInteraction))
	{
		chosen.write(PlayerChoseRespawnPoi { poi: pending.highlighted });
	}
}

/// Skip the mesh / material / surface params on the live combat path
/// ([#1209](https://github.com/ramate-io/maybraid/issues/1209)).
fn world_player_respawn_pending(state: Res<WorldPlayerRespawnState>) -> bool {
	state.pending.is_some()
}

/// A locked picker with no pending respawn is leftover UI, not a spawn.
fn orphaned_locked_respawn_map(
	state: Res<WorldPlayerRespawnState>,
	map: Res<WorldMapView>,
) -> bool {
	state.pending.is_none() && map.close_locked
}

fn close_orphaned_locked_respawn_map(
	live_player: Query<(), With<VegetationPlayer>>,
	mut map: ResMut<WorldMapView>,
) {
	if live_player.is_empty() {
		return;
	}
	if map.close_locked {
		map.close();
	}
}

/// A mode that keeps the player respawns near a POI. A mode whose policy ends
/// the life replaces the body where it fell and writes [`PlayerLifeEnded`].
/// Leaving that mode mid-respawn replaces the body at once and ends nothing.
#[allow(clippy::too_many_arguments)]
fn respawn_world_player(
	time: Res<Time>,
	gameplay: Res<WorldGameplayEnabled>,
	config: Res<WorldPlayerRespawnConfig>,
	registry: Res<PoiRegistry>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	locomotion: Res<CharacterLocomotion>,
	surface: DurhamSurface,
	mode: Option<Res<State<ActiveGenerationMode>>>,
	mut ended: MessageWriter<PlayerLifeEnded>,
	mut chosen: MessageReader<PlayerChoseRespawnPoi>,
	live_player: Query<(), With<VegetationPlayer>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut map: ResMut<WorldMapView>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let now = mode.as_deref().and_then(|mode| mode.get().mode_id());
	let abandoned = state.pending.as_ref().is_some_and(|pending| pending.origin.abandoned(now));
	if !gameplay.0 && !abandoned {
		return;
	}
	if !live_player.is_empty() {
		// First-life despawn is still in flight this frame.
		if !state.pending.as_ref().is_some_and(|pending| pending.first_life) {
			// Only the death picker is locked. A player-toggled map must stay open.
			if map.close_locked {
				map.close();
			}
			state.pending = None;
			return;
		}
	}
	let last_poi = state.last_poi;
	let Some(pending) = state.pending.as_mut() else {
		return;
	};
	pending.timer.tick(time.delta());

	if pending.origin.replace_in_place(now) {
		if !pending.timer.is_finished() && !pending.origin.replace_immediately(now) {
			return;
		}
		let death_at = pending.death_at;
		let origin = pending.origin;
		close_respawn_map(&mut map);
		state.pending = None;
		if origin.ends_life(now) {
			ended.write(PlayerLifeEnded);
		}
		finish_world_player_spawn(
			&mut commands,
			&mut meshes,
			&mut materials,
			locomotion.as_ref(),
			loadout.as_deref(),
			origin,
			now,
			player_position_above_surface(death_at),
			false,
		);
		return;
	}

	if !pending.map_opened
		&& (pending.first_life
			|| pending.timer.elapsed_secs() >= MAP_OPEN_SECS
			|| pending.timer.is_finished())
	{
		open_respawn_picker(pending, &mut map, &registry, &config, last_poi);
	} else if pending.map_opened {
		refresh_picker_candidates(pending, &map, &registry, &config, last_poi);
	}

	let Some(choice) = chosen.read().next().copied() else {
		return;
	};
	let origin = pending.origin;
	let fallback_at = pending.fallback_at;
	let placed = choice.poi.and_then(|id| registry.get(id).copied());
	close_respawn_map(&mut map);
	state.pending = None;
	let spawn = match placed {
		Some(record) => {
			state.last_poi = Some(record.id);
			spawn_pose_for_chosen_poi(&record, &surface)
		}
		None => ChosenPoiSpawn {
			position: player_spawn_point_at(
				fallback_at.xz(),
				surface_or_hold_at(&surface, fallback_at.xz()),
			),
			interior: false,
		},
	};
	finish_world_player_spawn(
		&mut commands,
		&mut meshes,
		&mut materials,
		locomotion.as_ref(),
		loadout.as_deref(),
		origin,
		now,
		spawn.position,
		spawn.interior,
	);
}

fn open_respawn_picker(
	pending: &mut PendingPlayerRespawn,
	map: &mut WorldMapView,
	registry: &PoiRegistry,
	config: &WorldPlayerRespawnConfig,
	last_poi: Option<PoiId>,
) {
	refresh_picker_candidates(pending, map, registry, config, last_poi);
	pending.stick_resting = true;
	pending.map_opened = true;
	map.open_at(pending.death_at.xz(), true);
}

fn refresh_picker_candidates(
	pending: &mut PendingPlayerRespawn,
	map: &WorldMapView,
	registry: &PoiRegistry,
	config: &WorldPlayerRespawnConfig,
	last_poi: Option<PoiId>,
) {
	pending.fallback_at = place_nearby(
		Some(registry),
		pending.death_at,
		config.nearby_query(),
		Some(&config.interests),
		last_poi,
		fallback_seed(pending.death_at),
		config.fallback,
	)
	.position;
	registry.collect_nearby_in(
		pending.death_at,
		config.picker_query(map.height),
		&config.interests,
		last_poi.as_slice(),
		&mut pending.picker_nearby_scratch,
	);
	let ids = prefer_building_poi_ids(&pending.picker_nearby_scratch, registry, pending.death_at);
	let revision = registry.membership_revision();
	if pending.registry_revision != revision || pending.candidates != ids {
		pending.registry_revision = revision;
		pending.candidates = ids;
	}
	validate_highlight(pending, registry);
}

fn validate_highlight(pending: &mut PendingPlayerRespawn, registry: &PoiRegistry) {
	let still_valid = pending
		.highlighted
		.filter(|id| pending.candidates.contains(id) && registry.get(*id).is_some());
	if let Some(id) = still_valid {
		pending.highlighted_at = registry.get(id).map(|record| record.position.xz());
		return;
	}
	if let Some(nearest) = nearest_candidate(&pending.candidates, registry, pending.death_at.xz()) {
		set_highlighted(pending, nearest, registry);
	} else {
		pending.highlighted = None;
		pending.highlighted_at = None;
	}
}

fn fallback_seed(death_at: Vec3) -> u64 {
	u64::from(death_at.x.to_bits()) ^ u64::from(death_at.z.to_bits()).wrapping_shl(1)
}

fn close_respawn_map(map: &mut WorldMapView) {
	map.request_begin_life();
	if map.close_locked {
		map.close();
	}
}

fn apply_camera_begin_life(
	mut map: ResMut<WorldMapView>,
	mut cameras: Query<&mut CameraController, With<FollowCamera>>,
) {
	if !map.begin_life {
		return;
	}
	map.begin_life = false;
	for mut controller in &mut cameras {
		controller.begin_life();
	}
}

fn current_highlight_xz(pending: &PendingPlayerRespawn, registry: &PoiRegistry) -> Option<Vec2> {
	pending.highlighted_at.or_else(|| {
		pending
			.highlighted
			.and_then(|id| registry.get(id))
			.map(|record| record.position.xz())
	})
}

fn set_highlighted(pending: &mut PendingPlayerRespawn, id: PoiId, registry: &PoiRegistry) {
	pending.highlighted = Some(id);
	pending.highlighted_at = registry.get(id).map(|record| record.position.xz());
}

/// North-up map looks down with world +Z as screen up, so screen-right is world −X.
fn map_flick_dir(stick: Vec2) -> Vec2 {
	Vec2::new(-stick.x, stick.y)
}

fn consume_stick_flick(resting: &mut bool, stick: Vec2) -> Option<Vec2> {
	let mag = stick.length();
	if mag < STICK_REST {
		*resting = true;
		return None;
	}
	if *resting && mag >= STICK_FLICK {
		*resting = false;
		Some(stick)
	} else {
		None
	}
}

fn next_candidate_in_direction(
	candidates: &[PoiId],
	registry: &PoiRegistry,
	from: Vec2,
	current: Option<PoiId>,
	dir: Vec2,
) -> Option<PoiId> {
	let points: Vec<_> = candidates
		.iter()
		.filter_map(|id| registry.get(*id).map(|record| (*id, record.position.xz())))
		.collect();
	nearest_in_direction(from, current, dir, &points)
}

fn nearest_in_direction(
	from: Vec2,
	current: Option<PoiId>,
	dir: Vec2,
	points: &[(PoiId, Vec2)],
) -> Option<PoiId> {
	let dir = dir.normalize_or_zero();
	if dir == Vec2::ZERO {
		return None;
	}
	points
		.iter()
		.filter_map(|(id, xz)| {
			if Some(*id) == current {
				return None;
			}
			let delta = *xz - from;
			let dist = delta.length();
			if dist < 0.5 {
				return None;
			}
			let align = (delta / dist).dot(dir);
			if align < STICK_ALIGN {
				return None;
			}
			// Prefer a well-aimed far mark over a closer POI that is only loosely
			// in the cone, so a flick toward an on-screen grove can reach it.
			let score = dist / (align * align);
			Some((*id, score, dist, align))
		})
		.min_by(|a, b| {
			a.1.total_cmp(&b.1)
				.then_with(|| a.2.total_cmp(&b.2))
				.then_with(|| b.3.total_cmp(&a.3))
				.then_with(|| a.0.cmp(&b.0))
		})
		.map(|(id, _, _, _)| id)
}

fn nearest_candidate(candidates: &[PoiId], registry: &PoiRegistry, focus: Vec2) -> Option<PoiId> {
	candidates
		.iter()
		.filter_map(|id| registry.get(*id).copied())
		.min_by(|a, b| {
			a.position
				.xz()
				.distance(focus)
				.total_cmp(&b.position.xz().distance(focus))
				.then_with(|| a.id.cmp(&b.id))
		})
		.map(|record| record.id)
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	(a.xz() - b.xz()).length()
}

#[allow(clippy::too_many_arguments)]
fn finish_world_player_spawn(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	locomotion: &CharacterLocomotion,
	loadout: Option<&WorldPlayerLoadout>,
	origin: RespawnOrigin,
	now: Option<std::any::TypeId>,
	position: Vec3,
	interior: bool,
) {
	let player = spawn_player_body(commands, meshes, materials, locomotion, position);
	// First-drop hold: snap teleports onto composed height and waits for a
	// local trimesh. Without this marker, gravity can turn on while the body
	// is still sitting on a base-noise or death-pose Y under the mesh.
	// Interior rooms keep their authored floor Y; terrain snap would pull the
	// capsule under the storey into the building volume.
	commands.entity(player).insert(AwaitingTerrainSurface);
	if interior {
		commands.entity(player).insert(OffTerrainAnchor { translation: position });
	}
	crate::control::apply_world_player_motor(commands, player);
	if let Some(loadout) = loadout {
		if !origin.ends_life(now) {
			commands.entity(player).insert(WorldPlayerAppearanceRequested);
		}
		commands.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
	} else {
		commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
	}
}

fn death_glaze_alpha(timer: &Timer, map_open: bool) -> f32 {
	if map_open {
		return 0.12;
	}
	let fade_in = (timer.elapsed_secs() / MAP_OPEN_SECS).clamp(0.0, 1.0);
	0.68 * fade_in
}

fn death_glaze_color(alpha: f32) -> Color {
	Color::srgba(0.2, 0.005, 0.025, alpha)
}

const MAX_VEGETATION_RESPAWN: usize = 4;

fn default_player_respawn_interests() -> PoiInterests {
	PoiInterests::new([
		PoiInterest::new(SALOON_POI, 1.7),
		PoiInterest::new(LOCAL_POI, 1.6),
		PoiInterest::new(URBAN_POI, 1.55),
		PoiInterest::new(VEGETATION_POI, 0.4),
	])
}

fn is_building_poi(kind: PoiKind) -> bool {
	kind == LOCAL_POI || kind == URBAN_POI || kind == SALOON_POI
}

/// House / storey / lounge pins live inside a building. Urban settlement pins
/// stay outdoor even when the registry marks them local.
fn is_interior_building_poi(record: &PoiRecord) -> bool {
	record.local && (record.kind == LOCAL_POI || record.kind == SALOON_POI)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ChosenPoiSpawn {
	position: Vec3,
	interior: bool,
}

/// Local interiors keep the POI floor. Outdoor pins snap to composed terrain.
fn spawn_pose_for_chosen_poi(record: &PoiRecord, surface: &DurhamSurface) -> ChosenPoiSpawn {
	if is_interior_building_poi(record) {
		ChosenPoiSpawn { position: player_position_above_surface(record.position), interior: true }
	} else {
		ChosenPoiSpawn {
			position: player_spawn_point_at(
				record.position.xz(),
				surface_or_hold_at(surface, record.position.xz()),
			),
			interior: false,
		}
	}
}

fn prefer_building_poi_ids(ids: &[PoiId], registry: &PoiRegistry, death_at: Vec3) -> Vec<PoiId> {
	let nearer =
		|a: &(PoiId, f32), b: &(PoiId, f32)| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0));
	let mut buildings = Vec::new();
	let mut groves = Vec::new();
	let mut trees = Vec::new();
	let mut other = Vec::new();
	for id in ids {
		let record = registry.get(*id).expect("picker ids are indexed");
		let entry = (*id, xz_distance(death_at, record.position));
		if is_building_poi(record.kind) {
			buildings.push(entry);
		} else if record.kind == VEGETATION_POI && record.global {
			groves.push(entry);
		} else if record.kind == VEGETATION_POI {
			trees.push(entry);
		} else {
			other.push(entry);
		}
	}
	buildings.sort_by(nearer);
	groves.sort_by(nearer);
	other.sort_by(nearer);
	trees.sort_by(nearer);
	trees.truncate(MAX_VEGETATION_RESPAWN);
	buildings.extend(groves);
	buildings.extend(other);
	buildings.extend(trees);
	buildings.into_iter().map(|(id, _)| id).collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{FirstWave, UnitXTile};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::math::bounding::Aabb3d;
	use durham::TerrainCellLayout;
	use layer_stack::GenerationMode;
	use lod::hcsg::{spawn_worker, HcsgStorage};
	use world_player::WorldBaseTerrain;

	#[test]
	fn fallback_respawn_moves_away_from_the_death_point() {
		let death = Vec3::new(10.0, 4.0, -5.0);
		let config = WorldPlayerRespawnConfig::default();
		let placed = poi_intelligence::place_nearby(
			None,
			death,
			config.nearby_query(),
			None,
			None,
			42,
			config.fallback,
		);
		let distance = (placed.position - death).xz().length();
		assert!((config.fallback.min_radius..=config.fallback.max_radius).contains(&distance));
		assert_eq!(placed.position.y, death.y);
		assert!(placed.poi.is_none());
	}

	#[test]
	fn player_respawn_prefers_urban_pois() {
		let interests = WorldPlayerRespawnConfig::default().interests;
		assert_eq!(interests.weight(SALOON_POI), Some(1.7));
		assert_eq!(interests.weight(LOCAL_POI), Some(1.6));
		assert_eq!(interests.weight(URBAN_POI), Some(1.55));
		assert!(interests.weight(VEGETATION_POI).is_some_and(|weight| weight < 0.5));
	}

	fn registry_from_records(records: &[poi_intelligence::PoiRecord]) -> PoiRegistry {
		let mut registry = PoiRegistry::default();
		for record in records {
			let poi = poi_intelligence::Poi::new(record.id, record.kind)
				.with_arrival_radius(record.arrival_radius)
				.with_salience(record.salience);
			registry
				.upsert(record.entity, poi, record.position, record.local, record.global)
				.expect("test poi");
		}
		registry
	}

	#[test]
	fn respawn_picker_keeps_buildings_ahead_of_a_few_trees() {
		let death = Vec3::new(0.0, 1.0, 0.0);
		let mut records = vec![poi_intelligence::PoiRecord {
			id: PoiId(1),
			entity: Entity::from_bits(1),
			kind: LOCAL_POI,
			position: Vec3::new(90.0, 1.0, 0.0),
			arrival_radius: 6.0,
			salience: 1.0,
			local: true,
			global: false,
		}];
		for index in 0..6u64 {
			records.push(poi_intelligence::PoiRecord {
				id: PoiId(10 + index),
				entity: Entity::from_bits(10 + index),
				kind: VEGETATION_POI,
				position: Vec3::new(70.0 + index as f32 * 4.0, 1.0, 0.0),
				arrival_radius: 4.0,
				salience: 1.0,
				local: true,
				global: false,
			});
		}
		let registry = registry_from_records(&records);
		let ids = records.iter().map(|record| record.id).collect::<Vec<_>>();
		let ordered = prefer_building_poi_ids(&ids, &registry, death);
		assert_eq!(registry.get(ordered[0]).expect("ordered").kind, LOCAL_POI);
		assert_eq!(
			ordered
				.iter()
				.filter(|id| {
					let record = registry.get(**id).expect("ordered");
					record.kind == VEGETATION_POI && !record.global
				})
				.count(),
			MAX_VEGETATION_RESPAWN
		);
	}

	#[test]
	fn respawn_picker_keeps_named_groves_outside_the_tree_cap() {
		let death = Vec3::ZERO;
		let mut records = Vec::new();
		for index in 0..6u64 {
			records.push(poi_intelligence::PoiRecord {
				id: PoiId(10 + index),
				entity: Entity::from_bits(10 + index),
				kind: VEGETATION_POI,
				position: Vec3::new(70.0 + index as f32 * 4.0, 1.0, 0.0),
				arrival_radius: 4.0,
				salience: 0.55,
				local: true,
				global: false,
			});
		}
		records.push(poi_intelligence::PoiRecord {
			id: PoiId(99),
			entity: Entity::from_bits(99),
			kind: VEGETATION_POI,
			position: Vec3::new(0.0, 1.0, 280.0),
			arrival_radius: 24.0,
			salience: 1.25,
			local: false,
			global: true,
		});
		let registry = registry_from_records(&records);
		let ids = records.iter().map(|record| record.id).collect::<Vec<_>>();
		let ordered = prefer_building_poi_ids(&ids, &registry, death);
		assert!(ordered.contains(&PoiId(99)));
		assert_eq!(
			ordered
				.iter()
				.filter(|id| {
					let record = registry.get(**id).expect("ordered");
					record.kind == VEGETATION_POI && !record.global
				})
				.count(),
			MAX_VEGETATION_RESPAWN
		);
	}

	#[test]
	#[ignore]
	fn respawn_picker_refresh_timing() {
		use poi_intelligence::{NearbyChoice, NearbyQuery};

		const POI_COUNT: usize = 256;
		const REFRESHES: usize = 2_000;
		const RADIUS: f32 = 480.0;
		let death = Vec3::new(12.0, 8.0, -4.0);
		let interests = WorldPlayerRespawnConfig::default().interests;
		let query =
			NearbyQuery { radius: RADIUS, min_radius: 60.0, choice: NearbyChoice::Weighted };

		let mut registry = PoiRegistry::default();
		for index in 0..POI_COUNT {
			let id = PoiId(index as u64 + 1);
			let angle = (index as f32 / POI_COUNT as f32) * core::f32::consts::TAU;
			let position = death + Vec3::new(angle.cos() * 200.0, 0.0, angle.sin() * 200.0);
			let kind = if index % 5 == 0 {
				URBAN_POI
			} else if index % 3 == 0 {
				VEGETATION_POI
			} else {
				LOCAL_POI
			};
			let global = index % 11 == 0;
			let poi = poi_intelligence::Poi::new(id, kind).with_arrival_radius(8.0);
			registry
				.upsert(Entity::from_bits(index as u64 + 1), poi, position, true, global)
				.expect("upsert poi");
		}

		fn prefer_building_pois_records(
			records: Vec<poi_intelligence::PoiRecord>,
			death_at: Vec3,
		) -> Vec<PoiId> {
			let nearer = |a: &poi_intelligence::PoiRecord, b: &poi_intelligence::PoiRecord| {
				xz_distance(death_at, a.position)
					.total_cmp(&xz_distance(death_at, b.position))
					.then_with(|| a.id.cmp(&b.id))
			};
			let mut buildings = Vec::new();
			let mut groves = Vec::new();
			let mut trees = Vec::new();
			let mut other = Vec::new();
			for record in records {
				if is_building_poi(record.kind) {
					buildings.push(record);
				} else if record.kind == VEGETATION_POI && record.global {
					groves.push(record);
				} else if record.kind == VEGETATION_POI {
					trees.push(record);
				} else {
					other.push(record);
				}
			}
			buildings.sort_by(nearer);
			groves.sort_by(nearer);
			other.sort_by(nearer);
			trees.sort_by(nearer);
			trees.truncate(MAX_VEGETATION_RESPAWN);
			buildings.extend(groves);
			buildings.extend(other);
			buildings.extend(trees);
			buildings.into_iter().map(|record| record.id).collect()
		}

		let mut scratch = Vec::new();
		let clone_start = std::time::Instant::now();
		for _ in 0..REFRESHES {
			let records = registry.nearby_in(death, query, &interests, &[]);
			let ids = prefer_building_pois_records(records, death);
			std::hint::black_box(ids);
		}
		let clone_elapsed = clone_start.elapsed();

		let id_start = std::time::Instant::now();
		for _ in 0..REFRESHES {
			registry.collect_nearby_in(death, query, &interests, &[], &mut scratch);
			let ordered = prefer_building_poi_ids(&scratch, &registry, death);
			std::hint::black_box(ordered);
		}
		let id_elapsed = id_start.elapsed();

		eprintln!(
			"respawn_picker_refresh_timing: {POI_COUNT} pois × {REFRESHES} refreshes — nearby_in+prefer_records {:?}, collect+prefer_building_poi_ids {:?}",
			clone_elapsed, id_elapsed,
		);
	}

	#[test]
	fn default_respawn_waits_four_seconds_and_scans_nearby() {
		let config = WorldPlayerRespawnConfig::default();
		assert_eq!(config.delay_secs, 4.0);
		assert_eq!(config.poi_radius, 320.0);
		assert_eq!(config.fallback, NearbyFallback::new(60.0, 100.0));
		assert_eq!(config.nearby_query().min_radius, config.fallback.min_radius);
		assert!(config.picker_query(420.0).radius > config.poi_radius);
	}

	#[test]
	fn death_glaze_fades_in_and_holds_a_map_vignette() {
		let mut timer = Timer::from_seconds(4.0, TimerMode::Once);
		assert_eq!(death_glaze_alpha(&timer, false), 0.0);
		timer.tick(std::time::Duration::from_secs_f32(0.5));
		assert!((death_glaze_alpha(&timer, false) - 0.68).abs() < 1e-5);
		timer.tick(std::time::Duration::from_secs_f32(3.4));
		assert!((death_glaze_alpha(&timer, false) - 0.68).abs() < 1e-5);
		assert!((death_glaze_alpha(&timer, true) - 0.12).abs() < 1e-5);
	}

	#[test]
	fn downed_player_is_retired_and_queues_a_replacement() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnConfig>();
		world.init_resource::<WorldPlayerRespawnState>();
		let held = world.spawn(WeaponTrigger(true)).id();
		let bag = world.spawn_empty().id();
		let player = world
			.spawn((
				VegetationPlayer,
				MaybraidPlayer,
				CameraFollow,
				Transform::from_xyz(3.0, 4.0, 5.0),
				LinearVelocity(Vec3::X),
				FirearmUser::holding(held),
				InventoryUser::carrying(bag),
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
			))
			.id();

		world
			.run_system_once(queue_downed_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<VegetationPlayer>(player).is_none());
		assert!(world.get::<MaybraidPlayer>(player).is_none());
		assert_eq!(
			world.get::<LinearVelocity>(player).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		assert_eq!(world.get::<WeaponTrigger>(held).map(|trigger| trigger.0), Some(false));
		assert!(world.get::<DespawnAfter>(held).is_some());
		assert!(!world.entities().contains(bag));
		let pending = &world.resource::<WorldPlayerRespawnState>().pending;
		assert!(pending.is_some());
		Ok(())
	}

	struct EndsLife;
	struct OtherMode;

	impl GenerationMode for EndsLife {}
	impl GenerationMode for OtherMode {}

	fn respawn_world(timer_secs: f32, gameplay: bool, still_there: bool) -> World {
		let mut world = World::new();
		world.insert_resource(WorldPlayerRespawnConfig { delay_secs: timer_secs, ..default() });
		let began = Some(std::any::TypeId::of::<EndsLife>());
		world.insert_resource(WorldPlayerRespawnState {
			pending: Some(PendingPlayerRespawn {
				timer: Timer::from_seconds(timer_secs, TimerMode::Once),
				death_at: Vec3::new(3.0, 4.0, 5.0),
				origin: RespawnOrigin::began(began, true),
				candidates: Vec::new(),
				highlighted: None,
				highlighted_at: None,
				map_opened: false,
				stick_resting: true,
				first_life: false,
				fallback_at: Vec3::new(3.0, 4.0, 5.0),
				registry_revision: 0,
				picker_nearby_scratch: Vec::new(),
			}),
			..default()
		});
		world.insert_resource(Time::<()>::default());
		world.insert_resource(WorldGameplayEnabled(gameplay));
		world.init_resource::<WorldMapView>();
		world.init_resource::<Messages<PlayerChoseRespawnPoi>>();
		world.init_resource::<PoiRegistry>();
		world.init_resource::<CharacterLocomotion>();
		world.init_resource::<HcsgStorage>();
		world.init_resource::<TerrainCellLayout>();
		world.insert_resource(WorldBaseTerrain(durham::BaseTerrainNoise::from_config(
			&durham::TerrainConfig::new(42),
		)));
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.init_resource::<Messages<PlayerLifeEnded>>();
		world.insert_resource(State::new(if still_there {
			ActiveGenerationMode::of::<EndsLife>()
		} else {
			ActiveGenerationMode::of::<OtherMode>()
		}));
		world.insert_resource(crate::WorldPlayerLoadout::new(
			"life",
			characters::CharacterAppearance::default(),
			character_items::Inventory::default(),
		));
		world
	}

	#[test]
	fn respawn_system_gate_is_pending_only() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnState>();
		let pending = world
			.run_system_once(world_player_respawn_pending)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!pending);
		Ok(())
	}

	#[test]
	fn orphaned_locked_map_closes_when_a_live_player_has_no_pending() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnState>();
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		let orphaned = world
			.run_system_once(orphaned_locked_respawn_map)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(orphaned);
		world.spawn(VegetationPlayer);
		world
			.run_system_once(close_orphaned_locked_respawn_map)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldMapView>().open);
		assert!(!world.resource::<WorldMapView>().close_locked);
		assert!(world.resource::<WorldMapView>().begin_life);
		Ok(())
	}

	#[test]
	fn a_live_player_does_not_close_an_unlocked_map() -> anyhow::Result<()> {
		let mut world = respawn_world(0.0, true, true);
		world.spawn(VegetationPlayer);
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::new(3.0, 5.0),
			height: crate::map_view::DEFAULT_MAP_HEIGHT,
			close_locked: false,
			begin_life: false,
		});
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldMapView>().open);
		assert!(!world.resource::<WorldMapView>().close_locked);
		assert!(!world.resource::<WorldMapView>().begin_life);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		Ok(())
	}

	#[test]
	fn a_life_ending_respawn_stays_where_it_fell() -> anyhow::Result<()> {
		let mut world = respawn_world(0.0, true, true);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let ended: Vec<_> = world.resource_mut::<Messages<PlayerLifeEnded>>().drain().collect();
		assert_eq!(ended, vec![PlayerLifeEnded]);
		let mut bodies = world
			.query_filtered::<(&Transform, Has<WorldPlayerAppearanceRequested>), With<VegetationPlayer>>(
			);
		let (body, requested) = bodies.single(&world)?;
		assert_eq!(body.translation.xz(), Vec2::new(3.0, 5.0));
		assert!(!requested, "the next life's loadout dresses the body");
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(!world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().begin_life);
		Ok(())
	}

	#[test]
	fn leaving_the_mode_mid_respawn_replaces_the_body_at_once() -> anyhow::Result<()> {
		let mut world = respawn_world(4.0, false, true);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(
			world.resource::<WorldPlayerRespawnState>().pending.is_some(),
			"a paused death in the same mode keeps waiting"
		);

		world.insert_resource(State::new(ActiveGenerationMode::of::<OtherMode>()));
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let ended: Vec<_> = world.resource_mut::<Messages<PlayerLifeEnded>>().drain().collect();
		assert!(ended.is_empty(), "leaving ends no life");
		let mut bodies = world.query_filtered::<(), With<VegetationPlayer>>();
		assert_eq!(bodies.iter(&world).count(), 1);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none(), "glaze clears");
		assert!(!world.resource::<WorldMapView>().open);
		Ok(())
	}

	fn discovery_respawn_world(elapsed: f32) -> World {
		let mut world = respawn_world(4.0, true, true);
		world.insert_resource(WorldPlayerRespawnConfig { delay_secs: 4.0, ..default() });
		let mut state = world.resource_mut::<WorldPlayerRespawnState>();
		let pending = state.pending.as_mut().unwrap();
		pending.origin = RespawnOrigin::began(Some(std::any::TypeId::of::<EndsLife>()), false);
		pending.timer.set_elapsed(std::time::Duration::from_secs_f32(elapsed));
		world
	}

	#[test]
	fn discovery_respawn_waits_for_a_map_pick() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		let mut bodies = world.query_filtered::<(), With<VegetationPlayer>>();
		assert_eq!(bodies.iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn discovery_respawn_spawns_at_the_chosen_poi() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		let poi = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			poi,
			poi_intelligence::Poi::new(PoiId(1), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(80.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.write_message(PlayerChoseRespawnPoi { poi: Some(PoiId(1)) });
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut bodies =
			world.query_filtered::<(&Transform, Has<OffTerrainAnchor>), With<VegetationPlayer>>();
		let (body, anchored) = bodies.single(&world)?;
		assert_eq!(body.translation.xz(), Vec2::new(80.0, 5.0));
		assert!(!anchored, "urban pins stay outdoor");
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(!world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().begin_life);
		Ok(())
	}

	#[test]
	fn discovery_respawn_holds_above_unpublished_terrain() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		let poi = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			poi,
			poi_intelligence::Poi::new(PoiId(1), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(80.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.write_message(PlayerChoseRespawnPoi { poi: Some(PoiId(1)) });
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut bodies = world
			.query_filtered::<(&Transform, Has<AwaitingTerrainSurface>), With<VegetationPlayer>>();
		let (body, awaiting) = bodies.single(&world)?;
		let base = &world.resource::<WorldBaseTerrain>().0;
		let expected = player_spawn_point_at(
			Vec2::new(80.0, 5.0),
			world_player::holding_elevation(base, 80.0, 5.0),
		);
		assert_eq!(body.translation, expected);
		assert!(body.translation.y > base.height_at(80.0, 5.0) + 50.0);
		assert!(awaiting);
		Ok(())
	}

	#[test]
	fn discovery_respawn_keeps_an_interior_poi_floor() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		let poi = world.spawn_empty().id();
		let floor = Vec3::new(80.0, 12.0, 5.0);
		world.resource_mut::<PoiRegistry>().upsert(
			poi,
			poi_intelligence::Poi::new(PoiId(1), LOCAL_POI).with_arrival_radius(4.0),
			floor,
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.write_message(PlayerChoseRespawnPoi { poi: Some(PoiId(1)) });
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut bodies = world.query_filtered::<
			(&Transform, Has<AwaitingTerrainSurface>, Has<OffTerrainAnchor>),
			With<VegetationPlayer>,
		>();
		let (body, awaiting, anchored) = bodies.single(&world)?;
		assert_eq!(body.translation, player_position_above_surface(floor));
		assert!(awaiting);
		assert!(anchored);
		Ok(())
	}

	#[test]
	fn local_rooms_are_interior_and_urban_pins_are_not() {
		let room = PoiRecord {
			id: PoiId(1),
			entity: Entity::from_bits(1),
			kind: LOCAL_POI,
			position: Vec3::new(8.0, 12.0, 3.0),
			arrival_radius: 4.0,
			salience: 1.0,
			local: true,
			global: false,
		};
		let lounge = PoiRecord { kind: SALOON_POI, ..room };
		let urban = PoiRecord { kind: URBAN_POI, ..room };
		let grove = PoiRecord { kind: VEGETATION_POI, local: false, global: true, ..room };
		assert!(is_interior_building_poi(&room));
		assert!(is_interior_building_poi(&lounge));
		assert!(!is_interior_building_poi(&urban));
		assert!(!is_interior_building_poi(&grove));
	}

	#[test]
	fn discovery_respawn_waits_until_the_player_picks() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		let mut bodies = world.query_filtered::<(), With<VegetationPlayer>>();
		assert_eq!(bodies.iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn a_new_life_forces_third_person() -> anyhow::Result<()> {
		use player_camera::CameraPov;

		let mut world = World::new();
		world.insert_resource(WorldMapView { begin_life: true, ..default() });
		world.spawn((
			FollowCamera::default(),
			CameraController {
				pov: CameraPov::Map,
				resume_pov: CameraPov::FirstPerson,
				..default()
			},
		));
		world
			.run_system_once(apply_camera_begin_life)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let controller = world.query::<&CameraController>().single(&world)?;
		assert_eq!(controller.pov, CameraPov::ThirdPerson);
		assert!(!world.resource::<WorldMapView>().begin_life);
		Ok(())
	}

	#[test]
	fn a_stick_flick_picks_the_next_closest_poi_in_that_direction() {
		let west = PoiId(1);
		let near_east = PoiId(2);
		let far_east = PoiId(3);
		let north = PoiId(4);
		let picked = nearest_in_direction(
			Vec2::ZERO,
			Some(west),
			Vec2::X,
			&[
				(west, Vec2::new(-10.0, 0.0)),
				(near_east, Vec2::new(20.0, 2.0)),
				(far_east, Vec2::new(80.0, 1.0)),
				(north, Vec2::new(4.0, 40.0)),
			],
		);
		assert_eq!(picked, Some(near_east));
	}

	#[test]
	fn a_stick_flick_can_reach_a_well_aimed_far_poi() {
		let here = PoiId(1);
		let beside = PoiId(2);
		let ahead = PoiId(3);
		let picked = nearest_in_direction(
			Vec2::ZERO,
			Some(here),
			Vec2::Y,
			&[(here, Vec2::ZERO), (beside, Vec2::new(20.0, 8.0)), (ahead, Vec2::new(4.0, 90.0))],
		);
		assert_eq!(picked, Some(ahead));
	}

	#[test]
	fn a_stick_flick_waits_for_the_stick_to_come_home() {
		let mut resting = true;
		assert_eq!(consume_stick_flick(&mut resting, Vec2::X), Some(Vec2::X));
		assert!(!resting);
		assert_eq!(consume_stick_flick(&mut resting, Vec2::X), None);
		assert_eq!(consume_stick_flick(&mut resting, Vec2::ZERO), None);
		assert!(resting);
		assert_eq!(consume_stick_flick(&mut resting, Vec2::NEG_X), Some(Vec2::NEG_X));
	}

	#[test]
	fn a_stick_flick_moves_the_picker_highlight() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		let west = world.spawn_empty().id();
		let east = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			west,
			poi_intelligence::Poi::new(PoiId(1), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(-30.0, 4.0, 5.0),
			true,
			false,
		)?;
		world.resource_mut::<PoiRegistry>().upsert(
			east,
			poi_intelligence::Poi::new(PoiId(2), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(40.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		{
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let pending = state.pending.as_mut().unwrap();
			pending.candidates = vec![PoiId(1), PoiId(2)];
			pending.highlighted = Some(PoiId(1));
			pending.highlighted_at = Some(Vec2::new(-30.0, 5.0));
			pending.stick_resting = true;
		}
		world.insert_resource(WorldMapView {
			open: true,
			focus: Vec2::new(3.0, 5.0),
			height: crate::map_view::DEFAULT_MAP_HEIGHT,
			close_locked: true,
			begin_life: false,
		});
		let mut pad = VirtualPad::default();
		pad.move_stick = Vec2::NEG_X;
		world.insert_resource(pad);
		world.init_resource::<Messages<PlayerChoseRespawnPoi>>();
		world.init_resource::<Messages<CharacterIntent>>();
		world
			.run_system_once(drive_respawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let pending = world.resource::<WorldPlayerRespawnState>().pending.as_ref().unwrap();
		assert_eq!(pending.highlighted, Some(PoiId(2)));
		assert_eq!(pending.highlighted_at, Some(Vec2::new(40.0, 5.0)));
		assert_eq!(
			world.resource::<WorldMapView>().focus,
			Vec2::new(3.0, 5.0),
			"the view stays on the death point; flicks do not recenter"
		);
		Ok(())
	}

	#[test]
	fn map_flicks_treat_stick_right_as_screen_right() {
		assert_eq!(map_flick_dir(Vec2::X), Vec2::NEG_X);
		assert_eq!(map_flick_dir(Vec2::Y), Vec2::Y);
	}

	struct PicksFirst;
	struct NoFirstPick;

	impl GenerationMode for PicksFirst {}
	impl GenerationMode for NoFirstPick {}

	fn first_spawn_world(pick: bool, spawn: Option<Vec2>) -> World {
		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnState>();
		world.init_resource::<WorldMapView>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(PlayerSpawnXz(spawn));
		let mut policies = ModePlayerPolicies::default();
		if pick {
			policies.register(
				std::any::TypeId::of::<PicksFirst>(),
				world_player::ModePlayerPolicy {
					home: Vec2::ZERO,
					keep_waypoints: true,
					respawn_ends_life: false,
					pick_first_spawn: true,
				},
			);
			world.insert_resource(State::new(ActiveGenerationMode::of::<PicksFirst>()));
		} else {
			policies.register(
				std::any::TypeId::of::<NoFirstPick>(),
				world_player::ModePlayerPolicy {
					home: Vec2::ZERO,
					keep_waypoints: false,
					respawn_ends_life: true,
					pick_first_spawn: false,
				},
			);
			world.insert_resource(State::new(ActiveGenerationMode::of::<NoFirstPick>()));
		}
		world.insert_resource(policies);
		world
	}

	#[test]
	fn first_spawn_picker_retires_the_startup_body() -> anyhow::Result<()> {
		let mut world = first_spawn_world(true, None);
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(12.0, 4.0, -8.0))).id();
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.entities().contains(player));
		let state = world.resource::<WorldPlayerRespawnState>();
		let pending = state.pending.as_ref().expect("first life pending");
		assert!(pending.first_life);
		assert_eq!(pending.death_at.xz(), Vec2::new(12.0, -8.0));
		assert!(state.first_spawn_offered);
		Ok(())
	}

	#[test]
	fn first_spawn_picker_skips_an_explicit_start_at() -> anyhow::Result<()> {
		let mut world = first_spawn_world(true, Some(Vec2::new(-1500.0, -600.0)));
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(12.0, 4.0, -8.0))).id();
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.entities().contains(player));
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(world.resource::<WorldPlayerRespawnState>().first_spawn_offered);
		Ok(())
	}

	#[test]
	fn first_spawn_picker_latches_a_saved_trail_before_first_wave() -> anyhow::Result<()> {
		let mut world = first_spawn_world(true, None);
		world.insert_resource(FirstWave { undiscovered: 1, remaining: 40, passed: false });
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(durham::BaseTerrainNoise::from_config(
			&durham::TerrainConfig::new(42),
		)));
		world.insert_resource(PlayerPositionWaypoints {
			positions: vec![Vec3::new(2178.3, 36.6, 1383.6)],
			just_loaded: false,
			resume_pending: false,
			..default()
		});
		let player =
			world.spawn((VegetationPlayer, Transform::from_xyz(2178.3, 36.6, 1383.6))).id();
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.entities().contains(player));
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(world.resource::<WorldPlayerRespawnState>().first_spawn_offered);
		Ok(())
	}

	#[test]
	fn first_spawn_picker_skips_a_character_with_a_saved_trail() -> anyhow::Result<()> {
		let mut world = first_spawn_world(true, None);
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(durham::BaseTerrainNoise::from_config(
			&durham::TerrainConfig::new(42),
		)));
		world.insert_resource(PlayerPositionWaypoints {
			positions: vec![Vec3::new(2178.3, 36.6, 1383.6)],
			resume_pending: true,
			..default()
		});
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(-49.7, 78.3, 50.0))).id();
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.entities().contains(player));
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(world.resource::<WorldPlayerRespawnState>().first_spawn_offered);
		Ok(())
	}

	#[test]
	fn first_spawn_picker_skips_a_mode_that_does_not_pick() -> anyhow::Result<()> {
		let mut world = first_spawn_world(false, None);
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(12.0, 4.0, -8.0))).id();
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.entities().contains(player));
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		assert!(!world.resource::<WorldPlayerRespawnState>().first_spawn_offered);
		Ok(())
	}

	#[test]
	fn first_spawn_picker_waits_for_near_hcsg() -> anyhow::Result<()> {
		let mut world = first_spawn_world(true, None);
		let storage = HcsgStorage::default();
		let demand = HcsgDemand::default();
		world.insert_resource(demand.clone());
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(12.0, 4.0, -8.0))).id();
		let region = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(8.0, 1.0, 1.0));
		demand.subscribe::<UnitXTile>(None, vec![region], None, crate::HcsgClass::Near);

		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(
			world.entities().contains(player),
			"picker must not open while near HCSG work remains"
		);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());

		let _worker = spawn_worker(storage, demand.clone())?;
		anyhow::ensure!(demand.wait_idle(std::time::Duration::from_secs(10)));
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.entities().contains(player), "picker opens once near work drains");
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		Ok(())
	}

	#[test]
	fn first_spawn_picker_opens_once_first_wave_has_passed() -> anyhow::Result<()> {
		let mut world = first_spawn_world(true, None);
		let demand = HcsgDemand::default();
		world.insert_resource(demand.clone());
		let region = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(8.0, 1.0, 1.0));
		demand.subscribe::<UnitXTile>(None, vec![region], None, crate::HcsgClass::Near);
		world.insert_resource(FirstWave { undiscovered: 1, remaining: 40, passed: true });
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(12.0, 4.0, -8.0))).id();
		world
			.run_system_once(queue_first_spawn_picker)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.entities().contains(player), "unveil timeout must not pin the picker");
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		Ok(())
	}

	#[test]
	fn first_life_opens_the_map_immediately() -> anyhow::Result<()> {
		let mut world = respawn_world(4.0, true, true);
		{
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let pending = state.pending.as_mut().unwrap();
			pending.origin = RespawnOrigin::began(Some(std::any::TypeId::of::<EndsLife>()), false);
			pending.first_life = true;
			pending.timer = Timer::from_seconds(0.0, TimerMode::Once);
		}
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		Ok(())
	}

	#[test]
	fn a_live_player_does_not_cancel_a_first_life_pending() -> anyhow::Result<()> {
		let mut world = respawn_world(0.0, true, true);
		{
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let pending = state.pending.as_mut().unwrap();
			pending.origin = RespawnOrigin::began(Some(std::any::TypeId::of::<EndsLife>()), false);
			pending.first_life = true;
		}
		world.spawn(VegetationPlayer);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		Ok(())
	}

	#[test]
	fn first_life_does_not_paint_death_glaze() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldPlayerRespawnState {
			pending: Some(PendingPlayerRespawn {
				timer: Timer::from_seconds(0.0, TimerMode::Once),
				death_at: Vec3::ZERO,
				origin: RespawnOrigin::began(None, false),
				candidates: Vec::new(),
				highlighted: None,
				highlighted_at: None,
				map_opened: true,
				stick_resting: true,
				first_life: true,
				fallback_at: Vec3::ZERO,
				registry_revision: 0,
				picker_nearby_scratch: Vec::new(),
			}),
			..default()
		});
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		world.spawn((
			PlayerDeathGlaze,
			BackgroundColor(death_glaze_color(0.5)),
			Visibility::Visible,
		));
		world
			.run_system_once(sync_player_death_glaze)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let (color, visibility) =
			world.query::<(&BackgroundColor, &Visibility)>().single(&world)?;
		assert_eq!(color.0, death_glaze_color(0.0));
		assert_eq!(*visibility, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn death_glaze_hides_when_gameplay_is_off() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldPlayerRespawnState {
			pending: Some(PendingPlayerRespawn {
				timer: Timer::from_seconds(0.0, TimerMode::Once),
				death_at: Vec3::ZERO,
				origin: RespawnOrigin::began(None, false),
				candidates: Vec::new(),
				highlighted: None,
				highlighted_at: None,
				map_opened: true,
				stick_resting: true,
				first_life: false,
				fallback_at: Vec3::ZERO,
				registry_revision: 0,
				picker_nearby_scratch: Vec::new(),
			}),
			..default()
		});
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		world.spawn((
			PlayerDeathGlaze,
			BackgroundColor(death_glaze_color(0.5)),
			Visibility::Visible,
		));
		world
			.run_system_once(sync_player_death_glaze)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_some());
		assert!(world.resource::<WorldMapView>().open);
		assert!(world.resource::<WorldMapView>().close_locked);
		let (color, visibility) =
			world.query::<(&BackgroundColor, &Visibility)>().single(&world)?;
		assert_eq!(color.0, death_glaze_color(0.0));
		assert_eq!(*visibility, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn reset_first_spawn_offer_clears_the_session_flag() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldPlayerRespawnState {
			first_spawn_offered: true,
			last_poi: Some(PoiId(9)),
			pending: Some(PendingPlayerRespawn {
				timer: Timer::from_seconds(0.0, TimerMode::Once),
				death_at: Vec3::ZERO,
				origin: RespawnOrigin::began(None, false),
				candidates: Vec::new(),
				highlighted: None,
				highlighted_at: None,
				map_opened: false,
				stick_resting: true,
				first_life: true,
				fallback_at: Vec3::ZERO,
				registry_revision: 0,
				picker_nearby_scratch: Vec::new(),
			}),
			..default()
		});
		world
			.run_system_once(reset_first_spawn_offer)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let state = world.resource::<WorldPlayerRespawnState>();
		assert!(!state.first_spawn_offered);
		assert!(state.last_poi.is_none());
		assert!(state.pending.is_none());
		Ok(())
	}

	#[test]
	fn empty_registry_confirm_uses_the_fallback_ring() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let fallback_at = {
			let pending = world
				.resource::<WorldPlayerRespawnState>()
				.pending
				.as_ref()
				.ok_or_else(|| anyhow::anyhow!("pending"))?;
			assert!(pending.map_opened);
			assert!(pending.candidates.is_empty());
			assert!(pending.highlighted.is_none());
			assert_ne!(pending.fallback_at.xz(), pending.death_at.xz());
			pending.fallback_at
		};
		world.write_message(PlayerChoseRespawnPoi { poi: None });
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let mut bodies = world.query_filtered::<&Transform, With<VegetationPlayer>>();
		let body = bodies.single(&world)?;
		assert_eq!(body.translation.xz(), fallback_at.xz());
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		Ok(())
	}

	#[test]
	fn picker_refreshes_when_pois_arrive() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world
			.resource::<WorldPlayerRespawnState>()
			.pending
			.as_ref()
			.is_some_and(|pending| pending.candidates.is_empty()));

		let poi = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			poi,
			poi_intelligence::Poi::new(PoiId(7), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(80.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let pending = world
			.resource::<WorldPlayerRespawnState>()
			.pending
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("pending"))?;
		assert_eq!(pending.candidates, vec![PoiId(7)]);
		assert_eq!(pending.highlighted, Some(PoiId(7)));
		Ok(())
	}

	#[test]
	fn registry_refresh_does_not_rearm_a_held_stick() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		{
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let pending = state.pending.as_mut().ok_or_else(|| anyhow::anyhow!("pending"))?;
			pending.stick_resting = false;
		}
		let poi = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			poi,
			poi_intelligence::Poi::new(PoiId(7), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(80.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let pending = world
			.resource::<WorldPlayerRespawnState>()
			.pending
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("pending"))?;
		assert_eq!(pending.candidates, vec![PoiId(7)]);
		assert!(!pending.stick_resting, "a held stick must come home before the next flick");
		Ok(())
	}

	#[test]
	fn picker_drops_a_removed_highlight() -> anyhow::Result<()> {
		let mut world = discovery_respawn_world(0.2);
		let keep = world.spawn_empty().id();
		let gone = world.spawn_empty().id();
		world.resource_mut::<PoiRegistry>().upsert(
			keep,
			poi_intelligence::Poi::new(PoiId(1), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(80.0, 4.0, 5.0),
			true,
			false,
		)?;
		world.resource_mut::<PoiRegistry>().upsert(
			gone,
			poi_intelligence::Poi::new(PoiId(2), URBAN_POI).with_arrival_radius(8.0),
			Vec3::new(40.0, 4.0, 5.0),
			true,
			false,
		)?;
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		{
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let pending = state.pending.as_mut().unwrap();
			pending.highlighted = Some(PoiId(2));
			pending.highlighted_at = Some(Vec2::new(40.0, 5.0));
		}
		world.resource_mut::<PoiRegistry>().remove_entity(gone);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let pending = world
			.resource::<WorldPlayerRespawnState>()
			.pending
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("pending"))?;
		assert_eq!(pending.candidates, vec![PoiId(1)]);
		assert_eq!(pending.highlighted, Some(PoiId(1)));
		Ok(())
	}

	#[test]
	fn a_stick_flick_is_measured_from_the_current_poi_not_the_player() {
		let current = PoiId(1);
		let east_of_current = PoiId(2);
		let east_of_player = PoiId(3);
		let picked = nearest_in_direction(
			Vec2::ZERO,
			Some(current),
			Vec2::X,
			&[
				(current, Vec2::ZERO),
				(east_of_current, Vec2::new(15.0, 0.0)),
				(east_of_player, Vec2::new(-40.0, 0.0)),
			],
		);
		assert_eq!(picked, Some(east_of_current));
	}
}
