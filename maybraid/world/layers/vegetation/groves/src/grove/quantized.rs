//! Default-noise helpers and wrapper macros for remixed [`QuantizedPlant`] silhouettes.
//!
//! Cache identity is the wrapper type, not the base tree. `build_unit` bakes
//! [`GroveFrontend::default`] noise (and default chain noise for bushes), not
//! CLI-overridden grove noise. Palette / placement stay per-cell.

use super::frontend::GroveFrontend;
use super::vc_tuft::variant_noise;
use procedural_common::NoiseParams;

/// Variant-keyed construction noise from [`GroveFrontend::default`].
pub fn unit_build_noise(num: u32) -> NoiseParams {
	variant_noise(GroveFrontend::default().noise, num)
}

/// Variant-keyed chain noise from `Params` defaults (`0, 1, 1, 1`).
pub fn unit_chain_noise(num: u32) -> NoiseParams {
	variant_noise(NoiseParams::from_scalar(0.0, 1.0, 1.0, 1), num)
}

/// One remixed SBS silhouette: `params.geometry = authored.build_with_noise(default)`.
macro_rules! remixed_sbs_plant {
	($name:ident, $unit:ty, $params:ty, $authored:expr) => {
		struct $name;
		impl ::sbs_trees::QuantizedPlant for $name {
			type Unit = $unit;
			fn build_unit(num: u32) -> ($unit, f32) {
				use ::procedural_common::BuildWithNoise;
				let mut params = <$params>::default();
				params.geometry =
					($authored).build_with_noise($crate::grove::unit_build_noise(num));
				let (unit, world_size) = params.into_unit_from_num(num);
				(unit.build(), world_size)
			}
		}
	};
}

/// One remixed High Bush silhouette, including default `chain_noise`.
macro_rules! remixed_bush_plant {
	($name:ident, $authored:expr) => {
		struct $name;
		impl ::sbs_trees::QuantizedPlant for $name {
			type Unit = ::sbs_trees::HighBushShoots;
			fn build_unit(num: u32) -> (::sbs_trees::HighBushShoots, f32) {
				use ::procedural_common::BuildWithNoise;
				let mut shape = ($authored).build_with_noise($crate::grove::unit_build_noise(num));
				shape.chain_noise = $crate::grove::unit_chain_noise(num);
				let (unit, world_size) =
					::sbs_trees::HighBushShootsParams::new(shape).into_unit_from_num(num);
				(unit.build(), world_size)
			}
		}
	};
}

/// One remixed [`GroveTuftPatch`] silhouette.
macro_rules! remixed_tuft_plant {
	($name:ident, $authored:expr, $default_foliage:expr) => {
		struct $name;
		impl ::sbs_trees::QuantizedPlant for $name {
			type Unit = ::sbs_trees::TuftPatch;
			fn build_unit(num: u32) -> (::sbs_trees::TuftPatch, f32) {
				$crate::grove::remixed_tuft_unit(&$authored, num, $default_foliage)
			}
		}
	};
}

/// One remixed single-clump blade tuft.
macro_rules! remixed_blade_tuft_plant {
	($name:ident, $authored:expr, $default_foliage:expr) => {
		struct $name;
		impl ::sbs_trees::QuantizedPlant for $name {
			type Unit = ::sbs_trees::TuftPatch;
			fn build_unit(num: u32) -> (::sbs_trees::TuftPatch, f32) {
				$crate::grove::remixed_blade_tuft_unit(&$authored, num, $default_foliage)
			}
		}
	};
}

/// One remixed spear clump approximated as a blade tuft.
macro_rules! remixed_spear_tuft_plant {
	($name:ident, $authored:expr, $default_foliage:expr) => {
		struct $name;
		impl ::sbs_trees::QuantizedPlant for $name {
			type Unit = ::sbs_trees::TuftPatch;
			fn build_unit(num: u32) -> (::sbs_trees::TuftPatch, f32) {
				$crate::grove::remixed_spear_tuft_unit(&$authored, num, $default_foliage)
			}
		}
	};
}

pub(crate) use remixed_blade_tuft_plant;
pub(crate) use remixed_bush_plant;
pub(crate) use remixed_sbs_plant;
pub(crate) use remixed_spear_tuft_plant;
pub(crate) use remixed_tuft_plant;
