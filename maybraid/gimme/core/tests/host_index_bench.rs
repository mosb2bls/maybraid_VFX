//! Index A/B as an integration test so it shares the `gimme-core` library
//! artifact with `gimme-bench-support` (unit tests would be a second crate).

use std::hint::black_box;

use anyhow::Result;
use bevy_math::DVec3;
use bevy_platform::collections::HashSet;
use gimme_bench_support::{
	AbReport, ExtendIntersectingFill, HostGrid, Microbench, PairWalkFill, PassLifetime,
};
use gimme_core::SpatialIndex;

const PASSES: u32 = 200;

fn sorted(out: &HashSet<u32>) -> Vec<u32> {
	let mut got: Vec<_> = out.iter().copied().collect();
	got.sort();
	got
}

fn query_union(idx: &SpatialIndex<u32>, regions: &[bevy_math::bounding::Aabb3d]) -> Vec<u32> {
	let mut expected = HashSet::new();
	for region in regions {
		expected.extend(idx.query(*region));
	}
	sorted(&expected)
}

#[test]
#[ignore]
fn host_index_accumulation_microbench() -> Result<()> {
	let bench = Microbench::default();

	for grid in HostGrid::lattice_scenarios() {
		let mut idx = SpatialIndex::new(DVec3::new(64.0, 4_096.0, 64.0))?;
		for i in 0..grid.hosts {
			idx.insert(i as u32, grid.host_aabb(i))?;
		}
		let regions = grid.overlapping_regions();
		let expected = query_union(&idx, &regions);
		let per_region: Vec<usize> =
			regions.iter().map(|region| idx.query(*region).len()).collect();
		assert!(!expected.is_empty());

		let mut original = PairWalkFill::new(&idx);
		let mut revised = ExtendIntersectingFill::new(&idx);

		// Setup is the index above. Frame prep is the static region list.
		// Cull: destination cleared each pass, capacity retained.
		let mut original_cull_out = HashSet::new();
		let mut revised_cull_out = HashSet::new();
		let (original_cull, revised_cull) = bench.collect_ab_alternating(
			|| {
				for _ in 0..PASSES {
					original_cull_out.clear();
					for region in regions {
						original.extend_hosts_in_region(region, &mut original_cull_out);
					}
					black_box(&original_cull_out);
				}
				Ok(())
			},
			|| {
				for _ in 0..PASSES {
					revised_cull_out.clear();
					for region in regions {
						revised.extend_hosts_in_region(region, &mut revised_cull_out);
					}
					black_box(&revised_cull_out);
				}
				Ok(())
			},
		)?;
		assert_eq!(sorted(&original_cull_out), expected);
		assert_eq!(sorted(&revised_cull_out), expected);
		AbReport {
			label: &format!("host_index_cull[{}-hosts]", grid.hosts),
			original_name: "pair-walk query-and-insert",
			revised_name: "extend_intersecting",
			original: &original_cull,
			revised: &revised_cull,
			operations: PASSES,
			operation_name: "pass",
			lifetime: PassLifetime::ClearedPerPass,
			notes: &format!(
				"{PASSES} passes × {} overlapping regions × {} hosts, {} unique hits {:?}/region, {} cells; dest cap {}",
				regions.len(),
				grid.hosts,
				expected.len(),
				per_region,
				idx.cell_count(),
				revised_cull_out.capacity(),
			),
		}
		.write_to_stderr()?;

		// Produce: a fresh destination HashSet per pass (per domain).
		let (original_produce, revised_produce) = bench.collect_ab_alternating(
			|| {
				for _ in 0..PASSES {
					let mut out = HashSet::new();
					for region in regions {
						original.extend_hosts_in_region(region, &mut out);
					}
					black_box(&out);
				}
				Ok(())
			},
			|| {
				for _ in 0..PASSES {
					let mut out = HashSet::new();
					for region in regions {
						revised.extend_hosts_in_region(region, &mut out);
					}
					black_box(&out);
				}
				Ok(())
			},
		)?;
		let mut original_produce_probe = HashSet::new();
		let mut revised_produce_probe = HashSet::new();
		for region in regions {
			original.extend_hosts_in_region(region, &mut original_produce_probe);
			revised.extend_hosts_in_region(region, &mut revised_produce_probe);
		}
		assert_eq!(sorted(&original_produce_probe), expected);
		assert_eq!(sorted(&revised_produce_probe), expected);
		AbReport {
			label: &format!("host_index_produce[{}-hosts]", grid.hosts),
			original_name: "pair-walk query-and-insert",
			revised_name: "extend_intersecting",
			original: &original_produce,
			revised: &revised_produce,
			operations: PASSES,
			operation_name: "pass",
			lifetime: PassLifetime::FreshPerPass,
			notes: &format!(
				"{PASSES} passes × {} overlapping regions × {} hosts, {} unique hits {:?}/region, {} cells; dest cap {}",
				regions.len(),
				grid.hosts,
				expected.len(),
				per_region,
				idx.cell_count(),
				revised_produce_probe.capacity(),
			),
		}
		.write_to_stderr()?;
	}
	Ok(())
}
