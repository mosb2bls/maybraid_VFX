//! Production-path A/B for cull / produce fill against main's query-and-insert.

use std::hint::black_box;
use std::time::{Duration, Instant};

use anyhow::Result;
use bevy::ecs::system::{RunSystemOnce, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use lod::{
	fill_lod_cull_produce_cache, fill_lod_produce_cache, LodCullProduceCache, LodHostBounds,
	LodNode, LodProduceCache, LodRefreshDomain, LodRefreshMembership, LodSceneCullAabb,
	LodSceneHost, LodSceneHostIndex, LodSceneRefreshAabb,
};

use super::{GimmeLodHostIndex, GimmeLodHostPlugin, GimmeLodSceneHostIndex};

/// Main's Gimme adapter: `hosts_in_region` is the sorted pair walk; fill uses the
/// default `out.extend(hosts_in_region(region))`.
#[derive(SystemParam)]
struct OriginalGimmeLodSceneHostIndex<'w> {
	index: Res<'w, GimmeLodHostIndex>,
}

impl LodSceneHostIndex for OriginalGimmeLodSceneHostIndex<'_> {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a {
		self.index.0.query_ids_from_sorted_pairs(region).into_iter()
	}
}

struct HostGrid {
	hosts: usize,
	spacing: f32,
}

impl HostGrid {
	fn side(&self) -> usize {
		(self.hosts as f64).sqrt().ceil().max(1.0) as usize
	}

	fn extent(&self) -> f32 {
		self.side() as f32 * self.spacing
	}

	fn host_translation(&self, i: usize) -> Vec3 {
		let side = self.side();
		Vec3::new((i % side) as f32 * self.spacing, 0.0, (i / side) as f32 * self.spacing)
	}

	fn host_bounds() -> LodHostBounds {
		LodHostBounds(Aabb3d::from_min_max(Vec3::new(-8.0, -2.0, -8.0), Vec3::new(8.0, 8.0, 8.0)))
	}

	fn overlapping_regions(&self) -> [Aabb3d; 4] {
		let extent = self.extent();
		let mid = extent * 0.55;
		[
			Aabb3d::from_min_max(Vec3::new(0.0, -16.0, 0.0), Vec3::new(mid, 32.0, mid)),
			Aabb3d::from_min_max(Vec3::new(extent - mid, -16.0, 0.0), Vec3::new(extent, 32.0, mid)),
			Aabb3d::from_min_max(Vec3::new(0.0, -16.0, extent - mid), Vec3::new(mid, 32.0, extent)),
			Aabb3d::from_min_max(
				Vec3::new(extent - mid, -16.0, extent - mid),
				Vec3::new(extent, 32.0, extent),
			),
		]
	}
}

struct FillScene {
	app: App,
	regions: [Aabb3d; 4],
	domain: LodRefreshDomain,
}

impl FillScene {
	fn spawn(grid: &HostGrid) -> Self {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, GimmeLodHostPlugin))
			.init_resource::<LodCullProduceCache>()
			.init_resource::<LodProduceCache>()
			.add_message::<LodSceneCullAabb>()
			.add_message::<LodSceneRefreshAabb>();
		app.world_mut().spawn((LodNode, Transform::IDENTITY));
		let domain = LodRefreshDomain::of::<ProduceDomain>();
		for i in 0..grid.hosts {
			app.world_mut().spawn((
				LodSceneHost,
				Transform::from_translation(grid.host_translation(i)),
				HostGrid::host_bounds(),
				LodRefreshMembership(domain),
			));
		}
		app.update();
		Self { app, regions: grid.overlapping_regions(), domain }
	}

	fn write_cull_regions(&mut self) {
		for region in self.regions {
			self.app.world_mut().write_message(LodSceneCullAabb { region });
		}
	}

	fn write_produce_regions(&mut self) {
		for region in self.regions {
			self.app
				.world_mut()
				.write_message(LodSceneRefreshAabb { region, domain: self.domain });
		}
	}

	fn run_cull<I>(&mut self) -> Result<()>
	where
		I: SystemParam + 'static,
		for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
	{
		self.app
			.world_mut()
			.run_system_once(fill_lod_cull_produce_cache::<I>)
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		Ok(())
	}

	fn run_produce<I>(&mut self) -> Result<()>
	where
		I: SystemParam + 'static,
		for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
	{
		self.app
			.world_mut()
			.run_system_once(fill_lod_produce_cache::<I>)
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		Ok(())
	}

	fn cull_hits(&self) -> usize {
		self.app.world().resource::<LodCullProduceCache>().hit_entities.len()
	}
}

#[derive(Clone, Copy)]
struct ProduceDomain;

fn timed<F: FnMut() -> Result<()>>(repeats: usize, mut run: F) -> Result<Vec<Duration>> {
	run()?;
	let mut samples = Vec::with_capacity(repeats);
	for _ in 0..repeats {
		let started = Instant::now();
		run()?;
		samples.push(started.elapsed());
	}
	Ok(samples)
}

fn summarize(samples: &[Duration]) -> Result<(Duration, Duration, Duration, Duration)> {
	if samples.is_empty() {
		return Err(anyhow::anyhow!("microbench produced no samples"));
	}
	let mut sorted = samples.to_vec();
	sorted.sort();
	Ok((
		sorted[0],
		sorted[sorted.len() / 2],
		samples.iter().copied().sum::<Duration>() / u32::try_from(samples.len())?,
		sorted[sorted.len() - 1],
	))
}

fn report(
	label: &str,
	grid: &HostGrid,
	hits: usize,
	original: &[Duration],
	revised: &[Duration],
	dest_cap: usize,
) -> Result<()> {
	let (o_min, o_med, o_mean, o_max) = summarize(original)?;
	let (r_min, r_med, r_mean, r_max) = summarize(revised)?;
	eprintln!(
		"{label}: 80 passes × {} overlapping regions × {} hosts, {hits} unique hits; original query-and-insert fill min/med/mean/max {:?} / {:?} / {:?} / {:?}; GimmeLodSceneHostIndex fill {:?} / {:?} / {:?} / {:?}; dest cap {dest_cap}",
		grid.overlapping_regions().len(),
		grid.hosts,
		o_min,
		o_med,
		o_mean,
		o_max,
		r_min,
		r_med,
		r_mean,
		r_max,
	);
	Ok(())
}

#[test]
#[ignore]
fn lod_fill_systems_microbench() -> Result<()> {
	const REPEATS: usize = 7;
	const PASSES: usize = 80;
	let scenarios =
		[HostGrid { hosts: 64, spacing: 96.0 }, HostGrid { hosts: 4_096, spacing: 16.0 }];

	for grid in scenarios {
		let mut original = FillScene::spawn(&grid);
		let mut revised = FillScene::spawn(&grid);

		original.write_cull_regions();
		original.run_cull::<OriginalGimmeLodSceneHostIndex>()?;
		revised.write_cull_regions();
		revised.run_cull::<GimmeLodSceneHostIndex>()?;
		let hits = revised.cull_hits();
		assert_eq!(original.cull_hits(), hits);
		assert!(hits > 0);

		let original_cull = timed(REPEATS, || {
			for _ in 0..PASSES {
				original.write_cull_regions();
				original.run_cull::<OriginalGimmeLodSceneHostIndex>()?;
				black_box(original.cull_hits());
			}
			Ok(())
		})?;
		let revised_cull = timed(REPEATS, || {
			for _ in 0..PASSES {
				revised.write_cull_regions();
				revised.run_cull::<GimmeLodSceneHostIndex>()?;
				black_box(revised.cull_hits());
			}
			Ok(())
		})?;
		report(
			&format!("lod_fill_cull[{}-hosts]", grid.hosts),
			&grid,
			hits,
			&original_cull,
			&revised_cull,
			revised.app.world().resource::<LodCullProduceCache>().hit_entities.capacity(),
		)?;

		let original_produce = timed(REPEATS, || {
			for _ in 0..PASSES {
				original.write_produce_regions();
				original.run_produce::<OriginalGimmeLodSceneHostIndex>()?;
				black_box(original.app.world().resource::<LodProduceCache>().snapshots.len());
			}
			Ok(())
		})?;
		let revised_produce = timed(REPEATS, || {
			for _ in 0..PASSES {
				revised.write_produce_regions();
				revised.run_produce::<GimmeLodSceneHostIndex>()?;
				black_box(revised.app.world().resource::<LodProduceCache>().snapshots.len());
			}
			Ok(())
		})?;
		report(
			&format!("lod_fill_produce[{}-hosts]", grid.hosts),
			&grid,
			hits,
			&original_produce,
			&revised_produce,
			0,
		)?;
	}
	Ok(())
}
