use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{SpotDirective, SpottedContact};

/// Independent owner of one semantic spotting hint contribution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpottingHintSource(u8);

impl SpottingHintSource {
	pub const EXPLICIT: Self = Self(0);
	pub const THREAT: Self = Self(1);
}

/// Source-owned semantic reasons to consider one subject during discovery.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpottingHint {
	priorities: BTreeMap<SpottingHintSource, i32>,
}

impl SpottingHint {
	pub fn new(priority: i32) -> Self {
		let mut hint = Self::default();
		hint.set(SpottingHintSource::EXPLICIT, priority);
		hint
	}

	pub fn priority(&self) -> i32 {
		self.priorities.values().copied().max().unwrap_or(0)
	}

	pub fn has_source(&self, source: SpottingHintSource) -> bool {
		self.priorities.contains_key(&source)
	}

	pub fn priority_for(&self, source: SpottingHintSource) -> Option<i32> {
		self.priorities.get(&source).copied()
	}

	pub fn set(&mut self, source: SpottingHintSource, priority: i32) {
		self.priorities.insert(source, priority);
	}

	pub fn remove(&mut self, source: SpottingHintSource) {
		self.priorities.remove(&source);
	}

	pub fn is_empty(&self) -> bool {
		self.priorities.is_empty()
	}
}

/// Per-user caps and contact retention time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpottingSettings {
	pub candidate_budget: usize,
	pub vision_samples: usize,
	pub memory_secs: f32,
}

impl SpottingSettings {
	pub fn new(candidate_budget: usize, vision_samples: usize, memory_secs: f32) -> Self {
		Self { candidate_budget, vision_samples, memory_secs: memory_secs.max(0.0) }
	}
}

impl Default for SpottingSettings {
	fn default() -> Self {
		Self { candidate_budget: 32, vision_samples: 48, memory_secs: 5.0 }
	}
}

/// Observation policy and remembered contacts for one entity.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SpottingUser {
	/// Eye position relative to the user's local transform.
	pub eye_offset: Vec3,
	pub directives: Vec<SpotDirective>,
	pub hints: BTreeMap<Entity, SpottingHint>,
	pub contacts: BTreeMap<Entity, SpottedContact>,
	pub settings: SpottingSettings,
	/// Earliest elapsed time at which broadphase discovery may run again.
	pub next_discovery_at: f32,
	sample_cursor: usize,
}

impl SpottingUser {
	pub fn new(eye_offset: Vec3, directives: impl IntoIterator<Item = SpotDirective>) -> Self {
		Self { eye_offset, directives: directives.into_iter().collect(), ..Self::default() }
	}

	pub fn with_settings(mut self, settings: SpottingSettings) -> Self {
		self.settings = settings;
		self
	}

	pub fn hint(&mut self, subject: Entity, hint: SpottingHint) {
		let priority = hint.priority();
		self.hints
			.entry(subject)
			.or_default()
			.set(SpottingHintSource::EXPLICIT, priority);
	}

	pub fn has_hint_from(&self, subject: Entity, source: SpottingHintSource, priority: i32) -> bool {
		self.hints.get(&subject).is_some_and(|hint| hint.priority_for(source) == Some(priority))
	}

	pub fn hint_from(&mut self, subject: Entity, source: SpottingHintSource, priority: i32) {
		if self.has_hint_from(subject, source, priority) {
			return;
		}
		self.hints.entry(subject).or_default().set(source, priority);
	}

	pub fn remove_hint(&mut self, subject: Entity) {
		self.hints.remove(&subject);
	}

	pub fn remove_hint_source(&mut self, subject: Entity, source: SpottingHintSource) {
		let Some(hint) = self.hints.get_mut(&subject) else {
			return;
		};
		hint.remove(source);
		if hint.is_empty() {
			self.hints.remove(&subject);
		}
	}

	pub fn forget_stale(&mut self, now: f32) {
		let memory_secs = self.settings.memory_secs;
		self.contacts.retain(|_, contact| !contact.should_forget(now, memory_secs));
	}

	/// Advance the feature offset used by the next discovery pass.
	pub fn advance_sample_cursor(&mut self) -> usize {
		let current = self.sample_cursor;
		self.sample_cursor = self.sample_cursor.wrapping_add(1);
		current
	}
}

impl Default for SpottingUser {
	fn default() -> Self {
		Self {
			eye_offset: Vec3::Y * 1.6,
			directives: Vec::new(),
			hints: BTreeMap::new(),
			contacts: BTreeMap::new(),
			settings: SpottingSettings::default(),
			next_discovery_at: 0.0,
			sample_cursor: 0,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn user_forgets_only_contacts_beyond_memory() -> anyhow::Result<()> {
		let keep = Entity::from_bits(1);
		let forget = Entity::from_bits(2);
		let mut user = SpottingUser::default().with_settings(SpottingSettings::new(4, 8, 2.0));
		user.contacts.insert(
			keep,
			SpottedContact::new(keep, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 1.0, 0.1),
		);
		user.contacts.insert(
			forget,
			SpottedContact::new(forget, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 0.9, 0.1),
		);
		user.forget_stale(3.0);
		assert!(user.contacts.contains_key(&keep));
		assert!(!user.contacts.contains_key(&forget));
		Ok(())
	}

	#[test]
	fn discovery_sample_cursor_advances_one_feature_per_pass() {
		let mut user = SpottingUser::default();
		assert_eq!(user.advance_sample_cursor(), 0);
		assert_eq!(user.advance_sample_cursor(), 1);
		assert_eq!(user.advance_sample_cursor(), 2);
	}

	#[test]
	fn hint_from_skips_an_identical_source_priority() {
		let subject = Entity::from_bits(1);
		let mut user = SpottingUser::default();
		user.hint_from(subject, SpottingHintSource::THREAT, 5);
		assert!(user.has_hint_from(subject, SpottingHintSource::THREAT, 5));
		user.hint_from(subject, SpottingHintSource::THREAT, 5);
		assert_eq!(user.hints.get(&subject).map(SpottingHint::priority), Some(5));
		user.hint_from(subject, SpottingHintSource::THREAT, 8);
		assert!(user.has_hint_from(subject, SpottingHintSource::THREAT, 8));
	}

	#[test]
	fn removing_threat_hint_preserves_explicit_owner() {
		let subject = Entity::from_bits(1);
		let mut user = SpottingUser::default();
		user.hint(subject, SpottingHint::new(2));
		user.hint_from(subject, SpottingHintSource::THREAT, 5);
		assert_eq!(user.hints.get(&subject).map(SpottingHint::priority), Some(5));
		user.remove_hint_source(subject, SpottingHintSource::THREAT);
		assert!(user.hints.get(&subject).is_some_and(|hint| {
			hint.has_source(SpottingHintSource::EXPLICIT) && hint.priority() == 2
		}));
	}
}
