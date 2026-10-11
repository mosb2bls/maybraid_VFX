//! Generic occupancy reward rules, including the configurable Self demonstration.

use std::time::Duration;

use bevy::prelude::Resource;

use crate::identity::VegetationLayer;
use crate::tag::{AffiliationTag, ConstructionTag, GroveKindTag, LayeringTag};
use lubikans::LubikanAccountId;

/// Stable rule identity. Timers and awards key off this, not insertion order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RuleId(pub u32);

impl RuleId {
	/// Configurable Self demonstration shipped with this crate.
	pub const SELF_DEMONSTRATION: Self = Self(1);
}

/// How overlapping qualifying constructions combine on one occupancy key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OverlapPolicy {
	/// One award per occupancy key even if several constructions qualify.
	#[default]
	Dedup,
	/// One timer per matching construction tag.
	Stack,
}

/// What happens when the actor leaves a match key and later returns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ReentryPolicy {
	/// Fresh interval after leaving the geometric cell.
	#[default]
	Reset,
	/// Keep leftover progress for the same `(rule, cell[, tag])` key.
	Persist,
}

/// Which semantic footprint a construction predicate uses.
///
/// This is the [#1111](https://github.com/ramate-io/maybraid/issues/1111) open
/// decision: "forest cell with a construction" is not inferred. Each rule
/// names the footprint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ConstructionScope {
	/// Any published construction in the actor's 1600 m forest cell.
	ForestCell,
	/// Construction in the actor's 100 m grove tile.
	#[default]
	GroveTile,
	/// Construction in the planting sub-cell that owns the actor.
	PlantingCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagFilter<T> {
	Any,
	AnyOf(Vec<T>),
	AllOf(Vec<T>),
}

impl<T: Copy + PartialEq> TagFilter<T> {
	pub fn matches(&self, present: &[T]) -> bool {
		match self {
			Self::Any => !present.is_empty(),
			Self::AnyOf(wanted) => wanted.iter().any(|tag| present.contains(tag)),
			Self::AllOf(wanted) => wanted.iter().all(|tag| present.contains(tag)),
		}
	}

	pub fn matching(&self, present: &[T]) -> Vec<T> {
		match self {
			Self::Any => present.to_vec(),
			Self::AnyOf(wanted) => {
				wanted.iter().copied().filter(|tag| present.contains(tag)).collect()
			}
			Self::AllOf(wanted) if self.matches(present) => wanted.clone(),
			Self::AllOf(_) => Vec::new(),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerFilter {
	Any,
	Only(VegetationLayer),
	AnyOf(Vec<VegetationLayer>),
}

impl LayerFilter {
	pub fn allows(&self, layer: VegetationLayer) -> bool {
		match self {
			Self::Any => true,
			Self::Only(wanted) => *wanted == layer,
			Self::AnyOf(wanted) => wanted.contains(&layer),
		}
	}
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForestMatch {
	pub layering: Option<TagFilter<LayeringTag>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroveMatch {
	pub layers: Option<LayerFilter>,
	pub kinds: Option<TagFilter<GroveKindTag>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstructionMatch {
	pub tags: TagFilter<ConstructionTag>,
	pub scope: ConstructionScope,
}

impl Default for ConstructionMatch {
	fn default() -> Self {
		Self { tags: TagFilter::Any, scope: ConstructionScope::GroveTile }
	}
}

/// Predicate a rule evaluates against an [`crate::OccupancyView`].
///
/// Empty filters mean "any resolved occupancy". Filters that need published
/// facts fail closed while that metadata is unloaded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OccupancyMatch {
	pub affiliation: Option<TagFilter<AffiliationTag>>,
	pub forest: Option<ForestMatch>,
	pub grove: Option<GroveMatch>,
	pub construction: Option<ConstructionMatch>,
}

impl OccupancyMatch {
	pub fn needs_metadata(&self) -> bool {
		self.affiliation.is_some()
			|| self.forest.is_some()
			|| self.grove.is_some()
			|| self.construction.is_some()
	}
}

/// One occupancy reward rule.
#[derive(Clone, Debug, PartialEq)]
pub struct RewardRule {
	pub id: RuleId,
	pub interval: Duration,
	pub amount: f32,
	pub credited: LubikanAccountId,
	pub matcher: OccupancyMatch,
	pub overlap: OverlapPolicy,
	pub reentry: ReentryPolicy,
}

impl RewardRule {
	/// Self demonstration: +1 charge every 5 s in a grove tile that has a
	/// construction. No Bikanwarda content required.
	pub fn self_demonstration() -> Self {
		Self {
			id: RuleId::SELF_DEMONSTRATION,
			interval: Duration::from_secs(5),
			amount: 1.0,
			credited: LubikanAccountId::SELF,
			matcher: OccupancyMatch {
				construction: Some(ConstructionMatch::default()),
				..OccupancyMatch::default()
			},
			overlap: OverlapPolicy::Dedup,
			reentry: ReentryPolicy::Reset,
		}
	}

	/// Award while the actor occupies any resolved cell. Useful in tests that
	/// do not publish construction metadata.
	pub fn self_always(interval: Duration, amount: f32) -> Self {
		Self {
			id: RuleId(2),
			interval,
			amount,
			credited: LubikanAccountId::SELF,
			matcher: OccupancyMatch::default(),
			overlap: OverlapPolicy::Dedup,
			reentry: ReentryPolicy::Reset,
		}
	}

	pub fn with_id(mut self, id: RuleId) -> Self {
		self.id = id;
		self
	}

	pub fn with_interval(mut self, interval: Duration) -> Self {
		self.interval = interval;
		self
	}

	pub fn with_amount(mut self, amount: f32) -> Self {
		self.amount = amount;
		self
	}

	pub fn with_overlap(mut self, overlap: OverlapPolicy) -> Self {
		self.overlap = overlap;
		self
	}

	pub fn with_reentry(mut self, reentry: ReentryPolicy) -> Self {
		self.reentry = reentry;
		self
	}

	pub fn with_match(mut self, matcher: OccupancyMatch) -> Self {
		self.matcher = matcher;
		self
	}

	pub fn interval_us(&self) -> u128 {
		self.interval.as_micros().max(1)
	}
}

/// Configured occupancy rules. Applications own cadence; this set is data.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct RewardRuleSet {
	rules: Vec<RewardRule>,
}

impl RewardRuleSet {
	pub fn new(rules: impl IntoIterator<Item = RewardRule>) -> Self {
		Self { rules: rules.into_iter().collect() }
	}

	pub fn demonstration() -> Self {
		Self::new([RewardRule::self_demonstration()])
	}

	pub fn rules(&self) -> &[RewardRule] {
		&self.rules
	}

	pub fn push(&mut self, rule: RewardRule) {
		self.rules.push(rule);
	}

	pub fn rule(&self, id: RuleId) -> Option<&RewardRule> {
		self.rules.iter().find(|rule| rule.id == id)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn self_demonstration_uses_grove_tile_construction_scope() -> anyhow::Result<()> {
		let rule = RewardRule::self_demonstration();
		assert_eq!(rule.credited, LubikanAccountId::SELF);
		let construction = rule
			.matcher
			.construction
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("demo rule should match constructions"))?;
		assert_eq!(construction.scope, ConstructionScope::GroveTile);
		assert_eq!(rule.overlap, OverlapPolicy::Dedup);
		Ok(())
	}

	#[test]
	fn tag_filter_any_requires_at_least_one() -> anyhow::Result<()> {
		assert!(!TagFilter::<ConstructionTag>::Any.matches(&[]));
		assert!(TagFilter::Any.matches(&[ConstructionTag::SOPES_BANYAN]));
		Ok(())
	}
}
