//! Semantic mob LodScene host with trickled High roster stubs.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::{cull_non_adjacent_bands, lod_host_scene_pending, SceneChunk};
use mob_characters::{CharacterSpecies, FromMobNumber};

use crate::roster_ref::MemberRosterRef;
use crate::{MobBrain, MobKind, MobMemberRecipe, MobRosterRecipe};

/// Semantic High band radius (metres) for streamed world mobs.
/// Playgrounds override with [`MobScene::with_high_radius`].
pub const DEFAULT_MOB_HIGH_RADIUS: f32 = 200.0;

#[derive(Clone, Debug)]
pub struct Mob<Roster = MobRosterRecipe, Intelligence = MobBrain> {
	pub num: f32,
	pub kind: MobKind,
	pub roster: Roster,
	pub intelligence: Intelligence,
}

impl Mob {
	pub fn from_num(num: f32) -> Self {
		Self::of_kind(MobKind::from_num(num), num)
	}

	pub fn of_kind(kind: MobKind, num: f32) -> Self {
		Self::of_kind_among(kind, num, &CharacterSpecies::VALUES)
	}

	/// [`Self::of_kind`] rolling members only from `species`
	/// (see [`MobRosterRecipe::from_kind_among`]).
	pub fn of_kind_among(kind: MobKind, num: f32, species: &[CharacterSpecies]) -> Self {
		let intelligence = MobBrain::for_kind(kind);
		let roster = MobRosterRecipe::from_kind_among(kind, num, intelligence.leash, species);
		Self { num, kind, roster, intelligence }
	}

	pub fn into_scene(self) -> MobScene {
		MobScene { mob: self, center: Vec3::ZERO, high_radius: DEFAULT_MOB_HIGH_RADIUS }
	}
}

#[derive(Component, Clone, Debug)]
pub struct MobScene {
	pub mob: Mob,
	pub center: Vec3,
	pub high_radius: f32,
}

impl Default for MobScene {
	fn default() -> Self {
		Mob::from_num(0.0).into_scene()
	}
}

impl MobScene {
	pub fn from_num(num: f32) -> Self {
		Mob::from_num(num).into_scene()
	}

	pub fn of_kind(kind: MobKind, num: f32) -> Self {
		Mob::of_kind(kind, num).into_scene()
	}

	pub fn with_high_radius(mut self, radius: f32) -> Self {
		self.high_radius = radius.max(1.0);
		self
	}

	pub fn at(mut self, center: Vec3) -> Self {
		self.center = center;
		self
	}

	/// The mob's host at `transform`, its High band centered there.
	pub fn scene(&self, transform: Transform) -> impl Scene + 'static {
		let scene = self.clone().at(transform.translation);
		(
			lod_host_scene_pending(scene.level_for(&transform), scene.scene_bounds()),
			scene.contents(),
			bsn! {
				template_value(transform)
			},
		)
	}

	fn contents(&self) -> impl Scene + 'static {
		let host = self.clone();
		let kind = self.mob.kind;
		bsn! {
			template_value(host)
			template_value(kind)
			Name::new(format!("{kind:?} mob"))
			Visibility::default()
		}
	}

	pub fn spawn(&self, commands: &mut Commands, transform: Transform) -> Entity {
		commands.spawn_scene(self.scene(transform)).id()
	}

	fn level_for(&self, transform: &Transform) -> LodSceneLevel {
		if transform.translation.distance(self.center) <= self.high_radius {
			LodSceneLevel::High
		} else {
			LodSceneLevel::UltraLow
		}
	}

	fn member_stub(member: &MobMemberRecipe, slot: u16) -> impl Scene + 'static {
		let roster = MemberRosterRef::new(Arc::clone(&member.character), slot, member.offset);
		bsn! {
			Name::new("roster-stub")
			template_value(roster)
			Transform::default()
			Visibility::Hidden
		}
	}

	fn children_scene(&self) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.mob
			.roster
			.members
			.iter()
			.enumerate()
			.map(|(slot, member)| {
				Box::new(Self::member_stub(member, slot as u16)) as Box<dyn Scene>
			})
			.collect();
		bsn! {
			Transform::default()
			Visibility::default()
			Children [ {children} ]
		}
	}

	fn empty_scene() -> impl Scene + 'static {
		bsn! {
			Visibility::Inherited
		}
	}
}

impl LodScene for MobScene {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = self.level_for(lod_ref.previous_transform);
		let current = self.level_for(lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		cull_non_adjacent_bands(current).with_customs()
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		match level {
			LodSceneLevel::High => Box::new(self.children_scene()) as Box<dyn Scene>,
			_ => Box::new(Self::empty_scene()) as Box<dyn Scene>,
		}
	}

	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		if level != LodSceneLevel::High {
			return SceneChunk::primitive(Self::empty_scene());
		}
		let members = self.mob.roster.members.clone();
		let count = members.len();
		let mut iter = members.into_iter().enumerate();
		SceneChunk::lazy(count as u32, count, move || {
			let (slot, member) = iter.next()?;
			Some(SceneChunk::weighted(1, Self::member_stub(&member, slot as u16)))
		})
	}

	fn scene_bounds(&self) -> Aabb3d {
		let radius = self.mob.intelligence.leash.max(2.0);
		Aabb3d::from_min_max(Vec3::new(-radius, -2.0, -radius), Vec3::new(radius, 8.0, radius))
	}

	fn host_contents(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		self.contents()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn with_lod<R>(distance: f32, f: impl FnOnce(&LodRef<'_>) -> R) -> R {
		let transform = Transform::from_xyz(distance, 0.0, 0.0);
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &transform,
			current_transform: &transform,
			bounds: &bounds,
		};
		f(&lod_ref)
	}

	#[test]
	fn characters_exist_only_in_high_semantic_lod() {
		let mob = MobScene::of_kind(MobKind::Pleb, 4.0);
		with_lod(0.0, |lod_ref| {
			let high = mob.scene_chunks_with_level(lod_ref, LodSceneLevel::High);
			assert_eq!(high.total_primitives(), mob.mob.roster.members.len());
			let low = mob.scene_chunks_with_level(lod_ref, LodSceneLevel::UltraLow);
			assert_eq!(low.total_primitives(), 1);
		});
	}

	#[test]
	fn far_mobs_keep_host_but_drop_high() {
		let mob = MobScene::default().with_high_radius(100.0);
		with_lod(99.0, |lod_ref| assert_eq!(mob.scene_lod_level(lod_ref), LodSceneLevel::High));
		with_lod(101.0, |lod_ref| {
			assert_eq!(mob.scene_lod_level(lod_ref), LodSceneLevel::UltraLow);
			assert!(mob
				.scene_lod_culls(lod_ref, LodSceneLevel::UltraLow)
				.should_cull(LodSceneLevel::High));
		});
	}

	#[test]
	fn default_high_radius_is_two_hundred_metres() {
		assert_eq!(DEFAULT_MOB_HIGH_RADIUS, 200.0);
		assert_eq!(MobScene::default().high_radius, DEFAULT_MOB_HIGH_RADIUS);
	}

	#[test]
	fn high_plants_are_roster_ref_stubs() {
		let mob = MobScene::of_kind(MobKind::Pleb, 4.0);
		assert!(!mob.mob.roster.members.is_empty());
		for (slot, member) in mob.mob.roster.members.iter().enumerate() {
			let stub =
				MemberRosterRef::new(Arc::clone(&member.character), slot as u16, member.offset);
			assert_eq!(stub.slot, slot as u16);
			assert_eq!(stub.offset, member.offset);
			assert!(Arc::ptr_eq(&stub.recipe, &member.character));
		}
	}

	#[test]
	fn generated_mob_retains_a_real_character_recipe() {
		let mob = MobScene::of_kind(MobKind::Brawler, 11.0);
		assert!(mob.mob.roster.members.iter().all(|member| {
			member.character.species.is_biped()
				&& member.character.brains == mob_characters::CharacterBrains::Brawler
				&& member.character.armed()
		}));
	}
}
