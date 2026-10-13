//! Rechargeable grenade catalog types. Runtime throw lives in `grenades`.

use serde::{Deserialize, Serialize};

/// Visual / grip identity for a grenade.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GrenadeMesh {
	#[default]
	Standard,
}

impl GrenadeMesh {
	pub const VALUES: &'static [Self] = &[Self::Standard];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Standard => "Grenade",
		}
	}

	pub const fn nouns(self) -> &'static [&'static str] {
		match self {
			Self::Standard => &["Grenade", "Charge", "Canister", "Pineapple"],
		}
	}
}

/// Authored model and animation profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GrenadeSpec {
	pub mesh: GrenadeMesh,
}

impl GrenadeSpec {
	pub const fn standard() -> Self {
		Self { mesh: GrenadeMesh::Standard }
	}
}

/// Fuse, recharge, and launch numbers. Seconds, not process-time deadlines.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GrenadeStats {
	pub fuse: f32,
	pub recharge: f32,
	pub launch_speed: f32,
	pub upward_bias: f32,
	pub inherit_velocity: f32,
	pub radius: f32,
	pub mass: f32,
	pub restitution: f32,
	pub friction: f32,
	pub throw_secs: f32,
	pub release_at: f32,
	pub weight: u16,
	#[serde(default = "default_blast_radius")]
	pub blast_radius: f32,
	#[serde(default = "default_blast_damage")]
	pub blast_damage: f32,
	pub effect_scale: f32,
	pub effect_intensity: f32,
	pub effect_playback: f32,
}

fn default_blast_radius() -> f32 {
	GrenadeStats::standard().blast_radius
}

fn default_blast_damage() -> f32 {
	GrenadeStats::standard().blast_damage
}

impl PartialEq for GrenadeStats {
	fn eq(&self, other: &Self) -> bool {
		self.fuse.to_bits() == other.fuse.to_bits()
			&& self.recharge.to_bits() == other.recharge.to_bits()
			&& self.launch_speed.to_bits() == other.launch_speed.to_bits()
			&& self.upward_bias.to_bits() == other.upward_bias.to_bits()
			&& self.inherit_velocity.to_bits() == other.inherit_velocity.to_bits()
			&& self.radius.to_bits() == other.radius.to_bits()
			&& self.mass.to_bits() == other.mass.to_bits()
			&& self.restitution.to_bits() == other.restitution.to_bits()
			&& self.friction.to_bits() == other.friction.to_bits()
			&& self.throw_secs.to_bits() == other.throw_secs.to_bits()
			&& self.release_at.to_bits() == other.release_at.to_bits()
			&& self.weight == other.weight
			&& self.blast_radius.to_bits() == other.blast_radius.to_bits()
			&& self.blast_damage.to_bits() == other.blast_damage.to_bits()
			&& self.effect_scale.to_bits() == other.effect_scale.to_bits()
			&& self.effect_intensity.to_bits() == other.effect_intensity.to_bits()
			&& self.effect_playback.to_bits() == other.effect_playback.to_bits()
	}
}

impl Eq for GrenadeStats {}

impl Default for GrenadeStats {
	fn default() -> Self {
		Self::standard()
	}
}

impl GrenadeStats {
	pub const fn standard() -> Self {
		Self {
			fuse: 2.5,
			recharge: 5.0,
			launch_speed: 10.0,
			upward_bias: 0.28,
			inherit_velocity: 0.28,
			radius: 0.08,
			mass: 0.35,
			restitution: 0.28,
			friction: 0.55,
			throw_secs: 0.92,
			release_at: 0.56,
			weight: 12,
			blast_radius: 10.5,
			blast_damage: 95.0,
			effect_scale: 13.0,
			effect_intensity: 1.0,
			effect_playback: 1.0,
		}
	}

	pub fn realize(rng: &mut crate::ItemRng) -> Self {
		crate::realize_grenade_stats(rng)
	}

	pub fn generate(spec: &GrenadeSpec) -> Self {
		crate::generate_grenade_stats(spec)
	}

	pub fn catalog_detail(&self) -> String {
		format!("fuse {:.1}s · blast {:.0}m · {:>3} wt", self.fuse, self.blast_radius, self.weight)
	}

	pub fn stat_rows(&self) -> Vec<(String, String)> {
		vec![
			(String::from("Fuse"), format!("{:.1}s", self.fuse)),
			(String::from("Recharge"), format!("{:.1}s", self.recharge)),
			(String::from("Blast"), format!("{:.0}m", self.blast_radius)),
			(String::from("Damage"), format!("{:.0}", self.blast_damage)),
			(String::from("Throw"), format!("{:.0} m/s", self.launch_speed)),
			(String::from("Weight"), self.weight.to_string()),
		]
	}
}

/// Remaining recharge on this owned instance. Use [`InventoryItem::same_catalog_identity`]
/// when comparing authored identity without cooldown.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GrenadeRecharge {
	pub remaining: f32,
}

impl PartialEq for GrenadeRecharge {
	fn eq(&self, other: &Self) -> bool {
		self.remaining.to_bits() == other.remaining.to_bits()
	}
}

impl Eq for GrenadeRecharge {}

impl GrenadeRecharge {
	pub fn ready(self) -> bool {
		self.remaining <= 1e-4
	}

	pub fn start(&mut self, duration: f32) {
		self.remaining = duration.max(0.0);
	}

	pub fn tick(&mut self, dt: f32) {
		if self.remaining <= 0.0 {
			return;
		}
		let dt = dt.max(0.0);
		if dt <= 0.0 {
			return;
		}
		self.remaining = (self.remaining - dt).max(0.0);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn recharge_remaining_affects_equality() -> anyhow::Result<()> {
		let ready = GrenadeRecharge { remaining: 0.0 };
		let cooling = GrenadeRecharge { remaining: 4.0 };
		anyhow::ensure!(ready != cooling);
		anyhow::ensure!(ready == GrenadeRecharge { remaining: 0.0 });
		Ok(())
	}

	#[test]
	fn grenade_catalog_identity_ignores_recharge() -> anyhow::Result<()> {
		use crate::InventoryItem;

		let spec = GrenadeSpec::standard();
		let stats = GrenadeStats::standard();
		let ready =
			InventoryItem::Grenade { spec, stats, recharge: GrenadeRecharge { remaining: 0.0 } };
		let cooling =
			InventoryItem::Grenade { spec, stats, recharge: GrenadeRecharge { remaining: 4.0 } };
		anyhow::ensure!(ready != cooling);
		anyhow::ensure!(ready.same_catalog_identity(&cooling));
		Ok(())
	}

	#[test]
	fn release_is_inside_the_throw_window() {
		let stats = GrenadeStats::standard();
		assert!(stats.release_at > 0.0);
		assert!(stats.release_at < stats.throw_secs);
		assert!(stats.recharge > stats.fuse);
	}

	#[test]
	fn standard_blast_is_street_scale() {
		let stats = GrenadeStats::standard();
		assert!(stats.blast_radius >= 10.0);
		assert!(stats.blast_damage >= 90.0);
		assert!((stats.effect_scale - 13.0).abs() < 1e-4);
	}

	#[test]
	fn tick_at_zero_remaining_is_noop() -> anyhow::Result<()> {
		let mut recharge = GrenadeRecharge { remaining: 0.0 };
		recharge.tick(0.5);
		anyhow::ensure!(recharge.remaining == 0.0, "idle recharge must stay at zero");
		Ok(())
	}

	#[test]
	fn tick_with_positive_dt_decreases_remaining() -> anyhow::Result<()> {
		let mut recharge = GrenadeRecharge { remaining: 2.0 };
		recharge.tick(0.75);
		anyhow::ensure!(
			(recharge.remaining - 1.25).abs() < 1e-4,
			"active recharge must count down by dt"
		);
		Ok(())
	}
}
