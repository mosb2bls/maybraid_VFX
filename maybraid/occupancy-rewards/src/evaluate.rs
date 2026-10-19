//! Pure occupancy step: identity sync, rule match, timer tick, awards.

use std::time::Duration;

use bevy::prelude::Entity;

use crate::identity::{GroveLayerId, OccupancyIdentity, OccupancyMatchKey};
use crate::metadata::{CellMetadataStore, OccupancyView};
use crate::occupancy::{OccupancyAward, OccupancyEvent, OccupancySession};
use crate::rule::{
	ConstructionScope, OccupancyMatch, OverlapPolicy, ReentryPolicy, RewardRule, RewardRuleSet,
};
use crate::tag::ConstructionTag;
use crate::timer::TimerKey;

/// Enter/exit plus awards produced by one evaluation step.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OccupancyStep {
	pub events: Vec<OccupancyEvent>,
	pub awards: Vec<OccupancyAward>,
}

impl OccupancyStep {
	pub fn award_count(&self) -> usize {
		self.awards.len()
	}

	pub fn award_total(&self) -> f32 {
		self.awards.iter().map(|award| award.amount).sum()
	}
}

#[derive(Clone, Debug)]
struct RuleHit {
	cell: OccupancyMatchKey,
	tags: Vec<ConstructionTag>,
}

/// Advance one actor: occupancy changes, then matching rule timers.
///
/// Timers tick only while the rule currently matches and evaluation is not
/// paused. They are dropped only when the actor leaves the geometric cell, so
/// metadata eviction / visual streaming cannot reset an interval.
pub fn step_occupancy(
	actor: Entity,
	session: &mut OccupancySession,
	identity: OccupancyIdentity,
	store: &CellMetadataStore,
	rules: &RewardRuleSet,
	dt: Duration,
	paused: bool,
) -> OccupancyStep {
	if session.is_closed() {
		return OccupancyStep::default();
	}
	let events = session.sync_identity(actor, identity);
	session.drop_unoccupied(&identity, |key| {
		rules.rule(key.rule).map(|rule| rule.reentry).unwrap_or(ReentryPolicy::Reset)
	});
	let view = store.view(&identity);
	let mut awards = Vec::new();
	if paused {
		return OccupancyStep { events, awards };
	}
	for rule in rules.rules() {
		let Some(hit) = match_rule(rule, &identity, &view) else {
			continue;
		};
		for key in timer_keys(rule, &hit) {
			let timer = session.bind_timer(key, rule.reentry);
			let ticks = timer.tick(dt, rule.interval);
			for _ in 0..ticks {
				awards.push(OccupancyAward {
					recipient: actor,
					rule: rule.id,
					cell: key.cell,
					amount: rule.amount,
					origin: identity.position,
				});
			}
		}
	}
	OccupancyStep { events, awards }
}

fn match_rule(
	rule: &RewardRule,
	identity: &OccupancyIdentity,
	view: &OccupancyView,
) -> Option<RuleHit> {
	if !match_filters(&rule.matcher, view) {
		return None;
	}
	let cell = match_key(&rule.matcher, identity, view)?;
	let tags = rule
		.matcher
		.construction
		.as_ref()
		.map(|construction| construction.tags.matching(view.constructions_at(construction.scope)))
		.unwrap_or_default();
	Some(RuleHit { cell, tags })
}

fn match_filters(matcher: &OccupancyMatch, view: &OccupancyView) -> bool {
	if let Some(affiliation) = &matcher.affiliation {
		let present = view.affiliation.as_slice();
		if view.affiliation.is_none() || !affiliation.matches(present) {
			return false;
		}
	}
	if let Some(forest) = &matcher.forest {
		if !view.forest_loaded {
			return false;
		}
		if let Some(layering) = &forest.layering {
			let present = view.forest.and_then(|info| info.layering);
			if !layering.matches(present.as_slice()) {
				return false;
			}
		}
	}
	if let Some(grove) = &matcher.grove {
		if !view.grove_loaded {
			return false;
		}
		let kinds: Vec<_> = view
			.layers
			.iter()
			.filter_map(|layer| {
				let info = (*layer)?;
				if grove.layers.as_ref().is_none_or(|filter| filter.allows(info.layer)) {
					info.kind
				} else {
					None
				}
			})
			.collect();
		if let Some(filter) = &grove.kinds {
			if !filter.matches(&kinds) {
				return false;
			}
		} else if grove.layers.as_ref().is_some()
			&& !view
				.layers
				.iter()
				.flatten()
				.any(|info| grove.layers.as_ref().is_some_and(|filter| filter.allows(info.layer)))
		{
			return false;
		}
	}
	if let Some(construction) = &matcher.construction {
		if !view.scope_loaded(construction.scope) {
			return false;
		}
		if !construction.tags.matches(view.constructions_at(construction.scope)) {
			return false;
		}
	}
	true
}

fn match_key(
	matcher: &OccupancyMatch,
	identity: &OccupancyIdentity,
	view: &OccupancyView,
) -> Option<OccupancyMatchKey> {
	if let Some(construction) = &matcher.construction {
		return match construction.scope {
			ConstructionScope::ForestCell => Some(OccupancyMatchKey::Forest(identity.forest)),
			ConstructionScope::GroveTile => grove_or_layer_key(matcher, identity, view),
			ConstructionScope::PlantingCell => identity.planting.map(OccupancyMatchKey::Planting),
		};
	}
	if matcher.grove.is_some() {
		return grove_or_layer_key(matcher, identity, view);
	}
	if matcher.forest.is_some() || matcher.affiliation.is_some() {
		return Some(OccupancyMatchKey::Forest(identity.forest));
	}
	Some(OccupancyMatchKey::Grove(identity.grove))
}

fn grove_or_layer_key(
	matcher: &OccupancyMatch,
	identity: &OccupancyIdentity,
	view: &OccupancyView,
) -> Option<OccupancyMatchKey> {
	if let Some(crate::rule::LayerFilter::Only(layer)) =
		matcher.grove.as_ref().and_then(|grove| grove.layers.as_ref())
	{
		if view.layer(*layer).is_some() {
			return Some(OccupancyMatchKey::GroveLayer(GroveLayerId {
				tile: identity.grove,
				layer: *layer,
			}));
		}
		return None;
	}
	Some(OccupancyMatchKey::Grove(identity.grove))
}

fn timer_keys(rule: &RewardRule, hit: &RuleHit) -> Vec<TimerKey> {
	match rule.overlap {
		OverlapPolicy::Dedup => vec![TimerKey::new(rule.id, hit.cell, None)],
		OverlapPolicy::Stack => {
			if hit.tags.is_empty() {
				vec![TimerKey::new(rule.id, hit.cell, None)]
			} else {
				hit.tags
					.iter()
					.map(|tag| TimerKey::new(rule.id, hit.cell, Some(*tag)))
					.collect()
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{Entity, Vec3};

	use super::*;
	use crate::identity::{
		ForestCellId, GroveTileId, OccupancyIdentity, OccupiedRegion, SemanticCellKey,
		VegetationLayer,
	};
	use crate::metadata::{CellMetadata, GroveInfo};
	use crate::occupancy::OccupancyEvent;
	use crate::rule::{
		ConstructionMatch, ConstructionScope, ForestMatch, GroveMatch, LayerFilter, OccupancyMatch,
		OverlapPolicy, RewardRule, TagFilter,
	};
	use crate::tag::{ConstructionTag, GroveKindTag, LayeringTag};

	fn actor() -> Entity {
		Entity::from_bits(11)
	}

	fn grove_construction(position: Vec3, tags: &[ConstructionTag]) -> CellMetadataStore {
		let mut store = CellMetadataStore::default();
		let identity = OccupancyIdentity::from_position(position);
		store.publish(
			CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId {
				tile: identity.grove,
				layer: VegetationLayer::UpperCanopy,
			}))
			.with_grove(GroveInfo {
				layer: VegetationLayer::UpperCanopy,
				kind: Some(GroveKindTag::named("TradeWinds")),
			})
			.with_constructions(tags.iter().copied()),
		);
		store
	}

	fn step(
		session: &mut OccupancySession,
		position: Vec3,
		store: &CellMetadataStore,
		rules: &RewardRuleSet,
		dt: Duration,
	) -> OccupancyStep {
		step_occupancy(
			actor(),
			session,
			OccupancyIdentity::resolve(position, store),
			store,
			rules,
			dt,
			false,
		)
	}

	#[test]
	fn configured_self_rule_awards_on_interval() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		let first = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(5));
		assert_eq!(first.award_count(), 1);
		assert_eq!(first.awards[0].amount, 1.0);
		assert_eq!(first.awards[0].cell, OccupancyMatchKey::Grove(GroveTileId { ix: 8, iz: 8 }));
		Ok(())
	}

	#[test]
	fn leaving_a_qualifying_region_stops_awards() -> anyhow::Result<()> {
		let inside = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let outside = CellMetadataStore::default();
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &inside, &rules, Duration::from_secs(5));
		let left = step(
			&mut session,
			Vec3::new(150.0, 0.0, 0.0),
			&outside,
			&rules,
			Duration::from_secs(5),
		);
		assert_eq!(left.award_count(), 0);
		assert!(left.events.iter().any(|event| matches!(
			event,
			OccupancyEvent::Exit { region: OccupiedRegion::Grove(_), .. }
		)));
		Ok(())
	}

	#[test]
	fn reentry_resets_interval_by_default() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let empty = CellMetadataStore::default();
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(4));
		step(&mut session, Vec3::new(150.0, 0.0, 0.0), &empty, &rules, Duration::from_secs(4));
		let back = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(4));
		assert_eq!(back.award_count(), 0);
		let later = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(1));
		assert_eq!(later.award_count(), 1);
		Ok(())
	}

	#[test]
	fn persist_reentry_keeps_progress() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let empty = CellMetadataStore::default();
		let rules = RewardRuleSet::new([
			RewardRule::self_demonstration().with_reentry(ReentryPolicy::Persist)
		]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(4));
		step(&mut session, Vec3::new(150.0, 0.0, 0.0), &empty, &rules, Duration::from_secs(1));
		let back = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(1));
		assert_eq!(back.award_count(), 1);
		Ok(())
	}

	#[test]
	fn overlapping_constructions_do_not_multiply_unless_stacked() -> anyhow::Result<()> {
		let tags = [ConstructionTag::SOPES_BANYAN, ConstructionTag::named("other")];
		let store = grove_construction(Vec3::ZERO, &tags);
		let dedup = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &dedup, Duration::from_secs(5)).award_count(),
			1
		);
		let stack = RewardRuleSet::new([
			RewardRule::self_demonstration().with_overlap(OverlapPolicy::Stack)
		]);
		let mut stacked = OccupancySession::default();
		assert_eq!(
			step(&mut stacked, Vec3::ZERO, &store, &stack, Duration::from_secs(5)).award_count(),
			2
		);
		Ok(())
	}

	#[test]
	fn overlapping_layers_share_one_grove_tile_reward() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		let identity = OccupancyIdentity::from_position(Vec3::ZERO);
		for layer in [VegetationLayer::UpperCanopy, VegetationLayer::LowerCanopy] {
			store.publish(
				CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId {
					tile: identity.grove,
					layer,
				}))
				.with_grove(GroveInfo { layer, kind: Some(GroveKindTag::named("TradeWinds")) })
				.with_construction(ConstructionTag::SOPES_BANYAN),
			);
		}
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(5)).award_count(),
			1
		);
		Ok(())
	}

	#[test]
	fn construction_scope_forest_vs_grove_is_explicit() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		let here = OccupancyIdentity::from_position(Vec3::ZERO);
		let other_grove = GroveTileId { ix: here.grove.ix + 2, iz: here.grove.iz };
		store.publish(
			CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId {
				tile: other_grove,
				layer: VegetationLayer::UpperCanopy,
			}))
			.with_construction(ConstructionTag::SOPES_BANYAN),
		);
		store.publish(CellMetadata::new(SemanticCellKey::Forest(here.forest)));
		let grove_rule = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let forest_rule =
			RewardRuleSet::new([RewardRule::self_demonstration().with_match(OccupancyMatch {
				construction: Some(ConstructionMatch {
					tags: TagFilter::Any,
					scope: ConstructionScope::ForestCell,
				}),
				..OccupancyMatch::default()
			})]);
		let mut grove_session = OccupancySession::default();
		let mut forest_session = OccupancySession::default();
		assert_eq!(
			step(&mut grove_session, Vec3::ZERO, &store, &grove_rule, Duration::from_secs(5))
				.award_count(),
			0
		);
		assert_eq!(
			step(&mut forest_session, Vec3::ZERO, &store, &forest_rule, Duration::from_secs(5))
				.award_count(),
			1
		);
		Ok(())
	}

	#[test]
	fn absent_metadata_does_not_award() -> anyhow::Result<()> {
		let store = CellMetadataStore::default();
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(5)).award_count(),
			0
		);
		Ok(())
	}

	#[test]
	fn metadata_eviction_pauses_awards_without_resetting_the_timer() -> anyhow::Result<()> {
		let present = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &present, &rules, Duration::from_secs(4));
		let empty = CellMetadataStore::default();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &empty, &rules, Duration::from_secs(4)).award_count(),
			0
		);
		assert_eq!(
			step(&mut session, Vec3::ZERO, &present, &rules, Duration::from_secs(1)).award_count(),
			1
		);
		Ok(())
	}

	#[test]
	fn republished_equal_metadata_does_not_duplicate_awards() -> anyhow::Result<()> {
		let mut store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(5));
		let identity = OccupancyIdentity::from_position(Vec3::ZERO);
		assert_eq!(
			store.publish(
				CellMetadata::new(SemanticCellKey::GroveLayer(GroveLayerId {
					tile: identity.grove,
					layer: VegetationLayer::UpperCanopy,
				}))
				.with_grove(GroveInfo {
					layer: VegetationLayer::UpperCanopy,
					kind: Some(GroveKindTag::named("TradeWinds")),
				})
				.with_construction(ConstructionTag::SOPES_BANYAN)
			),
			crate::PublishOutcome::Unchanged
		);
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(5)).award_count(),
			1
		);
		Ok(())
	}

	#[test]
	fn large_time_step_emits_one_award_per_elapsed_interval() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		let stepped = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_millis(12_500));
		assert_eq!(stepped.award_count(), 2);
		assert_eq!(stepped.award_total(), 2.0);
		Ok(())
	}

	#[test]
	fn closed_session_never_awards() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		session.close();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(5)).award_count(),
			0
		);
		assert!(session.is_closed());
		Ok(())
	}

	#[test]
	fn pause_does_not_bank_time_for_resume() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(4));
		let paused = step_occupancy(
			actor(),
			&mut session,
			OccupancyIdentity::resolve(Vec3::ZERO, &store),
			&store,
			&rules,
			Duration::from_secs(10),
			true,
		);
		assert_eq!(paused.award_count(), 0);
		let resumed = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(1));
		assert_eq!(resumed.award_count(), 1);
		Ok(())
	}

	#[test]
	fn pause_freezes_timers_but_still_tracks_exit() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_demonstration()]);
		let mut session = OccupancySession::default();
		step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(4));
		let paused = step_occupancy(
			actor(),
			&mut session,
			OccupancyIdentity::resolve(Vec3::new(150.0, 0.0, 0.0), &store),
			&store,
			&rules,
			Duration::from_secs(5),
			true,
		);
		assert_eq!(paused.award_count(), 0);
		assert!(paused.events.iter().any(|event| matches!(event, OccupancyEvent::Exit { .. })));
		Ok(())
	}

	#[test]
	fn always_rule_awards_without_metadata() -> anyhow::Result<()> {
		let store = CellMetadataStore::default();
		let rules = RewardRuleSet::new([RewardRule::self_always(Duration::from_secs(1), 4.0)]);
		let mut session = OccupancySession::default();
		let stepped = step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(1));
		assert_eq!(stepped.award_count(), 1);
		assert_eq!(stepped.awards[0].amount, 4.0);
		Ok(())
	}

	#[test]
	fn grove_kind_filter_requires_loaded_matching_kind() -> anyhow::Result<()> {
		let store = grove_construction(Vec3::ZERO, &[ConstructionTag::SOPES_BANYAN]);
		let rules = RewardRuleSet::new([RewardRule::self_always(Duration::from_secs(1), 1.0)
			.with_match(OccupancyMatch {
				grove: Some(GroveMatch {
					layers: Some(LayerFilter::Only(VegetationLayer::UpperCanopy)),
					kinds: Some(TagFilter::AnyOf(vec![GroveKindTag::named("TradeWinds")])),
				}),
				..OccupancyMatch::default()
			})]);
		let mut session = OccupancySession::default();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(1)).award_count(),
			1
		);
		let miss = RewardRuleSet::new([RewardRule::self_always(Duration::from_secs(1), 1.0)
			.with_match(OccupancyMatch {
				grove: Some(GroveMatch {
					layers: None,
					kinds: Some(TagFilter::AnyOf(vec![GroveKindTag::named("Orchard")])),
				}),
				..OccupancyMatch::default()
			})]);
		let mut other = OccupancySession::default();
		assert_eq!(
			step(&mut other, Vec3::ZERO, &store, &miss, Duration::from_secs(1)).award_count(),
			0
		);
		Ok(())
	}

	#[test]
	fn forest_layering_filter_uses_forest_metadata() -> anyhow::Result<()> {
		let mut store = CellMetadataStore::default();
		store.publish(
			CellMetadata::new(SemanticCellKey::Forest(ForestCellId { ix: 0, iz: 0 })).with_forest(
				crate::ForestInfo { layering: Some(LayeringTag::named("LushJungle")) },
			),
		);
		let rules = RewardRuleSet::new([RewardRule::self_always(Duration::from_secs(1), 1.0)
			.with_match(OccupancyMatch {
				forest: Some(ForestMatch {
					layering: Some(TagFilter::AnyOf(vec![LayeringTag::named("LushJungle")])),
				}),
				..OccupancyMatch::default()
			})]);
		let mut session = OccupancySession::default();
		assert_eq!(
			step(&mut session, Vec3::ZERO, &store, &rules, Duration::from_secs(1)).award_count(),
			1
		);
		Ok(())
	}
}
