//! Trigger cadence sitting beside [`Weapon`] interval.

use bevy::prelude::*;

/// Max shots one fire tick may emit after a hitch. Drops leftover debt past this.
pub const CATCH_UP_SHOTS: u8 = 3;

const CATCH_UP_CAP: usize = CATCH_UP_SHOTS as usize;

/// Shots due this tick, oldest first. `ages[i]` is how late that round is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShotClockTick {
	ages: [f32; CATCH_UP_CAP],
	/// How many entries in [`Self::ages`] are live.
	pub shots: u8,
}

impl ShotClockTick {
	/// Leave-times, oldest first. Each value is seconds before now, in `0..=dt`.
	pub fn ages(self) -> impl Iterator<Item = f32> {
		self.ages.into_iter().take(self.shots as usize)
	}
}

/// How many shots this tick may catch up for `control`. Semi is one rising edge.
pub fn catch_up_limit(control: Option<&FireControl>) -> u8 {
	match control.map(|control| control.cadence) {
		Some(Cadence::Semi) => 1,
		Some(Cadence::Burst) => {
			CATCH_UP_SHOTS.min(control.map(|control| control.burst_left).unwrap_or(0))
		}
		_ => CATCH_UP_SHOTS,
	}
}

/// Drain unused trigger time without banking shots for the next pull.
pub fn idle_shot_clock(cooldown: &mut f32, dt: f32) {
	*cooldown = (*cooldown - dt).max(0.0);
}

/// Subtract `dt` from `cooldown` and return the shots this tick should emit.
///
/// Leftover time stays on `cooldown` (`+= interval` per shot). Hitting `max_shots`
/// while still overdue snaps `cooldown` to `interval` so a pause cannot dump a magazine.
pub fn advance_shot_clock(
	cooldown: &mut f32,
	interval: f32,
	dt: f32,
	max_shots: u8,
) -> ShotClockTick {
	let max_shots = max_shots.min(CATCH_UP_SHOTS);
	*cooldown -= dt;
	if *cooldown > 0.0 || max_shots == 0 {
		return ShotClockTick::default();
	}
	let interval = interval.max(0.0);
	if interval <= 0.0 {
		*cooldown = 0.0;
		return ShotClockTick { ages: [0.0; CATCH_UP_CAP], shots: 1 };
	}
	let mut tick = ShotClockTick::default();
	while *cooldown <= 0.0 && tick.shots < max_shots {
		tick.ages[tick.shots as usize] = (-*cooldown).clamp(0.0, dt);
		tick.shots += 1;
		*cooldown += interval;
	}
	if tick.shots >= max_shots && *cooldown <= 0.0 {
		*cooldown = interval;
	}
	tick
}

/// How a held trigger becomes shots. Interval still lives on [`crate::Weapon`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cadence {
	#[default]
	Auto,
	Semi,
	Burst,
	Gated,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TriggerIntent {
	pub held: bool,
	pub rising: bool,
}

/// Per-gun burst / semi state. Missing means auto.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireControl {
	pub cadence: Cadence,
	pub burst_rounds: u8,
	pub burst_left: u8,
	pub trigger_was: bool,
}

impl Default for FireControl {
	fn default() -> Self {
		Self::auto()
	}
}

impl FireControl {
	pub fn auto() -> Self {
		Self { cadence: Cadence::Auto, burst_rounds: 0, burst_left: 0, trigger_was: false }
	}

	pub fn semi() -> Self {
		Self { cadence: Cadence::Semi, burst_rounds: 0, burst_left: 0, trigger_was: false }
	}

	pub fn burst(rounds: u8) -> Self {
		let rounds = rounds.max(1);
		Self {
			cadence: Cadence::Burst,
			burst_rounds: rounds,
			burst_left: rounds,
			trigger_was: false,
		}
	}

	pub fn gated() -> Self {
		Self { cadence: Cadence::Gated, burst_rounds: 0, burst_left: 0, trigger_was: false }
	}

	pub fn poll(&mut self, held: bool) -> TriggerIntent {
		let rising = held && !self.trigger_was;
		if !held {
			self.burst_left = self.burst_rounds;
		}
		self.trigger_was = held;
		TriggerIntent { held, rising }
	}

	pub fn allows(&self, intent: TriggerIntent) -> bool {
		match self.cadence {
			Cadence::Auto | Cadence::Gated => intent.held,
			Cadence::Semi => intent.rising,
			Cadence::Burst => intent.held && self.burst_left > 0,
		}
	}

	pub fn note_shot(&mut self) {
		if self.cadence == Cadence::Burst {
			self.burst_left = self.burst_left.saturating_sub(1);
		}
	}
}

/// Look/camera kick strength in radians after a shot. Zero skips the pattern.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponRecoil(pub f32);

#[derive(Message, Clone, Copy, Debug)]
pub struct WeaponFired {
	pub shooter: Entity,
	pub recoil: f32,
}

pub fn trigger_allows_fire(control: Option<&mut FireControl>, manual: bool, held: bool) -> bool {
	if !manual {
		return true;
	}
	let Some(control) = control else {
		return held;
	};
	let intent = control.poll(held);
	control.allows(intent)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn auto_fires_while_held() {
		let mut control = FireControl::auto();
		let intent = control.poll(true);
		assert!(control.allows(intent));
		let intent = control.poll(false);
		assert!(!control.allows(intent));
	}

	#[test]
	fn semi_fires_on_rising_edge_only() {
		let mut control = FireControl::semi();
		let intent = control.poll(true);
		assert!(control.allows(intent));
		let intent = control.poll(true);
		assert!(!control.allows(intent));
		let intent = control.poll(false);
		assert!(!control.allows(intent));
		let intent = control.poll(true);
		assert!(control.allows(intent));
	}

	#[test]
	fn burst_stops_until_release() {
		let mut control = FireControl::burst(2);
		let intent = control.poll(true);
		assert!(control.allows(intent));
		control.note_shot();
		let intent = control.poll(true);
		assert!(control.allows(intent));
		control.note_shot();
		let intent = control.poll(true);
		assert!(!control.allows(intent));
		let intent = control.poll(false);
		assert!(!control.allows(intent));
		let intent = control.poll(true);
		assert!(control.allows(intent));
	}

	#[test]
	fn six_hundred_rpm_at_sixteen_ms_is_ten_shots_per_second() {
		let interval = 60.0 / 600.0;
		let mut cooldown = 0.0;
		let mut shots = 0u32;
		for _ in 0..62 {
			shots +=
				u32::from(advance_shot_clock(&mut cooldown, interval, 0.016, CATCH_UP_SHOTS).shots);
		}
		assert!((9..=11).contains(&shots), "{shots}");
	}

	#[test]
	fn fifteen_hundred_rpm_at_thirty_three_ms_stays_near_catalog() {
		let interval = 60.0 / 1500.0;
		let mut cooldown = 0.0;
		let mut shots = 0u32;
		let dt = 0.033;
		let ticks = (1.0_f32 / dt).round() as u32;
		for _ in 0..ticks {
			shots +=
				u32::from(advance_shot_clock(&mut cooldown, interval, dt, CATCH_UP_SHOTS).shots);
		}
		// Tick quantization can rise toward 1/dt (30 Hz); leftover-discard used to fall to ~15 Hz.
		assert!(shots >= 24, "{shots}");
		assert!(shots <= ticks, "{shots} vs {ticks}");
	}

	#[test]
	fn hitch_does_not_exceed_catch_up_cap() {
		let interval = 60.0 / 1500.0;
		let mut cooldown = 0.0;
		let tick = advance_shot_clock(&mut cooldown, interval, 1.0, CATCH_UP_SHOTS);
		assert_eq!(tick.shots, CATCH_UP_SHOTS);
		assert!(cooldown > 0.0);
	}

	#[test]
	fn hitch_ages_are_spaced_by_interval() {
		let interval = 0.04;
		let dt = 0.1;
		let mut cooldown = 0.0;
		let ages: Vec<f32> =
			advance_shot_clock(&mut cooldown, interval, dt, CATCH_UP_SHOTS).ages().collect();
		assert_eq!(ages.len(), 3);
		assert!((ages[0] - 0.1).abs() < 1e-5, "{ages:?}");
		assert!((ages[1] - 0.06).abs() < 1e-5, "{ages:?}");
		assert!((ages[2] - 0.02).abs() < 1e-5, "{ages:?}");
	}

	#[test]
	fn semi_is_one_shot_per_tick() {
		assert_eq!(catch_up_limit(Some(&FireControl::semi())), 1);
		let mut cooldown = 0.0;
		let tick = advance_shot_clock(&mut cooldown, 0.05, 0.25, 1);
		assert_eq!(tick.shots, 1);
	}

	#[test]
	fn idle_does_not_bank_owed_shots() {
		let mut cooldown = 0.0;
		for _ in 0..60 {
			idle_shot_clock(&mut cooldown, 0.016);
		}
		assert!(cooldown.abs() < 1e-6, "{cooldown}");
		let tick = advance_shot_clock(&mut cooldown, 60.0 / 900.0, 0.016, CATCH_UP_SHOTS);
		assert_eq!(tick.shots, 1);
	}

	#[test]
	fn missing_barrel_does_not_consume_a_semi_edge() {
		let mut control = FireControl::semi();
		let held = true;
		let barrel_present = false;
		if barrel_present {
			let intent = control.poll(held);
			assert!(control.allows(intent));
		}
		assert!(!control.trigger_was);
		let intent = control.poll(true);
		assert!(control.allows(intent));
	}

	#[test]
	fn held_auto_counts_match_when_the_barrel_is_always_present() {
		fn held_auto_shots(interval: f32, dt: f32, secs: f32) -> u32 {
			let mut cooldown = 0.0;
			let mut shots = 0u32;
			let mut t = 0.0;
			while t < secs - 1e-6 {
				t += dt;
				shots += u32::from(
					advance_shot_clock(&mut cooldown, interval, dt, CATCH_UP_SHOTS).shots,
				);
			}
			shots
		}
		let hip = held_auto_shots(0.1, 0.016, 2.0);
		let ads = held_auto_shots(0.1, 0.016, 2.0);
		assert_eq!(hip, ads);
		assert!((19..=21).contains(&hip), "{hip}");
	}

	fn simulate_held(mut control: FireControl, interval: f32, dt: f32, secs: f32) -> u32 {
		let mut cooldown = 0.0;
		let mut shots = 0u32;
		let mut t = 0.0;
		while t < secs - 1e-6 {
			t += dt;
			let allowed = trigger_allows_fire(Some(&mut control), true, true);
			if !allowed {
				idle_shot_clock(&mut cooldown, dt);
				continue;
			}
			let tick =
				advance_shot_clock(&mut cooldown, interval, dt, catch_up_limit(Some(&control)));
			for _ in tick.ages() {
				if control.cadence == Cadence::Burst && control.burst_left == 0 {
					break;
				}
				shots += 1;
				control.note_shot();
			}
		}
		shots
	}

	fn simulate_idle_then_hold(
		mut control: FireControl,
		interval: f32,
		dt: f32,
		idle: f32,
		hold: f32,
	) -> Vec<u8> {
		let mut cooldown = 0.0;
		let mut t = 0.0;
		while t < idle - 1e-6 {
			t += dt;
			idle_shot_clock(&mut cooldown, dt);
		}
		let mut per_tick = Vec::new();
		t = 0.0;
		while t < hold - 1e-6 {
			t += dt;
			let allowed = trigger_allows_fire(Some(&mut control), true, true);
			if !allowed {
				idle_shot_clock(&mut cooldown, dt);
				per_tick.push(0);
				continue;
			}
			let tick =
				advance_shot_clock(&mut cooldown, interval, dt, catch_up_limit(Some(&control)));
			let mut n = 0u8;
			for _ in tick.ages() {
				if control.cadence == Cadence::Burst && control.burst_left == 0 {
					break;
				}
				n += 1;
				control.note_shot();
			}
			per_tick.push(n);
		}
		per_tick
	}

	#[test]
	fn idle_then_burst_is_one_shot_per_tick() {
		let interval = 60.0 / 900.0;
		let dt = 0.016;
		let per_tick = simulate_idle_then_hold(FireControl::burst(3), interval, dt, 1.0, 0.25);
		assert!(per_tick.iter().all(|n| *n <= 1), "{per_tick:?}");
		assert_eq!(per_tick.iter().sum::<u8>(), 3);
		let fired: Vec<usize> =
			per_tick.iter().enumerate().filter_map(|(i, n)| (*n > 0).then_some(i)).collect();
		assert_eq!(fired.len(), 3);
		let span = (fired[2] - fired[0]) as f32 * dt;
		assert!((span - 2.0 * interval).abs() < dt + 1e-4, "span {span}");
	}

	#[test]
	fn locked_dt_hip_and_ads_spawn_counts_match() {
		let dt = 0.016;
		let secs = 2.0;
		let cases: [(FireControl, f32); 5] = [
			(FireControl::auto(), 60.0 / 600.0),
			(FireControl::auto(), 60.0 / 1500.0),
			(FireControl::burst(3), 60.0 / 900.0),
			(FireControl::semi(), 1.0 / 20.0),
			(FireControl::gated(), 1.2),
		];
		for (control, interval) in cases {
			let hip = simulate_held(control, interval, dt, secs);
			let ads = simulate_held(control, interval, dt, secs);
			assert_eq!(hip, ads, "cadence {:?}", control.cadence);
		}
	}

	#[test]
	fn locked_dt_held_cadence_counts() {
		let dt = 0.016;
		assert!((19..=21).contains(&simulate_held(FireControl::auto(), 0.1, dt, 2.0)));
		let fast = simulate_held(FireControl::auto(), 60.0 / 1500.0, dt, 2.0);
		assert!((48..=52).contains(&fast), "{fast}");
		assert_eq!(simulate_held(FireControl::burst(3), 60.0 / 900.0, dt, 2.0), 3);
		assert_eq!(simulate_held(FireControl::semi(), 1.0 / 20.0, dt, 2.0), 1);
		assert_eq!(simulate_held(FireControl::gated(), 1.2, dt, 2.0), 2);
	}

	#[test]
	fn skipped_barrel_ticks_slow_held_auto() {
		let interval = 0.1;
		let dt = 0.016;
		let mut full = 0u32;
		let mut skipped = 0u32;
		let mut cooldown_full = 0.0;
		let mut cooldown_skip = 0.0;
		for i in 0..125 {
			full += u32::from(
				advance_shot_clock(&mut cooldown_full, interval, dt, CATCH_UP_SHOTS).shots,
			);
			if i % 4 == 0 {
				continue;
			}
			skipped += u32::from(
				advance_shot_clock(&mut cooldown_skip, interval, dt, CATCH_UP_SHOTS).shots,
			);
		}
		assert!(skipped < full, "skip {skipped} vs full {full}");
	}
}
