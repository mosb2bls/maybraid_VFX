//! Buff-deviation firearm generation.
//!
//! Contributors ([`FirearmBuff`]) add bin weight and shift axis mean / scale
//! deviation. Sampling is staged: family → load → fire kind → realize.

use crate::{
	names::mix, BoltMaterial, FireMode, FirearmBarrel, FirearmGrip, FirearmMaterial, FirearmMesh,
	FirearmScales, FirearmSight, FirearmSpec, FirearmStats, FirearmStock, FirearmTriggerBox,
	ItemColor, ItemRng, ProjectileKind, SlotLook, SlotScale, IRON_SIGHT_FOV,
};

const FIREARM_SEED: u64 = 0xF1A4_A11A_0002_C0DE;

const FAMILY_LASER: f32 = 1.0;
const FAMILY_RATE_OF_FIRE: f32 = 11.0;

const PEN_PER_LENGTH_TENTH: f32 = 0.10;
const BARREL_RANGE_PER_TENTH: f32 = 25.0;
const DPC_PER_THICK_TENTH: f32 = 0.8;
const WEIGHT_PER_THICK_TENTH: f32 = 0.35;
const WEIGHT_PER_LENGTH_TENTH: f32 = 0.25;
const COLOR_SD_GAIN: f32 = 0.4;
const RECOIL_MEAN: f32 = 1.9;
const RECOIL_MIN: f32 = 0.25;
const RECOIL_MAX: f32 = 8.0;
const RECOIL_MEAN_BLUE: f32 = -0.25;
const RECOIL_MEAN_GRIP: f32 = -0.25;
const RECOIL_MEAN_FROM_DAMAGE: f32 = 0.05;
const RECOIL_SD_FROM_DAMAGE: f32 = 1.025;

/// Mean and standard deviation for one continuous axis.
#[derive(Clone, Copy, Debug)]
pub struct Dist {
	pub mean: f32,
	pub sd: f32,
}

impl Dist {
	pub const fn new(mean: f32, sd: f32) -> Self {
		Self { mean, sd }
	}

	pub(crate) fn apply(self, delta: DistDelta) -> Self {
		let scaled = self.sd * delta.mul_sd;
		Self {
			mean: self.mean + delta.add_mean,
			sd: if scaled.is_finite() { scaled.max(0.01) } else { 0.01 },
		}
	}

	/// Sample \(\mathcal{N}(\mu, \sigma)\) into `[min, max]`. Non-finite or inverted
	/// bounds fall back instead of panicking in `f32::clamp`.
	pub(crate) fn sample_clamped(self, rng: &mut ItemRng, min: f32, max: f32) -> f32 {
		let lo = if min.is_finite() { min } else { 0.0 };
		let hi = if max.is_finite() { max } else { lo };
		let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
		let mean = if self.mean.is_finite() { self.mean } else { lo.midpoint(hi) };
		let sd = if self.sd.is_finite() { self.sd.max(0.0) } else { 0.0 };
		let sample = rng.sample_normal(mean, sd);
		if sample.is_finite() {
			sample.clamp(lo, hi)
		} else {
			mean.clamp(lo, hi)
		}
	}
}

/// Additive mean and multiplicative deviation contributed before sampling.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DistDelta {
	pub add_mean: f32,
	pub mul_sd: f32,
}

impl DistDelta {
	pub(crate) const IDENTITY: Self = Self { add_mean: 0.0, mul_sd: 1.0 };

	pub(crate) fn add_mean(&mut self, delta: f32) {
		self.add_mean += delta;
	}

	pub(crate) fn mul_sd(&mut self, factor: f32) {
		self.mul_sd *= factor;
	}
}

fn add_damage(priors: &mut FirearmPriors, dpc: f32) {
	priors.dpc.add_mean(dpc);
	priors.recoil.add_mean(RECOIL_MEAN_FROM_DAMAGE);
	priors.recoil.mul_sd(RECOIL_SD_FROM_DAMAGE);
}

/// Categorical weights. Sampled with `weight / sum`.
#[derive(Clone, Copy, Debug)]
struct FirearmBins {
	laser: f32,
	rate_of_fire: f32,
	bolt: f32,
	bullet: f32,
	auto: f32,
	semi: f32,
	burst: f32,
	gated: f32,
}

impl FirearmBins {
	fn base() -> Self {
		Self {
			laser: FAMILY_LASER,
			rate_of_fire: FAMILY_RATE_OF_FIRE,
			bolt: 1.0,
			bullet: 1.0,
			auto: 1.0,
			semi: 1.0,
			burst: 1.0,
			gated: 1.0,
		}
	}
}

/// Accumulated priors. Bins are sampled first; axis deltas apply to the
/// mode-selected bases.
#[derive(Clone, Copy, Debug)]
pub struct FirearmPriors {
	bins: FirearmBins,
	rpm: DistDelta,
	burst: DistDelta,
	burst_rpm: DistDelta,
	refresh: DistDelta,
	dpc: DistDelta,
	speed: DistDelta,
	pen: DistDelta,
	range: DistDelta,
	recoil: DistDelta,
	weight: DistDelta,
}

impl FirearmPriors {
	fn new() -> Self {
		Self {
			bins: FirearmBins::base(),
			rpm: DistDelta::IDENTITY,
			burst: DistDelta::IDENTITY,
			burst_rpm: DistDelta::IDENTITY,
			refresh: DistDelta::IDENTITY,
			dpc: DistDelta::IDENTITY,
			speed: DistDelta::IDENTITY,
			pen: DistDelta::IDENTITY,
			range: DistDelta::IDENTITY,
			recoil: DistDelta::IDENTITY,
			weight: DistDelta::IDENTITY,
		}
	}
}

/// Something that shifts firearm bins and/or axis moments.
pub trait FirearmBuff {
	fn contribute(&self, priors: &mut FirearmPriors);
}

impl FirearmBuff for FirearmSpec {
	fn contribute(&self, priors: &mut FirearmPriors) {
		self.kit.body.contribute(priors);
		self.looks.body.contribute(priors);
		self.kit.barrel.contribute(priors);
		if !matches!(self.kit.barrel, FirearmBarrel::None) {
			self.looks.barrel.contribute(priors);
		}
		self.kit.grip.contribute(priors);
		if !matches!(self.kit.grip, FirearmGrip::None) {
			self.looks.grip.contribute(priors);
		}
		self.kit.trigger_box.contribute(priors);
		if !matches!(self.kit.trigger_box, FirearmTriggerBox::None) {
			self.looks.trigger_box.contribute(priors);
		}
		self.kit.stock.contribute(priors);
		if !matches!(self.kit.stock, FirearmStock::None) {
			self.looks.stock.contribute(priors);
		}
		self.kit.sight.contribute(priors);
		if !matches!(self.kit.sight, FirearmSight::None) {
			self.looks.sight.contribute(priors);
		}
		self.scales.contribute(priors);
		self.bolt.contribute(priors);
	}
}

impl FirearmBuff for SlotLook {
	fn contribute(&self, priors: &mut FirearmPriors) {
		self.material.contribute(priors);
		self.color.contribute(priors);
	}
}

impl FirearmBuff for FirearmMesh {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::Bullpup => {
				priors.bins.auto += 0.8;
				priors.bins.burst += 0.3;
				priors.bins.laser -= 0.2;
				priors.rpm.add_mean(50.0);
				priors.rpm.mul_sd(1.1);
				priors.dpc.mul_sd(1.1);
				priors.pen.add_mean(-0.10);
				priors.pen.mul_sd(0.9);
				priors.speed.add_mean(20.0);
				priors.range.add_mean(-40.0);
				priors.weight.add_mean(-2.0);
			}
			Self::Silopup => {
				priors.bins.bullet += 0.6;
				priors.bins.auto += 0.2;
				priors.bins.semi += 0.4;
				priors.speed.add_mean(-15.0);
				priors.range.add_mean(-30.0);
				priors.recoil.add_mean(-1.2);
				priors.recoil.mul_sd(0.85);
				priors.dpc.add_mean(-2.0);
			}
			Self::Reltor => {
				priors.bins.burst += 0.5;
				priors.bins.auto += 0.3;
				priors.bins.semi += 0.2;
				priors.rpm.mul_sd(1.15);
				add_damage(priors, 1.0);
				priors.recoil.add_mean(0.5);
				priors.weight.add_mean(2.0);
			}
			Self::Samsonist => {
				priors.bins.gated += 1.2;
				priors.bins.semi += 0.4;
				priors.bins.auto -= 0.4;
				priors.bins.laser -= 0.1;
				priors.pen.add_mean(0.12);
				priors.range.add_mean(120.0);
				priors.speed.add_mean(40.0);
				priors.rpm.add_mean(-80.0);
				add_damage(priors, 6.0);
				priors.dpc.mul_sd(1.2);
				priors.weight.add_mean(6.0);
			}
			Self::Snailer => {
				priors.bins.laser += 2.0;
				priors.bins.gated += 0.5;
				priors.bins.auto -= 0.3;
				priors.range.add_mean(4.0);
				add_damage(priors, 2.0);
				priors.speed.add_mean(-20.0);
				priors.weight.add_mean(-1.0);
			}
		}
	}
}

impl FirearmBuff for FirearmBarrel {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::None => {}
			Self::Bullpup => {
				priors.bins.auto += 0.2;
				priors.range.add_mean(20.0);
				priors.pen.add_mean(0.02);
			}
			Self::Laznard => {
				priors.bins.laser += 0.8;
				priors.bins.gated += 0.3;
				priors.range.add_mean(3.0);
				priors.speed.add_mean(15.0);
				add_damage(priors, 2.0);
			}
		}
	}
}

impl FirearmBuff for FirearmGrip {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::None => {
				priors.bins.semi += 0.1;
				priors.recoil.mul_sd(1.1);
			}
			Self::BumpHandle => {
				priors.bins.auto += 0.3;
				priors.bins.burst += 0.2;
				priors.recoil.add_mean(RECOIL_MEAN_GRIP);
				priors.recoil.mul_sd(0.9);
				priors.rpm.add_mean(20.0);
			}
		}
	}
}

impl FirearmBuff for FirearmTriggerBox {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::None => {
				priors.refresh.mul_sd(1.1);
			}
			Self::Keelripe => {
				priors.bins.auto += 0.6;
				priors.bins.burst += 0.3;
				priors.rpm.add_mean(40.0);
				priors.dpc.add_mean(-1.0);
			}
			Self::Paddle => {
				priors.bins.semi += 0.8;
				priors.bins.gated += 0.2;
				add_damage(priors, 4.0);
				priors.recoil.add_mean(0.6);
			}
			Self::Reltor => {
				priors.bins.burst += 0.7;
				priors.bins.auto += 0.2;
				priors.burst.add_mean(0.3);
				priors.rpm.add_mean(30.0);
			}
		}
	}
}

impl FirearmBuff for FirearmStock {
	fn contribute(&self, _priors: &mut FirearmPriors) {}
}

impl FirearmBuff for FirearmSight {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::None => {}
			Self::Holorand => {
				priors.range.add_mean(20.0);
				priors.weight.add_mean(0.4);
			}
			Self::Leskop => {
				priors.range.add_mean(80.0);
				priors.rpm.add_mean(-30.0);
				priors.weight.add_mean(1.2);
			}
		}
	}
}

impl FirearmBuff for FirearmScales {
	fn contribute(&self, priors: &mut FirearmPriors) {
		for (is_barrel, scale) in self.slots() {
			contribute_slot_scale(priors, scale, is_barrel);
		}
	}
}

fn contribute_slot_scale(priors: &mut FirearmPriors, scale: SlotScale, is_barrel: bool) {
	let length_t = scale.length_tenths();
	let thick_t = scale.thickness_tenths();
	priors.pen.add_mean(PEN_PER_LENGTH_TENTH * length_t);
	priors.dpc.add_mean(DPC_PER_THICK_TENTH * thick_t);
	priors
		.weight
		.add_mean(WEIGHT_PER_THICK_TENTH * thick_t + WEIGHT_PER_LENGTH_TENTH * length_t);
	if is_barrel {
		priors.range.add_mean(BARREL_RANGE_PER_TENTH * length_t);
	}
}

impl FirearmBuff for FirearmMaterial {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::Glitter => {
				priors.bins.auto += 0.4;
				priors.bins.laser += 0.2;
				priors.rpm.add_mean(70.0);
				priors.range.add_mean(-60.0);
				priors.dpc.add_mean(-2.0);
				priors.weight.add_mean(-1.0);
			}
			Self::Scales => {
				priors.range.add_mean(90.0);
				priors.pen.add_mean(0.08);
				priors.range.mul_sd(1.25);
				priors.pen.mul_sd(1.25);
			}
			Self::LavaVeins => {
				priors.bins.gated += 0.2;
				add_damage(priors, 8.0);
				priors.dpc.mul_sd(1.15);
				priors.recoil.add_mean(0.4);
			}
			Self::WizardsVeins => {
				priors.bins.laser += 0.6;
				priors.bins.gated += 0.8;
				priors.bins.auto -= 0.3;
				priors.dpc.mul_sd(1.2);
				priors.refresh.add_mean(-0.08);
				add_damage(priors, 3.0);
			}
			Self::BrushedMetal => {
				priors.rpm.mul_sd(0.72);
				priors.burst.mul_sd(0.72);
				priors.burst_rpm.mul_sd(0.72);
				priors.refresh.mul_sd(0.72);
				priors.dpc.mul_sd(0.72);
				priors.speed.mul_sd(0.72);
				priors.pen.mul_sd(0.72);
				priors.range.mul_sd(0.72);
				priors.recoil.mul_sd(0.72);
				priors.weight.mul_sd(0.72);
				priors.recoil.add_mean(-0.3);
			}
		}
	}
}

impl FirearmBuff for ItemColor {
	fn contribute(&self, priors: &mut FirearmPriors) {
		let [r, g, b] = self.rgb();
		priors.dpc.mul_sd(channel_sd(r));
		priors.rpm.mul_sd(channel_sd(g));
		priors.burst_rpm.mul_sd(channel_sd(g));
		priors.speed.mul_sd(channel_sd(g));
		priors.range.mul_sd(channel_sd(b));
		if matches!(self, Self::Blue) {
			priors.recoil.add_mean(RECOIL_MEAN_BLUE);
		}
	}
}

fn channel_sd(channel: f32) -> f32 {
	1.0 + COLOR_SD_GAIN * (channel - 0.5)
}

impl FirearmBuff for BoltMaterial {
	fn contribute(&self, priors: &mut FirearmPriors) {
		match self {
			Self::PlainLaser => {
				priors.bins.laser += 0.2;
				priors.speed.add_mean(20.0);
			}
			Self::FizzingLaser => {
				priors.bins.laser += 0.7;
				add_damage(priors, 2.0);
				priors.pen.add_mean(-0.05);
			}
		}
	}
}

/// Identity-hashed buff-deviation roll.
pub fn generate_firearm_stats(spec: &FirearmSpec) -> FirearmStats {
	let seed = mix(FIREARM_SEED, &spec.identity_label());
	realize(&mut ItemRng::from_seed(seed), spec)
}

/// Continue `rng` instead of reseeding from identity. Gallery / FFA rolls use
/// this so projectile kind and cadence follow the session stream.
pub fn realize_firearm_stats(rng: &mut ItemRng, spec: &FirearmSpec) -> FirearmStats {
	realize(rng, spec)
}

fn realize(rng: &mut ItemRng, spec: &FirearmSpec) -> FirearmStats {
	let mut priors = FirearmPriors::new();
	spec.contribute(&mut priors);

	let mut stats = if sample_weighted(rng, priors.bins.laser, priors.bins.rate_of_fire) {
		realize_laser(rng, priors)
	} else {
		let bolt = sample_weighted(rng, priors.bins.bolt, priors.bins.bullet);
		let fire_kind = sample_fire_kind(rng, &priors.bins);
		realize_ballistic(rng, priors, bolt, fire_kind)
	};
	stats.sight_fov = spec.kit.sight.sample_ads_fov(rng);
	stats
}

enum FireKind {
	Auto,
	Semi,
	Burst,
	Gated,
}

fn sample_fire_kind(rng: &mut ItemRng, bins: &FirearmBins) -> FireKind {
	let weights =
		[bins.auto.max(0.0), bins.semi.max(0.0), bins.burst.max(0.0), bins.gated.max(0.0)];
	match sample_index(rng, &weights) {
		0 => FireKind::Auto,
		1 => FireKind::Semi,
		2 => FireKind::Burst,
		_ => FireKind::Gated,
	}
}

fn realize_laser(rng: &mut ItemRng, priors: FirearmPriors) -> FirearmStats {
	let speed = Dist::new(90.0, 15.0).apply(priors.speed);
	let range = Dist::new(95.0, 8.0).apply(priors.range);
	let dpc = Dist::new(22.0, 6.0).apply(priors.dpc);
	let weight = Dist::new(12.0, 4.0).apply(priors.weight);
	FirearmStats {
		projectile: ProjectileKind::Laser,
		speed: sample_u16(rng, speed, 60, 120),
		penetration: 0,
		range: sample_u16(rng, range, 80, 100),
		fire: None,
		recoil: 0.0,
		damage: sample_u16(rng, dpc, 8, 40),
		weight: sample_u16(rng, weight, 4, 40),
		sight_fov: IRON_SIGHT_FOV,
	}
}

fn realize_ballistic(
	rng: &mut ItemRng,
	priors: FirearmPriors,
	bolt: bool,
	kind: FireKind,
) -> FirearmStats {
	let pen_mean = if bolt { 0.85 } else { 0.50 };
	let speed = Dist::new(280.0, 70.0).apply(priors.speed);
	let pen = Dist::new(pen_mean, 0.12).apply(priors.pen);
	let range = Dist::new(400.0, 120.0).apply(priors.range);
	let mut recoil = Dist::new(RECOIL_MEAN, 1.8).apply(priors.recoil);
	let weight = Dist::new(14.0, 4.0).apply(priors.weight);

	let (fire, dpc_base, recoil_extra) = match kind {
		FireKind::Auto => {
			let rpm = Dist::new(600.0, 100.0).apply(priors.rpm);
			let rpm = sample_u16(rng, rpm, 400, 1500);
			(Some(FireMode::FullAuto { rpm }), Dist::new(24.0, 4.0).apply(priors.dpc), 1.0)
		}
		FireKind::Burst => {
			let rounds = Dist::new(3.0, 1.0).apply(priors.burst);
			let rounds = sample_f32(rng, rounds, 3.0, 5.0).round() as u8;
			let rpm = Dist::new(700.0, 120.0).apply(priors.burst_rpm);
			let rpm = sample_u16(rng, rpm, 400, 1500);
			(Some(FireMode::Burst { rounds, rpm }), Dist::new(34.0, 5.0).apply(priors.dpc), 0.0)
		}
		FireKind::Semi => {
			recoil = Dist::new(RECOIL_MEAN, 1.5).apply(priors.recoil);
			(Some(FireMode::SemiAuto), Dist::new(40.0, 6.0).apply(priors.dpc), 0.0)
		}
		FireKind::Gated => {
			let refresh = Dist::new(0.5, 0.1).apply(priors.refresh);
			let tenths = (sample_f32(rng, refresh, 0.3, 3.5) * 10.0).round() as u8;
			(
				Some(FireMode::Gated { recharge_tenths: tenths.clamp(3, 35) }),
				Dist::new(58.0, 10.0).apply(priors.dpc),
				0.0,
			)
		}
	};
	recoil.mean += recoil_extra;

	let (pen_min, pen_max) = if bolt { (0.60, 1.20) } else { (0.25, 0.70) };
	let (dpc_min, dpc_max) = match kind {
		FireKind::Auto => (8.0, 80.0),
		FireKind::Burst => (12.0, 90.0),
		FireKind::Semi => (16.0, 100.0),
		FireKind::Gated => (24.0, 140.0),
	};

	FirearmStats {
		projectile: if bolt { ProjectileKind::Bolt } else { ProjectileKind::Bullet },
		speed: sample_u16(rng, speed, 120, 500),
		penetration: (sample_f32(rng, pen, pen_min, pen_max) * 1000.0).round() as u16,
		range: sample_u16(rng, range, 50, 1000),
		fire,
		recoil: sample_f32(rng, recoil, RECOIL_MIN, RECOIL_MAX),
		damage: sample_u16(rng, dpc_base, dpc_min as u16, dpc_max as u16),
		weight: sample_u16(rng, weight, 4, 40),
		sight_fov: IRON_SIGHT_FOV,
	}
}

fn sample_weighted(rng: &mut ItemRng, a: f32, b: f32) -> bool {
	let a = a.max(0.0);
	let b = b.max(0.0);
	if a + b <= 0.0 {
		return false;
	}
	rng.unit() * (a + b) < a
}

fn sample_index(rng: &mut ItemRng, weights: &[f32]) -> usize {
	let total: f32 = weights.iter().copied().sum();
	if total <= 0.0 {
		return 0;
	}
	let mut pick = rng.unit() * total;
	for (index, weight) in weights.iter().enumerate() {
		if pick < *weight {
			return index;
		}
		pick -= *weight;
	}
	weights.len().saturating_sub(1)
}

fn sample_f32(rng: &mut ItemRng, dist: Dist, min: f32, max: f32) -> f32 {
	dist.sample_clamped(rng, min, max)
}

fn sample_u16(rng: &mut ItemRng, dist: Dist, min: u16, max: u16) -> u16 {
	sample_f32(rng, dist, f32::from(min), f32::from(max)).round() as u16
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::FirearmSpec;

	#[test]
	fn identity_is_stable() {
		let spec = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		assert_eq!(generate_firearm_stats(&spec), generate_firearm_stats(&spec));
	}

	#[test]
	fn blue_lowers_recoil_mean() {
		let mut natural = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		let mut blue = natural;
		natural.looks.body.color = ItemColor::Natural;
		blue.looks.body.color = ItemColor::Blue;
		let mut natural_priors = FirearmPriors::new();
		let mut blue_priors = FirearmPriors::new();
		natural.contribute(&mut natural_priors);
		blue.contribute(&mut blue_priors);
		assert!((natural_priors.recoil.add_mean - blue_priors.recoil.add_mean - 0.25).abs() < 1e-4);
	}

	#[test]
	fn grip_lowers_recoil_mean() {
		let mut none = FirearmPriors::new();
		let mut grip = FirearmPriors::new();
		FirearmGrip::None.contribute(&mut none);
		FirearmGrip::BumpHandle.contribute(&mut grip);
		assert!((none.recoil.add_mean - grip.recoil.add_mean - 0.25).abs() < 1e-4);
	}

	#[test]
	fn fizzing_laser_adds_damage_and_drops_pen_on_ballistic() {
		let mut plain = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		plain.bolt = BoltMaterial::PlainLaser;
		let mut fizz = plain;
		fizz.bolt = BoltMaterial::FizzingLaser;
		let mut plain_priors = FirearmPriors::new();
		let mut fizz_priors = FirearmPriors::new();
		plain.contribute(&mut plain_priors);
		fizz.contribute(&mut fizz_priors);
		assert!((fizz_priors.dpc.add_mean - plain_priors.dpc.add_mean - 2.0).abs() < 1e-4);
		assert!((fizz_priors.recoil.add_mean - plain_priors.recoil.add_mean - 0.05).abs() < 1e-4);
		assert!((fizz_priors.recoil.mul_sd / plain_priors.recoil.mul_sd - 1.025).abs() < 1e-4);
		assert!((fizz_priors.pen.add_mean - plain_priors.pen.add_mean + 0.05).abs() < 1e-4);
		assert!((plain_priors.speed.add_mean - fizz_priors.speed.add_mean - 20.0).abs() < 1e-4);
	}

	#[test]
	fn clamps_hold_for_concept_guns() {
		for mesh in FirearmMesh::VALUES {
			let stats = generate_firearm_stats(&FirearmSpec::from_mesh(*mesh));
			assert!((4..=40).contains(&stats.weight), "{mesh:?} weight {}", stats.weight);
			match stats.projectile {
				ProjectileKind::Laser => {
					assert!(stats.fire.is_none());
					assert_eq!(stats.penetration, 0);
					assert_eq!(stats.recoil, 0.0);
					assert!((60..=120).contains(&stats.speed));
					assert!((80..=100).contains(&stats.range));
					assert!((8..=40).contains(&stats.damage));
				}
				ProjectileKind::Bolt => {
					assert!(stats.fire.is_some());
					assert!((600..=1200).contains(&stats.penetration));
					assert!((120..=500).contains(&stats.speed));
					assert!((50..=1000).contains(&stats.range));
					assert!((RECOIL_MIN..=RECOIL_MAX).contains(&stats.recoil));
				}
				ProjectileKind::Bullet => {
					assert!(stats.fire.is_some());
					assert!((250..=700).contains(&stats.penetration));
					assert!((50..=1000).contains(&stats.range));
					assert!((RECOIL_MIN..=RECOIL_MAX).contains(&stats.recoil));
				}
			}
		}
	}

	#[test]
	fn session_stream_rolls_are_not_all_laser_or_one_color() {
		let mut rng = ItemRng::from_seed(1);
		let mut lasers = 0usize;
		let mut greens = 0usize;
		let mut colors = std::collections::BTreeSet::new();
		let mut kinds = std::collections::BTreeSet::new();
		const N: usize = 80;
		for _ in 0..N {
			let body = *rng.choose(FirearmMesh::VALUES).unwrap();
			let spec = FirearmSpec::roll(&mut rng, body);
			let stats = realize_firearm_stats(&mut rng, &spec);
			if spec.looks.body.color == ItemColor::Green {
				greens += 1;
			}
			colors.insert(spec.looks.body.color.label());
			kinds.insert(stats.projectile.label());
			if stats.projectile == ProjectileKind::Laser {
				lasers += 1;
			}
		}
		assert!(colors.len() >= 5, "body colors {colors:?}");
		assert!(kinds.len() >= 2, "projectiles {kinds:?}");
		assert!(greens < N / 2, "green {greens}/{N}");
		assert!(lasers < N / 3, "lasers {lasers}/{N}");
	}

	#[test]
	fn base_laser_family_probability_is_one_twelfth() {
		let probability = FAMILY_LASER / (FAMILY_LASER + FAMILY_RATE_OF_FIRE);
		assert!((probability - 1.0 / 12.0).abs() < f32::EPSILON);
	}

	#[test]
	fn concept_guns_keep_iron_sight_fov() {
		for mesh in FirearmMesh::VALUES {
			let stats = generate_firearm_stats(&FirearmSpec::from_mesh(*mesh));
			assert!((stats.sight_fov - IRON_SIGHT_FOV).abs() < 1e-5, "{mesh:?}");
		}
	}

	#[test]
	fn holorand_and_leskop_sample_within_zoom_fov() {
		let mut holorand = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		holorand.kit.sight = FirearmSight::Holorand;
		let mut leskop = holorand;
		leskop.kit.sight = FirearmSight::Leskop;
		let holorand_stats = generate_firearm_stats(&holorand);
		let leskop_stats = generate_firearm_stats(&leskop);
		let (h_wide, h_tight) = FirearmSight::Holorand.ads_fov_range();
		let (l_wide, l_tight) = FirearmSight::Leskop.ads_fov_range();
		assert!(holorand_stats.sight_fov <= h_wide + 1e-4);
		assert!(holorand_stats.sight_fov >= h_tight - 1e-4);
		assert!(leskop_stats.sight_fov <= l_wide + 1e-4);
		assert!(leskop_stats.sight_fov >= l_tight - 1e-4);
		assert!(leskop_stats.sight_fov <= holorand_stats.sight_fov + 1e-4);
	}

	#[test]
	fn sample_clamped_survives_nan_or_inverted_bounds() {
		let mut rng = ItemRng::from_seed(7);
		let dist = Dist::new(1.0, 0.25);
		assert!(dist.sample_clamped(&mut rng, 0.0, f32::NAN).is_finite());
		assert_eq!(dist.sample_clamped(&mut rng, 0.0, f32::NAN), 0.0);
		assert!(dist.sample_clamped(&mut rng, 3.0, 1.0).is_finite());
		assert!(Dist::new(f32::NAN, f32::NAN).sample_clamped(&mut rng, 0.0, 2.0).is_finite());
	}
}
