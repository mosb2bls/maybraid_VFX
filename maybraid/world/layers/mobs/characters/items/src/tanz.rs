//! Shared Tanz identity for inventory, persistence, and menus.
//!
//! Action timing, clips, and motor wishes live in `tanz-user`. This crate only
//! owns the serializable kind, display names, and stable script IDs.

use serde::{Deserialize, Serialize};

/// Authored Tanz a character can own. Equipment priority is inventory order,
/// not this enum order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TanzId {
	HealingSalute,
	LeftStrafe,
	RightStrafe,
	ForwardDash,
}

impl TanzId {
	/// Canonical starter kinds, highest matching priority first.
	pub const STARTER: [Self; 4] =
		[Self::HealingSalute, Self::LeftStrafe, Self::RightStrafe, Self::ForwardDash];

	pub const fn script_id(self) -> &'static str {
		match self {
			Self::HealingSalute => "healing_salute",
			Self::LeftStrafe => "left_strafe",
			Self::RightStrafe => "right_strafe",
			Self::ForwardDash => "forward_dash",
		}
	}

	pub fn from_script_id(raw: &str) -> Option<Self> {
		match raw.trim().to_ascii_lowercase().as_str() {
			"healing_salute" | "salute" => Some(Self::HealingSalute),
			"left_strafe" | "left" => Some(Self::LeftStrafe),
			"right_strafe" | "right" => Some(Self::RightStrafe),
			"forward_dash" | "dash" | "forward" => Some(Self::ForwardDash),
			_ => None,
		}
	}

	pub const fn display_name(self) -> &'static str {
		match self {
			Self::HealingSalute => "Healing Salute",
			Self::LeftStrafe => "Left Strafe",
			Self::RightStrafe => "Right Strafe",
			Self::ForwardDash => "Forward Dash",
		}
	}

	/// Compact input hint for inventory tiles. Gameplay ranges stay in `tanz-user`.
	pub const fn input_summary(self) -> &'static str {
		match self {
			Self::HealingSalute => "A · Forward",
			Self::LeftStrafe => "Left stick",
			Self::RightStrafe => "Right stick",
			Self::ForwardDash => "Forward stick",
		}
	}

	pub const fn capture_id_label(self) -> &'static str {
		match self {
			Self::HealingSalute => "tanz:healing_salute",
			Self::LeftStrafe => "tanz:left_strafe",
			Self::RightStrafe => "tanz:right_strafe",
			Self::ForwardDash => "tanz:forward_dash",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn script_ids_round_trip_and_accept_aliases() {
		assert_eq!(TanzId::from_script_id("left_strafe"), Some(TanzId::LeftStrafe));
		assert_eq!(TanzId::from_script_id("DASH"), Some(TanzId::ForwardDash));
		assert_eq!(TanzId::LeftStrafe.script_id(), "left_strafe");
		assert_eq!(TanzId::LeftStrafe.display_name(), "Left Strafe");
		assert_eq!(TanzId::HealingSalute.input_summary(), "A · Forward");
		assert!(TanzId::from_script_id("spin").is_none());
	}
}
