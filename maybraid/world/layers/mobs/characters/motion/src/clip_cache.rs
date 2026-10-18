//! Prepared clip tables used by the mailbox.
//!
//! Low-level animations only evaluate a canonical sample when a variant is
//! constructed or a lazy idle bin is first touched. Warm playback is time-bin
//! calculation, indexed access, and applying stored rotation deltas onto rest.
//! Crossfade, aiming, and terrain correction stay outside this module.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

use bevy::prelude::*;
use character_animations::animations::{Idle, Run, Walk};
use character_animations::rigs::humanoid::write_masks::{
	IDLE_WRITE_BONES, RUN_WRITE_BONES, WALK_WRITE_BONES,
};
use character_animations::{
	finite_parameter_bits, interval_seconds, ClipTimePolicy, Effects, SampleAddress,
	DEFAULT_SAMPLE_INTERVAL_US,
};
use character_rigs::authoring::{
	humanoid_parent_rotation_delta, humanoid_write_mask, HumanoidPose, PoseBuffer,
	HUMANOID_V0_BONES,
};

use crate::clip::AnimClip;
use crate::rig::RigSkeletonKind;

/// Clip-content generation included in variant identity. Bump to drop stale tables.
pub const CURRENT_CLIP_REVISION: u64 = 0;

/// Default distinct prepared-clip cap.
pub const DEFAULT_MAX_VARIANTS: usize = 64;
/// Lazy idle slots so a long rest does not allocate an unbounded table.
pub const DEFAULT_MAX_UNBOUNDED_BINS: usize = 4096;

/// Compatible-rig identity. Humanoid V0 is the first prepared family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RigVariantId(pub u16);

impl RigVariantId {
	pub const HUMANOID_V0: Self = Self(1);
}

impl RigSkeletonKind {
	pub const fn sample_rig_variant(self) -> Option<RigVariantId> {
		match self {
			Self::Humanoid => Some(RigVariantId::HUMANOID_V0),
			Self::Quadruped | Self::Forelimbed | Self::Neck => None,
		}
	}
}

/// How a prepared clip maps continuous time onto its sample table.
pub type PlaybackMode = ClipTimePolicy;

/// Quantization used when a clip is prepared. Default is 10 ms bins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplingSettings {
	pub sample_interval_us: u32,
	pub max_unbounded_bins: usize,
}

impl Default for SamplingSettings {
	fn default() -> Self {
		Self {
			sample_interval_us: DEFAULT_SAMPLE_INTERVAL_US,
			max_unbounded_bins: DEFAULT_MAX_UNBOUNDED_BINS,
		}
	}
}

/// Interned complete parameter set. Equality is by stored values, not hash alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipParametersId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipParametersKey {
	Idle { arm_hang: u32, elbow_hang: u32, arm_sway: u32, neck_roll: u32, hip_shift: u32 },
	Walk { stride: u32, bounce: u32, rotation: u32 },
	Run { stride: u32, bounce: u32, rotation: u32 },
}

impl ClipParametersKey {
	pub fn idle(idle: &Idle) -> Option<Self> {
		Some(Self::Idle {
			arm_hang: finite_parameter_bits(idle.arm_hang)?,
			elbow_hang: finite_parameter_bits(idle.elbow_hang)?,
			arm_sway: finite_parameter_bits(idle.arm_sway)?,
			neck_roll: finite_parameter_bits(idle.neck_roll)?,
			hip_shift: finite_parameter_bits(idle.hip_shift)?,
		})
	}

	pub fn walk(walk: &Walk) -> Option<Self> {
		Some(Self::Walk {
			stride: finite_parameter_bits(walk.stride)?,
			bounce: finite_parameter_bits(walk.bounce)?,
			rotation: finite_parameter_bits(walk.rotation)?,
		})
	}

	pub fn run(run: &Run) -> Option<Self> {
		Some(Self::Run {
			stride: finite_parameter_bits(run.stride)?,
			bounce: finite_parameter_bits(run.bounce)?,
			rotation: finite_parameter_bits(run.rotation)?,
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipVariantKey {
	pub clip_id: crate::clip::AnimId,
	pub clip_revision: u64,
	pub parameters_id: ClipParametersId,
	pub rig_variant: RigVariantId,
	pub sample_interval_us: u32,
}

/// Evaluated parent-space rotation delta for one authored bone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvaluatedBoneOutput {
	pub bone: u16,
	pub rotation_delta: Quat,
}

/// One quantized sample of a clip's authored output, not a character's composite pose.
#[derive(Clone, Debug, PartialEq)]
pub struct CachedClipSample {
	pub bones: Box<[EvaluatedBoneOutput]>,
	pub effects: Effects,
}

enum SampleTable {
	Dense(Box<[CachedClipSample]>),
	Lazy {
		slots: Box<[OnceLock<CachedClipSample>]>,
		clip: AnimClip,
		write_bones: &'static [&'static str],
	},
}

/// Shared evaluated table for one clip variant.
pub struct CachedClip {
	pub sampling: SamplingSettings,
	pub playback_mode: PlaybackMode,
	pub bone_mask: u32,
	samples: SampleTable,
}

impl CachedClip {
	pub fn sample_count(&self) -> usize {
		match &self.samples {
			SampleTable::Dense(samples) => samples.len(),
			SampleTable::Lazy { slots, .. } => {
				slots.iter().filter(|slot| slot.get().is_some()).count()
			}
		}
	}

	fn sample(&self, clip_time: f32, evaluations: &AtomicU64) -> Option<&CachedClipSample> {
		let address = SampleAddress::from_clip_time(
			clip_time,
			self.sampling.sample_interval_us,
			self.playback_mode,
		);
		let index = address.index as usize;
		match &self.samples {
			SampleTable::Dense(samples) => samples.get(index),
			SampleTable::Lazy { slots, clip, write_bones } => {
				let slot = slots.get(index)?;
				if let Some(sample) = slot.get() {
					return Some(sample);
				}
				evaluations.fetch_add(1, Ordering::Relaxed);
				Some(
					slot.get_or_init(|| {
						evaluate_sample(*clip, write_bones, address.canonical_time)
					}),
				)
			}
		}
	}
}

/// Handle retained on the mailbox. Clone is an [`Arc`] bump; sampling does not lock the registry.
#[derive(Clone)]
pub struct PreparedClip {
	inner: Arc<CachedClip>,
}

impl PreparedClip {
	pub fn sampling(&self) -> SamplingSettings {
		self.inner.sampling
	}

	pub fn playback_mode(&self) -> PlaybackMode {
		self.inner.playback_mode
	}

	pub fn bone_mask(&self) -> u32 {
		self.inner.bone_mask
	}

	pub fn sample(&self, clip_time: f32) -> Option<&CachedClipSample> {
		self.inner.sample(clip_time, &NO_EVAL_COUNTER)
	}

	fn sample_counted<'a>(
		&'a self,
		clip_time: f32,
		evaluations: &AtomicU64,
	) -> Option<&'a CachedClipSample> {
		self.inner.sample(clip_time, evaluations)
	}

	pub fn ptr_eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.inner, &other.inner)
	}
}

static NO_EVAL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Registry of prepared clip tables. Warm sampling goes through [`PreparedClip`], not this lock.
#[derive(Resource)]
pub struct AnimClipCache {
	pub settings: AnimClipCacheSettings,
	inner: RwLock<CacheInner>,
	prepares: AtomicU64,
	hits: AtomicU64,
	misses: AtomicU64,
	evaluations: AtomicU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimClipCacheSettings {
	pub enabled: bool,
	pub sampling: SamplingSettings,
	pub max_variants: usize,
}

impl Default for AnimClipCacheSettings {
	fn default() -> Self {
		Self {
			enabled: true,
			sampling: SamplingSettings::default(),
			max_variants: DEFAULT_MAX_VARIANTS,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimClipCacheStats {
	pub prepares: u64,
	pub hits: u64,
	pub misses: u64,
	pub evaluations: u64,
	pub variants: usize,
	pub interned_parameters: usize,
	pub samples: usize,
}

struct ParameterRegistry {
	by_value: HashMap<ClipParametersKey, ClipParametersId>,
	by_id: Vec<ClipParametersKey>,
}

impl ParameterRegistry {
	fn intern(&mut self, key: ClipParametersKey) -> ClipParametersId {
		if let Some(&id) = self.by_value.get(&key) {
			return id;
		}
		let id = ClipParametersId(self.by_id.len() as u32);
		self.by_value.insert(key, id);
		self.by_id.push(key);
		id
	}

	fn get(&self, key: &ClipParametersKey) -> Option<ClipParametersId> {
		self.by_value.get(key).copied()
	}

	fn stored_eq(&self, id: ClipParametersId, key: &ClipParametersKey) -> bool {
		self.by_id.get(id.0 as usize).is_some_and(|stored| stored == key)
	}
}

struct CacheInner {
	params: ParameterRegistry,
	variants: HashMap<ClipVariantKey, Arc<CachedClip>>,
	order: Vec<ClipVariantKey>,
}

impl CacheInner {
	fn new() -> Self {
		Self {
			params: ParameterRegistry { by_value: HashMap::new(), by_id: Vec::new() },
			variants: HashMap::new(),
			order: Vec::new(),
		}
	}
}

impl Default for AnimClipCache {
	fn default() -> Self {
		Self {
			settings: AnimClipCacheSettings::default(),
			inner: RwLock::new(CacheInner::new()),
			prepares: AtomicU64::new(0),
			hits: AtomicU64::new(0),
			misses: AtomicU64::new(0),
			evaluations: AtomicU64::new(0),
		}
	}
}

impl AnimClipCache {
	pub fn with_settings(settings: AnimClipCacheSettings) -> Self {
		Self { settings, ..Self::default() }
	}

	pub fn uncached() -> Self {
		Self::with_settings(AnimClipCacheSettings { enabled: false, ..Default::default() })
	}

	fn lock_read(&self) -> std::sync::RwLockReadGuard<'_, CacheInner> {
		self.inner.read().unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	fn lock_write(&self) -> std::sync::RwLockWriteGuard<'_, CacheInner> {
		self.inner.write().unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	pub fn intern(&self, key: ClipParametersKey) -> ClipParametersId {
		{
			let inner = self.lock_read();
			if let Some(id) = inner.params.get(&key) {
				return id;
			}
		}
		self.lock_write().params.intern(key)
	}

	pub fn parameters_eq(&self, id: ClipParametersId, key: &ClipParametersKey) -> bool {
		self.lock_read().params.stored_eq(id, key)
	}

	/// Build or reuse a prepared table. Hashing and evaluation happen here, not on warm sample.
	pub fn prepare(
		&self,
		clip: AnimClip,
		rig_variant: RigVariantId,
		sampling: SamplingSettings,
	) -> Option<PreparedClip> {
		if !self.settings.enabled || !clip.is_sample_cacheable() {
			return None;
		}
		if rig_variant != RigVariantId::HUMANOID_V0 {
			return None;
		}
		let parameters = parameters_key(clip)?;
		let parameters_id = self.intern(parameters);
		let variant = ClipVariantKey {
			clip_id: clip.id(),
			clip_revision: clip.revision(),
			parameters_id,
			rig_variant,
			sample_interval_us: sampling.sample_interval_us,
		};
		{
			let inner = self.lock_read();
			if let Some(existing) = inner.variants.get(&variant) {
				self.hits.fetch_add(1, Ordering::Relaxed);
				return Some(PreparedClip { inner: Arc::clone(existing) });
			}
		}
		self.prepares.fetch_add(1, Ordering::Relaxed);
		self.misses.fetch_add(1, Ordering::Relaxed);
		let built = Arc::new(build_cached_clip(clip, sampling, &self.evaluations));
		let mut inner = self.lock_write();
		if let Some(existing) = inner.variants.get(&variant) {
			self.hits.fetch_add(1, Ordering::Relaxed);
			return Some(PreparedClip { inner: Arc::clone(existing) });
		}
		if inner.variants.len() >= self.settings.max_variants {
			if let Some(oldest) = inner.order.first().copied() {
				inner.variants.remove(&oldest);
				inner.order.remove(0);
			}
		}
		inner.order.push(variant);
		inner.variants.insert(variant, Arc::clone(&built));
		Some(PreparedClip { inner: built })
	}

	/// Indexed sample from a prepared handle. Does not lock the registry.
	pub fn sample<'a>(
		&self,
		prepared: &'a PreparedClip,
		clip_time: f32,
	) -> Option<&'a CachedClipSample> {
		prepared.sample_counted(clip_time, &self.evaluations)
	}

	pub fn stats(&self) -> AnimClipCacheStats {
		let inner = self.lock_read();
		AnimClipCacheStats {
			prepares: self.prepares.load(Ordering::Relaxed),
			hits: self.hits.load(Ordering::Relaxed),
			misses: self.misses.load(Ordering::Relaxed),
			evaluations: self.evaluations.load(Ordering::Relaxed),
			variants: inner.variants.len(),
			interned_parameters: inner.params.by_id.len(),
			samples: inner.variants.values().map(|clip| clip.sample_count()).sum(),
		}
	}

	pub fn retained_bytes(&self) -> usize {
		self.stats().samples * std::mem::size_of::<CachedClipSample>()
	}

	pub fn invalidate_clip(&self, clip_id: crate::clip::AnimId) {
		let mut inner = self.lock_write();
		inner.variants.retain(|key, _| key.clip_id != clip_id);
		inner.order.retain(|key| key.clip_id != clip_id);
	}

	pub fn clear(&self) {
		let mut inner = self.lock_write();
		inner.variants.clear();
		inner.order.clear();
	}
}

/// Apply a cached sample onto `rest`. Unmasked bones stay at the binding baseline.
///
/// Copies only unmasked bones from `rest`, then applies rotation deltas for
/// masked bones. Avoids a full-buffer `copy_from` when the clip touches a
/// strict subset of the rig (idle / walk / run tables).
pub fn apply_evaluated_sample(
	rest: &PoseBuffer,
	bone_mask: u32,
	sample: &CachedClipSample,
	out: &mut PoseBuffer,
) {
	let n = rest.local.len().min(out.local.len());
	for index in 0..n {
		if bone_mask & (1u32 << index) == 0 {
			out.local[index] = rest.local[index];
		}
	}
	for bone in sample.bones.iter() {
		let index = bone.bone as usize;
		let Some(rest_tf) = rest.local.get(index) else {
			continue;
		};
		let Some(slot) = out.local.get_mut(index) else {
			continue;
		};
		slot.translation = rest_tf.translation;
		slot.rotation = bone.rotation_delta * rest_tf.rotation;
		slot.scale = rest_tf.scale;
	}
}

pub fn parameters_key(clip: AnimClip) -> Option<ClipParametersKey> {
	match clip {
		AnimClip::Still => ClipParametersKey::idle(&Idle::default()),
		AnimClip::Walk(walk) => ClipParametersKey::walk(&walk),
		AnimClip::Run(run) => ClipParametersKey::run(&run),
		_ => None,
	}
}

pub fn clip_write_bones(clip: AnimClip) -> Option<&'static [&'static str]> {
	match clip {
		AnimClip::Still => Some(IDLE_WRITE_BONES),
		AnimClip::Walk(_) => Some(WALK_WRITE_BONES),
		AnimClip::Run(_) => Some(RUN_WRITE_BONES),
		_ => None,
	}
}

pub fn clip_bone_mask(clip: AnimClip) -> Option<u32> {
	clip_write_bones(clip).map(humanoid_write_mask)
}

fn build_cached_clip(
	clip: AnimClip,
	sampling: SamplingSettings,
	evaluations: &AtomicU64,
) -> CachedClip {
	let write_bones = clip_write_bones(clip).unwrap_or(&[]);
	let bone_mask = humanoid_write_mask(write_bones);
	let playback_mode = clip.time_policy();
	let samples = if let Some(len) = playback_mode.table_len(sampling.sample_interval_us) {
		let interval = interval_seconds(sampling.sample_interval_us);
		let table: Box<[CachedClipSample]> = (0..len)
			.map(|index| {
				evaluations.fetch_add(1, Ordering::Relaxed);
				evaluate_sample(clip, write_bones, index as f32 * interval)
			})
			.collect();
		SampleTable::Dense(table)
	} else {
		let slots = (0..sampling.max_unbounded_bins).map(|_| OnceLock::new()).collect();
		SampleTable::Lazy { slots, clip, write_bones }
	};
	CachedClip { sampling, playback_mode, bone_mask, samples }
}

fn evaluate_sample(clip: AnimClip, write_bones: &[&str], canonical_time: f32) -> CachedClipSample {
	let pose = match clip {
		AnimClip::Still => Idle::default().sample_pose(canonical_time),
		AnimClip::Walk(walk) => walk.sample_pose(canonical_time),
		AnimClip::Run(run) => run.sample_pose(canonical_time),
		_ => HumanoidPose::default(),
	};
	let bones: Box<[EvaluatedBoneOutput]> = write_bones
		.iter()
		.filter_map(|name| {
			let bone = HUMANOID_V0_BONES.iter().position(|candidate| *candidate == *name)? as u16;
			let rotation_delta = humanoid_parent_rotation_delta(name, &pose)?;
			Some(EvaluatedBoneOutput { bone, rotation_delta })
		})
		.collect();
	CachedClipSample { bones, effects: Effects::IDENTITY }
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_animations::Animation;
	use character_rigs::authoring::humanoid_bone_bit;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use std::hint::black_box;
	use std::time::Instant;

	/// Pre-change path: full rest copy then masked rotation deltas.
	fn apply_evaluated_sample_legacy(
		rest: &PoseBuffer,
		sample: &CachedClipSample,
		out: &mut PoseBuffer,
	) {
		out.copy_from(rest);
		for bone in sample.bones.iter() {
			let index = bone.bone as usize;
			let Some(slot) = out.local.get_mut(index) else {
				continue;
			};
			let Some(rest_tf) = rest.local.get(index) else {
				continue;
			};
			slot.rotation = bone.rotation_delta * rest_tf.rotation;
		}
	}

	fn prepare_walk(cache: &AnimClipCache) -> anyhow::Result<PreparedClip> {
		cache
			.prepare(AnimClip::walk(), RigVariantId::HUMANOID_V0, cache.settings.sampling)
			.ok_or_else(|| anyhow::anyhow!("expected prepared walk"))
	}

	#[test]
	fn intern_compares_complete_values_and_signed_zero() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let walk = Walk { stride: 0.0, bounce: 1.0, rotation: 1.0 };
		let signed_zero = Walk { stride: -0.0, bounce: 1.0, rotation: 1.0 };
		let other = Walk { stride: 0.8, bounce: 1.0, rotation: 1.0 };
		let a = ClipParametersKey::walk(&walk).ok_or_else(|| anyhow::anyhow!("finite walk"))?;
		let z =
			ClipParametersKey::walk(&signed_zero).ok_or_else(|| anyhow::anyhow!("signed zero"))?;
		let b = ClipParametersKey::walk(&other).ok_or_else(|| anyhow::anyhow!("other walk"))?;
		let id_a = cache.intern(a);
		if id_a != cache.intern(z) {
			anyhow::bail!("signed zero must intern with +0");
		}
		if id_a == cache.intern(b) {
			anyhow::bail!("different strides must not share a parameters id");
		}
		if !cache.parameters_eq(id_a, &a) || cache.parameters_eq(id_a, &b) {
			anyhow::bail!("intern identity must compare stored values");
		}
		Ok(())
	}

	#[test]
	fn prepare_reuses_the_same_table() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let first = prepare_walk(&cache)?;
		let second = prepare_walk(&cache)?;
		if !first.ptr_eq(&second) {
			anyhow::bail!("identical walk variants must share one prepared table");
		}
		if cache.stats().evaluations != 100 {
			anyhow::bail!("walk should precompute 100 bins, got {:?}", cache.stats());
		}
		if cache.stats().prepares != 1 || cache.stats().hits != 1 {
			anyhow::bail!("second prepare should hit, got {:?}", cache.stats());
		}
		Ok(())
	}

	#[test]
	fn warm_sample_is_indexed_and_canonical() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let prepared = prepare_walk(&cache)?;
		let evals = cache.stats().evaluations;
		let first = cache.sample(&prepared, 0.121).ok_or_else(|| anyhow::anyhow!("sample"))?;
		let second = cache.sample(&prepared, 0.123).ok_or_else(|| anyhow::anyhow!("sample"))?;
		if !std::ptr::eq(first, second) {
			anyhow::bail!("times in one bin must return the same table slot");
		}
		if cache.stats().evaluations != evals {
			anyhow::bail!("warm sample must not evaluate, got {:?}", cache.stats());
		}
		Ok(())
	}

	#[test]
	fn different_parameters_get_separate_tables() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let walk = prepare_walk(&cache)?;
		let long = cache
			.prepare(
				AnimClip::Walk(Walk { stride: 0.9, bounce: 1.0, rotation: 1.0 }),
				RigVariantId::HUMANOID_V0,
				cache.settings.sampling,
			)
			.ok_or_else(|| anyhow::anyhow!("long walk"))?;
		if walk.ptr_eq(&long) {
			anyhow::bail!("different strides must not share a table");
		}
		Ok(())
	}

	#[test]
	fn deltas_match_live_resolve_and_respect_rest() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let prepared = prepare_walk(&cache)?;
		let sample = cache.sample(&prepared, 0.2).ok_or_else(|| anyhow::anyhow!("sample"))?;
		let mut live = HumanoidV0Rig::imported();
		let mut cached = HumanoidV0Rig::imported();
		Walk::default().apply_for(&mut live, 0.2);
		apply_evaluated_sample(
			&cached.binding.effective_rest,
			prepared.bone_mask(),
			sample,
			&mut cached.pose,
		);
		if live.rotation("femur.L").angle_between(cached.rotation("femur.L")) > 1e-4 {
			anyhow::bail!("cached deltas must match live walk compose");
		}
		let mut edited = HumanoidV0Rig::imported();
		edited.seed_rest("femur.L", Transform::from_rotation(Quat::from_rotation_x(0.15)));
		apply_evaluated_sample(
			&edited.binding.effective_rest,
			prepared.bone_mask(),
			sample,
			&mut edited.pose,
		);
		if cached.rotation("femur.L") == edited.rotation("femur.L") {
			anyhow::bail!("deltas must compose onto each character's rest");
		}
		Ok(())
	}

	#[test]
	fn idle_mask_leaves_femur_on_the_rest_baseline() -> anyhow::Result<()> {
		let Some(mask) = clip_bone_mask(AnimClip::still()) else {
			anyhow::bail!("idle mask");
		};
		let Some(femur) = humanoid_bone_bit("femur.L") else {
			anyhow::bail!("femur bit");
		};
		if mask & femur != 0 {
			anyhow::bail!("idle must not claim the femur");
		}
		let cache = AnimClipCache::default();
		let prepared = cache
			.prepare(AnimClip::still(), RigVariantId::HUMANOID_V0, cache.settings.sampling)
			.ok_or_else(|| anyhow::anyhow!("idle"))?;
		let sample = cache.sample(&prepared, 0.25).ok_or_else(|| anyhow::anyhow!("sample"))?;
		let mut leftover = HumanoidV0Rig::imported();
		Walk::default().apply_for(&mut leftover, 0.2);
		apply_evaluated_sample(
			&leftover.binding.effective_rest,
			prepared.bone_mask(),
			sample,
			&mut leftover.pose,
		);
		if leftover.posed_angle("femur.L") > 1e-5 {
			anyhow::bail!("idle apply uses rest as baseline");
		}
		if leftover.posed_angle("humerus.L") < 0.2 {
			anyhow::bail!("idle should still hang the arms");
		}
		Ok(())
	}

	#[test]
	fn walk_loop_seam_reuses_bin_zero() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let prepared = prepare_walk(&cache)?;
		let a = cache.sample(&prepared, 0.996).ok_or_else(|| anyhow::anyhow!("seam"))?;
		let b = cache.sample(&prepared, 0.0).ok_or_else(|| anyhow::anyhow!("zero"))?;
		if !std::ptr::eq(a, b) {
			anyhow::bail!("loop seam must index bin 0");
		}
		Ok(())
	}

	#[test]
	fn invalidation_drops_registry_but_handles_stay_live() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let prepared = prepare_walk(&cache)?;
		cache.invalidate_clip(AnimClip::walk().id());
		if cache.stats().variants != 0 {
			anyhow::bail!("walk should be gone from the registry");
		}
		if cache.sample(&prepared, 0.3).is_none() {
			anyhow::bail!("existing handles must keep their table after invalidation");
		}
		Ok(())
	}

	#[test]
	fn disabled_cache_skips_prepare() {
		let cache = AnimClipCache::uncached();
		assert!(cache
			.prepare(AnimClip::walk(), RigVariantId::HUMANOID_V0, cache.settings.sampling)
			.is_none());
	}

	#[test]
	fn idle_fills_lazy_bins_once() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let prepared = cache
			.prepare(AnimClip::still(), RigVariantId::HUMANOID_V0, cache.settings.sampling)
			.ok_or_else(|| anyhow::anyhow!("idle"))?;
		if cache.stats().evaluations != 0 {
			anyhow::bail!("idle should not precompute a dense table");
		}
		let first = cache.sample(&prepared, 0.25).ok_or_else(|| anyhow::anyhow!("first"))?;
		let after_miss = cache.stats().evaluations;
		if after_miss != 1 {
			anyhow::bail!("first idle bin should evaluate once, got {after_miss}");
		}
		let second = cache.sample(&prepared, 0.254).ok_or_else(|| anyhow::anyhow!("second"))?;
		if !std::ptr::eq(first, second) {
			anyhow::bail!("the same idle bin must reuse the OnceLock");
		}
		if cache.stats().evaluations != after_miss {
			anyhow::bail!("warm idle sample must not evaluate again");
		}
		Ok(())
	}

	#[test]
	fn idle_beyond_capacity_falls_back() -> anyhow::Result<()> {
		let cache = AnimClipCache::with_settings(AnimClipCacheSettings {
			sampling: SamplingSettings { sample_interval_us: 10_000, max_unbounded_bins: 4 },
			..Default::default()
		});
		let prepared = cache
			.prepare(AnimClip::still(), RigVariantId::HUMANOID_V0, cache.settings.sampling)
			.ok_or_else(|| anyhow::anyhow!("idle"))?;
		if cache.sample(&prepared, 0.01).is_none() {
			anyhow::bail!("bin 1 should be in range");
		}
		if cache.sample(&prepared, 1.0).is_some() {
			anyhow::bail!("time past the lazy table must miss so the mailbox can sample live");
		}
		Ok(())
	}

	#[test]
	fn playback_speeds_share_bins_and_stay_near_live_pose() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let prepared = prepare_walk(&cache)?;
		let mut live = HumanoidV0Rig::imported();
		let mut quantized = HumanoidV0Rig::imported();
		for speed in [1.08, 0.3, 2.4] {
			let mut clip_time = 0.0;
			for _ in 0..40 {
				clip_time += 1.0 / 60.0 * speed;
				let sample =
					cache.sample(&prepared, clip_time).ok_or_else(|| anyhow::anyhow!("sample"))?;
				Walk::default().apply_for(&mut live, clip_time);
				apply_evaluated_sample(
					&quantized.binding.effective_rest,
					prepared.bone_mask(),
					sample,
					&mut quantized.pose,
				);
				let delta = live.posed_angle("femur.L") - quantized.posed_angle("femur.L");
				if delta.abs() > 0.08 {
					anyhow::bail!("10 ms bins should stay close to live walk at speed {speed}");
				}
			}
		}
		Ok(())
	}

	#[test]
	fn benchmark_prepared_clip_against_live() -> anyhow::Result<()> {
		const CHARACTERS: usize = 64;
		const FRAMES: usize = 90;
		let dt = 1.0 / 60.0;
		let clips = [AnimClip::still(), AnimClip::walk(), AnimClip::run()];
		let speeds = [0.2, 1.08, 1.68];

		let uncached_start = Instant::now();
		for (clip, speed) in clips.into_iter().zip(speeds) {
			for character in 0..CHARACTERS {
				let mut rig = HumanoidV0Rig::imported();
				let mut time = character as f32 * 0.01;
				for _ in 0..FRAMES {
					time += dt * speed;
					match clip {
						AnimClip::Still => Idle::default().apply_for(&mut rig, time),
						AnimClip::Walk(walk) => walk.apply_for(&mut rig, time),
						AnimClip::Run(run) => run.apply_for(&mut rig, time),
						_ => {}
					}
				}
			}
		}
		let uncached_ms = uncached_start.elapsed().as_secs_f64() * 1_000.0;

		let cache = AnimClipCache::default();
		let prepare_start = Instant::now();
		let prepared: Vec<PreparedClip> = clips
			.into_iter()
			.filter_map(|clip| {
				cache.prepare(clip, RigVariantId::HUMANOID_V0, cache.settings.sampling)
			})
			.collect();
		let prepare_ms = prepare_start.elapsed().as_secs_f64() * 1_000.0;
		if prepared.len() != 3 {
			anyhow::bail!("expected three prepared clips");
		}

		let warm_start = Instant::now();
		for (prepared, speed) in prepared.iter().zip(speeds) {
			for character in 0..CHARACTERS {
				let mut rig = HumanoidV0Rig::imported();
				let mut time = character as f32 * 0.01;
				for _ in 0..FRAMES {
					time += dt * speed;
					let Some(sample) = cache.sample(prepared, time) else {
						continue;
					};
					apply_evaluated_sample(
						&rig.binding.effective_rest,
						prepared.bone_mask(),
						sample,
						&mut rig.pose,
					);
				}
			}
		}
		let warm_ms = warm_start.elapsed().as_secs_f64() * 1_000.0;
		let stats = cache.stats();
		eprintln!(
			"prepared clip cache {CHARACTERS} chars × {FRAMES} frames × 3 clips: \
			uncached {uncached_ms:.2}ms, prepare {prepare_ms:.3}ms, warm {warm_ms:.2}ms, \
			evals {}, variants {}, samples {}, bytes {}",
			stats.evaluations,
			stats.variants,
			stats.samples,
			cache.retained_bytes()
		);
		Ok(())
	}

	#[test]
	fn fused_apply_matches_legacy_for_still_walk_and_run() -> anyhow::Result<()> {
		let cache = AnimClipCache::default();
		let clips = [AnimClip::still(), AnimClip::walk(), AnimClip::run()];
		let progresses = [0.0, 0.17, 0.42, 0.88];
		for clip in clips {
			let prepared = cache
				.prepare(clip, RigVariantId::HUMANOID_V0, cache.settings.sampling)
				.ok_or_else(|| anyhow::anyhow!("prepare {clip:?}"))?;
			let mask = prepared.bone_mask();
			for progress in progresses {
				let sample =
					cache.sample(&prepared, progress).ok_or_else(|| anyhow::anyhow!("sample"))?;
				let rest = HumanoidV0Rig::imported().binding.effective_rest.clone();
				let mut legacy = PoseBuffer::identity(rest.len());
				let mut fused = PoseBuffer::identity(rest.len());
				apply_evaluated_sample_legacy(&rest, sample, &mut legacy);
				apply_evaluated_sample(&rest, mask, sample, &mut fused);
				if legacy.local != fused.local {
					anyhow::bail!("pose mismatch for {clip:?} at {progress}");
				}
			}
		}
		Ok(())
	}

	fn bench_prepared_sample_loop(use_legacy: bool) -> Vec<u128> {
		const FRAMES: u32 = 2_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let clips = [AnimClip::still(), AnimClip::walk(), AnimClip::run()];
		let cache = AnimClipCache::default();
		let prepared: Vec<PreparedClip> = clips
			.iter()
			.filter_map(|clip| {
				cache.prepare(*clip, RigVariantId::HUMANOID_V0, cache.settings.sampling)
			})
			.collect();

		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let prepared = &prepared[index as usize % prepared.len()];
					let progress =
						black_box((frame as f32 * 0.013 + (index as f32 * 0.07)).rem_euclid(1.0));
					let sample = cache.sample(prepared, progress).expect("prepared sample");
					let rest = &rig.binding.effective_rest;
					if use_legacy {
						apply_evaluated_sample_legacy(rest, sample, &mut rig.pose);
					} else {
						apply_evaluated_sample(rest, prepared.bone_mask(), sample, &mut rig.pose);
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}

		run_ns.sort_unstable();
		run_ns
	}

	fn report_apply_bench(label: &str, run_ns: &[u128]) {
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		let mean = run_ns.iter().sum::<u128>() / run_ns.len() as u128;
		let spread = run_ns.last().expect("run") - min;
		eprintln!(
			"apply_evaluated_sample_microbench {label}: runs={runs:?} min={min} median={median} mean={mean} spread={spread} ns/sample",
			runs = run_ns,
		);
	}

	/// `cargo test -p character-motion apply_evaluated_sample_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn apply_evaluated_sample_microbench() {
		eprintln!(
			"32 humanoid characters, Still/Walk/Run prepared clips, real cache.sample + apply path"
		);
		report_apply_bench("legacy copy+delta", &bench_prepared_sample_loop(true));
		report_apply_bench("fused mask pass", &bench_prepared_sample_loop(false));
	}
}
