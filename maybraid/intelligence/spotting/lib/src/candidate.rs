use bevy::prelude::*;

/// Physics-independent ranking inputs for one observation candidate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotCandidate {
	pub subject: Entity,
	pub directive_priority: i32,
	pub salience: f32,
	pub distance: f32,
	pub known: bool,
	pub max_samples: usize,
}

impl SpotCandidate {
	/// Sort candidates by directive priority, salience, distance, known-contact
	/// continuity, and finally stable entity identity.
	pub fn rank(candidates: &mut [Self]) {
		candidates.sort_by(|a, b| {
			b.directive_priority
				.cmp(&a.directive_priority)
				.then_with(|| b.salience.total_cmp(&a.salience))
				.then_with(|| a.distance.total_cmp(&b.distance))
				.then_with(|| b.known.cmp(&a.known))
				.then_with(|| a.subject.to_bits().cmp(&b.subject.to_bits()))
		});
	}

	/// Retain only the highest-ranked candidate slots.
	pub fn apply_budget(candidates: &mut Vec<Self>, candidate_budget: usize) {
		candidates.truncate(candidate_budget);
	}

	/// Allocate a total sample budget over candidates in rank order.
	///
	/// Every candidate receives one sample before additional samples are
	/// distributed round-robin. Per-candidate limits are always respected.
	pub fn allocate_sample_budget(
		candidates: &[Self],
		candidate_budget: usize,
		vision_samples: usize,
	) -> Vec<usize> {
		let candidate_count = candidates.len().min(candidate_budget);
		let mut grants = vec![0; candidate_count];
		let mut remaining = vision_samples;
		while remaining > 0 {
			let mut progressed = false;
			for (grant, candidate) in grants.iter_mut().zip(candidates.iter()) {
				if remaining == 0 {
					break;
				}
				if *grant >= candidate.max_samples {
					continue;
				}
				*grant += 1;
				remaining -= 1;
				progressed = true;
			}
			if !progressed {
				break;
			}
		}
		grants
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn candidate(
		subject: u64,
		directive_priority: i32,
		salience: f32,
		distance: f32,
		known: bool,
		max_samples: usize,
	) -> SpotCandidate {
		SpotCandidate {
			subject: Entity::from_bits(subject),
			directive_priority,
			salience,
			distance,
			known,
			max_samples,
		}
	}

	#[test]
	fn budget_order_prefers_priority_salience_distance_then_known() -> anyhow::Result<()> {
		let mut candidates = vec![
			candidate(1, 0, 10.0, 1.0, true, 9),
			candidate(2, 2, 0.0, 20.0, false, 9),
			candidate(3, 2, 2.0, 20.0, false, 9),
			candidate(4, 2, 2.0, 20.0, true, 9),
			candidate(5, 2, 2.0, 5.0, true, 9),
			candidate(6, 2, 2.0, 1.0, false, 9),
		];
		SpotCandidate::rank(&mut candidates);
		assert_eq!(
			candidates.iter().map(|candidate| candidate.subject).collect::<Vec<_>>(),
			vec![
				Entity::from_bits(6),
				Entity::from_bits(5),
				Entity::from_bits(4),
				Entity::from_bits(3),
				Entity::from_bits(2),
				Entity::from_bits(1),
			]
		);
		Ok(())
	}

	#[test]
	fn sample_budget_caps_candidates_total_and_per_subject() -> anyhow::Result<()> {
		let candidates = vec![
			candidate(1, 0, 0.0, 0.0, false, 2),
			candidate(2, 0, 0.0, 0.0, false, 4),
			candidate(3, 0, 0.0, 0.0, false, 4),
		];
		assert_eq!(SpotCandidate::allocate_sample_budget(&candidates, 2, 5), vec![2, 3]);
		assert_eq!(SpotCandidate::allocate_sample_budget(&candidates, 3, 2), vec![1, 1, 0]);
		Ok(())
	}
}
