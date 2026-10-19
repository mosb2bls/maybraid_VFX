//! Production-path A/B for cull / produce fill adapters.
//!
//! Both sides run the **current** `fill_lod_cull_produce_cache` /
//! `fill_lod_produce_cache` systems. Only the [`LodSceneHostIndex`] adapter
//! differs (historical pair-walk + default `out.extend` vs
//! `extend_intersecting`). This does **not** measure the whole PR against its
//! base.

use std::hint::black_box;
use std::marker::PhantomData;

use anyhow::Result;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use gimme_bench_support::{
	AbReport, HostGrid, MessageBuffers, Microbench, PairWalkFill, PassLifetime, PersistentSystem,
};
use lod::{
	fill_lod_cull_produce_cache, fill_lod_produce_cache, LodCullProduceCache, LodHostBounds,
	LodNode, LodProduceCache, LodRefreshDomain, LodRefreshMembership, LodSceneCullAabb,
	LodSceneHost, LodSceneHostIndex, LodSceneRefreshAabb,
};

use super::{GimmeLodHostIndex, GimmeLodHostPlugin, GimmeLodSceneHostIndex};

const INPUT_MESSAGES: usize = 4;
const FILL_PASSES: u32 = 80;

/// Historical Gimme adapter: pair-walk `hosts_in_region`; fill uses the default
/// `out.extend(hosts_in_region(region))`. Same **new** fill systems as
/// [`GimmeLodSceneHostIndex`].
#[derive(SystemParam)]
struct OriginalGimmeLodSceneHostIndex<'w> {
	index: Res<'w, GimmeLodHostIndex>,
}

impl LodSceneHostIndex for OriginalGimmeLodSceneHostIndex<'_> {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a {
		PairWalkFill::new(&self.index.0).query_ids(region).into_iter()
	}
}

fn counted_cull_fill<I>(
	regions: MessageReader<LodSceneCullAabb>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &lod::LodNodePose, Option<&lod::LodNodeBounds>), With<LodNode>>,
	cache: ResMut<LodCullProduceCache>,
) -> usize
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	let consumed = regions.len();
	fill_lod_cull_produce_cache::<I>(regions, index, nodes, cache);
	consumed
}

fn counted_produce_fill<I>(
	regions: MessageReader<LodSceneRefreshAabb>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &lod::LodNodePose, Option<&lod::LodNodeBounds>), With<LodNode>>,
	membership: Query<&LodRefreshMembership>,
	cache: ResMut<LodProduceCache>,
) -> usize
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	let consumed = regions.len();
	fill_lod_produce_cache::<I>(regions, index, nodes, membership, cache);
	consumed
}

/// One world + persistent fill systems. Destination lifetimes are those of the
/// production fills (`PassLifetime`), not of this helper.
struct FillScene<I>
where
	I: SystemParam + 'static,
{
	app: App,
	regions: [Aabb3d; 4],
	domain: LodRefreshDomain,
	expected_hits: HashSet<Entity>,
	cull: PersistentSystem<usize>,
	produce: PersistentSystem<usize>,
	_index: PhantomData<I>,
}

impl<I> FillScene<I>
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn spawn(grid: &HostGrid) -> Result<Self> {
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
				LodHostBounds(HostGrid::local_host_bounds()),
				LodRefreshMembership(domain),
			));
		}
		app.update();

		let expected_hits = {
			let index = app.world().resource::<GimmeLodHostIndex>();
			let mut hits = HashSet::new();
			for region in grid.overlapping_regions() {
				hits.extend(index.hosts_in_region(region));
			}
			hits
		};
		assert!(!expected_hits.is_empty());

		let cull = PersistentSystem::initialize(app.world_mut(), counted_cull_fill::<I>);
		let produce = PersistentSystem::initialize(app.world_mut(), counted_produce_fill::<I>);
		Ok(Self {
			app,
			regions: grid.overlapping_regions(),
			domain,
			expected_hits,
			cull,
			produce,
			_index: PhantomData,
		})
	}

	fn prepare_cull(&mut self) {
		for region in self.regions {
			self.app.world_mut().write_message(LodSceneCullAabb { region });
		}
	}

	fn execute_cull(&mut self) -> Result<usize> {
		self.cull.run(self.app.world_mut())
	}

	fn maintain_cull(&mut self) {
		MessageBuffers::age::<LodSceneCullAabb>(self.app.world_mut());
	}

	fn validate_cull(&self) -> Result<()> {
		let hits = &self.app.world().resource::<LodCullProduceCache>().hit_entities;
		if hits != &self.expected_hits {
			return Err(anyhow::anyhow!(
				"cull hit membership {} != expected {}",
				hits.len(),
				self.expected_hits.len()
			));
		}
		Ok(())
	}

	fn prepare_produce(&mut self) {
		for region in self.regions {
			self.app
				.world_mut()
				.write_message(LodSceneRefreshAabb { region, domain: self.domain });
		}
	}

	fn execute_produce(&mut self) -> Result<usize> {
		self.produce.run(self.app.world_mut())
	}

	fn maintain_produce(&mut self) {
		MessageBuffers::age::<LodSceneRefreshAabb>(self.app.world_mut());
	}

	fn validate_produce(&self) -> Result<()> {
		let cache = self.app.world().resource::<LodProduceCache>();
		let Some(hits) = cache.hits_for(self.domain) else {
			return Err(anyhow::anyhow!("produce cache missing domain hits"));
		};
		if hits != &self.expected_hits {
			return Err(anyhow::anyhow!(
				"produce hit membership {} != expected {}",
				hits.len(),
				self.expected_hits.len()
			));
		}
		Ok(())
	}

	/// One timed sample: `passes` simulated frames. Validation stays outside.
	fn run_cull_window(&mut self, passes: u32) -> Result<()> {
		for _ in 0..passes {
			self.prepare_cull();
			let consumed = self.execute_cull()?;
			self.maintain_cull();
			if consumed != INPUT_MESSAGES {
				return Err(anyhow::anyhow!(
					"cull fill consumed {consumed} messages, expected {INPUT_MESSAGES}"
				));
			}
			black_box(self.app.world().resource::<LodCullProduceCache>().hit_entities.len());
		}
		Ok(())
	}

	fn run_produce_window(&mut self, passes: u32) -> Result<()> {
		for _ in 0..passes {
			self.prepare_produce();
			let consumed = self.execute_produce()?;
			self.maintain_produce();
			if consumed != INPUT_MESSAGES {
				return Err(anyhow::anyhow!(
					"produce fill consumed {consumed} messages, expected {INPUT_MESSAGES}"
				));
			}
			black_box(
				self.app
					.world()
					.resource::<LodProduceCache>()
					.hits_for(self.domain)
					.map(|hits| hits.len()),
			);
		}
		Ok(())
	}

	fn dest_cap(&self) -> usize {
		self.app.world().resource::<LodCullProduceCache>().hit_entities.capacity()
	}
}

#[derive(Clone, Copy)]
struct ProduceDomain;

fn report_fill(
	label: &str,
	grid: &HostGrid,
	hits: usize,
	lifetime: PassLifetime,
	original: &[std::time::Duration],
	revised: &[std::time::Duration],
	dest_cap: usize,
) -> Result<()> {
	AbReport {
		label,
		original_name: "pair-walk adapter, new fill systems",
		revised_name: "extend_intersecting adapter, new fill systems",
		original,
		revised,
		operations: FILL_PASSES,
		operation_name: "fill",
		lifetime,
		notes: &format!(
			"{FILL_PASSES} fills × {} overlapping regions × {} hosts, {hits} unique hits; dest cap {dest_cap}; adapter-only (not whole-PR vs base)",
			grid.overlapping_regions().len(),
			grid.hosts,
		),
	}
	.write_to_stderr()
}

#[test]
fn fill_pass_consumes_exactly_four_messages() -> Result<()> {
	let grid = HostGrid { hosts: 16, spacing: 96.0 };
	let mut original = FillScene::<OriginalGimmeLodSceneHostIndex>::spawn(&grid)?;
	let mut revised = FillScene::<GimmeLodSceneHostIndex>::spawn(&grid)?;
	exercise_two_fills(&mut original)?;
	exercise_two_fills(&mut revised)?;
	assert_eq!(original.expected_hits, revised.expected_hits);
	Ok(())
}

fn exercise_two_fills<I>(scene: &mut FillScene<I>) -> Result<()>
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	scene.prepare_cull();
	assert_eq!(scene.execute_cull()?, INPUT_MESSAGES);
	scene.maintain_cull();
	scene.validate_cull()?;

	scene.prepare_cull();
	assert_eq!(scene.execute_cull()?, INPUT_MESSAGES);
	scene.maintain_cull();
	scene.validate_cull()?;

	scene.prepare_produce();
	assert_eq!(scene.execute_produce()?, INPUT_MESSAGES);
	scene.maintain_produce();
	scene.validate_produce()?;

	scene.prepare_produce();
	assert_eq!(scene.execute_produce()?, INPUT_MESSAGES);
	scene.maintain_produce();
	scene.validate_produce()?;
	Ok(())
}

#[test]
#[ignore]
fn lod_fill_systems_microbench() -> Result<()> {
	let bench = Microbench::default();

	for grid in HostGrid::lattice_scenarios() {
		let mut original = FillScene::<OriginalGimmeLodSceneHostIndex>::spawn(&grid)?;
		let mut revised = FillScene::<GimmeLodSceneHostIndex>::spawn(&grid)?;
		assert_eq!(original.expected_hits.len(), revised.expected_hits.len());
		let hits = revised.expected_hits.len();

		original.run_cull_window(1)?;
		original.validate_cull()?;
		revised.run_cull_window(1)?;
		revised.validate_cull()?;

		let (original_cull, revised_cull) = bench.collect_ab_alternating(
			|| original.run_cull_window(FILL_PASSES),
			|| revised.run_cull_window(FILL_PASSES),
		)?;
		original.validate_cull()?;
		revised.validate_cull()?;
		report_fill(
			&format!("lod_fill_cull[{}-hosts]", grid.hosts),
			&grid,
			hits,
			PassLifetime::ClearedPerPass,
			&original_cull,
			&revised_cull,
			revised.dest_cap(),
		)?;

		original.run_produce_window(1)?;
		original.validate_produce()?;
		revised.run_produce_window(1)?;
		revised.validate_produce()?;

		let (original_produce, revised_produce) = bench.collect_ab_alternating(
			|| original.run_produce_window(FILL_PASSES),
			|| revised.run_produce_window(FILL_PASSES),
		)?;
		original.validate_produce()?;
		revised.validate_produce()?;
		report_fill(
			&format!("lod_fill_produce[{}-hosts]", grid.hosts),
			&grid,
			hits,
			PassLifetime::FreshPerPass,
			&original_produce,
			&revised_produce,
			0,
		)?;
	}
	Ok(())
}
