//! Kind selection: tiers, soft-goal gate, weighted pick among eligible specs.

use procedural_common::NoiseConfig;

use super::kind::{KindSpec, SoftGoalRole};

/// Program tier — eligibility changes as the pack progresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgramTier {
	Enclosure = 0,
	Appointed = 1,
	Filler = 2,
}

/// True when structure soft-goal is satisfied (ensuite and/or closet-like).
pub fn enclosure_soft_goal_met(has_ensuite: bool, closet_like_count: usize) -> bool {
	has_ensuite || closet_like_count > 0
}

/// Weighted pick among eligible [`KindSpec`]s.
///
/// Fillers stay out until `soft_goal_met`. Caps and zero weights exclude kinds.
/// Among remaining, noise picks with relative weights.
pub fn pick_kind<Id: Copy>(
	catalog: &[KindSpec<Id>],
	cfg: &NoiseConfig,
	salt: u32,
	soft_goal_met: bool,
	count_for: impl Fn(Id) -> usize,
) -> Option<Id> {
	let mut eligible: Vec<(Id, f32)> = Vec::new();
	for spec in catalog {
		if spec.weight <= 0.0 {
			continue;
		}
		if spec.tier == ProgramTier::Filler && !soft_goal_met {
			continue;
		}
		let n = count_for(spec.id);
		if let Some(max) = spec.max_count {
			if n >= max {
				continue;
			}
		}
		eligible.push((spec.id, spec.weight));
	}
	if eligible.is_empty() {
		return None;
	}
	let total: f32 = eligible.iter().map(|(_, w)| *w).sum::<f32>();
	if total <= 0.0 {
		return None;
	}
	let mut t = cfg.sample_unit_4d(salt as f32, 0.0, 0.0, 10.0) * total;
	for &(id, w) in &eligible {
		if t <= w {
			return Some(id);
		}
		t -= w;
	}
	eligible.last().map(|(id, _)| *id)
}

/// Whether a kind contributes closet-like / ensuite soft-goal credit.
pub fn soft_goal_credit(role: SoftGoalRole) -> (bool, bool) {
	match role {
		SoftGoalRole::None => (false, false),
		SoftGoalRole::ClosetLike | SoftGoalRole::Appointed => (true, false),
		SoftGoalRole::Ensuite => (false, true),
	}
}
