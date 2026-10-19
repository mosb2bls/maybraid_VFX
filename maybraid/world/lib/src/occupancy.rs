//! World occupancy: publish HCSG cell facts and mark the world player.
//!
//! [#1111](https://github.com/ramate-io/maybraid/issues/1111). Reads
//! [`ChicoForest`] / [`ChicoGrove`] from [`HcsgStorage`] — selected cell
//! metadata, not vegetation meshes or [`chico::ChicoGroveHost`]. The ordinary
//! world registers this plugin so occupancy runs with Self and no Bikanwarda.
//! Ledgers and the stock-capped award path belong to
//! [`crate::world_lubikan::WorldLubikanPlugin`] /
//! [`LubikansPlugin`]. This adapter only publishes cell facts and inserts
//! [`OccupancyRewardsUser`].
//!
//! Pause copies [`WorldGameplayEnabled`] onto [`OccupancyRewardsPaused`]
//! **before** occupancy evaluate. Retiring bodies and ended play sessions
//! close the occupancy user so they cannot credit.

use std::collections::BTreeSet;

use bevy::prelude::*;
use chico::{ChicoForest, ChicoGrove, ForestExtent, ForestGroveKind, ForestLayer};
use damage::{DamageSystems, Downed};
use lod::gen::Id;
use lod::hcsg::{Busy, HcsgStorage};
use lubikans::LubikansPlugin;
use occupancy_rewards::{
	CellMetadata, CellMetadataStore, ConstructionTag, ForestCellId, ForestInfo, GroveInfo,
	GroveKindTag, GroveLayerId, GroveTileId, LayeringTag, OccupancyDemoConfig,
	OccupancyRewardsPaused, OccupancyRewardsPlugin, OccupancyRewardsSystems, OccupancyRewardsUser,
	SemanticCellKey, VegetationLayer,
};
use world_player::Player as VegetationPlayer;

use crate::control::{WorldGameplayEnabled, WorldPlaySessionEnded};

/// Occupancy metadata publish and player mark.
///
/// Runs in Update: after [`lubikans::LubikanSystems::Owner`] (PreUpdate) and
/// before occupancy evaluate / [`lubikans::LubikanSystems::Award`] (PostUpdate).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldOccupancySystems {
	Publish,
	Attach,
	/// Copy world pause / session-end onto occupancy **before** Evaluate.
	Gate,
}

/// Registers occupancy in the ordinary world and fills the metadata store
/// from published HCSG cell facts.
pub struct WorldOccupancyPlugin;

impl Plugin for WorldOccupancyPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikansPlugin>() {
			app.add_plugins(LubikansPlugin);
		}
		if app.world().get_resource::<OccupancyDemoConfig>().is_none() {
			app.insert_resource(OccupancyDemoConfig::from_env());
		}
		if !app.is_plugin_added::<OccupancyRewardsPlugin>() {
			let demo = *app.world().resource::<OccupancyDemoConfig>();
			app.add_plugins(OccupancyRewardsPlugin { rules: demo.rule_set() });
		}
		app.init_resource::<WorldOccupancyPublishState>()
			.init_resource::<WorldOccupancySessionClosed>()
			.init_resource::<WorldGameplayEnabled>()
			.add_message::<WorldPlaySessionEnded>()
			.configure_sets(
				Update,
				WorldOccupancySystems::Attach.after(WorldOccupancySystems::Publish),
			)
			.configure_sets(
				PostUpdate,
				WorldOccupancySystems::Gate
					.after(DamageSystems::Down)
					.before(OccupancyRewardsSystems::Evaluate),
			)
			.add_systems(
				Update,
				publish_world_occupancy_metadata.in_set(WorldOccupancySystems::Publish),
			)
			.add_systems(
				Update,
				attach_world_occupancy_player.in_set(WorldOccupancySystems::Attach),
			)
			.add_systems(
				PostUpdate,
				(
					sync_occupancy_rewards_paused,
					close_occupancy_on_session_end,
					close_occupancy_on_retired_bodies,
				)
					.in_set(WorldOccupancySystems::Gate),
			);
	}
}

/// Last successful actor-cell projection. Re-reads HCSG only when the occupied
/// cells or HCSG membership revisions change.
#[derive(Resource, Default)]
pub(crate) struct WorldOccupancyPublishState {
	keys: BTreeSet<SemanticCellKey>,
	actor_cells: BTreeSet<ActorCellId>,
	forest_revision: Option<u64>,
	grove_revision: Option<u64>,
}

/// Latch: play-session teardown keeps occupancy paused after the message drains.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WorldOccupancySessionClosed(pub bool);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ActorCellId {
	forest: ForestCellId,
	grove: GroveTileId,
}

impl ActorCellId {
	fn from_position(position: Vec3) -> Self {
		Self { forest: ForestCellId::containing(position), grove: GroveTileId::containing(position) }
	}

	fn forest_id(self) -> Id {
		ForestExtent::from_cell_index(self.forest.ix, self.forest.iz).id()
	}

	fn grove_ids(self) -> [Id; 4] {
		std::array::from_fn(|index| {
			Id::from_cell(
				GroveLayerId { tile: self.grove, layer: VegetationLayer::ALL[index] }.aabb(),
			)
		})
	}
}

type OccupancyActors<'w, 's> = Query<
	'w,
	's,
	&'static GlobalTransform,
	Or<(With<VegetationPlayer>, With<OccupancyRewardsUser>)>,
>;

type OccupancyPlayers<'w, 's> =
	Query<'w, 's, (Entity, Option<&'static OccupancyRewardsUser>), With<VegetationPlayer>>;

/// Copy selected forest / grove facts into [`CellMetadataStore`].
///
/// Reads only the forest cell and grove layers the occupancy actors occupy.
/// [`Busy`] leaves the store unchanged so a contended lock cannot evict live
/// occupancy. Equal cells + unchanged HCSG membership skip the read.
pub fn publish_world_occupancy_metadata(
	storage: Option<Res<HcsgStorage>>,
	mut store: ResMut<CellMetadataStore>,
	mut published: ResMut<WorldOccupancyPublishState>,
	actors: OccupancyActors,
) {
	let Some(storage) = storage else {
		return;
	};
	let cells = occupancy_actor_cells(&actors);
	if cells.is_empty() {
		return;
	}
	let Ok(forest_revision) = storage.try_membership_revision::<ChicoForest>() else {
		return;
	};
	let Ok(grove_revision) = storage.try_membership_revision::<ChicoGrove>() else {
		return;
	};
	if published.actor_cells == cells
		&& published.forest_revision == Some(forest_revision)
		&& published.grove_revision == Some(grove_revision)
	{
		return;
	}
	let Ok(scan) = OccupancyHcsgScan::try_collect(storage.as_ref(), &cells) else {
		return;
	};
	scan.publish(&mut store, &mut published);
	published.actor_cells = cells;
	published.forest_revision = Some(forest_revision);
	published.grove_revision = Some(grove_revision);
}

/// Mark the world player for occupancy. Does not create a ledger.
pub fn attach_world_occupancy_player(mut commands: Commands, players: OccupancyPlayers) {
	for (entity, user) in &players {
		if user.is_none() {
			commands.entity(entity).insert(OccupancyRewardsUser::default());
		}
	}
}

/// World pause and ended-session latch, applied before occupancy Evaluate.
pub fn sync_occupancy_rewards_paused(
	gameplay: Res<WorldGameplayEnabled>,
	closed: Res<WorldOccupancySessionClosed>,
	mut paused: ResMut<OccupancyRewardsPaused>,
) {
	let should_pause = !gameplay.0 || closed.0;
	if paused.0 != should_pause {
		paused.0 = should_pause;
	}
}

/// Play-session teardown closes every occupancy user and latches the pause.
pub fn close_occupancy_on_session_end(
	mut ended: MessageReader<WorldPlaySessionEnded>,
	mut closed: ResMut<WorldOccupancySessionClosed>,
	mut users: Query<&mut OccupancyRewardsUser>,
) {
	if ended.read().next().is_none() {
		return;
	}
	closed.0 = true;
	for mut user in &mut users {
		user.close();
	}
}

/// Downed / retiring bodies cannot keep accruing occupancy.
pub fn close_occupancy_on_retired_bodies(
	mut users: Query<&mut OccupancyRewardsUser, Added<Downed>>,
) {
	for mut user in &mut users {
		user.close();
	}
}

struct OccupancyHcsgScan {
	forests: Vec<CellMetadata>,
	groves: Vec<CellMetadata>,
}

impl OccupancyHcsgScan {
	fn try_collect(storage: &HcsgStorage, cells: &BTreeSet<ActorCellId>) -> Result<Self, Busy> {
		let forest_ids: Vec<Id> = cells.iter().map(|cell| cell.forest_id()).collect();
		let grove_ids: Vec<Id> = cells.iter().flat_map(|cell| cell.grove_ids()).collect();
		Ok(Self {
			forests: collect_entries::<ChicoForest, _>(storage, &forest_ids, |forest| {
				OccupancyForest(forest).metadata()
			})?,
			groves: collect_entries::<ChicoGrove, _>(storage, &grove_ids, |grove| {
				OccupancyGrove(grove).metadata()
			})?,
		})
	}

	fn publish(self, store: &mut CellMetadataStore, published: &mut WorldOccupancyPublishState) {
		let mut seen = BTreeSet::new();
		for meta in self.forests.into_iter().chain(self.groves) {
			seen.insert(meta.key);
			store.publish(meta);
		}
		for key in published.keys.difference(&seen) {
			store.evict(*key);
		}
		published.keys = seen;
	}
}

fn collect_entries<T, F>(
	storage: &HcsgStorage,
	ids: &[Id],
	into_meta: F,
) -> Result<Vec<CellMetadata>, Busy>
where
	T: lod::hcsg::HcsgValue,
	F: Fn(&T) -> CellMetadata,
{
	let mut metas = Vec::new();
	for entry in storage.try_entries::<T>(ids)? {
		let Some(entry) = entry else {
			continue;
		};
		metas.push(into_meta(entry.value.as_ref()));
	}
	Ok(metas)
}

fn occupancy_actor_cells(actors: &OccupancyActors) -> BTreeSet<ActorCellId> {
	actors.iter().map(|transform| ActorCellId::from_position(transform.translation())).collect()
}

struct OccupancyForest<'a>(&'a ChicoForest);

impl OccupancyForest<'_> {
	fn metadata(self) -> CellMetadata {
		let forest = self.0;
		CellMetadata::new(SemanticCellKey::Forest(occupancy_rewards::ForestCellId::containing(
			forest.extent.center(),
		)))
		.with_forest(ForestInfo {
			layering: Some(LayeringTag::named(forest.layers.layering.as_kebab())),
		})
		.with_bounds(forest.extent.aabb())
	}
}

struct OccupancyGrove<'a>(&'a ChicoGrove);

impl OccupancyGrove<'_> {
	fn metadata(self) -> CellMetadata {
		let grove = self.0;
		let layer = OccupancyForestLayer(grove.layer).vegetation();
		let tile = GroveTileId::containing(grove_center(grove));
		let kind = grove.recipes.first().map(|recipe| OccupancyGroveKind(recipe.kind).tag());
		let mut meta = CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId { tile, layer }))
			.with_grove(GroveInfo { layer, kind })
			.with_bounds(grove.aabb());
		for recipe in &grove.recipes {
			for tag in OccupancyGroveKind(recipe.kind).constructions() {
				meta = meta.with_construction(tag);
			}
			if meta.planting_extent_xz.is_none() {
				meta = meta.with_planting_extent_xz(recipe.kind.cell_extent_xz().x);
			}
		}
		meta
	}
}

fn grove_center(grove: &ChicoGrove) -> Vec3 {
	(grove.extent.min() + grove.extent.max()) * 0.5
}

struct OccupancyForestLayer(ForestLayer);

impl OccupancyForestLayer {
	fn vegetation(self) -> VegetationLayer {
		match self.0 {
			ForestLayer::Tufts => VegetationLayer::Tufts,
			ForestLayer::Understory => VegetationLayer::Understory,
			ForestLayer::LowerCanopy => VegetationLayer::LowerCanopy,
			ForestLayer::UpperCanopy => VegetationLayer::UpperCanopy,
		}
	}
}

/// Stable names for a selected grove recipe. Not a mesh identity.
#[derive(Clone, Copy)]
struct OccupancyGroveKind(ForestGroveKind);

impl OccupancyGroveKind {
	fn tag(self) -> GroveKindTag {
		GroveKindTag::named(self.names().tag)
	}

	fn constructions(self) -> impl Iterator<Item = ConstructionTag> {
		let names = self.names();
		let recipe = ConstructionTag::named(names.snake);
		std::iter::once(recipe).chain(self.well_known_constructions())
	}

	fn well_known_constructions(self) -> impl Iterator<Item = ConstructionTag> {
		let sope =
			matches!(self.0, ForestGroveKind::TradeWinds).then_some(ConstructionTag::SOPES_BANYAN);
		sope.into_iter()
	}

	fn names(self) -> OccupancyGroveKindNames {
		OccupancyGroveKindNames::of(self.0)
	}
}

struct OccupancyGroveKindNames {
	tag: &'static str,
	snake: &'static str,
}

impl OccupancyGroveKindNames {
	fn of(kind: ForestGroveKind) -> Self {
		match kind {
			ForestGroveKind::Alpine => Self { tag: "Alpine", snake: "alpine" },
			ForestGroveKind::AridConiferSapling => {
				Self { tag: "AridConiferSapling", snake: "arid_conifer_sapling" }
			}
			ForestGroveKind::BraidGrass => Self { tag: "BraidGrass", snake: "braid_grass" },
			ForestGroveKind::BushScrub => Self { tag: "BushScrub", snake: "bush_scrub" },
			ForestGroveKind::ChristmasTaiga => {
				Self { tag: "ChristmasTaiga", snake: "christmas_taiga" }
			}
			ForestGroveKind::CommonTufts => Self { tag: "CommonTufts", snake: "common_tufts" },
			ForestGroveKind::ConiferMassives => {
				Self { tag: "ConiferMassives", snake: "conifer_massives" }
			}
			ForestGroveKind::ConiferSapling => {
				Self { tag: "ConiferSapling", snake: "conifer_sapling" }
			}
			ForestGroveKind::DateGrove => Self { tag: "DateGrove", snake: "date_grove" },
			ForestGroveKind::Dryland => Self { tag: "Dryland", snake: "dryland" },
			ForestGroveKind::ForlornSavanna => {
				Self { tag: "ForlornSavanna", snake: "forlorn_savanna" }
			}
			ForestGroveKind::GoettingenFollow => {
				Self { tag: "GoettingenFollow", snake: "goettingen_follow" }
			}
			ForestGroveKind::HighBush => Self { tag: "HighBush", snake: "high_bush" },
			ForestGroveKind::JerrysChaparral => {
				Self { tag: "JerrysChaparral", snake: "jerrys_chaparral" }
			}
			ForestGroveKind::JungleLowerMassives => {
				Self { tag: "JungleLowerMassives", snake: "jungle_lower_massives" }
			}
			ForestGroveKind::JungleMassives => {
				Self { tag: "JungleMassives", snake: "jungle_massives" }
			}
			ForestGroveKind::Leeward => Self { tag: "Leeward", snake: "leeward" },
			ForestGroveKind::LevantineScrub => {
				Self { tag: "LevantineScrub", snake: "levantine_scrub" }
			}
			ForestGroveKind::LowBush => Self { tag: "LowBush", snake: "low_bush" },
			ForestGroveKind::MonsterGrass => Self { tag: "MonsterGrass", snake: "monster_grass" },
			ForestGroveKind::Orchard => Self { tag: "Orchard", snake: "orchard" },
			ForestGroveKind::PalmShade => Self { tag: "PalmShade", snake: "palm_shade" },
			ForestGroveKind::RiparianGeneral => {
				Self { tag: "RiparianGeneral", snake: "riparian_general" }
			}
			ForestGroveKind::RiparianMix => Self { tag: "RiparianMix", snake: "riparian_mix" },
			ForestGroveKind::RiverineGreen => {
				Self { tag: "RiverineGreen", snake: "riverine_green" }
			}
			ForestGroveKind::RollingOaks => Self { tag: "RollingOaks", snake: "rolling_oaks" },
			ForestGroveKind::Shamanhome => Self { tag: "Shamanhome", snake: "shamanhome" },
			ForestGroveKind::SpottyBushes => Self { tag: "SpottyBushes", snake: "spotty_bushes" },
			ForestGroveKind::Storytellers => Self { tag: "Storytellers", snake: "storytellers" },
			ForestGroveKind::StrangeOasis => Self { tag: "StrangeOasis", snake: "strange_oasis" },
			ForestGroveKind::TallGrass => Self { tag: "TallGrass", snake: "tall_grass" },
			ForestGroveKind::TemperateLowerMassives => {
				Self { tag: "TemperateLowerMassives", snake: "temperate_lower_massives" }
			}
			ForestGroveKind::TemperateMassives => {
				Self { tag: "TemperateMassives", snake: "temperate_massives" }
			}
			ForestGroveKind::TradeWinds => Self { tag: "TradeWinds", snake: "trade_winds" },
			ForestGroveKind::TropicalThicket => {
				Self { tag: "TropicalThicket", snake: "tropical_thicket" }
			}
			ForestGroveKind::TropicalTufts => {
				Self { tag: "TropicalTufts", snake: "tropical_tufts" }
			}
			ForestGroveKind::TropicalUndergrowth => {
				Self { tag: "TropicalUndergrowth", snake: "tropical_undergrowth" }
			}
			ForestGroveKind::UnendingJungle => {
				Self { tag: "UnendingJungle", snake: "unending_jungle" }
			}
			ForestGroveKind::Vineyard => Self { tag: "Vineyard", snake: "vineyard" },
			ForestGroveKind::WanderingAcacia => {
				Self { tag: "WanderingAcacia", snake: "wandering_acacia" }
			}
			ForestGroveKind::WildGrass => Self { tag: "WildGrass", snake: "wild_grass" },
		}
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::system::RunSystemOnce;
	use bevy::math::bounding::Aabb3d;
	use chico::{ForestExtent, ForestSelection, LayeringKind};
	use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};
	use lubikans::{
		LubikanAccountId, LubikanChargeChanged, LubikanChargeReason, LubikanChargeRequest,
		LubikanLedger, LubikanSelfConfig, LubikansPlugin,
	};
	use occupancy_rewards::{
		evaluate_occupancy_rewards, OccupancyIdentity, OccupancyRewardsPaused, RewardRule,
		RewardRuleSet, SemanticCellKey,
	};

	use super::*;

	const STAND: Vec3 = Vec3::new(50.0, 0.0, 50.0);

	fn lush_selection() -> ForestSelection {
		ForestSelection { layering: Some(LayeringKind::LushJungle), ..ForestSelection::default() }
	}

	fn origin_grove_region() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(1.0, 0.0, 1.0), Vec3::new(99.0, 4.0, 99.0))
	}

	fn publish_lush_origin_cells() -> anyhow::Result<HcsgStorage> {
		let storage = HcsgStorage::default();
		storage.seed(lush_selection(), universal_bounds());
		let mut cx = GenerationContext::new(&storage);
		let forest = cx
			.get_or_generate::<ChicoForest>(ForestExtent::default_cell().id())
			.ok_or_else(|| anyhow::anyhow!("origin forest should select under LushJungle"))?;
		anyhow::ensure!(forest.layers.upper_canopy == Some(ForestGroveKind::TradeWinds));
		let groves = cx.get_or_generate_in::<ChicoGrove>(origin_grove_region());
		anyhow::ensure!(!groves.is_empty(), "origin grove tiles should select under LushJungle");
		anyhow::ensure!(groves.iter().any(|grove| !grove.recipes.is_empty()));
		Ok(storage)
	}

	fn occupancy_app(storage: HcsgStorage) -> App {
		occupancy_app_with_self(storage, LubikanSelfConfig::default())
	}

	fn occupancy_app_with_self(storage: HcsgStorage, self_config: LubikanSelfConfig) -> App {
		occupancy_app_with_demo(storage, self_config, OccupancyDemoConfig::default())
	}

	fn occupancy_app_with_demo(
		storage: HcsgStorage,
		self_config: LubikanSelfConfig,
		demo: OccupancyDemoConfig,
	) -> App {
		let mut app = App::new();
		app.insert_resource(self_config)
			.insert_resource(demo)
			.add_plugins(MinimalPlugins)
			.add_plugins(WorldOccupancyPlugin)
			.insert_resource(storage);
		app
	}

	fn far_grove_region() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(301.0, 0.0, 301.0), Vec3::new(399.0, 4.0, 399.0))
	}

	fn publish_lush_stand_and_far_cells() -> anyhow::Result<HcsgStorage> {
		let storage = publish_lush_origin_cells()?;
		let mut cx = GenerationContext::new(&storage);
		let groves = cx.get_or_generate_in::<ChicoGrove>(far_grove_region());
		anyhow::ensure!(!groves.is_empty(), "far grove tiles should select under LushJungle");
		Ok(storage)
	}

	fn publish_lush_forest_only() -> anyhow::Result<HcsgStorage> {
		let storage = HcsgStorage::default();
		storage.seed(lush_selection(), universal_bounds());
		let mut cx = GenerationContext::new(&storage);
		anyhow::ensure!(cx
			.get_or_generate::<ChicoForest>(ForestExtent::default_cell().id())
			.is_some());
		Ok(storage)
	}

	fn ready_occupancy_player(app: &mut App, at: Vec3) -> anyhow::Result<Entity> {
		let player = spawn_player(app, at);
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.world_mut()
			.run_system_once(attach_world_occupancy_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		attach_runtime_ledger(app);
		let _ = drain_requests(app.world_mut());
		let _ = drain_changes(app.world_mut());
		Ok(player)
	}

	fn gate_before_evaluate(app: &mut App, dt: Duration) -> anyhow::Result<()> {
		app.world_mut()
			.run_system_once(sync_occupancy_rewards_paused)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		evaluate_with(app, dt)
	}

	fn spawn_player(app: &mut App, at: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				VegetationPlayer,
				player::Player,
				Transform::from_translation(at),
				GlobalTransform::from_translation(at),
			))
			.id()
	}

	fn attach_runtime_ledger(app: &mut App) {
		app.update();
	}

	fn drain_requests(world: &mut World) -> Vec<LubikanChargeRequest> {
		world.resource_mut::<Messages<LubikanChargeRequest>>().drain().collect()
	}

	fn drain_changes(world: &mut World) -> Vec<LubikanChargeChanged> {
		world.resource_mut::<Messages<LubikanChargeChanged>>().drain().collect()
	}

	fn evaluate_with(app: &mut App, dt: Duration) -> anyhow::Result<()> {
		let mut time = Time::<()>::default();
		time.advance_by(dt);
		app.insert_resource(time);
		app.world_mut()
			.run_system_once(evaluate_occupancy_rewards)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	/// Apply pending [`LubikanChargeRequest`]s through Award without ticking occupancy.
	fn flush_award(app: &mut App) {
		app.world_mut().resource_mut::<OccupancyRewardsPaused>().0 = true;
		app.update();
		app.world_mut().resource_mut::<OccupancyRewardsPaused>().0 = false;
	}

	#[test]
	fn world_occupancy_plugin_installs_self_rules_without_replacing_them() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(WorldOccupancyPlugin);
		assert!(app.is_plugin_added::<OccupancyRewardsPlugin>());
		assert!(app.is_plugin_added::<LubikansPlugin>());
		let rules = app.world().resource::<RewardRuleSet>();
		let rule = rules
			.rule(occupancy_rewards::RuleId::SELF_DEMONSTRATION)
			.ok_or_else(|| anyhow::anyhow!("ordinary world should ship the Self demonstration"))?;
		assert_eq!(rule.credited, LubikanAccountId::SELF);
		assert_eq!(rule.interval, Duration::from_secs(5));
		Ok(())
	}

	#[test]
	fn occupancy_attach_does_not_create_a_ledger() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_plugins(WorldOccupancyPlugin);
		let player = app
			.world_mut()
			.spawn((VegetationPlayer, Transform::IDENTITY, GlobalTransform::IDENTITY))
			.id();
		app.world_mut()
			.run_system_once(attach_world_occupancy_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let entity = app.world().entity(player);
		anyhow::ensure!(entity.contains::<OccupancyRewardsUser>());
		anyhow::ensure!(!entity.contains::<LubikanLedger>(), "occupancy must not create ledgers");
		Ok(())
	}

	#[test]
	fn publisher_fills_store_from_hcsg_grove_recipes_not_meshes() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let player = spawn_player(&mut app, STAND);
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.world_mut()
			.run_system_once(attach_world_occupancy_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		attach_runtime_ledger(&mut app);
		let identity = OccupancyIdentity::from_position(STAND);
		let store = app.world().resource::<CellMetadataStore>();
		anyhow::ensure!(store.get(SemanticCellKey::Forest(identity.forest)).is_some());
		let view = store.view(&identity);
		anyhow::ensure!(view.grove_loaded, "HCSG grove recipes should mark the tile loaded");
		anyhow::ensure!(
			!view.constructions_grove.is_empty(),
			"selected grove recipes are constructions"
		);
		anyhow::ensure!(view.constructions_grove.contains(&ConstructionTag::SOPES_BANYAN));
		let entity = app.world().entity(player);
		anyhow::ensure!(entity.contains::<OccupancyRewardsUser>());
		let ledger = entity
			.get::<LubikanLedger>()
			.ok_or_else(|| anyhow::anyhow!("#1149 runtime should attach Self"))?;
		anyhow::ensure!(ledger.selection().resolved() == LubikanAccountId::SELF);
		Ok(())
	}

	#[test]
	fn player_in_qualifying_grove_accrues_self_charge_at_configured_interval() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let player = spawn_player(&mut app, STAND);
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.world_mut()
			.run_system_once(attach_world_occupancy_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		attach_runtime_ledger(&mut app);
		let _ = drain_requests(app.world_mut());
		let _ = drain_changes(app.world_mut());
		let expected = RewardRule::self_demonstration().amount;
		let charge_before = app
			.world()
			.get::<LubikanLedger>(player)
			.ok_or_else(|| anyhow::anyhow!("#1149 runtime should attach Self"))?
			.equipped()
			.charge()
			.current();
		anyhow::ensure!(charge_before == 0.0, "fresh Self ledger starts at 0");

		evaluate_with(&mut app, Duration::from_secs(4))?;
		flush_award(&mut app);
		anyhow::ensure!(
			drain_changes(app.world_mut()).is_empty(),
			"Self demonstration is 5 s; 4 s must not emit LubikanChargeChanged"
		);
		let charge_at_four = app
			.world()
			.get::<LubikanLedger>(player)
			.ok_or_else(|| anyhow::anyhow!("ledger"))?
			.equipped()
			.charge()
			.current();
		anyhow::ensure!(charge_at_four == 0.0, "4 s must not mutate charge");

		evaluate_with(&mut app, Duration::from_secs(1))?;
		flush_award(&mut app);
		let changed = drain_changes(app.world_mut());
		anyhow::ensure!(changed.len() == 1, "exactly one LubikanChargeChanged at 5 s");
		let change = changed[0];
		assert_eq!(change.recipient, player);
		assert_eq!(change.lubikan, LubikanAccountId::SELF);
		assert_eq!(change.amount, expected);
		assert_eq!(change.points_earned, expected);
		assert_eq!(change.reason, LubikanChargeReason::OCCUPANCY);
		assert_eq!(change.origin, Some(STAND));
		assert!(!change.discarded_at_stock_cap());
		let charge_after = app
			.world()
			.get::<LubikanLedger>(player)
			.ok_or_else(|| anyhow::anyhow!("ledger"))?
			.equipped()
			.charge()
			.current();
		anyhow::ensure!(charge_after == expected, "Award must credit the ledger");
		Ok(())
	}

	#[test]
	fn occupancy_award_uses_stock_cap_and_keeps_points_earned() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app_with_self(
			storage,
			LubikanSelfConfig {
				capacity: 1.0,
				activation_cost: 1.0,
				max_stock: 1,
				..LubikanSelfConfig::default()
			},
		);
		let player = spawn_player(&mut app, STAND);
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.world_mut()
			.run_system_once(attach_world_occupancy_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		attach_runtime_ledger(&mut app);
		let _ = drain_requests(app.world_mut());
		let _ = drain_changes(app.world_mut());
		let requested = RewardRule::self_demonstration().amount;

		evaluate_with(&mut app, Duration::from_secs(5))?;
		flush_award(&mut app);
		let first = drain_changes(app.world_mut());
		anyhow::ensure!(first.len() == 1, "first occupancy tick should apply under the cap");
		assert_eq!(first[0].amount, requested);
		assert_eq!(first[0].points_earned, requested);
		assert!(!first[0].discarded_at_stock_cap());

		evaluate_with(&mut app, Duration::from_secs(5))?;
		flush_award(&mut app);
		let overflow = drain_changes(app.world_mut());
		anyhow::ensure!(overflow.len() == 1, "capped occupancy still emits one change");
		assert_eq!(overflow[0].amount, 0.0);
		assert_eq!(overflow[0].points_earned, requested);
		assert!(overflow[0].discarded_at_stock_cap());
		let charge = app
			.world()
			.get::<LubikanLedger>(player)
			.ok_or_else(|| anyhow::anyhow!("ledger"))?
			.equipped()
			.charge()
			.current();
		anyhow::ensure!(charge == requested, "overflow must not raise charge past stock");
		Ok(())
	}

	#[test]
	fn publisher_reads_only_actor_cells_not_lod_neighborhood() -> anyhow::Result<()> {
		let storage = publish_lush_stand_and_far_cells()?;
		let mut app = occupancy_app(storage);
		let _ = spawn_player(&mut app, STAND);
		app.world_mut().spawn((
			lod::LodViewer,
			Transform::from_translation(Vec3::new(350.0, 0.0, 350.0)),
			GlobalTransform::from_translation(Vec3::new(350.0, 0.0, 350.0)),
		));
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let actor = OccupancyIdentity::from_position(STAND);
		let far = OccupancyIdentity::from_position(Vec3::new(350.0, 0.0, 350.0));
		let store = app.world().resource::<CellMetadataStore>();
		anyhow::ensure!(store.get(SemanticCellKey::Forest(actor.forest)).is_some());
		anyhow::ensure!(store.view(&actor).grove_loaded, "actor grove should be projected");
		anyhow::ensure!(
			!store.view(&far).grove_loaded,
			"LodViewer neighborhood must not pull far grove tiles"
		);
		for key in app.world().resource::<WorldOccupancyPublishState>().keys.iter() {
			if let SemanticCellKey::GroveLayer(id) = *key {
				anyhow::ensure!(id.tile == actor.grove, "projection must stay on the actor tile");
			}
		}
		Ok(())
	}

	#[test]
	fn publisher_skips_reread_when_cells_and_membership_are_unchanged() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let _ = spawn_player(&mut app, STAND);
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let revision = app.world().resource::<CellMetadataStore>().revision();
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			app.world().resource::<CellMetadataStore>().revision(),
			revision,
			"unchanged actor cells must not rewrite the projection"
		);
		Ok(())
	}

	#[test]
	fn cell_transition_evicts_the_previous_grove() -> anyhow::Result<()> {
		let storage = publish_lush_stand_and_far_cells()?;
		let mut app = occupancy_app(storage);
		let player = spawn_player(&mut app, STAND);
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let first = OccupancyIdentity::from_position(STAND);
		anyhow::ensure!(app.world().resource::<CellMetadataStore>().view(&first).grove_loaded);
		app.world_mut().entity_mut(player).insert((
			Transform::from_translation(Vec3::new(350.0, 0.0, 350.0)),
			GlobalTransform::from_translation(Vec3::new(350.0, 0.0, 350.0)),
		));
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let next = OccupancyIdentity::from_position(Vec3::new(350.0, 0.0, 350.0));
		let store = app.world().resource::<CellMetadataStore>();
		anyhow::ensure!(store.view(&next).grove_loaded);
		anyhow::ensure!(!store.view(&first).grove_loaded, "left grove must be evicted");
		Ok(())
	}

	#[test]
	fn streaming_gap_unloads_grove_without_duplicate_credit() -> anyhow::Result<()> {
		let full = publish_lush_origin_cells()?;
		let mut app = occupancy_app(full);
		let player = ready_occupancy_player(&mut app, STAND)?;
		evaluate_with(&mut app, Duration::from_secs(4))?;
		app.insert_resource(publish_lush_forest_only()?);
		app.world_mut().resource_mut::<WorldOccupancyPublishState>().forest_revision = None;
		app.world_mut().resource_mut::<WorldOccupancyPublishState>().grove_revision = None;
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(
			!app.world()
				.resource::<CellMetadataStore>()
				.view(&OccupancyIdentity::from_position(STAND))
				.grove_loaded
		);
		evaluate_with(&mut app, Duration::from_secs(4))?;
		flush_award(&mut app);
		anyhow::ensure!(drain_changes(app.world_mut()).is_empty());
		app.insert_resource(publish_lush_origin_cells()?);
		app.world_mut().resource_mut::<WorldOccupancyPublishState>().forest_revision = None;
		app.world_mut().resource_mut::<WorldOccupancyPublishState>().grove_revision = None;
		app.world_mut()
			.run_system_once(publish_world_occupancy_metadata)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		evaluate_with(&mut app, Duration::from_secs(1))?;
		flush_award(&mut app);
		let changed = drain_changes(app.world_mut());
		anyhow::ensure!(changed.len() == 1, "timer must survive streaming eviction");
		assert_eq!(changed[0].recipient, player);
		assert_eq!(changed[0].reason, LubikanChargeReason::OCCUPANCY);
		Ok(())
	}

	#[test]
	fn world_pause_is_applied_before_evaluate() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let _ = ready_occupancy_player(&mut app, STAND)?;
		app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = false;
		gate_before_evaluate(&mut app, Duration::from_secs(5))?;
		anyhow::ensure!(app.world().resource::<OccupancyRewardsPaused>().0);
		flush_award(&mut app);
		anyhow::ensure!(
			drain_changes(app.world_mut()).is_empty(),
			"paused world must not credit occupancy"
		);
		app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = true;
		gate_before_evaluate(&mut app, Duration::from_secs(5))?;
		flush_award(&mut app);
		anyhow::ensure!(
			drain_changes(app.world_mut()).len() == 1,
			"resume must credit one interval, not paused time"
		);
		Ok(())
	}

	#[test]
	fn large_frame_delta_awards_one_credit_per_interval() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let _ = ready_occupancy_player(&mut app, STAND)?;
		evaluate_with(&mut app, Duration::from_millis(12_500))?;
		flush_award(&mut app);
		let changed = drain_changes(app.world_mut());
		anyhow::ensure!(changed.len() == 2, "12.5 s at 5 s should apply two occupancy awards");
		assert!(changed.iter().all(|change| change.reason == LubikanChargeReason::OCCUPANCY));
		Ok(())
	}

	#[test]
	fn retiring_downed_body_does_not_credit() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let player = ready_occupancy_player(&mut app, STAND)?;
		evaluate_with(&mut app, Duration::from_secs(4))?;
		app.world_mut().entity_mut(player).insert(damage::Downed {
			source: None,
			point: STAND,
			at: 0.0,
		});
		app.world_mut()
			.run_system_once(close_occupancy_on_retired_bodies)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		evaluate_with(&mut app, Duration::from_secs(5))?;
		flush_award(&mut app);
		anyhow::ensure!(
			drain_changes(app.world_mut()).is_empty(),
			"a retiring body must not keep occupancy credit"
		);
		Ok(())
	}

	#[test]
	fn body_replace_starts_a_fresh_occupancy_timer() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let previous = ready_occupancy_player(&mut app, STAND)?;
		evaluate_with(&mut app, Duration::from_secs(4))?;
		app.world_mut()
			.get_mut::<OccupancyRewardsUser>(previous)
			.ok_or_else(|| anyhow::anyhow!("previous user"))?
			.close();
		app.world_mut().entity_mut(previous).remove::<(
			VegetationPlayer,
			player::Player,
			lubikans::LubikanOwner,
			OccupancyRewardsUser,
		)>();
		let replacement = ready_occupancy_player(&mut app, STAND)?;
		evaluate_with(&mut app, Duration::from_secs(4))?;
		flush_award(&mut app);
		anyhow::ensure!(
			drain_changes(app.world_mut()).is_empty(),
			"replacement must not inherit the retired body's leftover interval"
		);
		evaluate_with(&mut app, Duration::from_secs(1))?;
		flush_award(&mut app);
		let changed = drain_changes(app.world_mut());
		anyhow::ensure!(changed.len() == 1);
		assert_eq!(changed[0].recipient, replacement);
		assert_eq!(changed[0].reason, LubikanChargeReason::OCCUPANCY);
		Ok(())
	}

	#[test]
	fn ended_session_latches_pause_and_closes_users() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		let player = ready_occupancy_player(&mut app, STAND)?;
		app.world_mut().write_message(WorldPlaySessionEnded);
		app.world_mut()
			.run_system_once(close_occupancy_on_session_end)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		gate_before_evaluate(&mut app, Duration::from_secs(5))?;
		anyhow::ensure!(app.world().resource::<WorldOccupancySessionClosed>().0);
		anyhow::ensure!(app.world().resource::<OccupancyRewardsPaused>().0);
		anyhow::ensure!(
			app.world()
				.get::<OccupancyRewardsUser>(player)
				.is_some_and(|user| user.session.is_closed())
		);
		flush_award(&mut app);
		anyhow::ensure!(drain_changes(app.world_mut()).is_empty());
		Ok(())
	}

	#[test]
	fn demo_config_overrides_self_interval_and_amount() -> anyhow::Result<()> {
		let storage = publish_lush_origin_cells()?;
		let demo = OccupancyDemoConfig::default().with_interval_secs(1.0).with_amount(3.0);
		let mut app = occupancy_app_with_demo(storage, LubikanSelfConfig::default(), demo);
		let _ = ready_occupancy_player(&mut app, STAND)?;
		let rule = app
			.world()
			.resource::<RewardRuleSet>()
			.rule(occupancy_rewards::RuleId::SELF_DEMONSTRATION)
			.ok_or_else(|| anyhow::anyhow!("demo rule"))?;
		assert_eq!(rule.interval, Duration::from_secs(1));
		assert_eq!(rule.amount, 3.0);
		evaluate_with(&mut app, Duration::from_secs(1))?;
		flush_award(&mut app);
		let changed = drain_changes(app.world_mut());
		anyhow::ensure!(changed.len() == 1);
		assert_eq!(changed[0].amount, 3.0);
		assert_eq!(changed[0].points_earned, 3.0);
		Ok(())
	}

	fn occupancy_changes_with_hud(visible: bool) -> anyhow::Result<Vec<LubikanChargeChanged>> {
		let storage = publish_lush_origin_cells()?;
		let mut app = occupancy_app(storage);
		app.add_plugins(combat_hud::CombatHudPlugin {
			hit_markers: false,
			directional_damage: false,
			player_vitals: false,
			lubikan_charge: true,
			lubikan_point_feedback: true,
			score: false,
		});
		app.insert_resource(combat_hud::CombatHudVisible(visible));
		let player = ready_occupancy_player(&mut app, STAND)?;
		evaluate_with(&mut app, Duration::from_secs(5))?;
		flush_award(&mut app);
		let changed = drain_changes(app.world_mut());
		anyhow::ensure!(changed.len() == 1, "HUD {visible} must not change Award");
		assert_eq!(changed[0].recipient, player);
		assert_eq!(changed[0].reason, LubikanChargeReason::OCCUPANCY);
		assert_eq!(changed[0].points_earned, RewardRule::self_demonstration().amount);
		Ok(changed)
	}

	#[test]
	fn occupancy_self_loop_awards_with_hud_enabled_and_disabled() -> anyhow::Result<()> {
		let on = occupancy_changes_with_hud(true)?;
		let off = occupancy_changes_with_hud(false)?;
		assert_eq!(on[0].amount, off[0].amount);
		assert_eq!(on[0].points_earned, off[0].points_earned);
		Ok(())
	}
}
