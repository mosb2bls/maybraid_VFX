//! Menu observation and matchers. Bevy-free so playback tests stay cheap.
//!
//! The game crate fills [`MenuObservation`]; the world agent plugin reads it
//! without importing menu types.

use serde::{Deserialize, Deserializer, Serialize};

use crate::command::AgentButton;

/// Live menu snapshot the game crate publishes for scripts and events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuObservation {
	/// `home`, `gallery`, `spin_reveal`, `character`, `in_game`, `settings`,
	/// `loading`, or `none`.
	pub screen: String,
	/// Leave prompt (`save` / `save_failed` / `discard`), `name`, a catalog
	/// overlay key, or `None` when no modal is open.
	pub modal: Option<String>,
	pub focus_row: Option<u32>,
	pub focus_label: Option<String>,
	/// `home`, `characters`, `loading_world`, `world`.
	pub flow: String,
	/// `playing` or `menu`.
	pub pause: String,
	/// First full-screen menu root is up.
	pub menu_ready: bool,
	/// Invariant breaks detected this frame (empty when healthy).
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub invariants: Vec<String>,
}

impl Default for MenuObservation {
	fn default() -> Self {
		Self {
			screen: "none".into(),
			modal: None,
			focus_row: None,
			focus_label: None,
			flow: String::new(),
			pause: String::new(),
			menu_ready: false,
			invariants: Vec::new(),
		}
	}
}

impl MenuObservation {
	pub fn describe(&self) -> String {
		let modal = self.modal.as_deref().unwrap_or("null");
		let mut detail = format!(
			"screen={} modal={} flow={} pause={}",
			self.screen, modal, self.flow, self.pause
		);
		if let Some(row) = self.focus_row {
			detail.push_str(&format!(" focus={row}"));
			if let Some(label) = &self.focus_label {
				detail.push_str(&format!("({label})"));
			}
		}
		detail
	}

	pub fn focus_changed(&self, previous: &Self) -> bool {
		self.screen != previous.screen
			|| self.focus_row != previous.focus_row
			|| self.focus_label != previous.focus_label
	}
}

/// Optional fields a `wait_for` / `expect` command can pin.
///
/// A missing field is "don't care". `"modal": null` means no modal.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MenuMatcher {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub screen: Option<String>,
	#[serde(
		default,
		skip_serializing_if = "Option::is_none",
		deserialize_with = "deserialize_present_option",
		serialize_with = "serialize_present_option"
	)]
	pub modal: Option<Option<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub flow: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub pause: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub focus_row: Option<u32>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub focus_label: Option<String>,
}

impl MenuMatcher {
	pub fn matches(&self, observed: &MenuObservation) -> bool {
		if let Some(screen) = &self.screen {
			if observed.screen != *screen {
				return false;
			}
		}
		if let Some(modal) = &self.modal {
			if &observed.modal != modal {
				return false;
			}
		}
		if let Some(flow) = &self.flow {
			if observed.flow != *flow {
				return false;
			}
		}
		if let Some(pause) = &self.pause {
			if observed.pause != *pause {
				return false;
			}
		}
		if let Some(row) = self.focus_row {
			if observed.focus_row != Some(row) {
				return false;
			}
		}
		if let Some(label) = &self.focus_label {
			if observed.focus_label.as_deref() != Some(label.as_str()) {
				return false;
			}
		}
		true
	}

	pub fn describe(&self) -> String {
		let mut parts = Vec::new();
		if let Some(screen) = &self.screen {
			parts.push(format!("screen={screen}"));
		}
		if let Some(modal) = &self.modal {
			parts.push(format!("modal={}", modal.as_deref().unwrap_or("null")));
		}
		if let Some(flow) = &self.flow {
			parts.push(format!("flow={flow}"));
		}
		if let Some(pause) = &self.pause {
			parts.push(format!("pause={pause}"));
		}
		if let Some(row) = self.focus_row {
			parts.push(format!("focus_row={row}"));
		}
		if let Some(label) = &self.focus_label {
			parts.push(format!("focus_label={label}"));
		}
		if parts.is_empty() {
			"any".into()
		} else {
			parts.join(" ")
		}
	}

	pub fn mismatch_detail(&self, observed: &MenuObservation) -> String {
		format!("want {} have {}", self.describe(), observed.describe())
	}
}

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
	D: Deserializer<'de>,
	T: Deserialize<'de>,
{
	Ok(Some(T::deserialize(deserializer)?))
}

fn serialize_present_option<S, T>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
	T: Serialize,
{
	match value {
		Some(inner) => inner.serialize(serializer),
		None => serializer.serialize_none(),
	}
}

/// SplitMix64. Same seed always yields the same `mash` press sequence.
pub fn mash_rng_next(state: &mut u64) -> u64 {
	*state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
	let mut z = *state;
	z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
	z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
	z ^ (z >> 31)
}

/// One seeded mash pulse: a single button, or a two-button chord (~1/4).
pub fn mash_press(state: &mut u64, buttons: &[AgentButton]) -> Vec<AgentButton> {
	if buttons.is_empty() {
		return Vec::new();
	}
	let first = buttons[(mash_rng_next(state) as usize) % buttons.len()];
	if buttons.len() > 1 && mash_rng_next(state) % 4 == 0 {
		let second = buttons[(mash_rng_next(state) as usize) % buttons.len()];
		if second != first {
			return vec![first, second];
		}
	}
	vec![first]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn omitted_modal_is_dont_care() -> anyhow::Result<()> {
		let matcher: MenuMatcher = serde_json::from_str(r#"{"screen":"in_game"}"#)?;
		assert_eq!(matcher.modal, None);
		assert!(matcher.matches(&MenuObservation {
			screen: "in_game".into(),
			modal: Some("discard".into()),
			..MenuObservation::default()
		}));
		Ok(())
	}

	#[test]
	fn null_modal_requires_closed() -> anyhow::Result<()> {
		let matcher: MenuMatcher = serde_json::from_str(r#"{"screen":"in_game","modal":null}"#)?;
		assert_eq!(matcher.modal, Some(None));
		let closed = MenuObservation { screen: "in_game".into(), ..MenuObservation::default() };
		let open = MenuObservation {
			screen: "in_game".into(),
			modal: Some("discard".into()),
			..MenuObservation::default()
		};
		assert!(matcher.matches(&closed));
		assert!(!matcher.matches(&open));
		Ok(())
	}

	#[test]
	fn focus_label_and_row_match() -> anyhow::Result<()> {
		let matcher: MenuMatcher =
			serde_json::from_str(r#"{"screen":"home","focus_label":"Characters"}"#)?;
		assert_eq!(matcher.focus_label.as_deref(), Some("Characters"));
		let characters = MenuObservation {
			screen: "home".into(),
			focus_row: Some(2),
			focus_label: Some("Characters".into()),
			..MenuObservation::default()
		};
		let reliquary = MenuObservation {
			screen: "home".into(),
			focus_row: Some(1),
			focus_label: Some("Reliquary".into()),
			..MenuObservation::default()
		};
		assert!(matcher.matches(&characters));
		assert!(!matcher.matches(&reliquary));
		let by_row: MenuMatcher = serde_json::from_str(r#"{"focus_row":2}"#)?;
		assert!(by_row.matches(&characters));
		assert!(!by_row.matches(&reliquary));
		Ok(())
	}

	#[test]
	fn mash_seed_is_deterministic() {
		let buttons =
			[AgentButton::Jump, AgentButton::Stance, AgentButton::Menu, AgentButton::DpadUp];
		let mut a = 42;
		let mut b = 42;
		let seq_a: Vec<_> = (0..32).map(|_| mash_press(&mut a, &buttons)).collect();
		let seq_b: Vec<_> = (0..32).map(|_| mash_press(&mut b, &buttons)).collect();
		assert_eq!(seq_a, seq_b);
		let mut c = 7;
		let seq_c: Vec<_> = (0..32).map(|_| mash_press(&mut c, &buttons)).collect();
		assert_ne!(seq_a, seq_c);
	}
}
