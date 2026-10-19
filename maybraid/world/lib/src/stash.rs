//! Claimable world stashes for NPC death loot, player drops, and chests.
//!
//! One host type ([`WorldStash`]) holds a bag via [`InventoryUser`]. Claim is
//! take-all on [`CharacterIntent::StartInteraction`] (pad **X**) inside
//! [`StashPolicy::claim_radius`]. Death samples a mob-kind fraction of the
//! bag ([`Inventory::take_fraction`]) then [`Inventory::explode`]s the kept
//! pieces so each is claimable on its own. Raiders and Guards leave about one
//! third, Brawlers about one twelfth, and other families drop nothing.
//! Unmarked NPCs still drop the full bag. Player drops still explode
//! everything. A closed loot crate swings its lid open on X and ejects one
//! ephemeral stash ([`crate::crate_loot`]); the crate restocks on its own clock.
//!
//! Loot TTL is [`StashPolicy::loot_secs`] (default 60 s), independent of
//! mob corpse lifetime (4 s). Persistent chests omit [`DespawnAfter`].
//! Absorb never auto-equips. Claim and drop refresh [`WorldPlayerLoadout`]
//! so the live bag can persist.
//!
//! In-range claim shows three concentric ground rings (yellow 1.5 m, green
//! 0.75 m, blue 0.25 m) and a Kenney outline interact chip on the player–item
//! line (Xbox **X** when a gamepad is connected, keyboard **E** otherwise),
//! plus a screen-space `Pick up <name>` caption. A closed loot crate uses the
//! same rings and an `Open` caption when no stash is already in range.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use bevy::text::FontSize;
use character_inventory_user::{spawn_bag, InventoryUser};
use character_items::{
	ClothingHost, Inventory, InventoryItem, InventorySlot, ItemRng, LootFraction, MaterialRefParams,
};
use characters::{
	add_character_components_host, character_bounds, CharacterComponents, ClothingLayer,
	ComponentsOnly, Layers, PartNode,
};
use damage::{DamageSystems, DespawnAfter, Downed};
use firearm_user::{held_scale_from_bounds, FirearmUser, FirearmUserSettings, GeneratedFirearm};
use firearms::{firearm_bounds, spawn_firearm_components};
use grenade_user::{GrenadeThrow, GrenadeUser, GrenadeUserSystems};
use grenades::{DropLiveGrenade, GrenadeEffect};
use lod::lod_ref::LodRef;
use lod::scene::LodSceneLevel;
use lod::LodScene;
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use player::PlayerUse;
use world_player::Player as VegetationPlayer;

use mob_characters::CharacterBrains;
use mob_scenes::MobKind;

use crate::control::WorldGameplayEnabled;
use crate::weapon::{AppliedWorldPlayerLoadout, WorldPlayerLoadout};

/// Default unclaimed-loot lifetime. Independent of the 4 s corpse clock.
pub const DEFAULT_LOOT_SECS: f32 = 60.0;

/// Default interact radius for take-all claim (XZ).
pub const DEFAULT_CLAIM_RADIUS: f32 = 4.0;

/// Ground pile, player drop, or authored chest. The bag is a related entity.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldStash;

/// Per-stash claim / lifetime knobs.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StashPolicy {
	pub claim_radius: f32,
	/// When true, claim empties the bag but keeps the host (chests).
	pub persist: bool,
	/// Ephemeral TTL. Ignored when [`Self::persist`] is set.
	pub loot_secs: f32,
}

impl Default for StashPolicy {
	fn default() -> Self {
		Self::ephemeral(DEFAULT_LOOT_SECS)
	}
}

impl StashPolicy {
	pub fn ephemeral(loot_secs: f32) -> Self {
		Self { claim_radius: DEFAULT_CLAIM_RADIUS, persist: false, loot_secs }
	}

	pub fn chest() -> Self {
		Self { claim_radius: DEFAULT_CLAIM_RADIUS, persist: true, loot_secs: 0.0 }
	}
}

/// World defaults for death piles and player drops.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct WorldStashSettings {
	pub loot_secs: f32,
	pub claim_radius: f32,
}

impl Default for WorldStashSettings {
	fn default() -> Self {
		Self { loot_secs: DEFAULT_LOOT_SECS, claim_radius: DEFAULT_CLAIM_RADIUS }
	}
}

impl WorldStashSettings {
	pub fn ephemeral_policy(&self) -> StashPolicy {
		StashPolicy { claim_radius: self.claim_radius, persist: false, loot_secs: self.loot_secs }
	}

	pub fn chest_policy(&self) -> StashPolicy {
		StashPolicy { claim_radius: self.claim_radius, persist: true, loot_secs: 0.0 }
	}
}

/// Marker that worn clothing / queued weapons have been spawned on the pile.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StashVisual;

/// Child spawned for a toggled clothing piece or queued firearm.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StashDisplayedItem {
	pub slot: InventorySlot,
}

/// Local offset from the stash host to the mesh center the halo should ring.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct StashHaloAnchor(pub Vec3);

/// World-space interact chip above the nearest claimable stash.
#[derive(Component)]
struct StashInteractPrompt;

/// Whether the chip shows [`INTERACT_PAD_ICON`] or [`INTERACT_KEY_ICON`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct StashInteractPromptIcon {
	pad: bool,
}

/// Screen-space action next to the chip (`Pick up <name>`).
#[derive(Component)]
struct StashInteractCaption;

/// Kenney outline Xbox X used for pad interact (`PadButton::X`).
pub const INTERACT_PAD_ICON: &str = "iconography/kenney/input-prompts/xbox_button_x_outline.png";

/// Kenney outline keyboard E when no gamepad is connected.
pub const INTERACT_KEY_ICON: &str = "iconography/kenney/input-prompts/keyboard_e_outline.png";

fn stash_interact_icon(pad_connected: bool) -> &'static str {
	if pad_connected {
		INTERACT_PAD_ICON
	} else {
		INTERACT_KEY_ICON
	}
}

/// Ground ring around the nearest claimable stash.
#[derive(Component)]
struct StashClaimHalo;

/// Was 0.18 m on the single-ring halo; concentric rings stay a bit thinner.
const HALO_RING_WIDTH: f32 = 0.12;
const HALO_YELLOW_RADIUS: f32 = 1.5;
const HALO_GREEN_RADIUS: f32 = 0.75;
const HALO_BLUE_RADIUS: f32 = 0.25;
const PROMPT_SIZE: f32 = 0.42;
const PROMPT_ABOVE: f32 = 0.62;
const PROMPT_TOWARD_PLAYER: f32 = 0.85;
const PROMPT_CAPTION_GAP_PX: f32 = 22.0;
const PROMPT_YELLOW: Color = Color::srgba(0.98, 0.86, 0.32, 1.0);
const PROMPT_EMISSIVE: LinearRgba = LinearRgba::new(1.4, 1.05, 0.28, 1.0);
const HALO_YELLOW: Color = Color::srgba(0.98, 0.86, 0.32, 0.88);
const HALO_YELLOW_EMISSIVE: LinearRgba = LinearRgba::new(1.4, 1.05, 0.28, 1.0);
const HALO_GREEN: Color = Color::srgba(0.22, 0.82, 0.38, 0.88);
const HALO_GREEN_EMISSIVE: LinearRgba = LinearRgba::new(0.22, 1.15, 0.38, 1.0);
const HALO_BLUE: Color = Color::srgba(0.25, 0.48, 0.98, 0.88);
const HALO_BLUE_EMISSIVE: LinearRgba = LinearRgba::new(0.28, 0.55, 1.35, 1.0);

/// Bind-pose garment so stash clothing keeps recipe + palette.
#[derive(Clone, PartialEq)]
struct StashClothingPreview {
	layer: ClothingLayer,
}

impl Default for StashClothingPreview {
	fn default() -> Self {
		Self {
			layer: ClothingLayer::new(
				character_items::ClothingMesh::TankTop,
				character_items::ItemColor::Natural,
				ClothingHost::HUMANOID,
			),
		}
	}
}

impl CharacterComponents for StashClothingPreview {
	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("clothing", vec![self.layer.preview_part_node()])
	}
}

pub struct WorldStashPlugin;

impl Plugin for WorldStashPlugin {
	fn build(&self, app: &mut App) {
		add_character_components_host::<StashClothingPreview>(app);
		app.init_resource::<WorldStashSettings>()
			.init_resource::<crate::crate_loot::CrateLoot>()
			.add_systems(
				Update,
				(
					(
						crate::crate_loot::stamp_closed_lids,
						claim_nearby_stashes,
						crate::crate_loot::tick_lid_swings,
						crate::crate_loot::sync_crate_lid_poses,
					)
						.chain(),
					drop_player_inventory,
					sync_stash_interact_prompt,
					sync_stash_claim_halo,
				)
					.after(CharacterControlSystems)
					.run_if(resource_equals(WorldGameplayEnabled(true))),
			);
		app.add_systems(
			PostUpdate,
			detach_downed_npc_loot
				.after(DamageSystems::Down)
				.after(GrenadeUserSystems::Interrupt),
		);
	}
}

/// Scatter radius for the first exploded item; grows slowly with count.
const EXPLODE_BASE_RADIUS: f32 = 0.55;

/// Spawn a stash at `transform` when `inventory` is non-empty.
///
/// The bag is parented so [`DespawnAfter`] on the host also removes loot.
/// Toggled clothing and queued weapons get [`StashDisplayedItem`] children.
pub fn spawn_world_stash(
	commands: &mut Commands,
	transform: Transform,
	inventory: Inventory,
	policy: StashPolicy,
	assets: Option<&AssetServer>,
) -> Option<Entity> {
	if inventory.items.is_empty() {
		return None;
	}
	let host = commands
		.spawn((
			Name::new(if policy.persist { "world-chest" } else { "world-stash" }),
			WorldStash,
			policy,
			StashVisual,
			transform,
			Visibility::default(),
		))
		.id();
	let bag = spawn_bag(commands, host, inventory.clone());
	commands.entity(bag).insert(ChildOf(host));
	if !policy.persist {
		commands.entity(host).insert(DespawnAfter::seconds(policy.loot_secs));
	}
	attach_stash_visuals(commands, host, &inventory, assets);
	Some(host)
}

/// One [`WorldStash`] per item, ring-scattered around `origin`.
///
/// Chests should keep calling [`spawn_world_stash`] so the pile stays take-all.
pub fn spawn_exploded_stashes(
	commands: &mut Commands,
	origin: Vec3,
	inventory: Inventory,
	policy: StashPolicy,
	assets: Option<&AssetServer>,
) -> Vec<Entity> {
	let pieces = inventory.explode();
	let count = pieces.len();
	pieces
		.into_iter()
		.enumerate()
		.filter_map(|(index, bag)| {
			let transform = Transform::from_translation(origin + explode_offset(index, count));
			spawn_world_stash(commands, transform, bag, policy, assets)
		})
		.collect()
}

fn explode_offset(index: usize, count: usize) -> Vec3 {
	if count <= 1 {
		return Vec3::new(0.0, 0.05, 0.0);
	}
	let angle = index as f32 * std::f32::consts::TAU / count as f32;
	let radius = EXPLODE_BASE_RADIUS + (count as f32).sqrt() * 0.12;
	Vec3::new(angle.cos() * radius, 0.05, angle.sin() * radius)
}

fn apply_claimed_loadout(
	commands: &mut Commands,
	player: Entity,
	inventory: &Inventory,
	loadout: &mut WorldPlayerLoadout,
) {
	loadout.retarget_inventory(inventory.clone());
	commands.entity(player).insert(AppliedWorldPlayerLoadout(loadout.clone()));
}

fn apply_dropped_loadout(loadout: &mut WorldPlayerLoadout, inventory: &Inventory) {
	loadout.retarget_inventory(inventory.clone());
}

fn attach_stash_visuals(
	commands: &mut Commands,
	host: Entity,
	inventory: &Inventory,
	assets: Option<&AssetServer>,
) {
	let displayed = inventory.clothing.len()
		+ inventory.weapons.len()
		+ inventory.skills.len()
		+ inventory.tanzes.len();
	let mut pile = 0usize;
	for &index in &inventory.clothing {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		spawn_displayed_item(
			commands,
			host,
			item,
			StashDisplayedItem { slot: InventorySlot::Clothing },
			display_offset(pile, displayed),
			assets,
		);
		pile += 1;
	}
	for &index in &inventory.weapons {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		spawn_displayed_item(
			commands,
			host,
			item,
			StashDisplayedItem { slot: InventorySlot::Weapons },
			display_offset(pile, displayed),
			assets,
		);
		pile += 1;
	}
	for &index in &inventory.skills {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		spawn_displayed_item(
			commands,
			host,
			item,
			StashDisplayedItem { slot: InventorySlot::Skills },
			display_offset(pile, displayed),
			assets,
		);
		pile += 1;
	}
	for &index in &inventory.tanzes {
		let Some(item) = inventory.items.get(index) else {
			continue;
		};
		spawn_displayed_item(
			commands,
			host,
			item,
			StashDisplayedItem { slot: InventorySlot::Tanzes },
			display_offset(pile, displayed),
			assets,
		);
		pile += 1;
	}
}

/// One item sits on the host; several fan out so they do not stack.
fn display_offset(index: usize, count: usize) -> Transform {
	if count <= 1 {
		return Transform::from_xyz(0.0, 0.08, 0.0).with_rotation(Quat::from_rotation_x(-0.35));
	}
	pile_offset(index)
}

fn pile_offset(index: usize) -> Transform {
	let angle = index as f32 * 0.7;
	Transform::from_xyz(angle.cos() * 0.28, 0.08, angle.sin() * 0.28)
		.with_rotation(Quat::from_rotation_y(angle) * Quat::from_rotation_x(-0.35))
}

fn visual_halo_local(item: &InventoryItem, slot: InventorySlot, transform: Transform) -> Vec3 {
	match slot {
		InventorySlot::Clothing | InventorySlot::Skills | InventorySlot::Tanzes => {
			transform.translation
		}
		InventorySlot::Weapons => item.firearm_spec().map_or(transform.translation, |spec| {
			let kit = GeneratedFirearm::from_spec(spec);
			let bounds = firearm_bounds(&kit);
			let center = (bounds.min + bounds.max) * 0.5;
			transform.transform_point(Vec3::from(center))
		}),
	}
}

fn clothing_material_ref(material: MaterialRefParams) -> MaterialRef {
	MaterialRef::named(material.id.recipe_id()).with_palette([material.color.color()])
}

fn spawn_displayed_item(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	match displayed.slot {
		InventorySlot::Clothing => {
			spawn_displayed_clothing(commands, host, item, displayed, transform, assets);
		}
		InventorySlot::Weapons => {
			spawn_displayed_weapon(commands, host, item, displayed, transform, assets);
		}
		InventorySlot::Skills | InventorySlot::Tanzes => {
			spawn_displayed_skill_map(commands, host, item, displayed, transform);
		}
	}
}

fn spawn_displayed_skill_map(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
) {
	commands.spawn((
		Name::new(format!("stash-{}", item.label())),
		displayed,
		StashHaloAnchor(visual_halo_local(item, displayed.slot, transform)),
		transform,
		Visibility::default(),
		ChildOf(host),
	));
}

fn spawn_displayed_clothing(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	let material = item.material().map(clothing_material_ref);
	if let (Some(mesh), Some(params), Some(_)) = (item.mesh(), item.material(), assets) {
		let preview = StashClothingPreview {
			layer: ClothingLayer::new(mesh, params.color, ClothingHost::HUMANOID)
				.with_material(params.id),
		};
		let bounds = character_bounds(&preview);
		let identity = Transform::IDENTITY;
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &identity,
			current_transform: &identity,
			bounds: &bounds,
		};
		let entity = commands
			.spawn_scene((
				ComponentsOnly(preview).host(&lod_ref),
				bsn! {
					template_value(transform)
				},
			))
			.id();
		commands.entity(entity).insert((
			Name::new(format!("stash-{}", item.label())),
			displayed,
			StashHaloAnchor(visual_halo_local(item, displayed.slot, transform)),
			ChildOf(host),
		));
		return;
	}
	let mut visual = commands.spawn((
		Name::new(format!("stash-{}", item.label())),
		displayed,
		StashHaloAnchor(visual_halo_local(item, displayed.slot, transform)),
		transform,
		Visibility::default(),
		ChildOf(host),
	));
	if let Some(material) = material {
		visual.insert((MaterialRefRoot(material), PropagateToDescendants));
	}
}

fn spawn_displayed_weapon(
	commands: &mut Commands,
	host: Entity,
	item: &InventoryItem,
	displayed: StashDisplayedItem,
	transform: Transform,
	assets: Option<&AssetServer>,
) {
	let transform = match item.firearm_spec() {
		Some(spec) => {
			let kit = GeneratedFirearm::from_spec(spec);
			let bounds = firearm_bounds(&kit);
			let scale = held_scale_from_bounds(bounds, FirearmUserSettings::default().held_length);
			let transform = transform.with_scale(Vec3::splat(scale));
			if assets.is_some() {
				for entity in spawn_firearm_components(commands, &kit, transform, bounds) {
					commands.entity(entity).insert((
						Name::new(format!("stash-{}", item.label())),
						displayed,
						StashHaloAnchor(visual_halo_local(item, displayed.slot, transform)),
						ChildOf(host),
					));
				}
				return;
			}
			transform
		}
		None => transform,
	};
	commands.spawn((
		Name::new(format!("stash-{}", item.label())),
		displayed,
		StashHaloAnchor(visual_halo_local(item, displayed.slot, transform)),
		transform,
		Visibility::default(),
		ChildOf(host),
	));
}

fn despawn_displayed_items(commands: &mut Commands, displayed: &[Entity]) {
	for entity in displayed {
		commands.entity(*entity).try_despawn();
	}
}

type DownedNpcLoot<'a> = (
	Entity,
	&'a Downed,
	Option<&'a InventoryUser>,
	Option<&'a FirearmUser>,
	Option<&'a GrenadeUser>,
	Option<&'a MobKind>,
	Option<&'a CharacterBrains>,
);

fn npc_loot_fraction(kind: Option<&MobKind>, brains: Option<&CharacterBrains>) -> LootFraction {
	kind.map(|kind| kind.loot_fraction())
		.or_else(|| brains.map(|brains| brains.loot_fraction()))
		.unwrap_or(LootFraction::ALL)
}

fn npc_loot_seed(entity: Entity, downed: &Downed) -> u64 {
	entity.to_bits()
		^ u64::from(downed.point.x.to_bits())
		^ (u64::from(downed.point.z.to_bits()) << 32)
}

fn detach_downed_npc_loot(
	settings: Res<WorldStashSettings>,
	mut commands: Commands,
	assets: Option<Res<AssetServer>>,
	downed: Query<DownedNpcLoot<'_>, (Added<Downed>, Without<VegetationPlayer>)>,
	mut bags: Query<&mut Inventory>,
) {
	let assets = assets.as_deref();
	for (body, downed, user, firearm, grenade, kind, brains) in &downed {
		let fraction = npc_loot_fraction(kind, brains);
		let loot = user.and_then(|user| bags.get_mut(user.bag).ok()).map_or_else(
			Inventory::default,
			|mut bag| {
				bag.take_fraction(&mut ItemRng::from_seed(npc_loot_seed(body, downed)), fraction)
			},
		);
		if let Some(user) = user {
			commands.entity(user.bag).try_despawn();
			commands.entity(body).remove::<InventoryUser>();
		}
		if let Some(firearm) = firearm {
			commands.entity(firearm.held).try_despawn();
			commands.entity(body).remove::<(FirearmUser, PlayerUse)>();
		}
		if let Some(grenade) = grenade {
			commands.entity(grenade.held).try_despawn();
			commands
				.entity(body)
				.remove::<(GrenadeUser, grenade_user::GrenadeThrow, PlayerUse)>();
		}
		let mut policy = settings.ephemeral_policy();
		policy.loot_secs = settings.loot_secs;
		spawn_exploded_stashes(&mut commands, downed.point, loot, policy, assets);
	}
}

fn player_origin(transform: &Transform) -> Vec3 {
	transform.translation
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	a.xz().distance(b.xz())
}

pub(crate) fn claim_nearby_stashes(
	mut intents: MessageReader<CharacterIntent>,
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	time: Option<Res<Time>>,
	mut crates: Option<ResMut<crate::crate_loot::CrateLoot>>,
	players: Query<(Entity, &Transform, &InventoryUser), With<VegetationPlayer>>,
	stashes: Query<(Entity, &Transform, &InventoryUser, &StashPolicy), With<WorldStash>>,
	displayed: Query<(Entity, &ChildOf), With<StashDisplayedItem>>,
	mut bags: Query<&mut Inventory>,
	lids: Query<
		(Entity, &furniture_assemblies::FurnitureKitPart, &GlobalTransform),
		(With<crate::crate_loot::ClosedLid>, Without<crate::crate_loot::LidSwing>),
	>,
	parts: Query<(Entity, &furniture_assemblies::FurnitureKitPart, &GlobalTransform)>,
	child_of: Query<&ChildOf>,
	hosts: Query<&lod::hcsg::HcsgNode<crate::WorldFurnished>>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::StartInteraction)) {
		return;
	}
	for (player, player_transform, player_user) in &players {
		let origin = player_origin(player_transform);
		let Some((stash, stash_bag, policy, _)) = nearest_stash_in_radius(origin, stashes.iter())
		else {
			if let (Some(time), Some(crates)) = (time.as_deref(), crates.as_deref_mut()) {
				crate::crate_loot::open_nearest_crate(
					origin,
					time.elapsed_secs(),
					&mut commands,
					crates,
					&lids,
					&parts,
					&child_of,
					&hosts,
				);
			}
			continue;
		};
		let Ok(mut source) = bags.get_mut(stash_bag) else {
			continue;
		};
		let loot = source.take_all();
		let Ok(mut player_bag) = bags.get_mut(player_user.bag) else {
			continue;
		};
		player_bag.absorb(loot);
		let snapshot = player_bag.clone();
		let visual: Vec<Entity> = displayed
			.iter()
			.filter(|(_, child)| child.parent() == stash)
			.map(|(entity, _)| entity)
			.collect();
		despawn_displayed_items(&mut commands, &visual);
		if let Some(loadout) = loadout.as_deref_mut() {
			apply_claimed_loadout(&mut commands, player, &snapshot, loadout);
		}
		if policy.persist {
			continue;
		}
		commands.entity(stash_bag).try_despawn();
		commands.entity(stash).try_despawn();
	}
}

fn nearest_stash_in_radius<'a>(
	origin: Vec3,
	stashes: impl IntoIterator<Item = (Entity, &'a Transform, &'a InventoryUser, &'a StashPolicy)>,
) -> Option<(Entity, Entity, StashPolicy, Vec3)> {
	stashes
		.into_iter()
		.filter_map(|(entity, transform, user, policy)| {
			let translation = transform.translation;
			let distance = xz_distance(translation, origin);
			(distance <= policy.claim_radius).then_some((
				distance,
				entity,
				user.bag,
				*policy,
				translation,
			))
		})
		.min_by(|a, b| a.0.total_cmp(&b.0))
		.map(|(_, entity, bag, policy, translation)| (entity, bag, policy, translation))
}

type DroppingPlayer<'a> = (
	Entity,
	&'a Transform,
	&'a InventoryUser,
	Option<&'a FirearmUser>,
	Option<&'a GrenadeUser>,
	Option<&'a GrenadeThrow>,
);

fn drop_player_inventory(
	settings: Res<WorldStashSettings>,
	mut intents: MessageReader<CharacterIntent>,
	mut commands: Commands,
	mut drops: MessageWriter<DropLiveGrenade>,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	assets: Option<Res<AssetServer>>,
	players: Query<DroppingPlayer<'_>, With<VegetationPlayer>>,
	mut bags: Query<&mut Inventory>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::Inventory)) {
		return;
	}
	let assets = assets.as_deref();
	for (player, transform, user, firearm, grenade, throw) in &players {
		let Ok(mut bag) = bags.get_mut(user.bag) else {
			continue;
		};
		if let Some((fuse, spec, stats)) = throw.and_then(|throw| {
			let fuse = throw.live_fuse()?;
			let spec = bag.primary_weapon().and_then(InventoryItem::grenade_spec)?;
			let stats = bag.primary_weapon().and_then(InventoryItem::grenade_stats)?;
			Some((fuse, spec, stats))
		}) {
			drops.write(DropLiveGrenade {
				origin: transform.translation + Vec3::Y * 1.0,
				velocity: Vec3::ZERO,
				spec,
				stats,
				effect: GrenadeEffect::from_stats(&stats),
				fuse,
				source: player,
			});
		}
		let loot = bag.take_all();
		let snapshot = bag.clone();
		if let Some(firearm) = firearm {
			commands.entity(firearm.held).try_despawn();
			commands.entity(player).remove::<(FirearmUser, PlayerUse)>();
		}
		if let Some(grenade) = grenade {
			commands.entity(grenade.held).try_despawn();
			commands
				.entity(player)
				.remove::<(GrenadeUser, grenade_user::GrenadeThrow, PlayerUse)>();
		}
		if let Some(loadout) = loadout.as_deref_mut() {
			apply_dropped_loadout(loadout, &snapshot);
		}
		spawn_exploded_stashes(
			&mut commands,
			player_origin(transform),
			loot,
			settings.ephemeral_policy(),
			assets,
		);
	}
}

fn spawn_stash_interact_prompt(
	commands: &mut Commands,
	assets: &AssetServer,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	pad_connected: bool,
) {
	let material = materials.add(StandardMaterial {
		base_color: PROMPT_YELLOW,
		base_color_texture: Some(assets.load(stash_interact_icon(pad_connected))),
		emissive: PROMPT_EMISSIVE,
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		cull_mode: None,
		..default()
	});
	commands.spawn((
		Name::new("stash-interact-prompt"),
		StashInteractPrompt,
		StashInteractPromptIcon { pad: pad_connected },
		Mesh3d(meshes.add(Rectangle::new(PROMPT_SIZE, PROMPT_SIZE))),
		MeshMaterial3d(material),
		Transform::IDENTITY,
		Visibility::Hidden,
	));
}

fn sync_stash_interact_prompt_icon(
	pad_connected: bool,
	assets: &AssetServer,
	materials: &mut Assets<StandardMaterial>,
	mut prompts: Query<
		(&mut StashInteractPromptIcon, &MeshMaterial3d<StandardMaterial>),
		With<StashInteractPrompt>,
	>,
) {
	let icon = stash_interact_icon(pad_connected);
	for (mut current, mesh_material) in &mut prompts {
		if current.pad == pad_connected {
			continue;
		}
		current.pad = pad_connected;
		if let Some(mut material) = materials.get_mut(&mesh_material.0) {
			material.base_color_texture = Some(assets.load(icon));
		}
	}
}

fn prompt_world_point(item: Vec3, player: Vec3) -> Vec3 {
	let delta = Vec3::new(player.x - item.x, 0.0, player.z - item.z);
	let dist = delta.length();
	let toward =
		if dist > 1e-4 { delta / dist * PROMPT_TOWARD_PLAYER.min(dist * 0.5) } else { Vec3::ZERO };
	item + toward + Vec3::Y * PROMPT_ABOVE
}

fn prompt_billboard(at: Vec3, camera: Option<Vec3>) -> Transform {
	let mut transform = Transform::from_translation(at);
	if let Some(camera) = camera {
		let away = at - camera;
		if away.length_squared() > 1e-6 {
			transform.look_to(away, Vec3::Y);
		}
	}
	transform
}

fn pickup_action_label(inventory: &Inventory) -> String {
	match inventory.items.as_slice() {
		[item] => format!("Pick up {}", item.name()),
		items if !items.is_empty() => format!("Pick up {} items", items.len()),
		_ => String::from("Pick up"),
	}
}

fn spawn_stash_interact_caption(commands: &mut Commands) {
	commands.spawn((
		Name::new("stash-interact-caption"),
		StashInteractCaption,
		Text::new(""),
		TextFont { font_size: FontSize::Px(16.0), ..default() },
		TextColor(PROMPT_YELLOW),
		TextShadow { offset: Vec2::new(1.0, 1.0), color: Color::srgba(0.0, 0.0, 0.0, 0.72) },
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			..default()
		},
		Visibility::Hidden,
		Pickable::IGNORE,
		ZIndex(24),
	));
}

fn project_prompt_caption(
	camera: &Camera,
	camera_transform: &GlobalTransform,
	world: Vec3,
) -> Option<Vec2> {
	let ndc = camera.world_to_ndc(camera_transform, world)?;
	if ndc.z <= 0.0 || ndc.z >= 1.0 {
		return None;
	}
	camera.world_to_viewport(camera_transform, world).ok()
}

fn sync_stash_interact_prompt(
	mut commands: Commands,
	time: Res<Time>,
	assets: Option<Res<AssetServer>>,
	mut meshes: Option<ResMut<Assets<Mesh>>>,
	mut materials: Option<ResMut<Assets<StandardMaterial>>>,
	gamepads: Query<&Gamepad>,
	players: Query<&Transform, (With<VegetationPlayer>, Without<StashInteractPrompt>)>,
	cameras: Query<(&Camera, &GlobalTransform), (With<Camera3d>, Without<StashInteractPrompt>)>,
	stashes: Query<
		(Entity, &Transform, &InventoryUser, &StashPolicy),
		(With<WorldStash>, Without<StashInteractPrompt>),
	>,
	anchors: Query<(Entity, &ChildOf, Option<&StashHaloAnchor>), With<StashDisplayedItem>>,
	crates: crate::crate_loot::ClosedCrateQuery,
	bags: Query<&Inventory>,
	mut prompt: Query<
		(&mut Transform, &mut Visibility),
		(With<StashInteractPrompt>, Without<WorldStash>, Without<VegetationPlayer>),
	>,
	prompt_icons: Query<
		(&mut StashInteractPromptIcon, &MeshMaterial3d<StandardMaterial>),
		With<StashInteractPrompt>,
	>,
	mut caption: Query<
		(&mut Text, &mut Node, &mut Visibility),
		(With<StashInteractCaption>, Without<StashInteractPrompt>),
	>,
) {
	let pad_connected = !gamepads.is_empty();
	let target = nearest_interact(players.iter(), stashes.iter(), anchors.iter(), &crates);
	if prompt.is_empty() {
		let Some(assets) = assets.as_deref() else {
			return;
		};
		let Some(meshes) = meshes.as_mut() else {
			return;
		};
		let Some(materials) = materials.as_mut() else {
			return;
		};
		spawn_stash_interact_prompt(&mut commands, assets, meshes, materials, pad_connected);
		return;
	}
	if let (Some(assets), Some(materials)) = (assets.as_deref(), materials.as_mut()) {
		sync_stash_interact_prompt_icon(pad_connected, assets, materials, prompt_icons);
	}
	if caption.is_empty() {
		spawn_stash_interact_caption(&mut commands);
	}
	let camera_at = cameras.iter().next().map(|(_, transform)| transform.translation());
	let pulse = 1.0 + 0.08 * (time.elapsed_secs() * 5.0).sin();
	let prompt_at = target.map(|claim| prompt_world_point(claim.at, claim.player));
	for (mut transform, mut visibility) in &mut prompt {
		match prompt_at {
			Some(at) => {
				*visibility = Visibility::Visible;
				*transform = prompt_billboard(at, camera_at);
				transform.scale = Vec3::splat(pulse);
			}
			None => *visibility = Visibility::Hidden,
		}
	}
	let label = target.map(|claim| match claim.bag {
		Some(bag) => {
			bags.get(bag).ok().map(pickup_action_label).unwrap_or_else(|| "Pick up".into())
		}
		None => String::from("Open"),
	});
	let camera = cameras.iter().next();
	let screen = prompt_at.and_then(|at| {
		let (camera, camera_transform) = camera?;
		project_prompt_caption(camera, camera_transform, at)
	});
	for (mut text, mut node, mut visibility) in &mut caption {
		match (target, label.as_deref()) {
			(Some(_), Some(action)) => {
				text.0 = action.to_string();
				if let Some(screen) = screen {
					node.left = Val::Px(screen.x + PROMPT_CAPTION_GAP_PX);
					node.top = Val::Px(screen.y - 10.0);
					*visibility = Visibility::Visible;
				} else if camera.is_some() {
					*visibility = Visibility::Hidden;
				} else {
					*visibility = Visibility::Visible;
				}
			}
			_ => *visibility = Visibility::Hidden,
		}
	}
}

#[derive(Clone, Copy)]
struct NearestClaim {
	at: Vec3,
	player: Vec3,
	bag: Option<Entity>,
}

fn nearest_claim<'a>(
	players: impl IntoIterator<Item = &'a Transform>,
	stashes: impl IntoIterator<Item = (Entity, &'a Transform, &'a InventoryUser, &'a StashPolicy)>,
	anchors: impl IntoIterator<Item = (Entity, &'a ChildOf, Option<&'a StashHaloAnchor>)>,
) -> Option<NearestClaim> {
	let listed: Vec<_> = stashes.into_iter().collect();
	let anchors: Vec<_> = anchors.into_iter().collect();
	players.into_iter().find_map(|transform| {
		let player = player_origin(transform);
		nearest_stash_in_radius(player, listed.iter().copied()).map(|(stash, bag, _, at)| {
			NearestClaim {
				at: halo_world_point(at, stash, anchors.iter().copied()),
				player,
				bag: Some(bag),
			}
		})
	})
}

fn nearest_interact<'a>(
	players: impl IntoIterator<Item = &'a Transform>,
	stashes: impl IntoIterator<Item = (Entity, &'a Transform, &'a InventoryUser, &'a StashPolicy)>,
	anchors: impl IntoIterator<Item = (Entity, &'a ChildOf, Option<&'a StashHaloAnchor>)>,
	crates: &crate::crate_loot::ClosedCrateQuery,
) -> Option<NearestClaim> {
	let players: Vec<_> = players.into_iter().collect();
	if let Some(claim) = nearest_claim(players.iter().copied(), stashes, anchors) {
		return Some(claim);
	}
	let transform = *players.first()?;
	let player = player_origin(transform);
	let at = crates.floor(player)?;
	Some(NearestClaim { at, player, bag: None })
}

fn halo_world_point<'a>(
	stash_at: Vec3,
	stash: Entity,
	anchors: impl IntoIterator<Item = (Entity, &'a ChildOf, Option<&'a StashHaloAnchor>)>,
) -> Vec3 {
	let offsets: Vec<Vec3> = anchors
		.into_iter()
		.filter(|(_, child, _)| child.parent() == stash)
		.map(|(_, _, anchor)| anchor.map_or(Vec3::ZERO, |anchor| anchor.0))
		.collect();
	if offsets.is_empty() {
		return stash_at + Vec3::Y * 0.08;
	}
	let sum: Vec3 = offsets.iter().copied().sum();
	let mean = sum / offsets.len() as f32;
	Vec3::new(stash_at.x + mean.x, stash_at.y + 0.08, stash_at.z + mean.z)
}

fn sync_stash_claim_halo(
	mut commands: Commands,
	time: Res<Time>,
	mut meshes: Option<ResMut<Assets<Mesh>>>,
	mut materials: Option<ResMut<Assets<StandardMaterial>>>,
	players: Query<&Transform, (With<VegetationPlayer>, Without<StashClaimHalo>)>,
	stashes: Query<
		(Entity, &Transform, &InventoryUser, &StashPolicy),
		(With<WorldStash>, Without<StashClaimHalo>),
	>,
	anchors: Query<(Entity, &ChildOf, Option<&StashHaloAnchor>), With<StashDisplayedItem>>,
	crates: crate::crate_loot::ClosedCrateQuery,
	mut halo: Query<
		(&mut Transform, &mut Visibility),
		(With<StashClaimHalo>, Without<WorldStash>, Without<VegetationPlayer>),
	>,
) {
	let target = nearest_interact(players.iter(), stashes.iter(), anchors.iter(), &crates)
		.map(|claim| claim.at);
	if halo.is_empty() {
		let Some(meshes) = meshes.as_mut() else {
			return;
		};
		let Some(materials) = materials.as_mut() else {
			return;
		};
		spawn_stash_claim_halo(&mut commands, meshes, materials);
		return;
	}
	let pulse = 1.0 + 0.07 * (time.elapsed_secs() * 5.0).sin();
	for (mut transform, mut visibility) in &mut halo {
		match target {
			Some(origin) => {
				*visibility = Visibility::Visible;
				*transform = Transform {
					translation: origin + Vec3::Y * 0.08,
					rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
					scale: Vec3::splat(pulse),
				};
			}
			None => *visibility = Visibility::Hidden,
		}
	}
}

fn halo_ring_material(color: Color, emissive: LinearRgba) -> StandardMaterial {
	StandardMaterial {
		base_color: color,
		emissive,
		alpha_mode: AlphaMode::Blend,
		unlit: true,
		cull_mode: None,
		..default()
	}
}

fn halo_rings() -> [(f32, Color, LinearRgba); 3] {
	[
		(HALO_YELLOW_RADIUS, HALO_YELLOW, HALO_YELLOW_EMISSIVE),
		(HALO_GREEN_RADIUS, HALO_GREEN, HALO_GREEN_EMISSIVE),
		(HALO_BLUE_RADIUS, HALO_BLUE, HALO_BLUE_EMISSIVE),
	]
}

fn spawn_stash_claim_halo(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	commands
		.spawn((
			Name::new("stash-claim-halo"),
			StashClaimHalo,
			Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
			Visibility::Hidden,
		))
		.with_children(|halo| {
			for (radius, color, emissive) in halo_rings() {
				halo.spawn((
					Mesh3d(meshes.add(Annulus::new(radius - HALO_RING_WIDTH, radius))),
					MeshMaterial3d(materials.add(halo_ring_material(color, emissive))),
				));
			}
		});
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use character_items::{ClothingMaterial, ClothingMesh, FirearmMesh, InventoryItem, ItemColor};
	use damage::tick_queued_despawns;
	use firearm_user::held_scale_from_bounds;
	use material_ref::MaterialId;
	use player::Npc;
	use world_player::Player as VegetationPlayer;

	fn mixed_bag() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::clothing(
					ClothingMesh::Robe,
					ClothingMaterial::Cloth,
					ItemColor::Cool,
				),
			],
			clothing: vec![0],
			weapons: vec![1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		}
	}

	fn spawn_stash_system(
		transform: Transform,
		inventory: Inventory,
		policy: StashPolicy,
	) -> impl FnMut(Commands) -> Option<Entity> {
		move |mut commands| {
			spawn_world_stash(&mut commands, transform, inventory.clone(), policy, None)
		}
	}

	#[test]
	fn default_loot_ttl_is_sixty_seconds_not_corpse_time() {
		assert_eq!(DEFAULT_LOOT_SECS, 60.0);
		assert_eq!(StashPolicy::default().loot_secs, 60.0);
		assert!(!StashPolicy::default().persist);
		assert!(StashPolicy::chest().persist);
		assert!(WorldStashSettings::default().loot_secs > 4.0);
	}

	#[test]
	fn empty_bag_does_not_spawn_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		let spawned = world
			.run_system_once(spawn_stash_system(
				Transform::from_xyz(1.0, 0.0, 0.0),
				Inventory::default(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(spawned.is_none());
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn toggled_items_are_displayed_and_filler_is_not() -> anyhow::Result<()> {
		let mut world = World::new();
		let stash = world
			.run_system_once(spawn_stash_system(
				Transform::from_translation(Vec3::ZERO),
				mixed_bag(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("expected stash"))?;

		let displayed: Vec<_> = world
			.query::<(&StashDisplayedItem, &Name)>()
			.iter(&world)
			.map(|(item, name)| (*item, name.as_str().to_string()))
			.collect();
		assert_eq!(displayed.len(), 2);
		assert!(displayed.iter().any(|(item, name)| {
			item.slot == InventorySlot::Clothing && name.contains("pants")
		}));
		assert!(displayed.iter().any(|(item, name)| {
			item.slot == InventorySlot::Weapons && name.contains("bullpup")
		}));
		assert!(!displayed.iter().any(|(_, name)| name.contains("robe")));

		let user = world.get::<InventoryUser>(stash).ok_or_else(|| anyhow::anyhow!("stash bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert_eq!(bag.items.len(), 3);
		assert_eq!(bag.clothing, vec![0]);
		assert_eq!(bag.weapons, vec![1]);
		Ok(())
	}

	#[test]
	fn npc_downed_spawns_stash_and_detaches_the_body_bag() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let point = Vec3::new(4.0, 1.0, -2.0);
		let bag = world.spawn(mixed_bag()).id();
		let body = world
			.spawn((
				Npc,
				Transform::from_translation(point),
				InventoryUser::carrying(bag),
				Downed { source: None, point, at: 0.0 },
				DespawnAfter::seconds(4.0),
			))
			.id();

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<InventoryUser>(body).is_none());
		assert!(!world.entities().contains(bag));
		assert!(world
			.get::<DespawnAfter>(body)
			.is_some_and(|timer| { (timer.remaining_secs() - 4.0).abs() < 1e-4 }));

		let stashes: Vec<_> = world
			.query::<(Entity, &Transform, &StashPolicy, &DespawnAfter)>()
			.iter(&world)
			.collect();
		assert_eq!(stashes.len(), 3);
		for (stash, transform, policy, despawn) in &stashes {
			assert!((transform.translation.xz() - point.xz()).length() < 2.0);
			assert!(!policy.persist);
			assert!((policy.loot_secs - DEFAULT_LOOT_SECS).abs() < 1e-4);
			assert!((despawn.remaining_secs() - DEFAULT_LOOT_SECS).abs() < 1e-3);
			assert_ne!(*stash, body);
		}
		Ok(())
	}

	fn spawn_downed_npc(
		world: &mut World,
		bag: Inventory,
		kind: Option<MobKind>,
		brains: Option<CharacterBrains>,
		point: Vec3,
	) -> Entity {
		let bag_id = world.spawn(bag).id();
		let mut entity = world.spawn((
			Npc,
			Transform::from_translation(point),
			InventoryUser::carrying(bag_id),
			Downed { source: None, point, at: 0.0 },
		));
		if let Some(kind) = kind {
			entity.insert(kind);
		}
		if let Some(brains) = brains {
			entity.insert(brains);
		}
		entity.id()
	}

	struct NpcLootCase {
		kind: Option<MobKind>,
		brains: Option<CharacterBrains>,
		point: Vec3,
		fraction: LootFraction,
	}

	impl NpcLootCase {
		fn drop_count(self) -> anyhow::Result<usize> {
			let mut world = World::new();
			world.init_resource::<WorldStashSettings>();
			let mut bag = mixed_bag();
			let body =
				spawn_downed_npc(&mut world, bag.clone(), self.kind, self.brains, self.point);
			let downed = Downed { source: None, point: self.point, at: 0.0 };
			let expected = bag.take_fraction(
				&mut ItemRng::from_seed(npc_loot_seed(body, &downed)),
				self.fraction,
			);
			world
				.run_system_once(detach_downed_npc_loot)
				.map_err(|error| anyhow::anyhow!("{error:?}"))?;
			assert_eq!(world.query::<&WorldStash>().iter(&world).count(), expected.items.len());
			Ok(expected.items.len())
		}
	}

	#[test]
	fn raider_downed_drops_one_third() -> anyhow::Result<()> {
		assert_eq!(
			NpcLootCase {
				kind: Some(MobKind::Raider),
				brains: Some(CharacterBrains::Roamer),
				point: Vec3::new(2.0, 0.0, 1.0),
				fraction: LootFraction::ONE_THIRD,
			}
			.drop_count()?,
			1
		);
		Ok(())
	}

	#[test]
	fn guard_downed_drops_one_third() -> anyhow::Result<()> {
		assert_eq!(
			NpcLootCase {
				kind: Some(MobKind::Guard),
				brains: None,
				point: Vec3::new(-1.0, 0.0, 3.0),
				fraction: LootFraction::ONE_THIRD,
			}
			.drop_count()?,
			1
		);
		Ok(())
	}

	#[test]
	fn brawler_downed_drops_one_twelfth() -> anyhow::Result<()> {
		let dropped = NpcLootCase {
			kind: Some(MobKind::Brawler),
			brains: None,
			point: Vec3::new(0.0, 0.0, 5.0),
			fraction: LootFraction::ONE_TWELFTH,
		}
		.drop_count()?;
		assert!(dropped <= 1);
		Ok(())
	}

	#[test]
	fn brains_set_loot_when_mob_kind_is_missing() -> anyhow::Result<()> {
		NpcLootCase {
			kind: None,
			brains: Some(CharacterBrains::Brawler),
			point: Vec3::ZERO,
			fraction: LootFraction::ONE_TWELFTH,
		}
		.drop_count()?;
		Ok(())
	}

	#[test]
	fn unmarked_npc_still_drops_the_full_bag() -> anyhow::Result<()> {
		assert_eq!(npc_loot_fraction(None, None), LootFraction::ALL);
		assert_eq!(
			npc_loot_fraction(Some(&MobKind::Raider), Some(&CharacterBrains::Roamer)),
			LootFraction::ONE_THIRD
		);
		Ok(())
	}

	#[test]
	fn empty_npc_bag_does_not_leave_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let bag = world.spawn(Inventory::default()).id();
		world.spawn((
			InventoryUser::carrying(bag),
			Downed { source: None, point: Vec3::ZERO, at: 0.0 },
		));

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		assert!(!world.entities().contains(bag));
		Ok(())
	}

	#[test]
	fn player_down_does_not_spawn_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let bag = world.spawn(mixed_bag()).id();
		world.spawn((
			VegetationPlayer,
			InventoryUser::carrying(bag),
			Downed { source: None, point: Vec3::ONE, at: 0.0 },
		));

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		assert!(world.entities().contains(bag));
		Ok(())
	}

	#[test]
	fn stash_outlives_corpse_ttl_and_despawns_at_loot_secs() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Last, tick_queued_despawns);
		let stash = app
			.world_mut()
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				mixed_bag(),
				StashPolicy::ephemeral(DEFAULT_LOOT_SECS),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("stash"))?;
		let remaining = app
			.world()
			.get::<DespawnAfter>(stash)
			.ok_or_else(|| anyhow::anyhow!("ttl"))?
			.remaining_secs();
		assert!((remaining - DEFAULT_LOOT_SECS).abs() < 1e-3);
		assert!(remaining > 4.0);

		app.world_mut().entity_mut(stash).insert(DespawnAfter::seconds(0.0));
		app.update();
		assert!(!app.world().entities().contains(stash));
		Ok(())
	}

	fn write_intent(world: &mut World, intent: CharacterIntent) -> anyhow::Result<()> {
		world
			.run_system_once(move |mut writer: MessageWriter<CharacterIntent>| {
				writer.write(intent);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn claim_setup(player_at: Vec3, stash_at: Vec3) -> anyhow::Result<(World, Entity, Entity)> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let player_bag = world.spawn(Inventory::default()).id();
		let player = world
			.spawn((
				VegetationPlayer,
				Transform::from_translation(player_at),
				InventoryUser::carrying(player_bag),
			))
			.id();
		let stash = world
			.run_system_once(spawn_stash_system(
				Transform::from_translation(stash_at),
				mixed_bag(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("stash"))?;
		Ok((world, player, stash))
	}

	#[test]
	fn x_in_radius_take_alls_and_despawns_an_ephemeral_stash() -> anyhow::Result<()> {
		let (mut world, player, stash) = claim_setup(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))?;
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.entities().contains(stash));
		let user = world
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert_eq!(bag.items.len(), 3);
		assert!(bag.clothing.is_empty());
		assert!(bag.weapons.is_empty());
		Ok(())
	}

	#[test]
	fn x_out_of_radius_is_a_noop() -> anyhow::Result<()> {
		let (mut world, player, stash) = claim_setup(Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0))?;
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.entities().contains(stash));
		let user = world
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert!(bag.items.is_empty());
		Ok(())
	}

	#[test]
	fn inventory_intent_explodes_the_bag_into_claimable_items() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<DropLiveGrenade>>();
		let player_bag = world.spawn(mixed_bag()).id();
		let player = world
			.spawn((
				VegetationPlayer,
				Transform::from_xyz(2.0, 0.0, 3.0),
				InventoryUser::carrying(player_bag),
			))
			.id();

		write_intent(&mut world, CharacterIntent::Inventory)?;
		world
			.run_system_once(drop_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let player_inv = world
			.get::<Inventory>(player_bag)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		assert!(player_inv.items.is_empty());

		let stashes: Vec<_> = world
			.query_filtered::<(Entity, &Transform), With<WorldStash>>()
			.iter(&world)
			.map(|(entity, transform)| (entity, transform.translation))
			.collect();
		assert_eq!(stashes.len(), 3);
		for (stash, translation) in &stashes {
			assert!((translation.xz() - Vec2::new(2.0, 3.0)).length() < 2.0);
			assert_ne!(*stash, player);
		}

		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let claimed =
			world.get::<Inventory>(player_bag).ok_or_else(|| anyhow::anyhow!("claimed"))?;
		assert_eq!(claimed.items.len(), 1);
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 2);
		Ok(())
	}

	#[test]
	fn explode_spawns_one_visible_stash_per_item() -> anyhow::Result<()> {
		let mut world = World::new();
		let spawned = world
			.run_system_once(|mut commands: Commands| {
				spawn_exploded_stashes(
					&mut commands,
					Vec3::ZERO,
					mixed_bag(),
					StashPolicy::default(),
					None,
				)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(spawned.len(), 3);
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 3);
		assert_eq!(world.query::<&StashDisplayedItem>().iter(&world).count(), 3);

		let origins: Vec<Vec3> = world
			.query_filtered::<&Transform, With<WorldStash>>()
			.iter(&world)
			.map(|transform| transform.translation)
			.collect();
		assert!(origins.windows(2).any(|pair| pair[0].distance(pair[1]) > 0.3));
		Ok(())
	}

	#[test]
	fn claim_updates_world_player_loadout() -> anyhow::Result<()> {
		use crate::weapon::WorldPlayerLoadout;
		use characters::CharacterAppearance;

		let (mut world, player, stash) = claim_setup(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))?;
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			Inventory::default(),
		));
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.entities().contains(stash));
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(loadout.inventory.items.len(), 3);
		let applied = world
			.get::<crate::weapon::AppliedWorldPlayerLoadout>(player)
			.ok_or_else(|| anyhow::anyhow!("applied"))?;
		assert_eq!(applied.0.inventory.items.len(), 3);
		Ok(())
	}

	#[test]
	fn drop_clears_world_player_loadout() -> anyhow::Result<()> {
		use crate::weapon::WorldPlayerLoadout;
		use characters::CharacterAppearance;

		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<DropLiveGrenade>>();
		let bag = mixed_bag();
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			bag.clone(),
		));
		let player_bag = world.spawn(bag).id();
		world.spawn((VegetationPlayer, Transform::IDENTITY, InventoryUser::carrying(player_bag)));

		write_intent(&mut world, CharacterIntent::Inventory)?;
		world
			.run_system_once(drop_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert!(loadout.inventory.items.is_empty());
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 3);
		Ok(())
	}

	#[test]
	fn persistent_chest_stays_after_claim() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let player_bag = world.spawn(Inventory::default()).id();
		world.spawn((VegetationPlayer, Transform::IDENTITY, InventoryUser::carrying(player_bag)));
		let chest = world
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				mixed_bag(),
				StashPolicy::chest(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("chest"))?;
		assert!(world.get::<DespawnAfter>(chest).is_none());

		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.entities().contains(chest));
		let user = world.get::<InventoryUser>(chest).ok_or_else(|| anyhow::anyhow!("chest bag"))?;
		let bag = world
			.get::<Inventory>(user.bag)
			.ok_or_else(|| anyhow::anyhow!("chest inventory"))?;
		assert!(bag.items.is_empty());
		let player_inv =
			world.get::<Inventory>(player_bag).ok_or_else(|| anyhow::anyhow!("player"))?;
		assert_eq!(player_inv.items.len(), 3);
		Ok(())
	}

	#[test]
	fn clothing_keeps_recipe_and_palette() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				mixed_bag(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let material = world
			.query::<(&StashDisplayedItem, &MaterialRefRoot)>()
			.iter(&world)
			.find(|(item, _)| item.slot == InventorySlot::Clothing)
			.map(|(_, root)| root.0.clone())
			.ok_or_else(|| anyhow::anyhow!("clothing material"))?;
		assert_eq!(material.name, MaterialId::named(ClothingMaterial::Cloth.recipe_id()));
		assert_eq!(material.palette[0], ItemColor::Natural.color());
		Ok(())
	}

	#[test]
	fn dropped_weapon_uses_held_kit_scale() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(spawn_stash_system(
				Transform::IDENTITY,
				Inventory {
					items: vec![InventoryItem::firearm(FirearmMesh::Bullpup)],
					clothing: Vec::new(),
					weapons: vec![0],
					skills: Vec::new(),
					tanzes: Vec::new(),
				},
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let transform = world
			.query::<(&StashDisplayedItem, &Transform)>()
			.iter(&world)
			.find(|(item, _)| item.slot == InventorySlot::Weapons)
			.map(|(_, transform)| *transform)
			.ok_or_else(|| anyhow::anyhow!("weapon visual"))?;
		let kit = GeneratedFirearm::from_spec(
			InventoryItem::firearm(FirearmMesh::Bullpup)
				.firearm_spec()
				.ok_or_else(|| anyhow::anyhow!("spec"))?,
		);
		let expected = held_scale_from_bounds(
			firearm_bounds(&kit),
			FirearmUserSettings::default().held_length,
		);
		assert!((transform.scale.x - expected).abs() < 1e-4);
		assert!(transform.scale.x < 1.0);
		Ok(())
	}

	#[test]
	fn xz_claim_ignores_height() -> anyhow::Result<()> {
		let (mut world, player, stash) =
			claim_setup(Vec3::new(0.0, 8.0, 0.0), Vec3::new(2.0, 0.0, 0.0))?;
		write_intent(&mut world, CharacterIntent::StartInteraction)?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.entities().contains(stash));
		let user = world
			.get::<InventoryUser>(player)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		let bag = world.get::<Inventory>(user.bag).ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert_eq!(bag.items.len(), 3);
		Ok(())
	}

	#[test]
	fn kenney_interact_icons_are_in_the_asset_tree() {
		let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
		assert!(root.join(INTERACT_PAD_ICON).is_file());
		assert!(root.join(INTERACT_KEY_ICON).is_file());
	}

	#[test]
	fn interact_icon_prefers_pad_when_connected() {
		assert_eq!(stash_interact_icon(true), INTERACT_PAD_ICON);
		assert_eq!(stash_interact_icon(false), INTERACT_KEY_ICON);
	}

	#[test]
	fn pickup_action_names_a_single_item() {
		let pants = one_garment();
		assert_eq!(pickup_action_label(&pants), format!("Pick up {}", pants.items[0].name()));
		assert_eq!(pickup_action_label(&mixed_bag()), "Pick up 3 items");
		assert_eq!(pickup_action_label(&Inventory::default()), "Pick up");
	}

	#[test]
	fn interact_prompt_toggles_in_radius() -> anyhow::Result<()> {
		let (mut world, _, stash) = claim_setup(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0))?;
		world.init_resource::<Time>();
		world.spawn((StashInteractPrompt, Transform::IDENTITY, Visibility::Hidden));
		world.spawn((StashInteractCaption, Text::new(""), Node::default(), Visibility::Hidden));
		world
			.run_system_once(sync_stash_interact_prompt)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let (visible, at) = world
			.query_filtered::<(&Visibility, &Transform), With<StashInteractPrompt>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*visible, Visibility::Visible);
		assert!(at.translation.x > 0.0 && at.translation.x < 1.0);
		assert!(at.translation.xz().length() < 1.0);
		assert!(at.translation.y > 0.4);
		let (caption, caption_visible) = world
			.query_filtered::<(&Text, &Visibility), With<StashInteractCaption>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(caption.0, "Pick up 3 items");
		assert_eq!(*caption_visible, Visibility::Visible);

		world.entity_mut(stash).insert(Transform::from_xyz(40.0, 0.0, 0.0));
		world
			.run_system_once(sync_stash_interact_prompt)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let hidden = world
			.query_filtered::<&Visibility, With<StashInteractPrompt>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*hidden, Visibility::Hidden);
		let caption_hidden = world
			.query_filtered::<&Visibility, With<StashInteractCaption>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*caption_hidden, Visibility::Hidden);
		Ok(())
	}

	#[test]
	fn interact_prompt_sits_between_player_and_item() {
		let item = Vec3::new(2.0, 0.08, 0.0);
		let player = Vec3::ZERO;
		let at = prompt_world_point(item, player);
		assert!(at.x > player.x && at.x < item.x);
		assert!(at.z.abs() < 1e-4);
		assert!((at.y - (item.y + PROMPT_ABOVE)).abs() < 1e-4);
		let beside = prompt_world_point(item, item + Vec3::X * 0.4);
		assert!((beside.x - (item.x + 0.2)).abs() < 1e-4);
	}

	#[test]
	fn interact_prompt_faces_the_camera() {
		let origin = Vec3::new(2.0, 1.0, 0.0);
		let camera = Vec3::new(2.0, 1.5, 4.0);
		let at = origin + Vec3::Y * PROMPT_ABOVE;
		let transform = prompt_billboard(at, Some(camera));
		let toward_camera = (camera - transform.translation).normalize();
		assert!(
			(-transform.forward()).dot(toward_camera) > 0.7,
			"quad +Z (opposite forward) should face the camera"
		);
		assert!((transform.translation - at).length() < 1e-4);
	}

	#[test]
	fn claim_halo_uses_three_thinner_rings() {
		let rings = halo_rings();
		assert_eq!(rings[0].0, 1.5);
		assert_eq!(rings[1].0, 0.75);
		assert_eq!(rings[2].0, 0.25);
		assert_eq!(rings[0].1, HALO_YELLOW);
		assert_eq!(rings[1].1, HALO_GREEN);
		assert_eq!(rings[2].1, HALO_BLUE);
		assert!(HALO_RING_WIDTH < 0.18);
		assert!(HALO_BLUE_RADIUS > HALO_RING_WIDTH);
	}

	fn one_garment() -> Inventory {
		Inventory {
			items: vec![InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			)],
			clothing: vec![0],
			weapons: Vec::new(),
			skills: Vec::new(),
			tanzes: Vec::new(),
		}
	}

	#[test]
	fn single_displayed_item_sits_on_the_host() -> anyhow::Result<()> {
		let mut world = World::new();
		world
			.run_system_once(spawn_stash_system(
				Transform::from_translation(Vec3::ZERO),
				one_garment(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let transform = world
			.query::<(&StashDisplayedItem, &Transform)>()
			.iter(&world)
			.next()
			.map(|(_, transform)| *transform)
			.ok_or_else(|| anyhow::anyhow!("visual"))?;
		assert!(transform.translation.xz().length() < 1e-3);
		Ok(())
	}

	#[test]
	fn claim_halo_marks_the_nearest_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Time>();
		let player_bag = world.spawn(Inventory::default()).id();
		world.spawn((VegetationPlayer, Transform::IDENTITY, InventoryUser::carrying(player_bag)));
		let near = world
			.run_system_once(spawn_stash_system(
				Transform::from_xyz(1.0, 0.0, 0.0),
				one_garment(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("near"))?;
		world
			.run_system_once(spawn_stash_system(
				Transform::from_xyz(3.0, 0.0, 0.0),
				one_garment(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.spawn((StashClaimHalo, Transform::IDENTITY, Visibility::Hidden));
		world
			.run_system_once(sync_stash_claim_halo)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let (transform, visibility) = world
			.query_filtered::<(&Transform, &Visibility), With<StashClaimHalo>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*visibility, Visibility::Visible);
		assert!((transform.translation.xz() - Vec2::new(1.0, 0.0)).length() < 1e-3);

		world.entity_mut(near).insert(Transform::from_xyz(40.0, 0.0, 0.0));
		world
			.run_system_once(sync_stash_claim_halo)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let far = world
			.query_filtered::<&Transform, With<StashClaimHalo>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.translation;
		assert!((far.xz() - Vec2::new(3.0, 0.0)).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn claim_halo_marks_a_closed_crate() -> anyhow::Result<()> {
		use crate::crate_loot::{ClosedLid, CrateLoot};
		use furniture_assemblies::{FurnitureKitPart, PartKind};
		use lod::gen::Id;

		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<CrateLoot>();
		let player_bag = world.spawn(Inventory::default()).id();
		world.spawn((VegetationPlayer, Transform::IDENTITY, InventoryUser::carrying(player_bag)));
		let at = Transform::from_xyz(2.0, 0.4, 0.0);
		let host = crate::crate_loot::spawn_furniture_host(&mut world, Id::Universal);
		world.spawn((
			FurnitureKitPart { kind: PartKind::ChestLid, finish_seed: 1, slot: 0 },
			ClosedLid(at),
			at,
			GlobalTransform::from(at),
			ChildOf(host),
		));
		world.spawn((StashClaimHalo, Transform::IDENTITY, Visibility::Hidden));
		world
			.run_system_once(sync_stash_claim_halo)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let (transform, visibility) = world
			.query_filtered::<(&Transform, &Visibility), With<StashClaimHalo>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*visibility, Visibility::Visible);
		assert!((transform.translation.xz() - Vec2::new(2.0, 0.0)).length() < 1e-3);

		world
			.run_system_once(spawn_stash_system(
				Transform::from_xyz(1.0, 0.0, 0.0),
				one_garment(),
				StashPolicy::default(),
			))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_stash_claim_halo)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let on_stash = world
			.query_filtered::<&Transform, With<StashClaimHalo>>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.translation;
		assert!((on_stash.xz() - Vec2::new(1.0, 0.0)).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn downed_without_inventory_despawns_the_held_kit() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldStashSettings>();
		let gun = world.spawn(Name::new("held-kit")).id();
		world.spawn((
			FirearmUser::holding(gun),
			Downed { source: None, point: Vec3::ZERO, at: 0.0 },
		));

		world
			.run_system_once(detach_downed_npc_loot)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.entities().contains(gun));
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		Ok(())
	}
}
