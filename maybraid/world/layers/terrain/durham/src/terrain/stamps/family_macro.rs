//! Expand one jersey family band: controller layout + controller cell + leaf stamp.

/// Defines an independent guillotine stack for one stamp family **band**.
///
/// Each band owns its own controller grid (`controller_cell_size` /
/// `origin_offset`), guillotine preferred leaf range (`cell_size: (min, max)`),
/// and cut seed (via [`crate::terrain::stamps::configs::TerrainStampConfigs`]).
/// Leaf identities are not stored: stamp `build_with_id` down-levels `Id` to
/// cell bounds. The playable-world stream walks that band's controllers only.
///
/// Consumers depend on `$Stamp`'s `GenerationScheme` alone; leaf discovery
/// (controller cells → guillotine leaves) is encapsulated in `$Stamp`'s scheme.
///
/// `config_family` / `config_band` select e.g. `configs.massif.low_pass`.
///
/// Defaults for `likelihood`, `spatial_correlation`, `strength`, and guillotine
/// `cell_size` live here and are copied into
/// [`crate::terrain::stamps::configs::TerrainStampConfigs`]`::default`.
macro_rules! define_stamp_family {
	(
		layout: $Layout:ident,
		controller: $Controller:ident,
		stamp: $Stamp:ident,
		family_salt: $family_salt:expr,
		cell_size: ($cell_min:expr, $cell_max:expr),
		controller_cell_size: $controller_cell_size:expr,
		origin_offset: ($ox:expr, $oz:expr),
		likelihood: $likelihood:expr,
		spatial_correlation: $spatial_correlation:expr,
		strength: ($strength_min:expr, $strength_max:expr),
		config_family: $config_family:ident,
		config_band: $config_band:ident,
		|$bounds:ident, $seed:ident, $height_at:ident, $params:ident| $build:expr
	) => {
		/// Controller-grid layout for this jersey family band.
		#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq)]
		pub struct $Layout {
			pub grid: $crate::terrain::stamps::shared::OffsetControllerGrid,
		}

		impl Default for $Layout {
			fn default() -> Self {
				Self {
					grid: $crate::terrain::stamps::shared::OffsetControllerGrid::new(
						$controller_cell_size,
						bevy::math::Vec2::new($ox, $oz),
					),
				}
			}
		}

		impl $Layout {
			/// Preferred guillotine leaf size lower bound (world units).
			pub const CELL_SIZE_MIN: f32 = $cell_min;
			/// Preferred guillotine leaf size upper bound (world units).
			pub const CELL_SIZE_MAX: f32 = $cell_max;
			/// Default target acceptance rate for this band (`0.0..=1.0`).
			pub const LIKELIHOOD: f32 = $likelihood;
			/// Occupancy spatial correlation length (world units).
			pub const SPATIAL_CORRELATION: f32 = $spatial_correlation;
			/// Stamp strength lower bound (`1.0` ≈ default vertical knobs).
			pub const STRENGTH_MIN: f32 = $strength_min;
			/// Stamp strength upper bound.
			pub const STRENGTH_MAX: f32 = $strength_max;

			pub fn cell_bounds(&self, ix: i32, iz: i32) -> bevy::math::bounding::Aabb3d {
				self.grid.cell_bounds(ix, iz)
			}

			pub fn region_in_grid_space(
				&self,
				region: bevy::math::bounding::Aabb3d,
			) -> bevy::math::bounding::Aabb3d {
				self.grid.region_in_grid_space(region)
			}
		}

		$crate::terrain::cell::derived_universal_scheme!($Layout, |_cx| Some($Layout::default()));

		impl $crate::terrain::cell::CellTiling for $Layout {
			fn cell_ids(&self, region: bevy::math::bounding::Aabb3d) -> Vec<lod::gen::OriginalId> {
				self.grid.cell_ids(region)
			}
		}

		/// Controller cell: owns this band's guillotine cuts.
		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $Controller {
			pub cell: bevy::math::bounding::Aabb3d,
			pub cuts: comproc::guillotine::GuillotineCuts<2>,
		}

		impl $Controller {
			pub fn from_family_config<P>(
				cell: bevy::math::bounding::Aabb3d,
				config: &$crate::terrain::stamps::configs::FamilyGuillotineConfig<P>,
			) -> Self {
				Self { cell, cuts: $crate::terrain::stamps::shared::guillotine_cuts(cell, config) }
			}
		}

		impl $crate::terrain::stamps::shared::LeafAabbs for $Controller {
			fn leaf_aabbs(&self) -> Vec<bevy::math::bounding::Aabb3d> {
				$crate::terrain::stamps::shared::leaf_aabbs(self.cell, &self.cuts)
			}
		}

		impl lod::hcsg::GenerationScheme for $Controller {
			lod::hcsg_index_scale!($crate::terrain::index::DURHAM_INDEX_SCALE);
			fn original_ids_for(
				cx: &mut lod::hcsg::GenerationContext,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				<$Layout as $crate::terrain::cell::CellTiling>::origin_ids_in(cx, region)
			}

			fn build_with_id(
				cx: &mut lod::hcsg::GenerationContext,
				id: lod::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				let bounds = id.origin_cell_bounds()?;
				let configs = cx
					.get_or_generate::<$crate::terrain::stamps::configs::TerrainStampConfigs>(
						lod::gen::Id::Universal,
					)?;
				let family = &configs.$config_family.$config_band;
				Some((Self::from_family_config(bounds, family), bounds))
			}
		}

		/// Stamp output on one leaf of this band's guillotine partition.
		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $Stamp {
			pub cell: bevy::math::bounding::Aabb3d,
			pub modulations: Vec<terrain_stamps::StampModulation>,
		}

		impl $crate::terrain::stamps::shared::StampLeaf for $Stamp {
			fn cell(&self) -> bevy::math::bounding::Aabb3d {
				self.cell
			}

			fn modulations(&self) -> &[terrain_stamps::StampModulation] {
				&self.modulations
			}
		}

		impl lod::hcsg::GenerationScheme for $Stamp {
			lod::hcsg_index_scale!($crate::terrain::index::DURHAM_INDEX_SCALE);
			fn original_ids_for(
				cx: &mut lod::hcsg::GenerationContext,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				<$Controller as $crate::terrain::stamps::shared::LeafAabbs>::leaf_ids_in(cx, region)
			}

			fn build_with_id(
				cx: &mut lod::hcsg::GenerationContext,
				id: lod::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				let cell = id.origin_cell_bounds()?;
				let configs = cx
					.get_or_generate::<$crate::terrain::stamps::configs::TerrainStampConfigs>(
						lod::gen::Id::Universal,
					)?;
				let base = cx.get_or_generate::<$crate::terrain::base_noise::BaseTerrainNoise>(
					lod::gen::Id::Universal,
				)?;
				Some((Self::on_leaf(cell, &configs, &base), cell))
			}
		}

		impl $Stamp {
			/// This band's stamp on leaf `cell`: empty unless the occupancy
			/// gate selects the leaf.
			pub fn on_leaf(
				cell: bevy::math::bounding::Aabb3d,
				configs: &$crate::terrain::stamps::configs::TerrainStampConfigs,
				base: &$crate::terrain::base_noise::BaseTerrainNoise,
			) -> Self {
				let family = &configs.$config_family.$config_band;
				let $seed =
					$crate::terrain::stamps::shared::family_seed(base.seed, cell, $family_salt);
				// Occupancy gate: spatially correlated value noise at leaf center.
				let occ_seed = $crate::terrain::stamps::shared::occupancy_seed(
					base.seed,
					family.seed,
					$family_salt,
				);
				if !$crate::terrain::stamps::shared::leaf_selected(
					cell,
					occ_seed,
					family.likelihood,
					family.spatial_correlation,
				) {
					return Self { cell, modulations: Vec::new() };
				}
				let $bounds = $crate::terrain::stamps::shared::bounds2(cell);
				let strength = $crate::terrain::stamps::shared::sample_strength(
					$seed,
					family.strength_min,
					family.strength_max,
				);
				let $params =
					terrain_stamps::StampStrength::with_strength(family.stamp.clone(), strength);
				let height = |x: f32, z: f32| base.height_at(x, z);
				let $height_at: Option<&dyn Fn(f32, f32) -> f32> = Some(&height);
				// Hard-clip + edge ease to the leaf AABB so support is identity
				// outside the leaf (neighbors may omit this stamp).
				let modulations = terrain_stamps::StampModulation::bind_all($build, $bounds);
				Self { cell, modulations }
			}
		}
	};
}

pub(crate) use define_stamp_family;
