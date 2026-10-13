//! Shared tuft-grove [`LodScene`]. Authored bands stay on the grove body.
//!
//! High / Medium kits come from stored plants (`lazy_posed_tuft_chunks`): begin
//! is [`std::sync::Arc::clone`] of the plant list. Low / UltraLow drain proxies
//! baked at grow (`TuftGroveBody::low_ultra_chunks`). Drain poses one kit at a time.

/// Mechanical [`LodScene`] for a tuft grove that already implements [`VegetationComponents`].
///
/// High / Medium use [`TuftGroveBody::high_medium_chunks`] on `self.body`. Low /
/// UltraLow use [`TuftGroveBody::low_ultra_chunks`]. Groves that emit extra kits
/// (Tropical Tufts palms) or store plants off-body (Monster Grass) implement
/// `tuft_scene_chunks` and take [`impl_tuft_grove_lod_emit!`].
#[macro_export]
macro_rules! impl_tuft_grove_lod {
	($Grove:ty) => {
		impl $Grove {
			fn tuft_scene_chunks(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::scene::LodSceneLevel,
			) -> lod::SceneChunk {
				match level {
					lod::scene::LodSceneLevel::High | lod::scene::LodSceneLevel::Medium => {
						self.body.high_medium_chunks(lod_ref, level)
					}
					_ => self.body.low_ultra_chunks(lod_ref, level),
				}
			}
		}
		$crate::impl_tuft_grove_lod_emit!($Grove);
	};
}

/// [`impl_tuft_grove_lod!`] for a grove that already has `tuft_scene_chunks`.
#[macro_export]
macro_rules! impl_tuft_grove_lod_emit {
	($Grove:ty) => {
		impl $Grove {
			/// Shared tuft plant list for forest present dropout.
			pub fn tuft_body(&self) -> &$crate::grove::vc_tuft::TuftGroveBody {
				&self.body
			}
		}
		impl lod::scene::LodScene for $Grove {
			fn scene_lod_level(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::scene::LodSceneLevel {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_level(band, lod_ref))
					.unwrap_or(lod::scene::LodSceneLevel::High)
			}

			fn scene_lod_status(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
			) -> lod::scene::LodSceneStatus {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_status(band, lod_ref))
					.unwrap_or(lod::scene::LodSceneStatus::Unchanged)
			}

			fn scene_lod_culls(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				_current: lod::scene::LodSceneLevel,
			) -> lod::scene::LodSceneCulls {
				self.structural_lod()
					.map(|band| $crate::grove::grove_lod_culls(band, lod_ref))
					.unwrap_or(lod::scene::LodSceneCulls::None)
			}

			fn scene_with_level(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::scene::LodSceneLevel,
			) -> impl bevy::scene::prelude::Scene + 'static {
				vegetation_components::flattened_component_scene(self, lod_ref, level)
			}

			fn scene_chunks_with_level(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
				level: lod::scene::LodSceneLevel,
			) -> lod::SceneChunk {
				self.tuft_scene_chunks(lod_ref, level)
			}

			fn scene_bounds(&self) -> bevy::math::bounding::Aabb3d {
				self.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_components::vegetation_bounds(self))
			}

			fn scene_with_lod(
				&self,
				lod_ref: &lod::lod_ref::LodRef,
			) -> impl bevy::scene::prelude::Scene + 'static {
				lod::lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
			}
		}
	};
}
