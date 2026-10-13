//! Kitchen plan packer: counter layouts (galley / L / peninsula) + optional island.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};
use crate::placer::{
	init_host_with, try_corner_l, try_free_extent, try_peninsula_from_run, try_wall_long, xz_area,
	FreeExtentKnobs, InitHostOpts, PackHost, WallLongKnobs, WALL_EPS,
};

pub const MIN_AREA: f32 = 2.4 * 2.0;

const COUNTER_DEPTH: f32 = 0.6;
const COUNTER_HEIGHT: f32 = 0.9;

/// Counter program subtype — drives which solid runs are authored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KitchenCounterLayout {
	/// Single wall run.
	Galley,
	/// Two wall runs that meet at a host corner.
	LShape,
	/// Wall run + peninsula stub into the room from one end.
	Peninsula,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KitchenPacked {
	pub counter_runs: Vec<Aabb3d>,
	pub peninsulas: Vec<Aabb3d>,
	pub islands: Vec<Aabb3d>,
	pub fillers: Vec<Aabb3d>,
	pub layout: Option<KitchenCounterLayout>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KitchenRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
	pub layout: Option<KitchenCounterLayout>,
}

impl KitchenRegions {
	pub fn pack(&self, confines: &Confines, noise: NoiseParams) -> Result<KitchenPacked, FitError> {
		let mut host = init_host_with(confines, InitHostOpts { passage_wall_lip: true })?;
		if host.room_area + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall { reason: "kitchen" });
		}
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let mut layout = self.layout.unwrap_or_else(|| pick_layout(&cfg, c, host.room_area));

		let depth = (COUNTER_DEPTH * self.spaciousness).clamp(0.5, 0.8);
		let height = COUNTER_HEIGHT * self.spaciousness.min(1.15);
		let along_a = sample_along(&cfg, c, self.spaciousness, host.host, 32.0);
		let along_b = sample_along(&cfg, c, self.spaciousness, host.host, 33.0);

		let mut packed = KitchenPacked { layout: Some(layout), ..KitchenPacked::default() };

		match layout {
			KitchenCounterLayout::Galley => {
				let primary = place_wall_run(&host, &cfg, along_a, depth, height, 10)
					.ok_or(FitError::TooSmall { reason: "kitchen counter" })?;
				host.commit_footprint(&primary);
				packed.counter_runs.push(primary);
			}
			KitchenCounterLayout::LShape => {
				if let Some((a, b)) = try_corner_l(
					&host.host3,
					host.host,
					&host.clearances,
					&cfg,
					along_a,
					along_b,
					depth,
					height,
				) {
					host.commit_footprint(&a);
					host.commit_footprint(&b);
					packed.counter_runs.push(a);
					packed.counter_runs.push(b);
				} else {
					// Soft-fall to galley when corner L is blocked (e.g. door clearance).
					let primary = place_wall_run(&host, &cfg, along_a, depth, height, 11)
						.ok_or(FitError::TooSmall { reason: "kitchen counter" })?;
					host.commit_footprint(&primary);
					packed.counter_runs.push(primary);
					layout = KitchenCounterLayout::Galley;
					packed.layout = Some(layout);
				}
			}
			KitchenCounterLayout::Peninsula => {
				let primary = place_wall_run(&host, &cfg, along_a, depth, height, 12)
					.ok_or(FitError::TooSmall { reason: "kitchen counter" })?;
				host.commit_footprint(&primary);
				packed.counter_runs.push(primary);
				let pen_along = sample_along(&cfg, c, self.spaciousness * 0.85, host.host, 34.0)
					.clamp(0.9, 2.6);
				if let Some(pen) = try_peninsula_from_run(
					&host.host3,
					host.host,
					&host.clearances,
					&packed.counter_runs[0],
					pen_along,
					depth,
					height,
					&cfg,
					35,
				) {
					host.commit_footprint(&pen);
					packed.peninsulas.push(pen);
				}
			}
		}

		finalize_optional(self, &mut host, &mut packed, &cfg, c)
	}
}

fn place_wall_run(
	host: &PackHost,
	cfg: &NoiseConfig,
	along: f32,
	depth: f32,
	height: f32,
	salt: u32,
) -> Option<Aabb3d> {
	try_wall_long(
		&host.host3,
		host.host,
		&host.clearances,
		cfg,
		salt,
		WallLongKnobs { extent: Vec3::new(along, height, depth), wall_eps: WALL_EPS, attempts: 20 },
	)
}

fn finalize_optional(
	regions: &KitchenRegions,
	host: &mut PackHost,
	packed: &mut KitchenPacked,
	cfg: &NoiseConfig,
	c: Vec3,
) -> Result<KitchenPacked, FitError> {
	let layout = packed.layout.unwrap_or(KitchenCounterLayout::Galley);
	let height = COUNTER_HEIGHT * regions.spaciousness.min(1.15);

	let island_ok = host.room_area >= 18.0
		&& matches!(layout, KitchenCounterLayout::Galley | KitchenCounterLayout::LShape)
		&& cfg.sample_unit_4d(c.x, c.y, c.z, 36.0) > 0.45;
	if island_ok {
		let ix =
			cfg.sample_range_f32_4d(0.9, 1.5, c.x, c.y, c.z, 37.0) * regions.spaciousness.min(1.2);
		let iz =
			cfg.sample_range_f32_4d(0.7, 1.1, c.x, c.y, c.z, 38.0) * regions.spaciousness.min(1.2);
		if let Some(island) = try_free_extent(
			&host.host3,
			host.host,
			&host.clearances,
			cfg,
			40,
			FreeExtentKnobs {
				extent: Vec3::new(ix, height, iz),
				prefer_wall: false,
				wall_eps: WALL_EPS,
				attempts: 16,
			},
		) {
			if packed_area_ratio(packed, host.room_area) + xz_area(&island) / host.room_area
				<= regions.occupancy + 1e-3
			{
				host.commit_footprint(&island);
				packed.islands.push(island);
			}
		}
	}

	if packed_area_ratio(packed, host.room_area) < regions.occupancy * 0.85 {
		// Fillers prefer walls and often land beside a door just outside the
		// strict passage band — require padded approach clearances too.
		let excludes = host.clearances_with_approach();
		if let Some(f) = try_free_extent(
			&host.host3,
			host.host,
			&excludes,
			cfg,
			50,
			FreeExtentKnobs {
				extent: Vec3::new(0.4 * regions.spaciousness, 0.5, 0.4 * regions.spaciousness),
				prefer_wall: true,
				wall_eps: WALL_EPS,
				attempts: 10,
			},
		) {
			if packed_area_ratio(packed, host.room_area) + xz_area(&f) / host.room_area
				<= regions.occupancy + 1e-3
			{
				host.commit_footprint(&f);
				packed.fillers.push(f);
			}
		}
	}

	if packed.counter_runs.is_empty() {
		return Err(FitError::TooSmall { reason: "kitchen counter" });
	}
	Ok(packed.clone())
}

fn sample_along(cfg: &NoiseConfig, c: Vec3, spaciousness: f32, host: Aabb2d, w: f32) -> f32 {
	let max_span = (host.max.x - host.min.x).max(host.max.y - host.min.y).max(1.5);
	cfg.sample_range_f32_4d(
		1.5 * spaciousness,
		(3.2 * spaciousness).min(max_span - 0.15),
		c.x,
		c.y,
		c.z,
		w,
	)
	.clamp(1.35, 4.5)
}

fn pick_layout(cfg: &NoiseConfig, c: Vec3, room_area: f32) -> KitchenCounterLayout {
	let t = cfg.sample_unit_4d(c.x, c.y, c.z, 31.0);
	if room_area < 12.0 {
		return if t < 0.55 { KitchenCounterLayout::Galley } else { KitchenCounterLayout::LShape };
	}
	if t < 0.28 {
		KitchenCounterLayout::Galley
	} else if t < 0.62 {
		KitchenCounterLayout::LShape
	} else {
		KitchenCounterLayout::Peninsula
	}
}

fn packed_area_ratio(packed: &KitchenPacked, room_area: f32) -> f32 {
	let a = packed
		.counter_runs
		.iter()
		.chain(packed.peninsulas.iter())
		.chain(packed.islands.iter())
		.chain(packed.fillers.iter())
		.map(xz_area)
		.sum::<f32>();
	a / room_area.max(1e-4)
}
