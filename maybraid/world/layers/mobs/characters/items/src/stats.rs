//! Rolled item attributes. Clothing stays integer so inventory is [`Eq`].
//! Firearm recoil is an [`f32`]; [`Eq`]/[`Hash`] use its bit pattern.
//!
//! Generation is identity-hashed (same mesh / look / color → same stats) so
//! old saves without a stats blob can recover a deterministic roll. Menus
//! display them and [`CharacterSheet`] compiles worn clothing plus queued
//! weapon weight. Firearm-user bakes [`FirearmStats`] into a live weapon
//! (`Weapon`, cadence, payload, recoil, ADS FOV); clothing `health` / `damage` feed
//! max HP and outgoing DPC.

use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

use crate::{ClothingMaterial, ClothingMesh, FirearmSpec, Inventory, ItemColor, IRON_SIGHT_FOV};

const BASE_HEALTH: i16 = 100;
const BASE_RUNNING: i16 = 100;
const BASE_JUMP: i16 = 100;
const BASE_AGILITY: i16 = 100;
const BASE_STRENGTH: i16 = 100;

/// Projectile family on a firearm. Mirrors the `firearms` crate conceptually
/// without taking a dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectileKind {
	Bolt,
	Bullet,
	Laser,
}

impl ProjectileKind {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Bolt => "Bolt",
			Self::Bullet => "Bullet",
			Self::Laser => "Laser",
		}
	}

	pub const fn is_ballistic(self) -> bool {
		!matches!(self, Self::Laser)
	}
}

/// Fire mode for bolts and bullets. Lasers have none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FireMode {
	FullAuto { rpm: u16 },
	Burst { rounds: u8, rpm: u16 },
	SemiAuto,
	Gated { recharge_tenths: u8 },
}

impl FireMode {
	pub fn label(self) -> String {
		match self {
			Self::FullAuto { rpm } => format!("Full auto {rpm} RPM"),
			Self::Burst { rounds, rpm } => format!("Burst {rounds} · {rpm} RPM"),
			Self::SemiAuto => String::from("Semi-auto"),
			Self::Gated { recharge_tenths } => {
				format!("Recharge {:.1}s", f32::from(recharge_tenths) / 10.0)
			}
		}
	}

	fn catalog_label(self) -> String {
		match self {
			Self::FullAuto { rpm } => format!("Auto {rpm}"),
			Self::Burst { rounds, .. } => format!("Burst {rounds}"),
			Self::SemiAuto => String::from("Semi"),
			Self::Gated { recharge_tenths } => {
				format!("Gate {:.1}s", f32::from(recharge_tenths) / 10.0)
			}
		}
	}
}

/// Clothing buffs plus carried weight. Mesh, look, and color contribute
/// priors; sampling picks one to three axes then Gaussian-realizes ±12–48.
/// A negative is never alone: a lone minus gains a paired plus. Weight
/// always rolls in a mesh bulk band.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ClothingStats {
	pub health: i16,
	pub running: i16,
	pub jump: i16,
	pub agility: i16,
	pub strength: i16,
	pub damage: i16,
	pub weight: u16,
}

impl ClothingStats {
	pub fn generate(mesh: ClothingMesh, material: ClothingMaterial, color: ItemColor) -> Self {
		crate::generate_clothing_stats(mesh, material, color)
	}

	pub fn catalog_detail(self) -> String {
		let mut parts = vec![format!("W{}", self.weight)];
		for (label, value) in self.buff_shorts() {
			parts.push(format!("{:+} {label}", value));
		}
		parts.join(" · ")
	}

	pub fn stat_rows(self) -> Vec<(String, String)> {
		let mut rows = Vec::new();
		for (label, value) in [
			("Health", self.health),
			("Running", self.running),
			("Jump", self.jump),
			("Agility", self.agility),
			("Strength", self.strength),
			("Damage", self.damage),
		] {
			if value != 0 {
				rows.push((String::from(label), format!("{value:+}")));
			}
		}
		rows.push((String::from("Weight"), self.weight.to_string()));
		rows
	}

	fn buff_shorts(self) -> Vec<(&'static str, i16)> {
		[
			("HP", self.health),
			("Run", self.running),
			("Jump", self.jump),
			("Agi", self.agility),
			("Str", self.strength),
			("Dmg", self.damage),
		]
		.into_iter()
		.filter(|(_, value)| *value != 0)
		.take(2)
		.collect()
	}
}

/// Firearm combat stats. Penetration is millunits (`600` = 0.60). Recoil is
/// catalog strength (typically `0.25..=8`). Recharge is tenths of a second.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FirearmStats {
	pub projectile: ProjectileKind,
	pub speed: u16,
	pub penetration: u16,
	pub range: u16,
	pub fire: Option<FireMode>,
	pub recoil: f32,
	pub damage: u16,
	pub weight: u16,
	/// ADS vertical FOV in radians. Iron sights keep [`IRON_SIGHT_FOV`].
	#[serde(default = "default_sight_fov")]
	pub sight_fov: f32,
}

fn default_sight_fov() -> f32 {
	IRON_SIGHT_FOV
}

impl PartialEq for FirearmStats {
	fn eq(&self, other: &Self) -> bool {
		self.projectile == other.projectile
			&& self.speed == other.speed
			&& self.penetration == other.penetration
			&& self.range == other.range
			&& self.fire == other.fire
			&& self.recoil.to_bits() == other.recoil.to_bits()
			&& self.damage == other.damage
			&& self.weight == other.weight
			&& self.sight_fov.to_bits() == other.sight_fov.to_bits()
	}
}

impl Eq for FirearmStats {}

impl Hash for FirearmStats {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.projectile.hash(state);
		self.speed.hash(state);
		self.penetration.hash(state);
		self.range.hash(state);
		self.fire.hash(state);
		self.recoil.to_bits().hash(state);
		self.damage.hash(state);
		self.weight.hash(state);
		self.sight_fov.to_bits().hash(state);
	}
}

impl FirearmStats {
	pub fn generate(spec: &FirearmSpec) -> Self {
		crate::generate_firearm_stats(spec)
	}

	/// Sample stats from `rng` instead of hashing [`FirearmSpec`] identity.
	pub fn realize(rng: &mut crate::ItemRng, spec: &FirearmSpec) -> Self {
		crate::realize_firearm_stats(rng, spec)
	}

	pub fn generate_for_mesh(mesh: crate::FirearmMesh) -> Self {
		Self::generate(&FirearmSpec::from_mesh(mesh))
	}

	pub fn catalog_detail(self) -> String {
		let mut parts = vec![String::from(self.projectile.label())];
		if let Some(fire) = self.fire {
			parts.push(fire.catalog_label());
		}
		parts.push(format!("{} DPC", self.damage));
		parts.join(" · ")
	}

	pub fn stat_rows(self) -> Vec<(String, String)> {
		let mut rows = vec![
			(String::from("Projectile"), String::from(self.projectile.label())),
			(String::from("Speed"), format!("{}/s", self.speed)),
		];
		if self.projectile.is_ballistic() {
			rows.push((
				String::from("Penetration"),
				format!("{:.2}", f32::from(self.penetration) / 1000.0),
			));
		}
		rows.push((String::from("Range"), format!("{} m", self.range)));
		if let Some(fire) = self.fire {
			rows.push((String::from("Fire"), fire.label()));
		}
		if self.projectile.is_ballistic() {
			rows.push((String::from("Recoil"), format!("{:.2}", self.recoil)));
		}
		rows.push((String::from("DPC"), self.damage.to_string()));
		rows.push((String::from("Weight"), self.weight.to_string()));
		rows.push((String::from("Sight FOV"), format!("{:.0}°", self.sight_fov.to_degrees())));
		rows
	}

	/// Inclusive DPC clamp for the realized projectile / fire mode.
	pub fn damage_band(self) -> Option<(u16, u16)> {
		Some(match (self.projectile, self.fire) {
			(ProjectileKind::Laser, _) => (8, 40),
			(_, Some(FireMode::FullAuto { .. })) => (8, 80),
			(_, Some(FireMode::Burst { .. })) => (12, 90),
			(_, Some(FireMode::SemiAuto)) => (16, 100),
			(_, Some(FireMode::Gated { .. })) => (24, 140),
			_ => (8, 140),
		})
	}
}

/// Species-agnostic compiled sheet. Worn clothing adds buffs and weight;
/// queued weapons add weight only. Primary-weapon combat stats stay on the
/// firearm, not mixed into health / strength.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharacterSheet {
	pub health: i16,
	pub running: i16,
	pub jump: i16,
	pub agility: i16,
	pub strength: i16,
	pub damage: i16,
	pub weight: u16,
}

impl CharacterSheet {
	pub const BASE: Self = Self {
		health: BASE_HEALTH,
		running: BASE_RUNNING,
		jump: BASE_JUMP,
		agility: BASE_AGILITY,
		strength: BASE_STRENGTH,
		damage: 0,
		weight: 0,
	};

	pub fn from_inventory(inventory: &Inventory) -> Self {
		let mut sheet = Self::BASE;
		let modifiers = Self::modifiers_from_inventory(inventory);
		sheet.add_sheet(modifiers);
		sheet
	}

	/// Worn clothing deltas plus queued-weapon weight. Baseline is zero.
	pub fn modifiers_from_inventory(inventory: &Inventory) -> Self {
		let mut sheet =
			Self { health: 0, running: 0, jump: 0, agility: 0, strength: 0, damage: 0, weight: 0 };
		for item in inventory.worn_items() {
			if let Some(stats) = item.clothing_stats() {
				sheet.add_clothing(stats);
			}
		}
		for &index in &inventory.weapons {
			let Some(item) = inventory.items.get(index) else {
				continue;
			};
			if let Some(stats) = item.firearm_stats() {
				sheet.weight = sheet.weight.saturating_add(stats.weight);
			}
			if let Some(stats) = item.grenade_stats() {
				sheet.weight = sheet.weight.saturating_add(stats.weight);
			}
		}
		sheet
	}

	fn add_sheet(&mut self, other: Self) {
		self.health = self.health.saturating_add(other.health);
		self.running = self.running.saturating_add(other.running);
		self.jump = self.jump.saturating_add(other.jump);
		self.agility = self.agility.saturating_add(other.agility);
		self.strength = self.strength.saturating_add(other.strength);
		self.damage = self.damage.saturating_add(other.damage);
		self.weight = self.weight.saturating_add(other.weight);
	}

	pub fn attribute_deltas(self) -> [(&'static str, i16); 6] {
		[
			("Health", self.health),
			("Running", self.running),
			("Jump", self.jump),
			("Agility", self.agility),
			("Strength", self.strength),
			("Damage", self.damage),
		]
	}

	fn add_clothing(&mut self, stats: ClothingStats) {
		self.health = self.health.saturating_add(stats.health);
		self.running = self.running.saturating_add(stats.running);
		self.jump = self.jump.saturating_add(stats.jump);
		self.agility = self.agility.saturating_add(stats.agility);
		self.strength = self.strength.saturating_add(stats.strength);
		self.damage = self.damage.saturating_add(stats.damage);
		self.weight = self.weight.saturating_add(stats.weight);
	}

	/// Strength used as carrying capacity. Floor at `1` so a dumped-stat
	/// sheet still has a defined pace instead of dividing by zero.
	fn carrying_strength(self) -> u32 {
		u32::try_from(self.strength.max(1)).unwrap_or(1)
	}

	/// Weight after strength offsets the load. At [`BASE_STRENGTH`] this
	/// equals carried weight; above that, load feels lighter.
	fn felt_weight(self) -> u32 {
		let base = u32::try_from(BASE_STRENGTH).unwrap_or(1);
		u32::from(self.weight).saturating_mul(base) / self.carrying_strength()
	}

	/// Preview locomotion factor as a percent. `100` at zero weight; not
	/// applied to movement this pass. Strength scales down the weight term
	/// (`felt = weight * 100 / strength`), so a stronger character keeps more
	/// of the unladen pace under the same load.
	pub fn pace(self) -> u16 {
		(10_000 / (100 + self.felt_weight())).min(100) as u16
	}

	/// `68% = 10000 / (100 + 46*100/100)` so the loadout can show how pace
	/// is derived: strength in the denominator shrinks felt weight.
	pub fn pace_equation(self) -> String {
		format!(
			"{}% = 10000 / (100 + {}*100/{})",
			self.pace(),
			self.weight,
			self.carrying_strength()
		)
	}

	pub fn stat_rows(self) -> Vec<(String, String)> {
		vec![
			(String::from("Health"), self.health.to_string()),
			(String::from("Running"), self.running.to_string()),
			(String::from("Jump"), self.jump.to_string()),
			(String::from("Agility"), self.agility.to_string()),
			(String::from("Strength"), self.strength.to_string()),
			(String::from("Damage Bonus"), signed_or_zero(self.damage)),
			(String::from("Added Weight"), self.weight.to_string()),
			(String::from("Pace"), self.pace_equation()),
		]
	}

	pub fn base_stat_rows() -> Vec<(String, String)> {
		vec![
			(String::from("Health"), BASE_HEALTH.to_string()),
			(String::from("Running"), BASE_RUNNING.to_string()),
			(String::from("Jump"), BASE_JUMP.to_string()),
			(String::from("Agility"), BASE_AGILITY.to_string()),
			(String::from("Strength"), BASE_STRENGTH.to_string()),
			(String::from("Damage Bonus"), String::from("0")),
			(String::from("Added Weight"), String::from("0")),
		]
	}

	/// Non-zero clothing deltas, signed, plus carried weight.
	pub fn buff_stat_rows(self) -> Vec<(String, String)> {
		let mut rows = Vec::new();
		for (label, value) in self.attribute_deltas() {
			if value != 0 {
				rows.push((String::from(label), format!("{value:+}")));
			}
		}
		if self.weight > 0 {
			rows.push((String::from("Weight"), self.weight.to_string()));
		}
		if rows.is_empty() {
			rows.push((String::from("—"), String::new()));
		}
		rows
	}
}

fn signed_or_zero(value: i16) -> String {
	if value == 0 {
		String::from("0")
	} else {
		format!("{value:+}")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::FirearmMesh;
	use crate::InventoryItem;

	#[test]
	fn clothing_identity_is_stable_and_has_weight() {
		let a = ClothingStats::generate(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let b = ClothingStats::generate(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert_eq!(a, b);
		assert!(a.weight >= 2);
		let nonzero = [a.health, a.running, a.jump, a.agility, a.strength, a.damage]
			.into_iter()
			.filter(|value| *value != 0)
			.count();
		assert!((1..=3).contains(&nonzero));
		assert!(a.weight <= 8);
	}

	#[test]
	fn robe_is_heavier_than_tank() {
		let tank = ClothingStats::generate(
			ClothingMesh::TankTop,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let robe = ClothingStats::generate(
			ClothingMesh::Robe,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		assert!(robe.weight >= tank.weight);
		assert!(robe.weight >= 14);
	}

	#[test]
	fn firearm_dpc_stays_in_formula_band() {
		for mesh in FirearmMesh::VALUES {
			let stats = FirearmStats::generate(&FirearmSpec::from_mesh(*mesh));
			let (min, max) = stats.damage_band().expect("band");
			assert!(
				(min..=max).contains(&stats.damage),
				"{:?} dpc {} not in {min}..={max}",
				mesh,
				stats.damage
			);
			match stats.projectile {
				ProjectileKind::Laser => {
					assert!(stats.fire.is_none());
					assert_eq!(stats.penetration, 0);
					assert!((60..=120).contains(&stats.speed));
					assert!((80..=100).contains(&stats.range));
				}
				ProjectileKind::Bolt => {
					assert!(stats.fire.is_some());
					assert!((600..=1200).contains(&stats.penetration));
					assert!((120..=500).contains(&stats.speed));
					assert!((50..=1000).contains(&stats.range));
				}
				ProjectileKind::Bullet => {
					assert!(stats.fire.is_some());
					assert!((250..=700).contains(&stats.penetration));
					assert!((50..=1000).contains(&stats.range));
				}
			}
		}
	}

	#[test]
	fn character_sheet_sums_worn_clothing_and_weapon_weight() {
		let clothing = InventoryItem::clothing(
			ClothingMesh::Pants,
			ClothingMaterial::Cloth,
			ItemColor::Natural,
		);
		let gun = InventoryItem::firearm(FirearmMesh::Bullpup);
		let clothing_stats = clothing.clothing_stats().unwrap();
		let gun_stats = gun.firearm_stats().unwrap();
		let inventory = Inventory {
			items: vec![clothing, gun],
			clothing: vec![0],
			weapons: vec![1],
			skills: Vec::new(),
		};
		let sheet = CharacterSheet::from_inventory(&inventory);
		assert_eq!(sheet.health, BASE_HEALTH + clothing_stats.health);
		assert_eq!(sheet.weight, clothing_stats.weight + gun_stats.weight);
		assert!(sheet.pace() <= 100);
		assert_eq!(
			sheet.pace_equation(),
			format!(
				"{}% = 10000 / (100 + {}*100/{})",
				sheet.pace(),
				sheet.weight,
				sheet.strength.max(1)
			)
		);
	}

	#[test]
	fn strength_reduces_weight_drag_on_pace() {
		let load = CharacterSheet { weight: 40, ..CharacterSheet::BASE };
		let strong = CharacterSheet { strength: 150, weight: 40, ..CharacterSheet::BASE };
		let weak = CharacterSheet { strength: 80, weight: 40, ..CharacterSheet::BASE };
		assert_eq!(load.pace(), (10_000 / (100 + 40)) as u16);
		assert!(strong.pace() > load.pace());
		assert!(load.pace() > weak.pace());
	}

	#[test]
	fn clothing_rolls_include_pluses_and_minuses() {
		let mut saw_plus = false;
		let mut saw_minus = false;
		for mesh in ClothingMesh::VALUES {
			for material in ClothingMaterial::VALUES {
				let stats = ClothingStats::generate(*mesh, *material, ItemColor::Natural);
				for value in [
					stats.health,
					stats.running,
					stats.jump,
					stats.agility,
					stats.strength,
					stats.damage,
				] {
					saw_plus |= value > 0;
					saw_minus |= value < 0;
				}
				let deltas: Vec<i16> = [
					stats.health,
					stats.running,
					stats.jump,
					stats.agility,
					stats.strength,
					stats.damage,
				]
				.into_iter()
				.filter(|value| *value != 0)
				.collect();
				if deltas.iter().any(|value| *value < 0) {
					assert!(
						deltas.iter().any(|value| *value > 0),
						"negative without a plus: {:?} {:?}",
						mesh,
						deltas
					);
				}
				if deltas.len() >= 2 {
					assert!(
						deltas.iter().any(|value| *value > 0)
							&& deltas.iter().any(|value| *value < 0),
						"{:?} {:?}",
						mesh,
						deltas
					);
				}
			}
		}
		assert!(saw_plus && saw_minus);
	}
}
