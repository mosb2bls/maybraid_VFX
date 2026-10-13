//! Latest-wins mailbox that transitions from the last visible pose and armature offset.
//!
//! [`tick_anim_mailbox`] advances clip time for every body host. [`apply_anim_mailbox`]
//! samples and writes only hosts with [`AnimateBones`] and/or [`AnimateEffects`].
//! [`select_mailbox_applies`] rank-fills a count budget among on-screen marked
//! bodies; off-screen Near holds the last pose. Visible High Mid/Far plants
//! keep [`AnimateBones`] from sync, so they compete here too. No published look
//! means every marked body competes (tests / playgrounds).

use bevy::ecs::batching::BatchingStrategy;
use bevy::ecs::query::{Has, Or};
use bevy::prelude::*;
use character_animations::{
	animations::{
		Idle, Jab, Prone, QuadrupedIdle, QuadrupedLeap, QuadrupedRun, Squat, SquatDescent, Tuck,
		TwoFootedTuckedFlip, UprightLeap,
	},
	Animation, Effects,
};
use character_rigs::{
	authoring::{
		forelimbed_v0_definition, humanoid_v0_definition, quadruped_v0_definition, ArmatureOffset,
		BlendCurve, PoseBuffer, RigBinding, RigDefinition,
	},
	rigs::{
		forelimbed_v0::ForelimbedV0Rig, humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig,
	},
	Name as RigName,
};
use intelligence_lod::{
	look_applies, IntelligenceFocus, IntelligenceLod, IntelligenceLookFrame, IntelligencePriority,
};

use crate::clip::{AnimClip, AnimId, AnimRefRoot};
use crate::clip_cache::{apply_evaluated_sample, AnimClipCache, PreparedClip};
use crate::markers::{AnimateBones, AnimateEffects, SuspendAnimation};
use crate::plant::plant_lod_entity;
use crate::rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};
use rigs::PoseSkipRotation;
use std::collections::HashSet;

/// Per-frame Near mailbox budget. Rank fills first; leftovers hold pose.
///
/// [`Self::max_applies`] is the NPC count. Missing-lod (local player) is extra
/// and always applies.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MailboxApplyLimits {
	pub max_applies: usize,
	pub fairness_cap: u8,
}

impl Default for MailboxApplyLimits {
	fn default() -> Self {
		Self { max_applies: 32, fairness_cap: IntelligenceLod::FAIRNESS_CAP }
	}
}

/// Who may tick+apply this frame. `only = None` means every marked body (tests).
#[derive(Resource, Clone, Debug, Default)]
pub struct MailboxApplySet {
	pub only: Option<HashSet<Entity>>,
}

impl MailboxApplySet {
	fn allows(&self, entity: Entity) -> bool {
		self.only.as_ref().is_none_or(|only| only.contains(&entity))
	}
}

fn allows_only(only: Option<&HashSet<Entity>>, entity: Entity) -> bool {
	only.is_none_or(|only| only.contains(&entity))
}

const BLEND_DURATION: f32 = 0.15;

/// Marks a bone owned by the animation mailbox (pose apply skips its rotation).
#[derive(Component, Clone)]
pub struct AnimBone {
	pub name: RigName,
	pub rest: Transform,
}

/// Last sampled pose + in-flight transition. Latest [`AnimRefRoot`] wins.
#[derive(Component, Clone)]
pub struct AnimMailbox {
	pub output: PoseBuffer,
	/// Rig-indexed bone entities captured at prepare. Avoids per-frame name lookup when writing.
	pub bone_entities: Box<[Entity]>,
	/// True after a bone sample has been written. Hold code waits for this.
	pub posed: bool,
	/// Offset currently shown on the armature. A new clip blends from here.
	pub displayed_offset: ArmatureOffset,
	/// Frames this Near body was skipped. Reset when it applies.
	pub apply_skips: u8,
	last: Option<AnimId>,
	clip_progress: f32,
	blend_progress: f32,
	from_pose: PoseBuffer,
	from_offset: ArmatureOffset,
	bind_transform: Transform,
	/// Prepared clip table. Rebuilt when the selected clip or its parameters change.
	pub prepared_clip: Option<PreparedClip>,
	prepared_from: Option<AnimClip>,
}

/// Sample coordinate for the current clip. When present, [`tick_anim_mailbox`]
/// uses this instead of advancing wall-clock clip time (jump / leap phases).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimProgress(pub f32);

/// Inserted when landmarks are ready but a required animation bone is still missing.
/// Prepare retries until the bone appears, and logs only on the first gap.
#[derive(Component, Clone, Copy, Default)]
pub struct AnimBindingGap;

impl AnimMailbox {
	pub fn new(bind_transform: Transform) -> Self {
		Self::with_bones(bind_transform, 0)
	}

	pub fn with_bones(bind_transform: Transform, len: usize) -> Self {
		Self {
			output: PoseBuffer::identity(len),
			bone_entities: Box::default(),
			posed: false,
			displayed_offset: ArmatureOffset::IDENTITY,
			apply_skips: 0,
			last: None,
			clip_progress: 0.0,
			blend_progress: 1.0,
			from_pose: PoseBuffer::identity(len),
			from_offset: ArmatureOffset::IDENTITY,
			bind_transform,
			prepared_clip: None,
			prepared_from: None,
		}
	}

	fn blending(&self) -> bool {
		self.blend_progress < 1.0
	}
}

/// Insert typed rigs, [`AnimBone`]s, and [`AnimMailbox`] once the bone map is ready.
///
/// Does not require [`AnimateBones`] — coming back into range should not rebuild
/// the mailbox.
pub fn prepare_anim_mailbox(
	mut commands: Commands,
	hosts: Query<
		(Entity, &AnimRefRoot, &BoneMap, &CharacterRig, &Transform, Has<AnimBindingGap>),
		(Without<AnimMailbox>, With<CharacterRig>),
	>,
	transforms: Query<&Transform>,
) {
	for (entity, _root, bone_map, character_rig, transform, gap) in &hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if !bone_map_ready(bone_map, character_rig.skeleton) {
			continue;
		}

		let definition = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => humanoid_v0_definition(),
			RigSkeletonKind::Quadruped => quadruped_v0_definition(),
			RigSkeletonKind::Forelimbed => forelimbed_v0_definition(),
			RigSkeletonKind::Neck => continue,
		};
		let Some((rest, entities)) = capture_rest(&definition, bone_map, &transforms) else {
			if !gap {
				let missing = missing_animation_bones(&definition, bone_map);
				error!(
					"animation bones missing for {entity:?} ({:?}): {}",
					character_rig.skeleton,
					missing.join(", ")
				);
				commands.entity(entity).insert(AnimBindingGap);
			}
			continue;
		};
		if gap {
			commands.entity(entity).remove::<AnimBindingGap>();
		}

		let names: Vec<&'static str> = definition.names.iter().copied().collect();
		let binding =
			RigBinding::from_rest(definition, entities.clone().into_boxed_slice(), rest.clone());
		match character_rig.skeleton {
			RigSkeletonKind::Humanoid => {
				let mut rig = HumanoidV0Rig::imported();
				rig.binding = binding;
				rig.segment_lengths = rig.binding.metrics.humanoid_leg;
				rig.pose.copy_from(&rest);
				commands.entity(entity).insert(rig);
			}
			RigSkeletonKind::Quadruped => {
				let mut rig = QuadrupedV0Rig::imported();
				rig.binding = binding;
				rig.segment_lengths = rig.binding.metrics.quadruped_leg;
				rig.pose.copy_from(&rest);
				commands.entity(entity).insert(rig);
			}
			RigSkeletonKind::Forelimbed => {
				let mut rig = ForelimbedV0Rig::imported();
				rig.binding = binding;
				rig.pose.copy_from(&rest);
				commands.entity(entity).insert(rig);
			}
			RigSkeletonKind::Neck => continue,
		}

		for (entity_bone, name) in entities.iter().zip(names.iter().copied()) {
			let Ok(bone_tf) = transforms.get(*entity_bone) else {
				continue;
			};
			commands
				.entity(*entity_bone)
				.insert((AnimBone { name: RigName::from(name), rest: *bone_tf }, PoseSkipRotation));
		}

		let mut mailbox = AnimMailbox::with_bones(*transform, rest.len());
		mailbox.bone_entities = entities.into_boxed_slice();
		mailbox.output.copy_from(&rest);
		mailbox.from_pose.copy_from(&rest);
		commands.entity(entity).insert(mailbox);
	}
}

/// Rank on-screen Near bodies, fill a count budget from the top, hold last pose
/// off-screen and on leftovers. Fairness only among in-cone leftovers.
pub fn select_mailbox_applies(
	mut set: ResMut<MailboxApplySet>,
	limits: Res<MailboxApplyLimits>,
	priority: Res<IntelligencePriority>,
	look_frame: Res<IntelligenceLookFrame>,
	focus: Res<IntelligenceFocus>,
	mut hosts: Query<
		(Entity, &mut AnimMailbox, &CharacterRig),
		(
			With<AnimMailbox>,
			Without<AnimBone>,
			Without<SuspendAnimation>,
			Or<(With<AnimateBones>, With<AnimateEffects>)>,
		),
	>,
	child_of: Query<&ChildOf>,
	lods: Query<&IntelligenceLod>,
	transforms: Query<&GlobalTransform>,
) {
	struct Candidate {
		entity: Entity,
		always: bool,
		in_cone: bool,
		rank: u32,
		skips: u8,
	}

	let mut candidates = Vec::new();
	for (entity, mailbox, character_rig) in &hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		let plant = plant_lod_entity(entity, &child_of, &lods);
		let always = plant.is_none();
		candidates.push(Candidate {
			entity,
			always,
			in_cone: always || in_apply_cone(plant, &look_frame, &focus, &transforms),
			rank: plant.map(|plant| priority.rank_of(plant)).unwrap_or(0),
			skips: mailbox.apply_skips,
		});
	}
	candidates
		.sort_by_key(|candidate| (!candidate.always, candidate.rank, candidate.entity.to_bits()));

	let mut selected = HashSet::new();
	for candidate in candidates.iter().filter(|candidate| candidate.always) {
		selected.insert(candidate.entity);
	}
	let mut filled = 0;
	for candidate in candidates.iter().filter(|candidate| !candidate.always && candidate.in_cone) {
		if filled >= limits.max_applies {
			break;
		}
		selected.insert(candidate.entity);
		filled += 1;
	}
	if let Some(fair) = candidates.iter().find(|candidate| {
		candidate.in_cone
			&& !selected.contains(&candidate.entity)
			&& candidate.skips >= limits.fairness_cap
	}) {
		selected.insert(fair.entity);
	}

	let in_cone: HashSet<Entity> = candidates
		.iter()
		.filter(|candidate| candidate.in_cone)
		.map(|candidate| candidate.entity)
		.collect();
	for (entity, mut mailbox, character_rig) in &mut hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if selected.contains(&entity) {
			mailbox.apply_skips = 0;
		} else if in_cone.contains(&entity) {
			mailbox.apply_skips = mailbox.apply_skips.saturating_add(1);
		}
	}
	set.only = Some(selected);
}

fn in_apply_cone(
	plant: Option<Entity>,
	look_frame: &IntelligenceLookFrame,
	focus: &IntelligenceFocus,
	transforms: &Query<&GlobalTransform>,
) -> bool {
	if look_frame.look.is_none() && focus.samples.is_empty() {
		return true;
	}
	let Some(plant) = plant else {
		return true;
	};
	let Ok(transform) = transforms.get(plant) else {
		return true;
	};
	look_applies(transform.translation(), look_frame.look, focus)
}

/// Advance clip / blend time for body mailboxes still owned by animation.
pub fn tick_anim_mailbox(
	time: Res<Time>,
	set: Option<Res<MailboxApplySet>>,
	cache: Option<Res<AnimClipCache>>,
	mut hosts: Query<
		(Entity, &AnimRefRoot, &mut AnimMailbox, Option<&AnimProgress>, &CharacterRig),
		(With<AnimMailbox>, Without<AnimBone>, Without<SuspendAnimation>),
	>,
) {
	let dt = time.delta_secs();
	let cache = cache.as_deref();
	for (entity, root, mut mailbox, progress, character_rig) in &mut hosts {
		if !set.as_deref().is_none_or(|set| set.allows(entity)) {
			continue;
		}
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}

		let requested = root.0.clip;
		let requested_id = requested.id();
		if mailbox.last != Some(requested_id) {
			if mailbox.posed {
				let AnimMailbox { output, from_pose, .. } = &mut *mailbox;
				from_pose.copy_from(output);
			}
			mailbox.from_offset = mailbox.displayed_offset;
			mailbox.blend_progress = 0.0;
			mailbox.clip_progress = 0.0;
			mailbox.last = Some(requested_id);
		}

		if let Some(progress) = progress {
			mailbox.clip_progress = progress.0;
		} else {
			mailbox.clip_progress += dt * root.0.speed;
		}
		if mailbox.blending() {
			mailbox.blend_progress = (mailbox.blend_progress + dt / BLEND_DURATION).min(1.0);
		}
		refresh_prepared_clip(&mut mailbox, requested, character_rig.skeleton, cache);
	}
}

/// Sample clips in parallel; write bone transforms serially (shared bone query).
pub fn apply_anim_mailbox(
	set: Option<Res<MailboxApplySet>>,
	cache: Option<Res<AnimClipCache>>,
	mut hosts: Query<
		(
			Entity,
			&AnimRefRoot,
			&mut AnimMailbox,
			&BoneMap,
			&CharacterRig,
			&mut Transform,
			Has<AnimateBones>,
			Has<AnimateEffects>,
			Option<&mut HumanoidV0Rig>,
			Option<&mut QuadrupedV0Rig>,
			Option<&mut ForelimbedV0Rig>,
		),
		(
			With<AnimMailbox>,
			Without<AnimBone>,
			Without<SuspendAnimation>,
			Or<(With<AnimateBones>, With<AnimateEffects>)>,
		),
	>,
	bones: Query<&AnimBone, Without<AnimMailbox>>,
	mut bone_tfs: Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) {
	let only = set.as_ref().and_then(|set| set.only.as_ref());
	let cache = cache.as_deref();
	hosts.par_iter_mut().batching_strategy(BatchingStrategy::fixed(8)).for_each(
		|(
			entity,
			root,
			mut mailbox,
			_bone_map,
			character_rig,
			mut armature,
			write_bones,
			write_effects,
			humanoid,
			quadruped,
			forelimbed,
		)| {
			if !allows_only(only, entity) {
				return;
			}
			if character_rig.role != CharacterRigRole::Body {
				return;
			}

			let requested = root.0.clip;
			let progress = clip_progress(requested, mailbox.clip_progress, entity);
			let weight = BlendCurve::SmoothStep.sample(mailbox.blend_progress);
			let prepared = mailbox.prepared_clip.as_ref();
			let effects = match character_rig.skeleton {
				RigSkeletonKind::Humanoid => {
					let mut rig = match humanoid {
						Some(rig) => rig,
						None => return,
					};
					if write_bones {
						sync_humanoid_rest(&mut rig, &bones);
					}
					let effects = sample_humanoid_prepared(
						requested,
						&mut rig,
						progress,
						write_bones,
						write_effects,
						cache,
						prepared,
					);
					if write_bones {
						publish_pose(&mut mailbox, &rig.pose, weight);
					}
					effects
				}
				RigSkeletonKind::Quadruped => {
					let mut rig = match quadruped {
						Some(rig) => rig,
						None => return,
					};
					if write_bones {
						sync_quadruped_rest(&mut rig, &bones);
					}
					let effects =
						sample_quadruped(requested, &mut rig, progress, write_bones, write_effects);
					if write_bones {
						publish_pose(&mut mailbox, &rig.pose, weight);
					}
					effects
				}
				RigSkeletonKind::Forelimbed => {
					let mut rig = match forelimbed {
						Some(rig) => rig,
						None => return,
					};
					if write_bones {
						sync_binding_rest(&mut rig.binding, &bones);
					}
					let effects = sample_forelimbed(
						requested,
						&mut rig,
						progress,
						write_bones,
						write_effects,
					);
					if write_bones {
						publish_pose(&mut mailbox, &rig.pose, weight);
					}
					effects
				}
				RigSkeletonKind::Neck => return,
			};
			if write_effects {
				let shown = ArmatureOffset::blend(mailbox.from_offset, effects, weight);
				mailbox.displayed_offset = shown;
				*armature = shown.apply_to_bind(mailbox.bind_transform);
			}
		},
	);

	for (entity, _, mailbox, bone_map, character_rig, _, write_bones, _, _, _, _) in &hosts {
		if !write_bones || character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if !allows_only(only, entity) {
			continue;
		}
		let names = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => &humanoid_v0_definition().names,
			RigSkeletonKind::Quadruped => &quadruped_v0_definition().names,
			RigSkeletonKind::Forelimbed => &forelimbed_v0_definition().names,
			RigSkeletonKind::Neck => continue,
		};
		if indexed_cache_valid(&mailbox.bone_entities, mailbox.output.local.len(), &bone_tfs) {
			write_pose_indexed(&mailbox.output, &mailbox.bone_entities, &mut bone_tfs);
		} else {
			write_pose_by_name(&mailbox.output, names, bone_map, &mut bone_tfs);
		}
	}
}

/// True when prepare-time bone entities still align with the sampled pose length and exist.
fn indexed_cache_valid(
	entities: &[Entity],
	pose_len: usize,
	transforms: &Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) -> bool {
	!entities.is_empty()
		&& entities.len() == pose_len
		&& entities.iter().all(|entity| transforms.get(*entity).is_ok())
}

fn clip_progress(clip: AnimClip, clip_progress: f32, entity: Entity) -> f32 {
	match clip {
		AnimClip::Still => clip_progress + Idle::phase_from_entity_bits(entity.to_bits()),
		_ => clip_progress,
	}
}

fn publish_pose(mailbox: &mut AnimMailbox, sampled: &PoseBuffer, weight: f32) {
	if weight < 1.0 {
		PoseBuffer::blend_into(&mailbox.from_pose, sampled, weight, &mut mailbox.output);
	} else {
		mailbox.output.copy_from(sampled);
	}
	mailbox.posed = true;
}

fn sync_humanoid_rest(rig: &mut HumanoidV0Rig, bones: &Query<&AnimBone, Without<AnimMailbox>>) {
	if sync_binding_rest(&mut rig.binding, bones) {
		rig.segment_lengths = rig.binding.metrics.humanoid_leg;
	}
}

fn sync_quadruped_rest(rig: &mut QuadrupedV0Rig, bones: &Query<&AnimBone, Without<AnimMailbox>>) {
	if sync_binding_rest(&mut rig.binding, bones) {
		rig.segment_lengths = rig.binding.metrics.quadruped_leg;
	}
}

/// Refresh rest and segment lengths when a bone rest changed.
fn sync_binding_rest(
	binding: &mut RigBinding,
	bones: &Query<&AnimBone, Without<AnimMailbox>>,
) -> bool {
	if binding
		.entities
		.iter()
		.zip(binding.effective_rest.local.iter())
		.all(|(entity, rest)| bones.get(*entity).is_ok_and(|bone| bone.rest == *rest))
	{
		return false;
	}
	let Some(rest) = rest_from_bones(binding, bones) else {
		return false;
	};
	binding.refresh_rest(rest);
	true
}

fn rest_from_bones(
	binding: &RigBinding,
	bones: &Query<&AnimBone, Without<AnimMailbox>>,
) -> Option<PoseBuffer> {
	let mut rest = PoseBuffer::identity(binding.definition.len());
	for (index, entity) in binding.entities.iter().enumerate() {
		rest.local[index] = bones.get(*entity).ok()?.rest;
	}
	Some(rest)
}

fn capture_rest(
	definition: &RigDefinition,
	bone_map: &BoneMap,
	transforms: &Query<&Transform>,
) -> Option<(PoseBuffer, Vec<Entity>)> {
	let mut rest = PoseBuffer::identity(definition.len());
	let mut entities = vec![Entity::PLACEHOLDER; definition.len()];
	for (index, name) in definition.names.iter().enumerate() {
		let bone_entity = *bone_map.by_name.get(*name)?;
		rest.local[index] = *transforms.get(bone_entity).ok()?;
		entities[index] = bone_entity;
	}
	Some((rest, entities))
}

fn missing_animation_bones(definition: &RigDefinition, bone_map: &BoneMap) -> Vec<&'static str> {
	definition
		.names
		.iter()
		.copied()
		.filter(|name| !bone_map.by_name.contains_key(*name))
		.collect()
}

fn write_pose_by_name(
	pose: &PoseBuffer,
	names: &[&'static str],
	bone_map: &BoneMap,
	transforms: &mut Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) {
	for (index, name) in names.iter().enumerate() {
		let Some(&entity) = bone_map.by_name.get(*name) else {
			continue;
		};
		let Some(desired) = pose.local.get(index) else {
			continue;
		};
		let Ok(mut transform) = transforms.get_mut(entity) else {
			continue;
		};
		if *transform != *desired {
			*transform = *desired;
		}
	}
}

fn write_pose_indexed(
	pose: &PoseBuffer,
	entities: &[Entity],
	transforms: &mut Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) {
	for (index, &entity) in entities.iter().enumerate() {
		let Some(desired) = pose.local.get(index) else {
			continue;
		};
		let Ok(mut transform) = transforms.get_mut(entity) else {
			continue;
		};
		if *transform != *desired {
			*transform = *desired;
		}
	}
}

fn refresh_prepared_clip(
	mailbox: &mut AnimMailbox,
	clip: AnimClip,
	skeleton: RigSkeletonKind,
	cache: Option<&AnimClipCache>,
) {
	let Some(cache) = cache else {
		mailbox.prepared_clip = None;
		mailbox.prepared_from = None;
		return;
	};
	if mailbox.prepared_from == Some(clip) {
		return;
	}
	mailbox.prepared_from = Some(clip);
	mailbox.prepared_clip = skeleton
		.sample_rig_variant()
		.and_then(|rig| cache.prepare(clip, rig, cache.settings.sampling));
}

fn sample_humanoid_prepared(
	clip: AnimClip,
	rig: &mut HumanoidV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
	cache: Option<&AnimClipCache>,
	prepared: Option<&PreparedClip>,
) -> Effects {
	if let (Some(cache), Some(prepared)) = (cache, prepared) {
		if let Some(sample) = cache.sample(prepared, progress) {
			if write_bones {
				apply_evaluated_sample(
					&rig.binding.effective_rest,
					prepared.bone_mask(),
					sample,
					&mut rig.pose,
				);
			}
			return if write_effects { sample.effects } else { Effects::IDENTITY };
		}
	}
	sample_humanoid(clip, rig, progress, write_bones, write_effects)
}

fn sample_split<A, R>(
	anim: &A,
	rig: &mut R,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects
where
	A: Animation<R>,
{
	if write_bones {
		anim.apply_for(rig, progress);
	}
	if write_effects {
		anim.effects_for(rig, progress)
	} else {
		Effects::IDENTITY
	}
}

fn sample_humanoid(
	clip: AnimClip,
	rig: &mut HumanoidV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects {
	match clip {
		AnimClip::Still => {
			sample_split(&Idle::default(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Walk(walk) => sample_split(&walk, rig, progress, write_bones, write_effects),
		AnimClip::Run(run) => sample_split(&run, rig, progress, write_bones, write_effects),
		AnimClip::Jump(params) => {
			sample_split(&params.apply_humanoid(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Leap(leap) => {
			sample_split(&UprightLeap::from_leap(&leap), rig, progress, write_bones, write_effects)
		}
		AnimClip::Tuck(params) => sample_split(
			&Tuck::new(params.tightness),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::TuckedFlip(params) => sample_split(
			&params.apply_humanoid(),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::TwoFootedTuckedFlip(params) => sample_split(
			&TwoFootedTuckedFlip::default()
				.with_jump(params.jump.apply_humanoid())
				.with_flip(params.flip.apply_humanoid()),
			rig,
			progress,
			write_bones,
			write_effects,
		),
		AnimClip::Soaring(soaring) => {
			sample_split(&soaring, rig, progress, write_bones, write_effects)
		}
		AnimClip::Flapping(flapping) => {
			sample_split(&flapping, rig, progress, write_bones, write_effects)
		}
		AnimClip::Jab(params) => sample_split(
			&Jab::new(params.side, params.backswing, params.target),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::Squat => sample_split(&Squat::held(), rig, progress, write_bones, write_effects),
		AnimClip::SquatDescent => sample_split(
			&SquatDescent::default(),
			rig,
			progress.clamp(0.0, 1.0),
			write_bones,
			write_effects,
		),
		AnimClip::Prone => {
			sample_split(&Prone::default(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Gallop(_)
		| AnimClip::QuadrupedRun(_)
		| AnimClip::LateralUndulation(_)
		| AnimClip::DorsoventralUndulation(_) => Effects::default(),
	}
}

fn sample_quadruped(
	clip: AnimClip,
	rig: &mut QuadrupedV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects {
	match clip {
		AnimClip::QuadrupedRun(run) => {
			sample_split(&run, rig, progress, write_bones, write_effects)
		}
		AnimClip::Run(_) => {
			sample_split(&QuadrupedRun::default(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Gallop(gallop) => {
			sample_split(&gallop, rig, progress, write_bones, write_effects)
		}
		AnimClip::Leap(leap) => sample_split(
			&QuadrupedLeap::from_leap(&leap),
			rig,
			progress,
			write_bones,
			write_effects,
		),
		AnimClip::Still => {
			sample_split(&QuadrupedIdle::default(), rig, progress, write_bones, write_effects)
		}
		_ => Effects::default(),
	}
}

fn sample_forelimbed(
	clip: AnimClip,
	rig: &mut ForelimbedV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects {
	match clip {
		AnimClip::LateralUndulation(wave) => {
			sample_split(&wave, rig, progress, write_bones, write_effects)
		}
		AnimClip::DorsoventralUndulation(wave) => {
			sample_split(&wave, rig, progress, write_bones, write_effects)
		}
		_ => Effects::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::AnimRef;
	use bevy::ecs::system::RunSystemOnce;
	use character_rigs::authoring::humanoid_v0_definition;
	use intelligence_lod::IntelligenceBand;
	use std::collections::HashMap;
	use std::hint::black_box;
	use std::time::Instant;

	fn run_select(world: &mut World) {
		world.init_resource::<MailboxApplySet>();
		world.init_resource::<MailboxApplyLimits>();
		world.init_resource::<IntelligencePriority>();
		world.init_resource::<IntelligenceLookFrame>();
		world.init_resource::<IntelligenceFocus>();
		world.run_system_once(select_mailbox_applies).expect("select_mailbox_applies");
	}

	fn looking_x_apply() -> intelligence_lod::IntelligenceLook {
		let tf =
			Transform::from_translation(Vec3::Y).looking_at(Vec3::new(20.0, 1.0, 0.0), Vec3::Y);
		intelligence_lod::IntelligenceLook::from_perspective_inset(
			&GlobalTransform::from(tf),
			&PerspectiveProjection {
				fov: 75_f32.to_radians(),
				aspect_ratio: 16.0 / 9.0,
				..default()
			},
			75_f32.to_radians(),
			intelligence_lod::LOOK_APPLY_FOV_INSET,
		)
	}

	fn near_body(world: &mut World, parent: Entity) -> Entity {
		world
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, ..default() },
				AnimMailbox::new(Transform::default()),
				AnimateBones,
				ChildOf(parent),
			))
			.id()
	}

	#[test]
	fn player_and_top_rank_apply_when_the_budget_is_one() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits {
			max_applies: 1,
			fairness_cap: IntelligenceLod::FAIRNESS_CAP,
		});
		let player_plant = world.spawn_empty().id();
		let top_plant = world.spawn(IntelligenceLod::missing()).id();
		let leftover_plant = world.spawn(IntelligenceLod::missing()).id();
		let player = near_body(&mut world, player_plant);
		let top = near_body(&mut world, top_plant);
		let leftover = near_body(&mut world, leftover_plant);
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(top_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(leftover_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&player));
		assert!(only.contains(&top));
		assert!(!only.contains(&leftover));
		assert_eq!(world.get::<AnimMailbox>(player).unwrap().apply_skips, 0);
		assert_eq!(world.get::<AnimMailbox>(top).unwrap().apply_skips, 0);
		assert_eq!(world.get::<AnimMailbox>(leftover).unwrap().apply_skips, 1);
	}

	#[test]
	fn leftover_applies_after_fairness_cap() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits { max_applies: 1, fairness_cap: 2 });
		let top_plant = world.spawn(IntelligenceLod::missing()).id();
		let leftover_plant =
			world.spawn(IntelligenceLod { band: IntelligenceBand::Near, skips: 0 }).id();
		let top = near_body(&mut world, top_plant);
		let leftover = near_body(&mut world, leftover_plant);
		world.get_mut::<AnimMailbox>(leftover).unwrap().apply_skips = 2;
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(top_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(leftover_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&top));
		assert!(only.contains(&leftover));
		assert_eq!(world.get::<AnimMailbox>(leftover).unwrap().apply_skips, 0);
	}

	#[test]
	fn still_samples_idle_on_humanoid() {
		let mut rig = HumanoidV0Rig::imported();
		let effects = sample_humanoid(AnimClip::Still, &mut rig, 0.25, true, true);
		assert!(effects.is_identity());
		assert!(rig.posed_angle("shoulder.L") > 0.0);
		assert!(rig.posed_angle("shoulder.L") < 0.15);
		assert!(rig.posed_angle("humerus.L") > 0.2, "Still should hang the arms");
	}

	#[test]
	fn squat_descent_matches_held_squat_at_full_depth() -> anyhow::Result<()> {
		use anyhow::anyhow;
		let mut descent = HumanoidV0Rig::for_clip_test();
		let mut held = HumanoidV0Rig::for_clip_test();
		sample_humanoid(AnimClip::squat_descent(), &mut descent, 1.0, true, true);
		sample_humanoid(AnimClip::squat(), &mut held, 1.0, true, true);
		for name in descent.animation_bone_names() {
			let a = descent.rotation(name);
			let b = held.rotation(name);
			if a.dot(b).abs() < 1.0 - 1e-5 {
				return Err(anyhow!("descent end must match held squat on {name}"));
			}
		}
		let mut mid = HumanoidV0Rig::for_clip_test();
		sample_humanoid(AnimClip::squat_descent(), &mut mid, 0.5, true, true);
		assert!(mid.posed_angle("femur.L") > 0.15, "mid descent should fold hips");
		Ok(())
	}

	#[test]
	fn stance_squat_has_no_root_move() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let mut rig = HumanoidV0Rig::imported();
		let effects = sample_humanoid(AnimClip::squat(), &mut rig, 1.0, true, true);
		if !effects.is_identity() {
			return Err(anyhow!("held squat must not move the armature"));
		}
		if rig.posed_angle("femur.L") < 0.3 {
			return Err(anyhow!("held squat should flex femurs"));
		}
		if rig.posed_angle("pelvis.L") < 0.1 {
			return Err(anyhow!("held squat should crease the pelvis"));
		}
		Ok(())
	}

	#[test]
	fn stance_prone_pitches_the_spine() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let mut rig = HumanoidV0Rig::imported();
		let effects = sample_humanoid(AnimClip::prone(), &mut rig, 1.0, true, true);
		if !effects.is_identity() {
			return Err(anyhow!("held prone must not move the armature"));
		}
		let root = rig.rotation("root") * Vec3::Y;
		if root.z.abs() < 0.2 || root.x.abs() > 0.2 {
			return Err(anyhow!("prone should pitch the spine, got {root:?}"));
		}
		Ok(())
	}

	#[test]
	fn still_samples_idle_on_quadruped() {
		let mut rig = QuadrupedV0Rig::imported();
		let effects =
			sample_quadruped(AnimClip::Still, &mut rig, QuadrupedIdle::graze_peak(), true, true);
		assert!(effects.is_identity());
		let neck = rig.rotation("neck") * Vec3::Y;
		assert!((neck - Vec3::Y).length() > 0.2, "neck leaves rest, got {neck:?}");
		let lumbar = rig.rotation("lumbar") * Vec3::Y;
		assert!((lumbar - Vec3::Y).length() > 0.05, "lumbar leaves rest, got {lumbar:?}");
	}

	#[test]
	fn still_progress_offsets_by_entity_bits() {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		assert_ne!(clip_progress(AnimClip::Still, 0.0, a), clip_progress(AnimClip::Still, 0.0, b));
		assert_eq!(clip_progress(AnimClip::walk(), 0.3, a), 0.3);
	}

	#[test]
	fn count_budget_keeps_the_top_thirty_two() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits {
			max_applies: 32,
			fairness_cap: IntelligenceLod::FAIRNESS_CAP,
		});
		world.init_resource::<IntelligencePriority>();
		let mut bodies = Vec::new();
		for rank in 0..40u32 {
			let plant = world.spawn(IntelligenceLod::missing()).id();
			let body = near_body(&mut world, plant);
			world.resource_mut::<IntelligencePriority>().rank.insert(plant, rank);
			bodies.push((rank, body));
		}

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert_eq!(only.len(), 32);
		for (rank, body) in &bodies {
			if *rank < 32 {
				assert!(only.contains(body), "rank {rank} should apply");
			} else {
				assert!(!only.contains(body), "rank {rank} should hold pose");
			}
		}
	}

	#[test]
	fn in_cone_near_applies_before_off_cone() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits {
			max_applies: 1,
			fairness_cap: IntelligenceLod::FAIRNESS_CAP,
		});
		world.insert_resource(IntelligenceLookFrame { look: Some(looking_x_apply()) });
		let ahead_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(40.0, 1.0, 0.0)),
			))
			.id();
		let behind_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(-40.0, 1.0, 0.0)),
			))
			.id();
		let ahead = near_body(&mut world, ahead_plant);
		let behind = near_body(&mut world, behind_plant);
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(behind_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(ahead_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&ahead));
		assert!(!only.contains(&behind));
		assert_eq!(world.get::<AnimMailbox>(behind).unwrap().apply_skips, 0);
	}

	#[test]
	fn off_cone_does_not_steal_fairness() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits { max_applies: 1, fairness_cap: 2 });
		world.insert_resource(IntelligenceLookFrame { look: Some(looking_x_apply()) });
		let ahead_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(40.0, 1.0, 0.0)),
			))
			.id();
		let behind_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(-40.0, 1.0, 0.0)),
			))
			.id();
		let ahead = near_body(&mut world, ahead_plant);
		let behind = near_body(&mut world, behind_plant);
		world.get_mut::<AnimMailbox>(behind).unwrap().apply_skips = 8;
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(ahead_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(behind_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&ahead));
		assert!(!only.contains(&behind));
		assert_eq!(world.get::<AnimMailbox>(behind).unwrap().apply_skips, 8);
	}

	fn humanoid_write_fixture() -> (World, PoseBuffer, BoneMap, Box<[Entity]>) {
		let definition = humanoid_v0_definition();
		let mut world = World::new();
		let mut bone_map = HashMap::new();
		let mut entities = Vec::with_capacity(definition.len());
		for (index, name) in definition.names.iter().enumerate() {
			let rest = Transform {
				translation: Vec3::new(index as f32 * 0.01, 0.0, 0.0),
				rotation: Quat::from_rotation_x(index as f32 * 0.02),
				scale: Vec3::ONE,
			};
			let entity = world.spawn((AnimBone { name: RigName::from(*name), rest }, rest)).id();
			bone_map.insert(name.to_string(), entity);
			entities.push(entity);
		}

		let mut pose = PoseBuffer::identity(definition.len());
		for (index, bone) in pose.local.iter_mut().enumerate() {
			*bone = Transform {
				translation: Vec3::new(index as f32 * 0.03, 0.1, 0.0),
				rotation: Quat::from_rotation_y(index as f32 * 0.04),
				scale: Vec3::splat(1.0 + index as f32 * 0.01),
			};
		}
		(world, pose, BoneMap { by_name: bone_map }, entities.into_boxed_slice())
	}

	fn capture_bone_transforms(world: &World, entities: &[Entity]) -> Vec<Transform> {
		entities
			.iter()
			.map(|entity| *world.get::<Transform>(*entity).expect("bone transform"))
			.collect()
	}

	fn reset_bone_transforms(world: &mut World, entities: &[Entity]) {
		for entity in entities {
			let rest = world.get::<AnimBone>(*entity).expect("bone").rest;
			*world.get_mut::<Transform>(*entity).expect("bone transform") = rest;
		}
	}

	fn write_pose_by_name_world(
		world: &mut World,
		pose: &PoseBuffer,
		names: &[&'static str],
		bone_map: &BoneMap,
	) {
		for (index, name) in names.iter().enumerate() {
			let Some(&entity) = bone_map.by_name.get(*name) else {
				continue;
			};
			let Some(desired) = pose.local.get(index) else {
				continue;
			};
			let Some(mut transform) = world.get_mut::<Transform>(entity) else {
				continue;
			};
			if *transform != *desired {
				*transform = *desired;
			}
		}
	}

	fn write_pose_indexed_world(world: &mut World, pose: &PoseBuffer, entities: &[Entity]) {
		for (index, &entity) in entities.iter().enumerate() {
			let Some(desired) = pose.local.get(index) else {
				continue;
			};
			let Some(mut transform) = world.get_mut::<Transform>(entity) else {
				continue;
			};
			if *transform != *desired {
				*transform = *desired;
			}
		}
	}

	#[test]
	fn indexed_write_matches_name_lookup_on_full_humanoid_pose() {
		let (mut world, pose, bone_map, entities) = humanoid_write_fixture();
		let names = &humanoid_v0_definition().names;

		write_pose_by_name_world(&mut world, &pose, &names, &bone_map);
		let legacy = capture_bone_transforms(&world, &entities);

		reset_bone_transforms(&mut world, &entities);
		write_pose_indexed_world(&mut world, &pose, &entities);
		let indexed = capture_bone_transforms(&world, &entities);

		assert_eq!(legacy, indexed, "indexed write must match legacy name lookup");
	}

	#[test]
	fn unprepared_mailbox_writes_pose_by_name() {
		let definition = humanoid_v0_definition();
		let mut world = World::new();
		let mut bone_map = HashMap::new();
		for (index, name) in definition.names.iter().enumerate() {
			let rest = Transform {
				translation: Vec3::new(index as f32 * 0.01, 0.0, 0.0),
				rotation: Quat::from_rotation_x(index as f32 * 0.02),
				scale: Vec3::ONE,
			};
			let entity = world.spawn((AnimBone { name: RigName::from(*name), rest }, rest)).id();
			bone_map.insert(name.to_string(), entity);
		}

		let mut rig = HumanoidV0Rig::imported();
		let mut pose = PoseBuffer::identity(definition.len());
		for (index, bone) in pose.local.iter_mut().enumerate() {
			*bone = Transform {
				translation: Vec3::new(index as f32 * 0.03, 0.2, 0.0),
				rotation: Quat::from_rotation_y(index as f32 * 0.04),
				scale: Vec3::ONE,
			};
		}
		rig.pose.copy_from(&pose);

		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		mailbox.output.copy_from(&pose);
		mailbox.posed = true;
		assert!(mailbox.bone_entities.is_empty(), "firearm-hold style mailbox has no cache");

		let host = world
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, skeleton: RigSkeletonKind::Humanoid },
				mailbox,
				BoneMap { by_name: bone_map },
				rig,
			))
			.id();
		world.init_resource::<MailboxApplySet>();
		world.resource_mut::<MailboxApplySet>().only = None;

		apply_mailbox_write_loop(&mut world, &[host], false);

		for (index, name) in definition.names.iter().enumerate() {
			let entity = world.get::<BoneMap>(host).unwrap().by_name.get(*name).unwrap();
			let transform = world.get::<Transform>(*entity).unwrap();
			assert_eq!(*transform, pose.local[index], "bone {name} should match mailbox output");
		}
	}

	#[test]
	fn stale_bone_entities_fall_back_to_name_lookup() {
		let definition = humanoid_v0_definition();
		let mut world = World::new();
		let host = spawn_mailbox_apply_host(&mut world, AnimClip::Still, 0.0);
		world.init_resource::<MailboxApplySet>();
		world.resource_mut::<MailboxApplySet>().only = None;

		let stale_entity = world.get::<AnimMailbox>(host).unwrap().bone_entities[3];
		world.despawn(stale_entity);
		let replacement = world
			.spawn((
				AnimBone { name: RigName::from(definition.names[3]), rest: Transform::IDENTITY },
				Transform::IDENTITY,
			))
			.id();
		let mut bone_map = world.get::<BoneMap>(host).unwrap().clone();
		bone_map.by_name.insert(definition.names[3].to_string(), replacement);
		world.entity_mut(host).insert(bone_map);
		assert!(
			world.get::<AnimMailbox>(host).unwrap().bone_entities.contains(&stale_entity),
			"mailbox cache should still reference the despawned entity"
		);

		let mut pose = world.get::<AnimMailbox>(host).unwrap().output.clone();
		pose.local[3].translation.y = 0.42;
		world.get_mut::<AnimMailbox>(host).unwrap().output.copy_from(&pose);

		apply_mailbox_write_loop(&mut world, &[host], false);

		let written = world.get::<Transform>(replacement).unwrap();
		assert_eq!(
			written.translation.y, 0.42,
			"replacement bone should receive pose via name fallback"
		);
	}

	#[test]
	fn indexed_write_leaves_unwritten_bones_unchanged() {
		let (mut world, mut pose, bone_map, entities) = humanoid_write_fixture();
		let names = &humanoid_v0_definition().names;

		let partial_indices = [0, 4, 7, 13, 17];
		for (index, entity) in entities.iter().enumerate() {
			let rest = world.get::<AnimBone>(*entity).expect("bone").rest;
			pose.local[index] = rest;
		}
		for index in partial_indices {
			pose.local[index].translation.y += 0.25;
			pose.local[index].rotation = Quat::from_rotation_z(0.5);
		}

		let before = capture_bone_transforms(&world, &entities);
		write_pose_indexed_world(&mut world, &pose, &entities);
		let after = capture_bone_transforms(&world, &entities);

		for index in 0..entities.len() {
			if partial_indices.contains(&index) {
				assert_ne!(before[index], after[index], "bone {index} should update");
			} else {
				assert_eq!(before[index], after[index], "bone {index} should stay at rest");
			}
		}

		reset_bone_transforms(&mut world, &entities);
		write_pose_by_name_world(&mut world, &pose, &names, &bone_map);
		let legacy = capture_bone_transforms(&world, &entities);

		reset_bone_transforms(&mut world, &entities);
		write_pose_indexed_world(&mut world, &pose, &entities);
		let indexed = capture_bone_transforms(&world, &entities);
		assert_eq!(legacy, indexed, "partial pose must match between write paths");
	}

	fn spawn_mailbox_apply_host(world: &mut World, clip: AnimClip, progress: f32) -> Entity {
		let definition = humanoid_v0_definition();
		let mut bone_map = HashMap::new();
		let mut entities = Vec::with_capacity(definition.len());
		for (index, name) in definition.names.iter().enumerate() {
			let rest = Transform {
				translation: Vec3::new(index as f32 * 0.01, 0.0, 0.0),
				rotation: Quat::from_rotation_x(index as f32 * 0.02),
				scale: Vec3::ONE,
			};
			let entity = world.spawn((AnimBone { name: RigName::from(*name), rest }, rest)).id();
			bone_map.insert(name.to_string(), entity);
			entities.push(entity);
		}
		let bone_entities = entities.clone().into_boxed_slice();

		let mut rig = HumanoidV0Rig::imported();
		rig.binding = RigBinding::from_rest(
			definition.clone(),
			bone_entities.clone(),
			PoseBuffer::identity(definition.len()),
		);
		rig.pose.copy_from(&rig.binding.effective_rest);

		let mut mailbox = AnimMailbox::with_bones(Transform::IDENTITY, definition.len());
		mailbox.bone_entities = bone_entities.clone();
		mailbox.output.copy_from(&rig.binding.effective_rest);
		mailbox.from_pose.copy_from(&rig.binding.effective_rest);
		mailbox.posed = true;
		mailbox.last = Some(clip.id());
		mailbox.clip_progress = progress;

		world
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, skeleton: RigSkeletonKind::Humanoid },
				AnimRefRoot(AnimRef::new(clip)),
				mailbox,
				BoneMap { by_name: bone_map },
				AnimateBones,
				AnimateEffects,
				rig,
				Transform::IDENTITY,
			))
			.id()
	}

	fn apply_mailbox_write_loop(world: &mut World, hosts: &[Entity], force_name_lookup: bool) {
		let only = world.resource::<MailboxApplySet>().only.clone();
		let definition = humanoid_v0_definition();
		let mut jobs = Vec::with_capacity(hosts.len());

		for host in hosts {
			if !allows_only(only.as_ref(), *host) {
				continue;
			}
			let Some(character_rig) = world.get::<CharacterRig>(*host) else {
				continue;
			};
			if character_rig.role != CharacterRigRole::Body {
				continue;
			}
			let mailbox = world.get::<AnimMailbox>(*host).expect("mailbox");
			jobs.push((
				mailbox.output.clone(),
				mailbox.bone_entities.clone(),
				world.get::<BoneMap>(*host).expect("bone map").clone(),
				force_name_lookup,
			));
		}

		for (output, bone_entities, bone_map, force_name_lookup) in jobs {
			let use_indexed = !force_name_lookup
				&& !bone_entities.is_empty()
				&& bone_entities.len() == output.local.len()
				&& bone_entities.iter().all(|entity| world.get::<Transform>(*entity).is_some());
			if use_indexed {
				write_pose_indexed_world(world, &output, &bone_entities);
			} else {
				write_pose_by_name_world(world, &output, &definition.names, &bone_map);
			}
		}
	}

	fn bench_clips() -> [AnimClip; 3] {
		[AnimClip::Still, AnimClip::walk(), AnimClip::run()]
	}

	fn bench_write_loop(use_name_lookup: bool) -> (u128, u128, u128) {
		const FRAMES: u32 = 2_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let clips = bench_clips();

		let mut worlds: Vec<(World, Entity)> = (0..CHARACTERS)
			.map(|index| {
				let mut world = World::new();
				world.init_resource::<MailboxApplySet>();
				world.resource_mut::<MailboxApplySet>().only = None;
				let clip = clips[index as usize % clips.len()];
				let progress = (index as f32 * 0.07).rem_euclid(1.0);
				let host = spawn_mailbox_apply_host(&mut world, clip, progress);
				(world, host)
			})
			.collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (world, host) in &mut worlds {
					let clip = clips[frame as usize % clips.len()];
					let progress = black_box(
						(frame as f32 * 0.013 + (host.to_bits() % 997) as f32 * 0.01)
							.rem_euclid(1.0),
					);
					let sampled = {
						let mut rig = world.get_mut::<HumanoidV0Rig>(*host).expect("rig");
						black_box(sample_humanoid(clip, &mut rig, progress, true, false));
						rig.pose.clone()
					};
					{
						let mut mailbox = world.get_mut::<AnimMailbox>(*host).expect("mailbox");
						publish_pose(&mut mailbox, &sampled, 1.0);
					}
					apply_mailbox_write_loop(world, &[*host], use_name_lookup);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}

		run_ns.sort_unstable();
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		(min, median, run_ns.iter().sum::<u128>() / run_ns.len() as u128)
	}

	fn bench_inner_write(use_name_lookup: bool) -> (u128, u128, u128) {
		const FRAMES: u32 = 10_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;

		let mut fixtures: Vec<_> = (0..CHARACTERS).map(|_| humanoid_write_fixture()).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (world, pose, bone_map, entities) in fixtures.iter_mut() {
					if use_name_lookup {
						write_pose_by_name_world(
							world,
							pose,
							&humanoid_v0_definition().names,
							bone_map,
						);
					} else {
						write_pose_indexed_world(world, pose, entities);
					}
					black_box(frame);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}

		run_ns.sort_unstable();
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		(min, median, run_ns.iter().sum::<u128>() / run_ns.len() as u128)
	}

	fn report_bench(label: &str, min: u128, median: u128, mean: u128) {
		eprintln!(
			"mailbox_write_microbench {label}: min={min} ns/sample median={median} ns/sample mean={mean} ns/sample"
		);
	}

	fn bench_prepared_clip_handle(use_clone: bool) -> Vec<u128> {
		use crate::clip_cache::RigVariantId;

		const FRAMES: u32 = 50_000;
		const RUNS: u32 = 5;
		let cache = AnimClipCache::default();
		let prepared = cache
			.prepare(AnimClip::walk(), RigVariantId::HUMANOID_V0, cache.settings.sampling)
			.expect("walk");

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				if use_clone {
					let handle = prepared.clone();
					black_box(handle.bone_mask());
				} else {
					black_box(prepared.bone_mask());
				}
				black_box(frame);
			}
			run_ns.push(start.elapsed().as_nanos() / FRAMES as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	fn bench_apply_set_lookup(use_clone: bool) -> Vec<u128> {
		const FRAMES: u32 = 50_000;
		const RUNS: u32 = 5;
		let mut selected = HashSet::new();
		for index in 1..=32 {
			selected.insert(Entity::from_bits(index as u64));
		}
		let set = MailboxApplySet { only: Some(selected) };

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				let entity = Entity::from_bits(((frame % 32) + 1) as u64);
				if use_clone {
					let only = set.only.clone();
					black_box(allows_only(only.as_ref(), entity));
				} else {
					black_box(allows_only(set.only.as_ref(), entity));
				}
			}
			run_ns.push(start.elapsed().as_nanos() / FRAMES as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	fn report_handle_bench(label: &str, run_ns: &[u128]) {
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		let mean = run_ns.iter().sum::<u128>() / run_ns.len() as u128;
		let spread = run_ns.last().expect("run") - min;
		eprintln!(
			"mailbox_handle_microbench {label}: runs={runs:?} min={min} median={median} mean={mean} spread={spread} ns/lookup",
			runs = run_ns,
		);
	}

	/// `cargo test -p character-motion mailbox_handle_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn mailbox_handle_microbench() {
		report_handle_bench("apply_set clone", &bench_apply_set_lookup(true));
		report_handle_bench("apply_set borrow", &bench_apply_set_lookup(false));
		report_handle_bench("prepared_clip clone", &bench_prepared_clip_handle(true));
		report_handle_bench("prepared_clip borrow", &bench_prepared_clip_handle(false));
	}

	/// Micro-benchmark for mailbox pose writes on a humanoid rig.
	/// Run with:
	/// `cargo test -p character-motion mailbox_write_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn mailbox_write_microbench() {
		eprintln!(
			"32 humanoid characters, Still/Walk/Run clips, 18 bones each; inner=write fn only, loop=sample+serial write path"
		);

		let (legacy_min, legacy_median, legacy_mean) = bench_inner_write(true);
		report_bench("inner legacy", legacy_min, legacy_median, legacy_mean);
		let (min, median, mean) = bench_inner_write(false);
		report_bench("inner indexed", min, median, mean);

		let (legacy_min, legacy_median, legacy_mean) = bench_write_loop(true);
		report_bench("apply_loop legacy", legacy_min, legacy_median, legacy_mean);
		let (min, median, mean) = bench_write_loop(false);
		report_bench("apply_loop indexed", min, median, mean);
	}

	#[test]
	fn cyclic_mailbox_playback_uses_prepared_path_almost_exclusively() -> anyhow::Result<()> {
		use anyhow::anyhow;

		use crate::clip_cache::RigVariantId;

		let cache = AnimClipCache::default();
		let clips = [AnimClip::still(), AnimClip::walk(), AnimClip::run()];
		let speeds = [0.2, 1.08, 1.68];
		let prepared: Vec<PreparedClip> = clips
			.iter()
			.filter_map(|clip| {
				cache.prepare(*clip, RigVariantId::HUMANOID_V0, cache.settings.sampling)
			})
			.collect();
		if prepared.len() != clips.len() {
			return Err(anyhow!("expected three prepared clips"));
		}

		let mut prepared_samples = 0u64;
		let mut live_samples = 0u64;
		const FRAMES: u32 = 3_600;
		const CHARACTERS: u32 = 32;
		let dt = 1.0 / 60.0;

		for character in 0..CHARACTERS {
			let clip_index = character as usize % clips.len();
			let speed = speeds[clip_index];
			let prepared = &prepared[clip_index];
			let mut time = character as f32 * 0.01;
			for _ in 0..FRAMES {
				time += dt * speed;
				if cache.sample(prepared, time).is_some() {
					prepared_samples += 1;
				} else {
					live_samples += 1;
				}
			}
		}

		let total = prepared_samples + live_samples;
		let live_pct = live_samples as f64 * 100.0 / total as f64;
		eprintln!(
			"mailbox prepared-path frequency: prepared={prepared_samples} live={live_samples} \
			({live_pct:.4}% live fallback over {CHARACTERS} chars × {FRAMES} frames)"
		);
		if live_samples > 0 {
			return Err(anyhow!(
				"default cache settings should not miss prepared bins for still/walk/run, got {live_samples} live samples"
			));
		}
		Ok(())
	}

	#[test]
	fn uncacheable_clips_and_disabled_cache_use_live_path() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let mut prepared_hits = 0u64;
		let mut live_hits = 0u64;
		let mut rig = HumanoidV0Rig::imported();

		let cache = AnimClipCache::uncached();
		let walk = AnimClip::walk();
		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		refresh_prepared_clip(&mut mailbox, walk, RigSkeletonKind::Humanoid, Some(&cache));
		if mailbox.prepared_clip.is_some() {
			return Err(anyhow!("disabled cache must not prepare"));
		}
		sample_humanoid_prepared(
			walk,
			&mut rig,
			0.25,
			true,
			false,
			Some(&cache),
			mailbox.prepared_clip.as_ref(),
		);
		live_hits += 1;

		let cache = AnimClipCache::default();
		refresh_prepared_clip(
			&mut mailbox,
			AnimClip::jab(),
			RigSkeletonKind::Humanoid,
			Some(&cache),
		);
		if mailbox.prepared_clip.is_some() {
			return Err(anyhow!("jab must not prepare"));
		}
		sample_humanoid_prepared(
			AnimClip::jab(),
			&mut rig,
			0.25,
			true,
			false,
			Some(&cache),
			mailbox.prepared_clip.as_ref(),
		);
		live_hits += 1;

		refresh_prepared_clip(&mut mailbox, walk, RigSkeletonKind::Humanoid, Some(&cache));
		let prepared = mailbox.prepared_clip.as_ref().ok_or_else(|| anyhow!("walk prepare"))?;
		if cache.sample(prepared, 0.25).is_some() {
			sample_humanoid_prepared(
				walk,
				&mut rig,
				0.25,
				true,
				false,
				Some(&cache),
				Some(prepared),
			);
			prepared_hits += 1;
		}

		eprintln!(
			"mailbox path selection smoke: prepared_hits={prepared_hits} live_hits={live_hits}"
		);
		assert_eq!(prepared_hits, 1);
		assert_eq!(live_hits, 2);
		Ok(())
	}

	#[test]
	fn tick_prepares_a_walk_handle_once() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let cache = AnimClipCache::default();
		let mut mailbox = AnimMailbox::new(Transform::IDENTITY);
		refresh_prepared_clip(
			&mut mailbox,
			AnimClip::walk(),
			RigSkeletonKind::Humanoid,
			Some(&cache),
		);
		let first = mailbox.prepared_clip.clone().ok_or_else(|| anyhow!("prepared walk"))?;
		refresh_prepared_clip(
			&mut mailbox,
			AnimClip::walk(),
			RigSkeletonKind::Humanoid,
			Some(&cache),
		);
		let second = mailbox.prepared_clip.as_ref().ok_or_else(|| anyhow!("still prepared"))?;
		if !first.ptr_eq(second) {
			return Err(anyhow!("the mailbox should retain the same prepared handle"));
		}
		if cache.stats().prepares != 1 {
			return Err(anyhow!("identical walk should prepare once, got {:?}", cache.stats()));
		}
		refresh_prepared_clip(
			&mut mailbox,
			AnimClip::jab(),
			RigSkeletonKind::Humanoid,
			Some(&cache),
		);
		if mailbox.prepared_clip.is_some() {
			return Err(anyhow!("uncacheable clips must drop the handle"));
		}
		Ok(())
	}
}
