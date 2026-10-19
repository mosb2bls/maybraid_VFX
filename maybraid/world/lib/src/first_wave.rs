//! Shared first-load progress for unveil and the spawn picker.

use bevy::prelude::*;
use lod::hcsg::{HcsgClass, HcsgDemand, Outstanding};
use lod::LodJobCounter;

/// Remaining Near HCSG ids plus pending-root tickets that still count as
/// "the first wave is finishing." Far, background, and ambient never hold
/// the gate. Streaming continues after unveil.
pub const FIRST_WAVE_JOB_THRESHOLD: u64 = 16;

/// Near outstanding work plus pending-root tickets, and whether first-load
/// unveil has already passed (including timeout).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FirstWave {
	pub undiscovered: u64,
	pub remaining: u64,
	pub passed: bool,
}

impl FirstWave {
	pub fn sample(demand: Option<&HcsgDemand>, jobs: u64) -> Self {
		let near = match demand {
			None => Outstanding::default(),
			Some(demand) => demand.try_outstanding(&[HcsgClass::Near]).unwrap_or_else(|_| {
				// Demand lock busy: treat as one undiscovered subscription so unveil
				// stays conservative until the next frame can read queue state.
				Outstanding { undiscovered: 1, remaining: 0 }
			}),
		};
		Self {
			undiscovered: near.undiscovered,
			remaining: jobs.saturating_add(near.remaining),
			passed: false,
		}
	}

	pub fn quiet(&self) -> bool {
		self.undiscovered == 0 && self.remaining <= FIRST_WAVE_JOB_THRESHOLD
	}

	pub fn ready(&self) -> bool {
		self.passed || self.quiet()
	}
}

pub(crate) fn refresh_first_wave(
	demand: Option<Res<HcsgDemand>>,
	jobs: Option<Res<LodJobCounter>>,
	mut wave: ResMut<FirstWave>,
) {
	let passed = wave.passed;
	*wave = FirstWave::sample(demand.as_deref(), jobs.map(|jobs| jobs.active()).unwrap_or(0));
	wave.passed = passed;
}

#[cfg(any(test, feature = "test-support"))]
pub use unit_x_tile::UnitXTile;

#[cfg(any(test, feature = "test-support"))]
mod unit_x_tile {
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use lod::gen::{Id, OriginalId};
	use lod::hcsg::{GenerationContext, GenerationScheme};

	/// One original id per unit of region x. Shared by first-load tests.
	pub struct UnitXTile;

	impl GenerationScheme for UnitXTile {
		fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
			let start = region.min.x.floor() as i32;
			let end = region.max.x.ceil() as i32;
			(start..end)
				.map(|x| {
					let bounds = Aabb3d::from_min_max(
						Vec3::new(x as f32, 0.0, 0.0),
						Vec3::new(x as f32 + 1.0, 1.0, 1.0),
					);
					OriginalId::new(Id::from_cell(bounds))
				})
				.collect()
		}

		fn build_with_id(_: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
			let bounds = id.origin_cell_bounds()?;
			Some((Self, bounds))
		}
	}
}
