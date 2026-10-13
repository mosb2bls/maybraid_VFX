//! Buff-deviation clothing generation.
//!
//! Contributors ([`ClothingBuff`]) add axis bin weight and shift mean / scale
//! deviation. Sampling is staged: how many axes → which axes → Gaussian
//! realize. A minus is never the only buff.

use crate::firearm_roll::{Dist, DistDelta};
use crate::{names::mix, ClothingMaterial, ClothingMesh, ClothingStats, ItemColor, ItemRng};

const CLOTHING_SEED: u64 = 0x51A7_0001_C10A_0001;
const COLOR_SD_GAIN: f32 = 0.4;
/// Realized clothing deltas. Authored mesh/look means stay in small units
/// and are scaled here so a garment's buffs can outweigh its weight tax.
const BUFF_SCALE: f32 = 3.0;
const BUFF_MIN: i16 = 12;
const BUFF_MAX: i16 = 48;
const BUFF_SD: f32 = 6.0 * BUFF_SCALE;

/// Categorical weights for which buff axes light up.
#[derive(Clone, Copy, Debug)]
struct ClothingBins {
	health: f32,
	running: f32,
	jump: f32,
	agility: f32,
	strength: f32,
	damage: f32,
}

impl ClothingBins {
	fn base() -> Self {
		Self { health: 1.0, running: 1.0, jump: 1.0, agility: 1.0, strength: 1.0, damage: 1.0 }
	}

	fn as_array(self) -> [f32; 6] {
		[self.health, self.running, self.jump, self.agility, self.strength, self.damage]
	}

	fn axis_mut(&mut self, axis: u8) -> &mut f32 {
		match axis {
			0 => &mut self.health,
			1 => &mut self.running,
			2 => &mut self.jump,
			3 => &mut self.agility,
			4 => &mut self.strength,
			_ => &mut self.damage,
		}
	}
}

/// Accumulated priors. Bins pick axes; axis deltas apply to a zero-mean base.
#[derive(Clone, Copy, Debug)]
pub struct ClothingPriors {
	bins: ClothingBins,
	health: DistDelta,
	running: DistDelta,
	jump: DistDelta,
	agility: DistDelta,
	strength: DistDelta,
	damage: DistDelta,
	weight: DistDelta,
}

impl ClothingPriors {
	fn new() -> Self {
		Self {
			bins: ClothingBins::base(),
			health: DistDelta::IDENTITY,
			running: DistDelta::IDENTITY,
			jump: DistDelta::IDENTITY,
			agility: DistDelta::IDENTITY,
			strength: DistDelta::IDENTITY,
			damage: DistDelta::IDENTITY,
			weight: DistDelta::IDENTITY,
		}
	}

	fn axis_delta(&self, axis: u8) -> DistDelta {
		match axis {
			0 => self.health,
			1 => self.running,
			2 => self.jump,
			3 => self.agility,
			4 => self.strength,
			_ => self.damage,
		}
	}

	fn axis_delta_mut(&mut self, axis: u8) -> &mut DistDelta {
		match axis {
			0 => &mut self.health,
			1 => &mut self.running,
			2 => &mut self.jump,
			3 => &mut self.agility,
			4 => &mut self.strength,
			_ => &mut self.damage,
		}
	}
}

/// Something that shifts clothing bins and/or axis moments.
pub trait ClothingBuff {
	fn contribute(&self, priors: &mut ClothingPriors);
}

impl ClothingBuff for ClothingMesh {
	fn contribute(&self, priors: &mut ClothingPriors) {
		match self {
			Self::TankTop => {
				priors.bins.health += 0.6;
				priors.bins.agility += 0.5;
				priors.bins.strength -= 0.2;
				priors.health.add_mean(2.0);
				priors.agility.add_mean(1.5);
				priors.weight.add_mean(-1.0);
			}
			Self::Tunic => {
				priors.bins.health += 0.4;
				priors.bins.strength += 0.3;
				priors.health.add_mean(1.5);
			}
			Self::LongDress => {
				priors.bins.health += 0.5;
				priors.bins.running -= 0.3;
				priors.bins.agility -= 0.2;
				priors.health.add_mean(2.0);
				priors.running.add_mean(-2.0);
				priors.agility.add_mean(-1.5);
			}
			Self::ShortDress => {
				priors.bins.agility += 0.6;
				priors.bins.running += 0.4;
				priors.agility.add_mean(2.0);
				priors.running.add_mean(1.5);
			}
			Self::FittedCoat => {
				priors.bins.strength += 0.6;
				priors.bins.health += 0.4;
				priors.strength.add_mean(2.5);
				priors.health.add_mean(1.5);
				priors.agility.add_mean(-1.0);
			}
			Self::RobeCoat => {
				priors.bins.damage += 0.7;
				priors.bins.health += 0.4;
				priors.bins.running -= 0.3;
				priors.damage.add_mean(3.0);
				priors.health.add_mean(1.5);
				priors.running.add_mean(-2.0);
			}
			Self::Robe => {
				priors.bins.damage += 0.8;
				priors.bins.health += 0.5;
				priors.bins.agility -= 0.3;
				priors.damage.add_mean(3.5);
				priors.health.add_mean(2.0);
				priors.agility.add_mean(-2.0);
			}
			Self::Pants => {
				priors.bins.running += 0.6;
				priors.bins.jump += 0.4;
				priors.running.add_mean(2.0);
				priors.jump.add_mean(1.5);
			}
			Self::KneeHighBoots => {
				priors.bins.jump += 0.8;
				priors.bins.running += 0.4;
				priors.bins.agility -= 0.2;
				priors.jump.add_mean(3.0);
				priors.running.add_mean(1.5);
				priors.agility.add_mean(-1.0);
			}
			Self::HaremPants => {
				priors.bins.running += 0.5;
				priors.bins.agility += 0.6;
				priors.running.add_mean(1.5);
				priors.agility.add_mean(2.0);
			}
			Self::HaremPantsUpper => {
				priors.bins.agility += 0.5;
				priors.bins.health += 0.3;
				priors.agility.add_mean(1.5);
				priors.weight.add_mean(-1.0);
			}
			Self::HaremPantsLowerWrap => {
				priors.bins.running += 0.4;
				priors.bins.jump += 0.5;
				priors.jump.add_mean(1.5);
				priors.running.add_mean(1.0);
			}
		}
	}
}

impl ClothingBuff for ClothingMaterial {
	fn contribute(&self, priors: &mut ClothingPriors) {
		match self {
			Self::SpaceSuit => {
				priors.bins.health += 0.7;
				priors.bins.strength += 0.5;
				priors.bins.agility -= 0.4;
				priors.bins.running -= 0.3;
				priors.health.add_mean(3.0);
				priors.strength.add_mean(2.0);
				priors.agility.add_mean(-2.5);
				priors.running.add_mean(-1.5);
				priors.weight.add_mean(2.0);
			}
			Self::Tattered => {
				priors.bins.agility += 0.5;
				priors.bins.health -= 0.2;
				priors.health.add_mean(-2.0);
				priors.strength.add_mean(-1.5);
				priors.agility.add_mean(2.0);
				priors.weight.add_mean(-1.5);
			}
			Self::Hawaiian => {
				priors.bins.running += 0.6;
				priors.bins.jump += 0.5;
				priors.bins.damage -= 0.2;
				priors.running.add_mean(2.5);
				priors.jump.add_mean(2.0);
				priors.damage.add_mean(-1.5);
			}
			Self::Cloth => {
				for axis in 0..6u8 {
					priors.axis_delta_mut(axis).mul_sd(0.85);
				}
				priors.weight.mul_sd(0.85);
			}
			Self::Scales => {
				priors.bins.strength += 0.7;
				priors.bins.health += 0.4;
				priors.bins.agility -= 0.3;
				priors.strength.add_mean(3.0);
				priors.health.add_mean(1.5);
				priors.agility.add_mean(-2.0);
				priors.weight.add_mean(2.0);
			}
			Self::WizardsVeins => {
				priors.bins.damage += 0.8;
				priors.bins.health += 0.4;
				priors.bins.strength -= 0.2;
				priors.damage.add_mean(4.0);
				priors.health.add_mean(2.0);
				priors.strength.add_mean(-1.5);
				priors.damage.mul_sd(1.2);
			}
			Self::Glitter => {
				priors.bins.agility += 0.6;
				priors.bins.running += 0.4;
				priors.bins.damage += 0.2;
				priors.agility.add_mean(2.5);
				priors.running.add_mean(1.5);
				priors.damage.add_mean(1.0);
				priors.weight.add_mean(-2.0);
			}
		}
	}
}

impl ClothingBuff for ItemColor {
	fn contribute(&self, priors: &mut ClothingPriors) {
		let [r, g, b] = self.rgb();
		priors.damage.mul_sd(channel_sd(r));
		priors.health.mul_sd(channel_sd(r));
		priors.running.mul_sd(channel_sd(g));
		priors.jump.mul_sd(channel_sd(g));
		priors.agility.mul_sd(channel_sd(g));
		priors.strength.mul_sd(channel_sd(b));
	}
}

fn channel_sd(channel: f32) -> f32 {
	1.0 + COLOR_SD_GAIN * (channel - 0.5)
}

/// Identity-hashed buff-deviation roll.
pub fn generate_clothing_stats(
	mesh: ClothingMesh,
	material: ClothingMaterial,
	color: ItemColor,
) -> ClothingStats {
	let seed = mix(mix(mix(CLOTHING_SEED, mesh.label()), material.label()), color.label());
	realize(&mut ItemRng::from_seed(seed), mesh, material, color)
}

fn realize(
	rng: &mut ItemRng,
	mesh: ClothingMesh,
	material: ClothingMaterial,
	color: ItemColor,
) -> ClothingStats {
	let mut priors = ClothingPriors::new();
	mesh.contribute(&mut priors);
	material.contribute(&mut priors);
	color.contribute(&mut priors);

	let count = sample_f32(rng, Dist::new(2.0, 0.7), 1.0, 3.0).round() as usize;
	let count = count.clamp(1, 3);
	let axes = sample_axes(rng, &priors.bins, count);
	let mut assigned = Vec::with_capacity(3);
	for axis in axes {
		assigned.push((axis, sample_buff(rng, axis_dist(&priors, axis, 0.0, BUFF_SD))));
	}
	pair_negative_with_positive(rng, &mut priors, &mut assigned);

	let (weight_min, weight_max) = clothing_weight_range(mesh);
	let weight_base = Dist::new(
		(weight_min + weight_max) as f32 * 0.5,
		((weight_max - weight_min) as f32 / 3.0).max(1.5),
	)
	.apply(priors.weight);
	let weight = sample_u16(rng, weight_base, weight_min as u16, weight_max as u16);

	let mut stats = ClothingStats { weight, ..ClothingStats::default() };
	for (axis, value) in assigned {
		match axis {
			0 => stats.health = value,
			1 => stats.running = value,
			2 => stats.jump = value,
			3 => stats.agility = value,
			4 => stats.strength = value,
			_ => stats.damage = value,
		}
	}
	stats
}

fn axis_dist(priors: &ClothingPriors, axis: u8, base_mean: f32, base_sd: f32) -> Dist {
	let mut delta = priors.axis_delta(axis);
	delta.add_mean *= BUFF_SCALE;
	Dist::new(base_mean, base_sd).apply(delta)
}

fn sample_axes(rng: &mut ItemRng, bins: &ClothingBins, count: usize) -> Vec<u8> {
	let mut weights = bins.as_array();
	let mut picked = Vec::with_capacity(count);
	for _ in 0..count {
		let index = sample_index(rng, &weights);
		picked.push(index as u8);
		weights[index] = 0.0;
	}
	picked
}

fn sample_buff(rng: &mut ItemRng, dist: Dist) -> i16 {
	let sampled = rng.sample_normal(dist.mean, dist.sd).clamp(-(BUFF_MAX as f32), BUFF_MAX as f32);
	let mut value = sampled.round() as i16;
	if value == 0 {
		value = if dist.mean < 0.0 { -BUFF_MIN } else { BUFF_MIN };
	} else if value.abs() < BUFF_MIN {
		value = BUFF_MIN * value.signum();
	}
	value.clamp(-BUFF_MAX, BUFF_MAX)
}

fn pair_negative_with_positive(
	rng: &mut ItemRng,
	priors: &mut ClothingPriors,
	assigned: &mut Vec<(u8, i16)>,
) {
	if assigned.len() >= 2 {
		let all_plus = assigned.iter().all(|(_, value)| *value > 0);
		let all_minus = assigned.iter().all(|(_, value)| *value < 0);
		if all_plus || all_minus {
			if let Some((_, value)) = assigned.last_mut() {
				*value = -*value;
			}
		}
		return;
	}
	if assigned.first().is_some_and(|(_, value)| *value < 0) {
		let used = assigned[0].0;
		*priors.bins.axis_mut(used) = 0.0;
		let axis = sample_index(rng, &priors.bins.as_array()) as u8;
		let dist = axis_dist(priors, axis, 6.0 * BUFF_SCALE, 4.0 * BUFF_SCALE);
		let plus = sample_buff(rng, dist).abs().max(BUFF_MIN);
		assigned.push((axis, plus));
	}
}

fn clothing_weight_range(mesh: ClothingMesh) -> (u32, u32) {
	match mesh {
		ClothingMesh::TankTop
		| ClothingMesh::HaremPantsUpper
		| ClothingMesh::HaremPantsLowerWrap => (2, 8),
		ClothingMesh::Tunic
		| ClothingMesh::Pants
		| ClothingMesh::HaremPants
		| ClothingMesh::ShortDress => (6, 14),
		ClothingMesh::FittedCoat | ClothingMesh::KneeHighBoots => (10, 18),
		ClothingMesh::LongDress | ClothingMesh::Robe | ClothingMesh::RobeCoat => (14, 24),
	}
}

fn sample_index(rng: &mut ItemRng, weights: &[f32]) -> usize {
	let total: f32 = weights.iter().copied().map(|weight| weight.max(0.0)).sum();
	if total <= 0.0 {
		return weights.iter().position(|weight| *weight > 0.0).unwrap_or(0);
	}
	let mut pick = rng.unit() * total;
	for (index, weight) in weights.iter().enumerate() {
		let weight = weight.max(0.0);
		if pick < weight {
			return index;
		}
		pick -= weight;
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

	#[test]
	fn identity_is_stable() {
		let a = generate_clothing_stats(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let b = generate_clothing_stats(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert_eq!(a, b);
	}

	#[test]
	fn spacesuit_shifts_health_mean_over_cloth() {
		let mut cloth = ClothingPriors::new();
		let mut suit = ClothingPriors::new();
		ClothingMaterial::Cloth.contribute(&mut cloth);
		ClothingMaterial::SpaceSuit.contribute(&mut suit);
		assert!(suit.health.add_mean > cloth.health.add_mean);
		assert!(suit.agility.add_mean < cloth.agility.add_mean);
	}

	#[test]
	fn robe_weight_band_is_above_tank() {
		let tank = generate_clothing_stats(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let robe = generate_clothing_stats(
			ClothingMesh::Robe,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert!(tank.weight <= 8);
		assert!(robe.weight >= 14);
		assert!(robe.weight >= tank.weight);
	}

	#[test]
	fn look_changes_the_roll() {
		let cloth = generate_clothing_stats(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let suit = generate_clothing_stats(
			ClothingMesh::TankTop,
			ClothingMaterial::SpaceSuit,
			ItemColor::Natural,
		);
		assert_ne!(cloth, suit);
	}

	#[test]
	fn realized_buffs_are_large_enough_to_matter() {
		for mesh in ClothingMesh::VALUES {
			for material in ClothingMaterial::VALUES {
				let stats = generate_clothing_stats(*mesh, *material, ItemColor::Natural);
				for value in [
					stats.health,
					stats.running,
					stats.jump,
					stats.agility,
					stats.strength,
					stats.damage,
				] {
					if value != 0 {
						assert!(
							(BUFF_MIN..=BUFF_MAX).contains(&value.abs()),
							"{mesh:?} {material:?} {value} outside {BUFF_MIN}..={BUFF_MAX}"
						);
					}
				}
			}
		}
	}
}
