use crate::generation::{GroupKind, MobGroup, MobPlantHost, MobWorldHosts, MobWorldSample};
use crate::index::{urban_leaf_arrival_radius, MobCellExtent};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use mob_scenes::MobKind;

#[test]
fn mob_cells_cover_a_four_hundred_metre_lattice() {
	let region = Aabb3d::from_min_max(Vec3::new(-199.0, 0.0, -199.0), Vec3::new(201.0, 1.0, 201.0));
	assert_eq!(MobCellExtent::cells_overlapping(region).len(), 4);
}

#[test]
fn bounds_extent_keeps_the_given_rectangle() -> anyhow::Result<()> {
	let min = Vec3::new(-36.0, 0.0, -28.0);
	let max = Vec3::new(36.0, 1.0, 28.0);
	let extent = MobCellExtent::from_bounds(min, max);
	anyhow::ensure!(extent.aabb() == Aabb3d::from_min_max(min, max));
	anyhow::ensure!(MobCellExtent::from_id(extent.id()).is_none(), "grid from_id stays 400 m");
	Ok(())
}

struct FrontierWorld {
	hosts: Vec<MobPlantHost>,
}

impl MobWorldSample for FrontierWorld {
	fn sample_mobs(&self, _xz: Vec2) -> crate::generation::MobEnvironmentSample {
		crate::generation::MobEnvironmentSample {
			elevation: Some(0.0),
			urbanization: 0.4,
			vegetation: 0.0,
		}
	}
}

impl MobWorldHosts for FrontierWorld {
	fn plant_hosts(&self, _origin: Vec2, _extent: f32) -> Vec<MobPlantHost> {
		self.hosts.clone()
	}
}

#[test]
fn frontier_hosts_keep_urban_families_inside_the_arrival_disk() {
	let world = FrontierWorld {
		hosts: vec![MobPlantHost { xz: Vec2::new(20.0, -8.0), arrival_radius: 6.0 }],
	};
	let group = MobGroup::generate(GroupKind::Frontier, 11, Vec2::ZERO, &world);
	let planted: Vec<_> = group
		.mobs
		.iter()
		.filter(|mob| {
			matches!(mob.scene.mob.kind, MobKind::Guard | MobKind::Brawler | MobKind::Pleb)
		})
		.collect();
	assert!(!planted.is_empty());
	for mob in planted {
		let xz = Vec2::new(mob.transform.translation.x, mob.transform.translation.z);
		assert!(xz.distance(Vec2::new(20.0, -8.0)) <= 6.0 + 1e-4);
		assert_eq!(mob.transform.translation.y, 0.0);
	}
}

#[test]
fn urban_leaf_arrival_matches_setting_formula() {
	let bounds = Aabb3d::from_min_max(Vec3::new(-40.0, 0.0, -20.0), Vec3::new(40.0, 1.0, 20.0));
	assert_eq!(urban_leaf_arrival_radius(bounds), 10.0);
}
