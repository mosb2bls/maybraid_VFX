//! Semantic cell occupancy, periodic timers, and occupancy reward rules.
//!
//! [#1111](https://github.com/ramate-io/maybraid/issues/1111) is the occupancy
//! lane. Charge identity comes from
//! [`lubikans`](https://github.com/ramate-io/maybraid/pull/1142)
//! ([#1107](https://github.com/ramate-io/maybraid/issues/1107)). This crate
//! emits [`lubikans::LubikanChargeRequest`] with
//! [`lubikans::LubikanChargeReason::OCCUPANCY`];
//! [`lubikans::LubikanSystems::Award`]
//! ([#1108](https://github.com/ramate-io/maybraid/issues/1108) /
//! [PR #1154](https://github.com/ramate-io/maybraid/pull/1154)) applies each
//! request once and emits one [`lubikans::LubikanChargeChanged`].

mod charge;
mod evaluate;
mod identity;
mod metadata;
mod occupancy;
mod plugin;
mod rule;
mod tag;
mod timer;

pub use charge::adapt_award;
pub use evaluate::{step_occupancy, OccupancyStep};
pub use identity::{
	ForestCellId, GroveLayerId, GroveTileId, OccupancyIdentity, OccupancyMatchKey, OccupiedRegion,
	PlantingCellId, SemanticCellKey, UrbanCellId, VegetationLayer, FOREST_EXTENT_XZ, GROVE_ORIGIN,
	GROVE_TILE_XZ,
};
pub use metadata::{
	CellMetadata, CellMetadataStore, ForestInfo, GroveInfo, OccupancyView, PublishOutcome,
};
pub use occupancy::{OccupancyAward, OccupancyEvent, OccupancySession};
pub use plugin::{
	evaluate_occupancy_rewards, OccupancyRewardsPaused, OccupancyRewardsPlugin,
	OccupancyRewardsSystems, OccupancyRewardsUser,
};
pub use rule::{
	ConstructionMatch, ConstructionScope, ForestMatch, GroveMatch, LayerFilter, OccupancyDemoConfig,
	OccupancyMatch, OverlapPolicy, ReentryPolicy, RewardRule, RewardRuleSet, RuleId, TagFilter,
};
pub use tag::{AffiliationTag, ConstructionTag, GroveKindTag, LayeringTag};
pub use timer::{OccupancyTimer, TimerKey};
