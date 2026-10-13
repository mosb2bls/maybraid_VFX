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

use std::collections::BTreeSet;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico::{ChicoForest, ChicoGrove, ForestGroveKind, ForestLayer, GROVE_PRESENT_RADIUS_M};
use lod::hcsg::{Busy, HcsgStorage};
use lod::LodViewer;
use lubikans::LubikansPlugin;
use occupancy_rewards::{
	CellMetadata, CellMetadataStore, ConstructionTag, ForestInfo, GroveInfo, GroveKindTag,
	GroveLayerId, GroveTileId, LayeringTag, OccupancyRewardsPlugin, OccupancyRewardsUser,
	SemanticCellKey, VegetationLayer,
};
use world_player::Player as VegetationPlayer;

/// Occupancy metadata publish and player mark.
///
/// Runs in Update: after [`lubikans::LubikanSystems::Owner`] (PreUpdate) and
/// before occupancy evaluate / [`lubikans::LubikanSystems::Award`] (PostUpdate).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldOccupancySystems {
	Publish,
	Attach,
}

/// Registers occupancy in the ordinary world and fills the metadata store
/// from published HCSG cell facts.
pub struct WorldOccupancyPlugin;

impl Plugin for WorldOccupancyPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikansPlugin>() {
			app.add_plugins(LubikansPlugin);
		}
		if !app.is_plugin_added::<OccupancyRewardsPlugin>() {
			app.add_plugins(OccupancyRewardsPlugin::default());
		}
		app.init_resource::<WorldOccupancyPublishState>()
			.configure_sets(
				Update,
				WorldOccupancySystems::Attach.after(WorldOccupancySystems::Publish),
			)
			.add_systems(
				Update,
				publish_world_occupancy_metadata.in_set(WorldOccupancySystems::Publish),
			)
			.add_systems(
				Update,
				attach_world_occupancy_player.in_set(WorldOccupancySystems::Attach),
			);
	}
}

/// Keys published from the last successful HCSG scan.
#[derive(Resource, Default)]
pub(crate) struct WorldOccupancyPublishState {
	keys: BTreeSet<SemanticCellKey>,
}

type OccupancyViewers<'w, 's> = Query<
	'w,
	's,
	&'static GlobalTransform,
	Or<(With<VegetationPlayer>, With<OccupancyRewardsUser>, With<LodViewer>)>,
>;

type OccupancyPlayers<'w, 's> =
	Query<'w, 's, (Entity, Option<&'static OccupancyRewardsUser>), With<VegetationPlayer>>;

/// Copy selected forest / grove facts into [`CellMetadataStore`].
///
/// [`Busy`] means try again next frame; the store is left unchanged so a
/// contended lock cannot evict live occupancy.
pub fn publish_world_occupancy_metadata(
	storage: Option<Res<HcsgStorage>>,
	mut store: ResMut<CellMetadataStore>,
	mut published: ResMut<WorldOccupancyPublishState>,
	viewers: OccupancyViewers,
) {
	let Some(storage) = storage else {
		return;
	};
	let Some(region) = occupancy_scan_region(&viewers) else {
		return;
	};
	let Ok(scan) = OccupancyHcsgScan::try_collect(storage.as_ref(), region) else {
		return;
	};
	scan.publish(&mut store, &mut published);
}

/// Mark the world player for occupancy. Does not create a ledger.
pub fn attach_world_occupancy_player(mut commands: Commands, players: OccupancyPlayers) {
	for (entity, user) in &players {
		if user.is_none() {
			commands.entity(entity).insert(OccupancyRewardsUser::default());
		}
	}
}

struct OccupancyHcsgScan {
	forests: Vec<CellMetadata>,
	groves: Vec<CellMetadata>,
}

impl OccupancyHcsgScan {
	fn try_collect(storage: &HcsgStorage, region: Aabb3d) -> Result<Self, Busy> {
		Ok(Self {
			forests: collect_published::<ChicoForest, _>(storage, region, |forest| {
				OccupancyForest(forest).metadata()
			})?,
			groves: collect_published::<ChicoGrove, _>(storage, region, |grove| {
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

fn collect_published<T, F>(
	storage: &HcsgStorage,
	region: Aabb3d,
	into_meta: F,
) -> Result<Vec<CellMetadata>, Busy>
where
	T: Send + Sync + 'static,
	F: Fn(&T) -> CellMetadata,
{
	let mut metas = Vec::new();
	for id in storage.try_overlapping::<T>(region)? {
		let Some(entry) = storage.try_entry::<T>(id)? else {
			continue;
		};
		metas.push(into_meta(entry.value.as_ref()));
	}
	Ok(metas)
}

fn occupancy_scan_region(viewers: &OccupancyViewers) -> Option<Aabb3d> {
	let mut region = None;
	for transform in viewers {
		let next = scan_region_around(transform.translation());
		region = Some(match region {
			None => next,
			Some(current) => union_aabb(current, next),
		});
	}
	region
}

fn scan_region_around(position: Vec3) -> Aabb3d {
	let radius = GROVE_PRESENT_RADIUS_M;
	Aabb3d::from_min_max(
		Vec3::new(position.x - radius, -1.0, position.z - radius),
		Vec3::new(position.x + radius, 5.0, position.z + radius),
	)
}

fn union_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max))
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
		let mut app = App::new();
		app.insert_resource(self_config)
			.add_plugins(MinimalPlugins)
			.add_plugins(WorldOccupancyPlugin)
			.insert_resource(storage);
		app
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
}
