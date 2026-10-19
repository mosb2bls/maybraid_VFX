//! Fulfill [`SceneRefRoot`] / [`MultiSceneMergeRoot`] → [`WorldAssetRoot`].

use bevy::asset::AssetServer;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::prelude::{
	Assets, Commands, Entity, Local, Query, Res, ResMut, StandardMaterial, Without,
};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use crate::handles::SceneRefHandles;
use crate::multi_merge::{MultiSceneMergeHandles, MultiSceneMergeRoot};
use crate::scene_ref::SceneRefRoot;
use crate::SceneRefAdmitBudget;

pub(crate) fn fulfill_scene_ref_roots(
	mut commands: Commands,
	query: Query<(Entity, &SceneRefRoot), Without<WorldAssetRoot>>,
	mut handles: ResMut<SceneRefHandles>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	type_registry: Res<AppTypeRegistry>,
	budget: Res<SceneRefAdmitBudget>,
	mut cursor: Local<u32>,
) {
	let mut remaining = budget.per_frame;
	if remaining == 0 || budget.inspect_per_frame == 0 {
		return;
	}
	let mut scan = FulfillInspectScan::new(*cursor, budget.inspect_per_frame);
	let mut admit = |(entity, root): (Entity, &SceneRefRoot)| {
		if let Some(handle) = handles.try_resolve(
			&root.0,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&type_registry,
		) {
			// Restamp / LOD teardown can retire the root before apply.
			commands.entity(entity).try_insert(WorldAssetRoot(handle));
			true
		} else {
			false
		}
	};
	scan.run_primary_pass(&query, &mut remaining, &mut admit);
	if scan.should_wrap() {
		scan.run_wrap_pass(&query, &mut remaining, &mut admit);
	}
	*cursor = scan.next_cursor();
}

pub(crate) fn fulfill_multi_scene_merge_roots(
	mut commands: Commands,
	query: Query<(Entity, &MultiSceneMergeRoot), Without<WorldAssetRoot>>,
	mut scene_handles: ResMut<SceneRefHandles>,
	mut merge_handles: ResMut<MultiSceneMergeHandles>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	type_registry: Res<AppTypeRegistry>,
	budget: Res<SceneRefAdmitBudget>,
	mut cursor: Local<u32>,
) {
	let mut remaining = budget.per_frame;
	let mut miss_budget = budget.new_merge_meshes_per_frame;
	if remaining == 0 || budget.inspect_per_frame == 0 {
		return;
	}
	let mut scan = FulfillInspectScan::new(*cursor, budget.inspect_per_frame);
	let mut admit = |(entity, root): (Entity, &MultiSceneMergeRoot)| {
		if let Some(handle) = merge_handles.try_resolve(
			&root.0,
			&mut scene_handles,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&mut materials,
			&type_registry,
			&mut miss_budget,
		) {
			commands.entity(entity).try_insert(WorldAssetRoot(handle));
			true
		} else {
			false
		}
	};
	scan.run_primary_pass(&query, &mut remaining, &mut admit);
	if scan.should_wrap() {
		scan.run_wrap_pass(&query, &mut remaining, &mut admit);
	}
	*cursor = scan.next_cursor();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectVisit {
	Skip,
	Probe,
	Stop,
}

/// Round-robin window over a pending-root query.
///
/// [`Self::visit`] covers `[start, …]`. [`Self::visit_wrap`] covers `[0, start)`
/// when the first pass did not fill the inspect budget (cursor past the end, or
/// a short tail).
struct FulfillInspectScan {
	start: u32,
	inspect: u32,
	index: u32,
	probed: u32,
	next: u32,
	wrapped: bool,
}

impl FulfillInspectScan {
	fn new(start: u32, inspect: u32) -> Self {
		Self { start, inspect, index: 0, probed: 0, next: start, wrapped: false }
	}

	fn visit(&mut self) -> InspectVisit {
		let i = self.index;
		self.index = self.index.saturating_add(1);
		if i < self.start {
			return InspectVisit::Skip;
		}
		self.record_probe(i)
	}

	fn should_wrap(&self) -> bool {
		self.probed < self.inspect && self.start > 0 && !self.wrapped
	}

	fn visit_wrap(&mut self) -> InspectVisit {
		if !self.wrapped {
			self.wrapped = true;
			self.index = 0;
		}
		let i = self.index;
		self.index = self.index.saturating_add(1);
		if i >= self.start {
			return InspectVisit::Stop;
		}
		self.record_probe(i)
	}

	fn record_probe(&mut self, i: u32) -> InspectVisit {
		if self.probed >= self.inspect {
			self.next = i;
			return InspectVisit::Stop;
		}
		self.probed = self.probed.saturating_add(1);
		self.next = i.saturating_add(1);
		InspectVisit::Probe
	}

	fn next_cursor(&self) -> u32 {
		if self.probed < self.inspect {
			0
		} else {
			self.next
		}
	}

	/// Primary inspect pass: [`Self::visit`] over `items` in order.
	fn run_primary_pass<'a, I, T, F>(&mut self, items: I, remaining: &mut u32, admit: &mut F)
	where
		I: IntoIterator<Item = T>,
		I::IntoIter: 'a,
		F: FnMut(T) -> bool,
	{
		for item in items {
			match self.visit() {
				InspectVisit::Skip => continue,
				InspectVisit::Stop => break,
				InspectVisit::Probe => {}
			}
			if *remaining == 0 {
				break;
			}
			if admit(item) {
				*remaining -= 1;
			}
		}
	}

	/// Wrap inspect pass: [`Self::visit_wrap`] over `items` from the start.
	fn run_wrap_pass<'a, I, T, F>(&mut self, items: I, remaining: &mut u32, admit: &mut F)
	where
		I: IntoIterator<Item = T>,
		I::IntoIter: 'a,
		F: FnMut(T) -> bool,
	{
		for item in items {
			match self.visit_wrap() {
				InspectVisit::Skip => continue,
				InspectVisit::Stop => break,
				InspectVisit::Probe => {}
			}
			if *remaining == 0 {
				break;
			}
			if admit(item) {
				*remaining -= 1;
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn take(start: u32, len: u32, inspect: u32) -> (Vec<u32>, u32) {
		let mut scan = FulfillInspectScan::new(start, inspect);
		let mut visited = Vec::new();
		let mut remaining = u32::MAX;
		scan.run_primary_pass(0..len, &mut remaining, &mut |i| {
			visited.push(i);
			false
		});
		if scan.should_wrap() {
			scan.run_wrap_pass(0..len, &mut remaining, &mut |i| {
				visited.push(i);
				false
			});
		}
		(visited, scan.next_cursor())
	}

	#[test]
	fn inspect_window_starts_at_cursor() -> anyhow::Result<()> {
		let (visited, next) = take(0, 10, 3);
		assert_eq!(visited, vec![0, 1, 2]);
		assert_eq!(next, 3);
		Ok(())
	}

	#[test]
	fn inspect_window_wraps_and_advances_cursor() -> anyhow::Result<()> {
		let (visited, next) = take(8, 10, 3);
		assert_eq!(visited, vec![8, 9, 0]);
		assert_eq!(next, 1);
		Ok(())
	}

	#[test]
	fn inspect_window_resets_when_the_query_is_smaller_than_the_budget() -> anyhow::Result<()> {
		let (visited, next) = take(0, 2, 10);
		assert_eq!(visited, vec![0, 1]);
		assert_eq!(next, 0);
		Ok(())
	}

	#[test]
	fn inspect_window_skips_a_zero_budget() -> anyhow::Result<()> {
		let (visited, next) = take(4, 10, 0);
		assert!(visited.is_empty());
		assert_eq!(next, 4);
		Ok(())
	}
}
