//! Dual-band Watershed LOD stack macro (low-pass = small, high-pass = large).
//!
//! Authoring knobs (`cell_size`, `likelihood`, …) live on the call site — same
//! pattern as [`crate::terrain::stamps::family_macro::define_stamp_family`].

/// Defines `PrePocketLayout` → `PrePocketCell` → `PocketCell` → `PocketWaters*` for one band.
///
/// Each level discovers its ids from the level above through its scheme's
/// `original_ids_for`, so consumers depend only on `$PocketWaters`.
macro_rules! define_marazion_band {
	(
		layout: $Layout:ident,
		layout_from_configs: $layout_fn:ident,
		pre_cell: $PreCell:ident,
		pocket: $Pocket:ident,
		pocket_waters: $PocketWaters:ident,
		band_field: $band_field:ident,
		band_pass: $band_pass:ident,
		family_salt: $family_salt:expr,
		cell_size: ($cell_min:expr, $cell_max:expr),
		pre_pocket_pitch: $pre_pitch:expr,
		pocket_pitches: $pocket_pitches:expr,
		origin_offset: ($ox:expr, $oz:expr),
		likelihood: $likelihood:expr,
		spatial_correlation: $spatial_correlation:expr,
	) => {
		/// Pre-pocket controller grid for this Watershed band.
		#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq)]
		pub struct $Layout {
			pub grid: $crate::terrain::stamps::shared::OffsetControllerGrid,
		}

		impl Default for $Layout {
			fn default() -> Self {
				Self {
					grid: $crate::terrain::stamps::shared::OffsetControllerGrid::new(
						$pre_pitch,
						bevy::math::Vec2::new($ox, $oz),
					),
				}
			}
		}

		impl $Layout {
			/// Guillotine leaf size lower bound / `min_span` (world units).
			pub const CELL_SIZE_MIN: f32 = $cell_min;
			/// Preferred max leaf / pocket side (world units).
			pub const CELL_SIZE_MAX: f32 = $cell_max;
			/// Pre-pocket controller pitch (world units).
			pub const PRE_POCKET_PITCH: f32 = $pre_pitch;
			/// Discrete pocket pitches (each must divide [`Self::PRE_POCKET_PITCH`]).
			pub const POCKET_PITCHES: [f32; 4] = $pocket_pitches;
			/// World origin offset for this band's controller grid.
			pub const ORIGIN_OFFSET: (f32, f32) = ($ox, $oz);
			/// Default leaf acceptance rate (`0.0..=1.0`).
			pub const LIKELIHOOD: f32 = $likelihood;
			/// Occupancy spatial correlation length (world units).
			pub const SPATIAL_CORRELATION: f32 = $spatial_correlation;
			/// Occupancy / cut salt for this band.
			pub const FAMILY_SALT: u32 = $family_salt;
		}

		/// This band's pre-pocket grid, derived from [`WatershedConfigs`](crate::terrain::watersheds::WatershedConfigs).
		pub fn $layout_fn(
			configs: &$crate::terrain::watersheds::config::WatershedConfigs,
		) -> $Layout {
			let band = &configs.$band_field;
			$Layout {
				grid: $crate::terrain::stamps::shared::OffsetControllerGrid::new(
					band.pre_pocket.pitch.max(1.0),
					band.pre_pocket.origin,
				),
			}
		}

		$crate::terrain::cell::derived_universal_scheme!($Layout, |cx| cx
			.get_or_generate::<$crate::terrain::watersheds::config::WatershedConfigs>(
				lod::gen::Id::Universal,
			)
			.map(|configs| $layout_fn(&configs)));

		impl $crate::terrain::cell::CellTiling for $Layout {
			fn cell_ids(&self, region: bevy::math::bounding::Aabb3d) -> Vec<lod::gen::OriginalId> {
				self.grid.cell_ids(region)
			}
		}

		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $PreCell {
			pub cell: bevy::math::bounding::Aabb3d,
			pub pre: terrain_watersheds::PrePocket,
		}

		/// Pocket AABBs on this pre-pocket's lattice.
		impl $crate::terrain::stamps::shared::LeafAabbs for $PreCell {
			fn leaf_aabbs(&self) -> Vec<bevy::math::bounding::Aabb3d> {
				let (vy_min, vy_max) = (self.cell.min.y, self.cell.max.y);
				(0..self.pre.nx)
					.flat_map(|px| (0..self.pre.nz).map(move |pz| (px, pz)))
					.map(|(px, pz)| {
						$crate::terrain::watersheds::pre_pocket::pocket_aabb(
							&self.pre, px, pz, vy_min, vy_max,
						)
					})
					.collect()
			}
		}

		impl lod::hcsg::GenerationScheme for $PreCell {
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
				let cell = id.origin_cell_bounds()?;
				let configs = cx
					.get_or_generate::<$crate::terrain::watersheds::config::WatershedConfigs>(
						lod::gen::Id::Universal,
					)?;
				Some((Self::on_cell(cell, &configs), cell))
			}
		}

		impl $PreCell {
			/// The pre-pocket containing `cell`'s center.
			pub fn on_cell(
				cell: bevy::math::bounding::Aabb3d,
				configs: &$crate::terrain::watersheds::config::WatershedConfigs,
			) -> Self {
				let cx = (cell.min.x + cell.max.x) * 0.5;
				let cz = (cell.min.z + cell.max.z) * 0.5;
				let mut params = configs.$band_field.pre_pocket;
				params.seed = configs.seed.wrapping_add(configs.$band_field.family_salt);
				let pre = terrain_watersheds::PrePocket::containing(cx, cz, &params);
				Self { cell, pre }
			}
		}

		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $Pocket {
			pub cell: bevy::math::bounding::Aabb3d,
			pub leaves: Vec<bevy::math::bounding::Aabb3d>,
		}

		impl $crate::terrain::stamps::shared::LeafAabbs for $Pocket {
			fn leaf_aabbs(&self) -> Vec<bevy::math::bounding::Aabb3d> {
				self.leaves.clone()
			}
		}

		impl lod::hcsg::GenerationScheme for $Pocket {
			lod::hcsg_index_scale!($crate::terrain::index::DURHAM_INDEX_SCALE);
			fn original_ids_for(
				cx: &mut lod::hcsg::GenerationContext,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				<$PreCell as $crate::terrain::stamps::shared::LeafAabbs>::leaf_ids_in(cx, region)
			}

			fn build_with_id(
				cx: &mut lod::hcsg::GenerationContext,
				id: lod::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				let cell = id.origin_cell_bounds()?;
				let configs = cx
					.get_or_generate::<$crate::terrain::watersheds::config::WatershedConfigs>(
						lod::gen::Id::Universal,
					)?;
				Some((Self::on_cell(cell, &configs), cell))
			}
		}

		impl $Pocket {
			/// The guillotine leaves partitioning pocket `cell`.
			pub fn on_cell(
				cell: bevy::math::bounding::Aabb3d,
				configs: &$crate::terrain::watersheds::config::WatershedConfigs,
			) -> Self {
				use procedural_common::Bounds2;
				let band = &configs.$band_field;
				let mut gparams = band.guillotine;
				gparams.seed = configs.seed.wrapping_add(band.family_salt).wrapping_add(0x6011);
				let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
				let leaves: Vec<_> = terrain_watersheds::guillotine_partition(bounds, &gparams)
					.into_iter()
					.map(|b| {
						$crate::terrain::watersheds::pre_pocket::aabb_from_bounds2(
							b, cell.min.y, cell.max.y,
						)
					})
					.collect();
				Self { cell, leaves }
			}
		}

		/// Guillotine leaf holding one authored pocket-water stamp (not a compiled complex).
		#[derive(Debug, Clone, bevy::prelude::Component)]
		pub struct $PocketWaters {
			pub cell: bevy::math::bounding::Aabb3d,
			pub band: $crate::terrain::watersheds::leaf_kind::WatershedBandPass,
			pub authored: $crate::terrain::watersheds::pocket_water::PocketWater,
		}

		impl $PocketWaters {
			pub fn hydro_nodes(&self) -> Vec<terrain_watersheds::HydroNode> {
				self.authored.hydro_nodes()
			}

			pub fn kind(&self) -> $crate::terrain::watersheds::leaf_kind::WatershedLeafKind {
				self.authored.kind()
			}

			pub fn leaf_bounds(
				&self,
			) -> $crate::terrain::watersheds::leaf_kind::WatershedLeafBounds {
				$crate::terrain::watersheds::leaf_kind::WatershedLeafBounds {
					cell: self.cell,
					kind: self.kind(),
					band: self.band,
				}
			}
		}

		// `PreWatershedTerrain` + `TerrainCellLayout` back the live height
		// sampler (`PreWatershedTerrain::sample_height`) used while authoring.

		impl lod::hcsg::GenerationScheme for $PocketWaters {
			lod::hcsg_index_scale!($crate::terrain::index::DURHAM_INDEX_SCALE);
			fn original_ids_for(
				cx: &mut lod::hcsg::GenerationContext,
				region: bevy::math::bounding::Aabb3d,
			) -> Vec<lod::gen::OriginalId> {
				<$Pocket as $crate::terrain::stamps::shared::LeafAabbs>::leaf_ids_in(cx, region)
			}

			fn build_with_id(
				cx: &mut lod::hcsg::GenerationContext,
				id: lod::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				let cell = id.origin_cell_bounds()?;
				let configs = cx
					.get_or_generate::<$crate::terrain::watersheds::config::WatershedConfigs>(
						lod::gen::Id::Universal,
					)?;
				let cx = std::cell::RefCell::new(cx);
				let height_at = |x: f32, z: f32| {
					$crate::terrain::PreWatershedTerrain::sample_height_in(
						&mut cx.borrow_mut(),
						x,
						z,
					)
					.unwrap_or(0.0)
				};
				Some((Self::author(cell, &configs, &height_at), cell))
			}
		}

		impl $PocketWaters {
			/// The pocket water authored on leaf `cell`, surveying the
			/// pre-watershed surface through `height_at`; empty unless the
			/// occupancy gate selects the leaf and some kind fits.
			pub fn author(
				cell: bevy::math::bounding::Aabb3d,
				configs: &$crate::terrain::watersheds::config::WatershedConfigs,
				height_at: &dyn Fn(f32, f32) -> f32,
			) -> Self {
				use procedural_common::Bounds2;
				let band = &configs.$band_field;
				let occ_seed = $crate::terrain::stamps::shared::occupancy_seed(
					configs.seed,
					0,
					band.family_salt,
				);
				let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
				let seed = configs.seed.wrapping_add(band.family_salt).wrapping_add(
					cell.min.x.to_bits().wrapping_mul(73856093)
						^ cell.min.z.to_bits().wrapping_mul(19349663),
				);
				let empty_cell = || Self {
					cell,
					band: $crate::terrain::watersheds::leaf_kind::WatershedBandPass::$band_pass,
					authored: $crate::terrain::watersheds::pocket_water::PocketWater::Empty,
				};
				if !$crate::terrain::stamps::shared::leaf_selected(
					cell,
					occ_seed,
					band.likelihood,
					band.spatial_correlation,
				) {
					return empty_cell();
				}
				let height_at = Some(height_at);

				// Occupied leaves: stream / streams-graph / bog / lake from a stable unit draw.
				let type_u =
					procedural_common::SeededHash::new(seed.wrapping_add(0x57EA_71FE)).unit(0);
				let stream_cut = band.stream_frac.clamp(0.0, 1.0);
				let graph_cut = (stream_cut + band.streams_graph_frac.clamp(0.0, 1.0)).min(1.0);
				let bog_cut = (graph_cut + band.bog_frac.clamp(0.0, 1.0)).min(1.0);
				let prefer = if type_u < stream_cut {
					0u8 // stream
				} else if type_u < graph_cut {
					1u8 // streams graph
				} else if type_u < bog_cut {
					2u8 // bog
				} else {
					3u8 // lake
				};

				use $crate::terrain::watersheds::pocket_water::PocketWater;
				let try_stream = || -> Option<PocketWater> {
					terrain_watersheds::Stream::from_bounds(bounds, seed, band.stream, height_at)
						.map(PocketWater::Stream)
				};
				let try_streams_graph = || -> Option<PocketWater> {
					terrain_watersheds::StreamsGraph::from_bounds(
						bounds,
						seed,
						band.streams_graph,
						height_at,
					)
					.map(PocketWater::StreamsGraph)
				};
				let try_bog = || -> Option<PocketWater> {
					terrain_watersheds::Bog::from_bounds(bounds, seed, band.bog, height_at)
						.map(PocketWater::Bog)
				};
				let try_lake = || -> Option<PocketWater> {
					terrain_watersheds::Lake::from_bounds(bounds, seed, band.lake, height_at)
						.map(PocketWater::Lake)
				};

				let authored = match prefer {
					0 => try_stream()
						.or_else(try_streams_graph)
						.or_else(try_bog)
						.or_else(try_lake)
						.unwrap_or(PocketWater::Empty),
					1 => try_streams_graph()
						.or_else(try_stream)
						.or_else(try_bog)
						.or_else(try_lake)
						.unwrap_or(PocketWater::Empty),
					2 => try_bog()
						.or_else(try_stream)
						.or_else(try_streams_graph)
						.or_else(try_lake)
						.unwrap_or(PocketWater::Empty),
					_ => try_lake()
						.or_else(try_stream)
						.or_else(try_streams_graph)
						.or_else(try_bog)
						.unwrap_or(PocketWater::Empty),
				};

				if authored.is_empty() {
					return empty_cell();
				}
				Self {
					cell,
					band: $crate::terrain::watersheds::leaf_kind::WatershedBandPass::$band_pass,
					authored,
				}
			}
		}
	};
}

pub(crate) use define_marazion_band;
