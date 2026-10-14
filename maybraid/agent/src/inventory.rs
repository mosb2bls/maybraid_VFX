//! Inventory-cycle observation and matchers. Bevy-free so playback tests stay cheap.
//!
//! The world plugin fills [`InventoryObservation`] from
//! `ActiveInventoryCyclingCategory` and the player bag / Lubikan ledger. It
//! does not import combat-hud or menu types.

use serde::{Deserialize, Serialize};

use crate::menu::{MenuMatcher, MenuObservation};

/// Live cycling snapshot the world plugin publishes for scripts and events.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InventoryObservation {
	/// `weapons`, `skill_maps`, or `lubikans`. Empty before the first publish.
	pub cycle_category: String,
	/// Equipped / active item in each category (`bullpup`, `fireball`, `self`).
	pub weapon: Option<String>,
	pub skill_map: Option<String>,
	pub lubikan: Option<String>,
	/// Equipped queues (script ids) so invariants can check membership.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub weapons: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub skill_maps: Vec<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub lubikans: Vec<String>,
	/// Cycling invariant breaks detected this frame (empty when healthy).
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub invariants: Vec<String>,
}

impl InventoryObservation {
	pub fn describe(&self) -> String {
		format!(
			"cycle_category={} weapon={} skill_map={} lubikan={}",
			empty_or(&self.cycle_category),
			self.weapon.as_deref().unwrap_or("null"),
			self.skill_map.as_deref().unwrap_or("null"),
			self.lubikan.as_deref().unwrap_or("null"),
		)
	}

	pub fn category_changed(&self, previous: &Self) -> bool {
		!self.cycle_category.is_empty() && self.cycle_category != previous.cycle_category
	}

	pub fn equipped_changed(&self, previous: &Self) -> bool {
		self.weapon != previous.weapon
			|| self.skill_map != previous.skill_map
			|| self.lubikan != previous.lubikan
	}
}

fn empty_or(value: &str) -> &str {
	if value.is_empty() {
		"none"
	} else {
		value
	}
}

/// Optional inventory fields a `wait_for` / `expect` command can pin.
///
/// A missing field is "don't care".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InventoryMatcher {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub cycle_category: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub weapon: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub skill_map: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub lubikan: Option<String>,
}

impl InventoryMatcher {
	pub fn matches(&self, observed: &InventoryObservation) -> bool {
		if let Some(category) = &self.cycle_category {
			if observed.cycle_category != *category {
				return false;
			}
		}
		if let Some(weapon) = &self.weapon {
			if observed.weapon.as_deref() != Some(weapon.as_str()) {
				return false;
			}
		}
		if let Some(skill_map) = &self.skill_map {
			if observed.skill_map.as_deref() != Some(skill_map.as_str()) {
				return false;
			}
		}
		if let Some(lubikan) = &self.lubikan {
			if observed.lubikan.as_deref() != Some(lubikan.as_str()) {
				return false;
			}
		}
		true
	}

	pub fn describe(&self) -> String {
		let mut parts = Vec::new();
		if let Some(category) = &self.cycle_category {
			parts.push(format!("cycle_category={category}"));
		}
		if let Some(weapon) = &self.weapon {
			parts.push(format!("weapon={weapon}"));
		}
		if let Some(skill_map) = &self.skill_map {
			parts.push(format!("skill_map={skill_map}"));
		}
		if let Some(lubikan) = &self.lubikan {
			parts.push(format!("lubikan={lubikan}"));
		}
		parts.join(" ")
	}

	pub fn is_empty(&self) -> bool {
		self.cycle_category.is_none()
			&& self.weapon.is_none()
			&& self.skill_map.is_none()
			&& self.lubikan.is_none()
	}
}

/// Flattened `wait_for` / `expect` matcher: menu snapshot plus inventory cycle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgentMatcher {
	#[serde(flatten)]
	pub menu: MenuMatcher,
	#[serde(flatten)]
	pub inventory: InventoryMatcher,
}

impl AgentMatcher {
	pub fn matches(&self, menu: &MenuObservation, inventory: &InventoryObservation) -> bool {
		self.menu.matches(menu) && self.inventory.matches(inventory)
	}

	pub fn describe(&self) -> String {
		let menu = self.menu.describe();
		let inventory = self.inventory.describe();
		match (menu.as_str(), inventory.as_str()) {
			("any", "") => "any".into(),
			("any", inventory) => inventory.into(),
			(menu, "") => menu.into(),
			(menu, inventory) => format!("{menu} {inventory}"),
		}
	}

	pub fn mismatch_detail(
		&self,
		menu: &MenuObservation,
		inventory: &InventoryObservation,
	) -> String {
		let have = if self.inventory.is_empty() {
			menu.describe()
		} else if self.menu.describe() == "any" {
			inventory.describe()
		} else {
			format!("{} {}", menu.describe(), inventory.describe())
		};
		format!("want {} have {have}", self.describe())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn omitted_inventory_fields_are_dont_care() -> anyhow::Result<()> {
		let matcher: AgentMatcher = serde_json::from_str(r#"{"cycle_category":"weapons"}"#)?;
		assert_eq!(matcher.inventory.cycle_category.as_deref(), Some("weapons"));
		assert!(matcher.inventory.weapon.is_none());
		assert!(matcher.matches(
			&MenuObservation::default(),
			&InventoryObservation {
				cycle_category: "weapons".into(),
				weapon: Some("bullpup".into()),
				..InventoryObservation::default()
			}
		));
		assert!(!matcher.matches(
			&MenuObservation::default(),
			&InventoryObservation { cycle_category: "lubikans".into(), ..Default::default() }
		));
		Ok(())
	}

	#[test]
	fn menu_and_inventory_fields_compose() -> anyhow::Result<()> {
		let matcher: AgentMatcher =
			serde_json::from_str(r#"{"pause":"playing","weapon":"reltor"}"#)?;
		assert_eq!(matcher.menu.pause.as_deref(), Some("playing"));
		assert_eq!(matcher.inventory.weapon.as_deref(), Some("reltor"));
		let menu = MenuObservation { pause: "playing".into(), ..MenuObservation::default() };
		let inventory = InventoryObservation {
			cycle_category: "weapons".into(),
			weapon: Some("reltor".into()),
			..InventoryObservation::default()
		};
		assert!(matcher.matches(&menu, &inventory));
		assert!(!matcher.matches(
			&menu,
			&InventoryObservation { weapon: Some("bullpup".into()), ..inventory.clone() }
		));
		assert!(!matcher.matches(
			&MenuObservation { pause: "menu".into(), ..MenuObservation::default() },
			&inventory
		));
		Ok(())
	}

	#[test]
	fn describe_joins_menu_and_inventory() {
		let matcher = AgentMatcher {
			menu: MenuMatcher { pause: Some("playing".into()), ..MenuMatcher::default() },
			inventory: InventoryMatcher {
				cycle_category: Some("skill_maps".into()),
				..InventoryMatcher::default()
			},
		};
		assert_eq!(matcher.describe(), "pause=playing cycle_category=skill_maps");
	}
}
