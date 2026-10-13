//! Entity-installed movement brain. Higher-order systems write [`Self::objective`] and [`ReplanMovement`].

use bevy::prelude::*;

use crate::ability::{MovementAbility, MovementBody};
use crate::candidate::MovementCandidate;
use crate::location::MovementLocation;
use crate::objective::MovementObjective;
use crate::step::{MovementDrive, MovementStep};

const FAILED_PLAN_RETRY_SECONDS: f32 = 0.6;
const MIN_TRANSIT_CORRIDOR: f32 = 0.55;
const MAX_TRANSIT_CORRIDOR: f32 = 0.9;
const TRANSIT_CORRIDOR_PADDING: f32 = 0.45;
const TRANSIT_MOVING_AWAY_SLOP: f32 = 0.06;
const DUPLICATE_TARGET_SLOP: f32 = 0.05;
const TRANSIT_LOOKAHEAD_MULTIPLIER: f32 = 0.65;
const MAX_OUTGOING_LEAD_FRACTION: f32 = 0.5;
const HAIRPIN_DOT_LIMIT: f32 = -0.25;
const PROGRESS_SLOP: f32 = 0.08;

/// Per-user scoring knobs. Budget and standoffs live on [`MovementAbility`].
/// Hide / sightline policy is carried on [`MovementObjective::VantageOn`] by the
/// higher-order writer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MovementIntelligenceSettings {
	pub stuck_timeout: f32,
	pub weight_surface: f32,
	/// Cost of using the mover's full tolerated fall, in surface-cost units.
	pub weight_fall: f32,
}

impl Default for MovementIntelligenceSettings {
	fn default() -> Self {
		Self { stuck_timeout: 1.6, weight_surface: 1.0, weight_fall: 6.0 }
	}
}

/// Capsule using movement intelligence. Inserting it is the FirearmUser-style install.
///
/// This crate does not follow other entities. A higher-order system writes [`Self::objective`]
/// and inserts [`ReplanMovement`] when it wants a new plan.
#[derive(Component, Debug, Clone)]
pub struct MovementIntelligence<I = MovementStep, A = MovementAbility>
where
	I: Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	pub objective: MovementObjective,
	pub settings: MovementIntelligenceSettings,
	pub ability: A,
	pub plan: Vec<I>,
	pub cursor: usize,
	pub stuck_seconds: f32,
	pub last_goal_distance: f32,
	pub failed_plan_retry_seconds: Option<f32>,
	pub transit_segment_start: Option<Vec3>,
	pub transit_closest_distance: f32,
}

impl MovementIntelligence {
	pub fn new(objective: MovementObjective) -> Self {
		Self {
			objective,
			settings: MovementIntelligenceSettings::default(),
			ability: MovementAbility::default(),
			plan: Vec::new(),
			cursor: 0,
			stuck_seconds: 0.0,
			last_goal_distance: f32::MAX,
			failed_plan_retry_seconds: None,
			transit_segment_start: None,
			transit_closest_distance: f32::MAX,
		}
	}
}

impl<I, A> MovementIntelligence<I, A>
where
	I: Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	/// Lower is better. Folds surface cost with objective hide / sightline.
	pub fn score_candidate(&self, candidate: &MovementCandidate<I>) -> f32 {
		let surface = self.settings.weight_surface * candidate.surface_cost;
		let hide = self.objective.hide_weight() * candidate.hints.hide;
		let sight = self.objective.sightline_weight() * candidate.hints.sightline;
		let fall = self.settings.weight_fall * candidate.hints.fall_risk;
		surface + fall - hide - sight
	}

	pub fn adopt_plan(&mut self, steps: Vec<I>) {
		self.plan = steps;
		self.cursor = 0;
		self.stuck_seconds = 0.0;
		self.last_goal_distance = f32::MAX;
		self.failed_plan_retry_seconds = None;
		self.transit_segment_start = None;
		self.transit_closest_distance = f32::MAX;
	}

	/// Keep an active route when a refresh finds nothing, then retry if that
	/// route runs out instead of silently stopping.
	pub fn note_replan_failed(&mut self) {
		self.failed_plan_retry_seconds = Some(0.0);
	}

	pub fn at_plan_end(&self) -> bool {
		self.cursor >= self.plan.len()
	}

	pub fn pick_best_candidate(
		&self,
		candidates: impl IntoIterator<Item = MovementCandidate<I>>,
	) -> Option<MovementCandidate<I>> {
		let mut best: Option<(f32, MovementCandidate<I>)> = None;
		for candidate in candidates {
			let score = self.score_candidate(&candidate);
			let take = best.as_ref().is_none_or(|(best_score, _)| score < *best_score);
			if take {
				best = Some((score, candidate));
			}
		}
		best.map(|(_, candidate)| candidate)
	}
}

impl<I, A> MovementIntelligence<I, A>
where
	I: Send + Sync + 'static,
	A: MovementBody + Send + Sync + 'static,
{
	/// Clear any active plan and hold at `at` using the mover's agent radius.
	pub fn hold_at(&mut self, at: Vec3) {
		self.objective =
			MovementObjective::Reach(MovementLocation::new(at, self.ability.agent_radius()));
		self.adopt_plan(Vec::new());
	}
}

/// Result of advancing a plan toward the next [`crate::MovementDrive`] target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementDriveResult {
	Wish(Vec3),
	Hold,
	/// A previous planning attempt failed and should be tried again.
	Retry,
	/// Planner is not approaching the current step. Drive still writes `wish`;
	/// local unstick belongs to the motor (strafe / jump / backup), not an immediate replan.
	Stuck {
		wish: Vec3,
	},
}

fn xz_wish(from: Vec3, toward_xz: Vec2) -> Vec3 {
	Vec3::new(toward_xz.x - from.x, 0.0, toward_xz.y - from.z).normalize_or_zero()
}

fn project_xz(start: Vec3, end: Vec3, position: Vec3) -> (f32, Vec2, f32) {
	let start_xz = start.xz();
	let span = end.xz() - start_xz;
	let length_sq = span.length_squared();
	if length_sq <= 1e-8 {
		return (0.0, start_xz, start_xz.distance(position.xz()));
	}
	let t = (position.xz() - start_xz).dot(span) / length_sq;
	let closest = start_xz + span * t.clamp(0.0, 1.0);
	(t, closest, closest.distance(position.xz()))
}

fn transit_wish(
	target: MovementLocation,
	next: Option<MovementLocation>,
	segment_start: Vec3,
	position: Vec3,
	corridor: f32,
) -> Vec3 {
	let direct = target.xz_wish_from(position);
	let Some(next) = next else {
		return direct;
	};
	let incoming = target.point.xz() - segment_start.xz();
	let incoming_length = incoming.length();
	let outgoing = next.point.xz() - target.point.xz();
	if incoming_length <= 1e-4 || outgoing.length_squared() <= 1e-6 {
		return direct;
	}
	let (t, closest, lateral) = project_xz(segment_start, target.point, position);
	if lateral > corridor {
		return xz_wish(position, closest);
	}
	let incoming_dir = incoming / incoming_length;
	let outgoing_dir = outgoing.normalize();
	if incoming_dir.dot(outgoing_dir) < HAIRPIN_DOT_LIMIT {
		return direct;
	}
	let lookahead = corridor * TRANSIT_LOOKAHEAD_MULTIPLIER;
	let remaining = (1.0 - t.clamp(0.0, 1.0)) * incoming_length;
	if remaining >= lookahead {
		return direct;
	}
	let lead_distance = (lookahead - remaining).min(outgoing.length() * MAX_OUTGOING_LEAD_FRACTION);
	xz_wish(position, target.point.xz() + outgoing_dir * lead_distance)
}

impl<I, A> MovementIntelligence<I, A>
where
	I: MovementDrive + Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	pub fn drive(&mut self, dt: f32, position: Vec3) -> MovementDriveResult {
		loop {
			if self.at_plan_end() {
				if let Some(elapsed) = &mut self.failed_plan_retry_seconds {
					*elapsed += dt;
					if *elapsed >= FAILED_PLAN_RETRY_SECONDS {
						*elapsed = 0.0;
						return MovementDriveResult::Retry;
					}
				}
				return MovementDriveResult::Hold;
			}
			let Some(target) = self.plan.get(self.cursor).and_then(I::drive_target) else {
				self.cursor += 1;
				self.transit_segment_start = Some(position);
				self.transit_closest_distance = f32::MAX;
				continue;
			};
			let segment_start = self.transit_segment_start.unwrap_or(position);
			if self.transit_segment_start.is_none() {
				self.transit_segment_start = Some(position);
			}
			let next_target =
				self.plan[self.cursor + 1..].iter().find_map(|step| step.drive_target());
			let has_later_target = next_target.is_some();
			let pass_corridor = (target.radius + TRANSIT_CORRIDOR_PADDING)
				.clamp(MIN_TRANSIT_CORRIDOR, MAX_TRANSIT_CORRIDOR);
			let target_distance = target.xz_distance(position);
			let within_vertical = (position.y - target.point.y).abs() <= target.vertical_slop();
			let within_pass_region = target_distance <= pass_corridor && within_vertical;
			let passed_outgoing = has_later_target
				&& next_target.is_some_and(|next| {
					target.following_xz_toward(next.point, position, pass_corridor)
				});
			let passed_closest = has_later_target
				&& within_pass_region
				&& target_distance > self.transit_closest_distance + TRANSIT_MOVING_AWAY_SLOP;
			let duplicate_transit = has_later_target
				&& (target.point.distance(segment_start) <= DUPLICATE_TARGET_SLOP
					|| next_target.is_some_and(|next| {
						next.point.distance(target.point) <= DUPLICATE_TARGET_SLOP
					}));
			if target.contains(position) || passed_outgoing || passed_closest || duplicate_transit {
				self.cursor += 1;
				self.transit_segment_start = Some(target.point);
				self.transit_closest_distance = f32::MAX;
				self.stuck_seconds = 0.0;
				self.last_goal_distance = f32::MAX;
				continue;
			}
			if within_vertical {
				self.transit_closest_distance = self.transit_closest_distance.min(target_distance);
			}
			let dist = target.approach_distance(position);
			let wish = transit_wish(target, next_target, segment_start, position, pass_corridor);
			if dist + PROGRESS_SLOP < self.last_goal_distance {
				self.last_goal_distance = dist;
				self.stuck_seconds = 0.0;
			} else {
				self.stuck_seconds += dt;
				if self.stuck_seconds >= self.settings.stuck_timeout {
					self.stuck_seconds = 0.0;
					return MovementDriveResult::Stuck { wish };
				}
			}
			return MovementDriveResult::Wish(wish);
		}
	}
}

/// Higher-order request to rebuild [`MovementIntelligence::plan`] for the current objective.
#[derive(Component, Debug, Clone, Copy)]
#[component(storage = "SparseSet")]
pub struct ReplanMovement;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::candidate::MovementCandidateHints;
	use crate::location::MovementLocation;
	use crate::step::MovementStep;

	fn vantage() -> MovementObjective {
		MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.0),
			hide_weight: 10.0,
			sightline_weight: 12.0,
		}
	}

	fn candidate(hide: f32, sightline: f32, surface_cost: f32) -> MovementCandidate<MovementStep> {
		MovementCandidate::new(
			vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.5))],
			surface_cost,
			MovementCandidateHints { hide, sightline, min_clearance: 1.0, fall_risk: 0.0 },
		)
	}

	#[test]
	fn score_prefers_hide_and_sightline_together() -> anyhow::Result<()> {
		let brain = MovementIntelligence::new(vantage());
		let peek = brain.score_candidate(&candidate(1.0, 1.0, 10.0));
		let cover = brain.score_candidate(&candidate(1.0, 0.0, 10.0));
		let open = brain.score_candidate(&candidate(0.0, 1.0, 10.0));
		assert!(peek < cover, "{peek} vs cover {cover}");
		assert!(peek < open, "{peek} vs open {open}");
		Ok(())
	}

	#[test]
	fn score_prefers_a_short_detour_to_a_tolerated_fall() -> anyhow::Result<()> {
		let brain = MovementIntelligence::new(vantage());
		let mut risky = candidate(0.0, 0.0, 5.0);
		risky.hints.fall_risk = 0.5;
		let safe = candidate(0.0, 0.0, 7.0);
		assert!(brain.score_candidate(&safe) < brain.score_candidate(&risky));
		Ok(())
	}

	#[test]
	fn drive_holds_when_plan_is_empty() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		assert_eq!(brain.drive(0.016, Vec3::ZERO), MovementDriveResult::Hold);
		Ok(())
	}

	#[test]
	fn drive_advances_when_inside_waypoint() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::ZERO, 1.0))]);
		assert_eq!(brain.drive(0.016, Vec3::new(0.2, 0.0, 0.1)), MovementDriveResult::Hold);
		assert!(brain.at_plan_end());
		Ok(())
	}

	#[test]
	fn drive_reports_stuck_when_distance_does_not_fall() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.settings.stuck_timeout = 0.08;
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X * 8.0, 0.4))]);
		let mut saw_stuck = false;
		for _ in 0..20 {
			if matches!(brain.drive(0.016, Vec3::ZERO), MovementDriveResult::Stuck { .. }) {
				saw_stuck = true;
			}
		}
		assert!(saw_stuck);
		Ok(())
	}

	#[test]
	fn failed_refresh_keeps_active_plan_and_retries_after_it_ends() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X * 2.0, 0.2))]);
		brain.note_replan_failed();

		assert_eq!(brain.plan.len(), 1);
		assert!(matches!(brain.drive(0.016, Vec3::ZERO), MovementDriveResult::Wish(_)));

		brain.cursor = brain.plan.len();
		assert_eq!(brain.drive(FAILED_PLAN_RETRY_SECONDS, Vec3::ZERO), MovementDriveResult::Retry);
		Ok(())
	}

	#[test]
	fn drive_advances_once_following_the_outgoing_segment() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![
			MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.1)),
			MovementStep::MoveTo(MovementLocation::new(Vec3::new(1.0, 0.0, 2.0), 0.1)),
		]);
		assert!(matches!(brain.drive(0.016, Vec3::ZERO), MovementDriveResult::Wish(_)));
		let position = Vec3::new(1.1, 0.0, 0.4);
		assert!(matches!(brain.drive(0.016, position), MovementDriveResult::Wish(_)));
		assert_eq!(brain.cursor, 1);
		Ok(())
	}

	#[test]
	fn drive_keeps_transit_waypoint_when_past_the_plane_off_the_outgoing_leg() -> anyhow::Result<()>
	{
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![
			MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.1)),
			MovementStep::MoveTo(MovementLocation::new(Vec3::new(1.0, 0.0, 2.0), 0.1)),
		]);
		brain.drive(0.016, Vec3::ZERO);
		brain.drive(0.016, Vec3::new(1.2, 0.0, -0.5));
		assert_eq!(brain.cursor, 0);
		Ok(())
	}

	#[test]
	fn drive_requires_radius_arrival_at_final_waypoint() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.1))]);
		brain.drive(0.016, Vec3::ZERO);
		brain.drive(0.016, Vec3::X * 1.2);
		assert_eq!(brain.cursor, 0);
		assert_eq!(brain.transit_segment_start, Some(Vec3::ZERO));
		Ok(())
	}

	#[test]
	fn adopting_plan_resets_transit_segment() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.1))]);
		brain.drive(0.016, Vec3::ZERO);
		assert_eq!(brain.transit_segment_start, Some(Vec3::ZERO));
		brain.adopt_plan(Vec::new());
		assert_eq!(brain.transit_segment_start, None);
		assert_eq!(brain.transit_closest_distance, f32::MAX);
		Ok(())
	}

	#[test]
	fn drive_releases_transit_after_close_orbit_starts_moving_away() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![
			MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.1)),
			MovementStep::MoveTo(MovementLocation::new(Vec3::new(1.0, 0.0, 2.0), 0.1)),
		]);
		brain.drive(0.016, Vec3::ZERO);
		brain.drive(0.016, Vec3::new(0.9, 0.0, -0.3));
		assert_eq!(brain.cursor, 0);
		brain.drive(0.016, Vec3::new(0.82, 0.0, -0.45));
		assert_eq!(brain.cursor, 1);
		Ok(())
	}

	#[test]
	fn drive_skips_duplicate_intermediate_target() -> anyhow::Result<()> {
		let duplicate = MovementLocation::new(Vec3::X, 0.1);
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![
			MovementStep::MoveTo(duplicate),
			MovementStep::MoveTo(duplicate),
			MovementStep::MoveTo(MovementLocation::new(Vec3::new(1.0, 0.0, 2.0), 0.1)),
		]);
		brain.drive(0.016, Vec3::ZERO);
		brain.drive(0.016, Vec3::new(1.1, 0.0, 0.1));
		assert_eq!(brain.cursor, 2);
		Ok(())
	}

	#[test]
	fn drive_anticipates_outgoing_segment_without_reducing_wish() -> anyhow::Result<()> {
		let mut brain = MovementIntelligence::new(vantage());
		brain.adopt_plan(vec![
			MovementStep::MoveTo(MovementLocation::new(Vec3::X, 0.1)),
			MovementStep::MoveTo(MovementLocation::new(Vec3::new(1.0, 0.0, 2.0), 0.1)),
		]);
		brain.drive(0.016, Vec3::ZERO);
		let MovementDriveResult::Wish(wish) = brain.drive(0.016, Vec3::X * 0.85) else {
			anyhow::bail!("expected movement wish");
		};
		assert!(wish.x > 0.0 && wish.z > 0.0, "{wish}");
		assert!((wish.length() - 1.0).abs() < 1e-4, "{wish}");
		Ok(())
	}

	#[test]
	fn transit_wish_does_not_cut_a_hairpin_or_final_target() -> anyhow::Result<()> {
		let target = MovementLocation::new(Vec3::X, 0.1);
		let hairpin = MovementLocation::new(Vec3::ZERO, 0.1);
		let position = Vec3::X * 0.85;
		assert_eq!(transit_wish(target, Some(hairpin), Vec3::ZERO, position, 0.55), Vec3::X);
		assert_eq!(transit_wish(target, None, Vec3::ZERO, position, 0.55), Vec3::X);
		Ok(())
	}

	#[test]
	fn transit_wish_pulls_back_onto_the_incoming_segment() -> anyhow::Result<()> {
		let target = MovementLocation::new(Vec3::X, 0.1);
		let next = MovementLocation::new(Vec3::new(1.0, 0.0, 2.0), 0.1);
		let wish = transit_wish(target, Some(next), Vec3::ZERO, Vec3::new(0.5, 0.0, 1.2), 0.55);
		assert!(wish.z < 0.0, "{wish}");
		assert!((wish.length() - 1.0).abs() < 1e-4, "{wish}");
		Ok(())
	}

	#[test]
	fn transit_wish_stays_horizontal_at_a_raised_waypoint() -> anyhow::Result<()> {
		let target = MovementLocation::new(Vec3::new(4.0, 3.0, 0.0), 0.1);
		let wish = transit_wish(target, None, Vec3::ZERO, Vec3::ZERO, 0.55);
		assert!(wish.x > 0.0 && wish.y.abs() < 1e-6, "{wish}");
		Ok(())
	}
}
