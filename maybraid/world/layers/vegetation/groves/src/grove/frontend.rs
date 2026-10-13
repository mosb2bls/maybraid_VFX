//! Shared grove assembly CLI surface reused by well-known grove render items.

use bevy_math::{Vec2, Vec3};
use procedural_common::NoiseParams;

#[cfg(feature = "render")]
use crate::grove::distribution::parse_variant_weights;
use crate::grove::distribution::VariantWeightOverrides;
use crate::grove::sampling::ForestGroveBiases;
use crate::grove::{Grove, GroveDefinition};

/// Forest biases, shared noise, authored-definition overrides, and perturbation origin.
///
/// One frontend serves every well-known grove: overrides default to `None`, which keeps the
/// grove's authored values.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "render", derive(clap::Args))]
#[cfg_attr(feature = "render", command(rename_all = "kebab-case"))]
pub struct GroveFrontend {
	#[cfg_attr(feature = "render", command(flatten, next_help_heading = "Grove Biases"))]
	pub biases: ForestGroveBiases,

	/// Shared deterministic noise for grove placement and bucket selection.
	///
	/// Defaults to `cellular` (per-cell hash values): gradient noise sampled at regularly
	/// spaced cell centers correlates between neighbours and reads as a grid, while cell
	/// values give independent, sign-balanced draws per cell.
	#[cfg_attr(
		feature = "render",
		arg(
			long = "grove-noise",
			default_value = "1337,1,1,1,cellular",
			value_parser = procedural_common::noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
			help_heading = "Grove Noise",
		)
	)]
	pub noise: NoiseParams,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			value_parser = parse_vec2_csv,
			value_name = "X,Z",
			help_heading = "Grove Assembly",
			help = "Vegetation cell footprint in metres; omit to use the grove's authored extent"
		)
	)]
	pub cell_extent_xz: Option<Vec2>,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			value_parser = parse_variant_weights,
			value_name = "W0,W1,...",
			help_heading = "Grove Distribution",
			help = "Per-bucket weights in distribution order; x keeps the authored default"
		)
	)]
	pub variant_weights: Option<VariantWeightOverrides>,

	#[cfg_attr(
		feature = "render",
		arg(
			long,
			default_value = "0,0,0",
			value_parser = parse_vec3_csv,
			value_name = "X,Y,Z",
			help_heading = "Grove Assembly",
		)
	)]
	pub perturbation_origin: Vec3,
}

impl Default for GroveFrontend {
	fn default() -> Self {
		Self {
			biases: ForestGroveBiases::default(),
			noise: NoiseParams {
				noise_type: procedural_common::NoiseType::Cellular,
				..NoiseParams::default()
			},
			cell_extent_xz: None,
			variant_weights: None,
			perturbation_origin: Vec3::ZERO,
		}
	}
}

impl GroveFrontend {
	/// Apply CLI overrides to an authored definition.
	///
	/// Mismatched variant weights warn and keep the authored weights rather than failing the
	/// whole render.
	pub fn definition<V>(&self, authored: GroveDefinition<V>) -> GroveDefinition<V> {
		let mut definition = authored;
		if let Some(cell_extent_xz) = self.cell_extent_xz {
			definition.cell_extent_xz = cell_extent_xz.max(Vec2::splat(0.1));
		}
		if let Some(ref overrides) = self.variant_weights {
			if let Err(err) = overrides.apply_to(&mut definition.distribution) {
				log::warn!("grove variant weights ignored: {err}");
			}
		}
		definition
	}

	/// Apply overrides and assemble the grove for selection.
	pub fn assemble<V: Clone>(&self, authored: GroveDefinition<V>) -> Grove<V> {
		Grove::assemble(
			self.definition(authored),
			self.biases,
			self.noise,
			self.perturbation_origin,
		)
	}
}

/// Two comma-separated floats (optional ASCII whitespace around commas).
pub fn parse_vec2_csv(s: &str) -> Result<Vec2, String> {
	match parse_f32_csv(s)?.as_slice() {
		&[x, y] => Ok(Vec2::new(x, y)),
		parts => Err(format!("expected two comma-separated numbers, got {}: {s:?}", parts.len())),
	}
}

/// Three comma-separated floats (optional ASCII whitespace around commas).
pub fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	match parse_f32_csv(s)?.as_slice() {
		&[x, y, z] => Ok(Vec3::new(x, y, z)),
		parts => Err(format!("expected three comma-separated numbers, got {}: {s:?}", parts.len())),
	}
}

fn parse_f32_csv(s: &str) -> Result<Vec<f32>, String> {
	s.split(',')
		.map(str::trim)
		.filter(|p| !p.is_empty())
		.map(|p| p.parse::<f32>().map_err(|e| format!("invalid float {p:?}: {e}")))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::distribution::parse_variant_weights;
	use anyhow::Result;

	#[test]
	fn definition_applies_cell_extent_and_weights() -> Result<()> {
		let frontend = GroveFrontend {
			cell_extent_xz: Some(Vec2::new(4.0, 3.0)),
			variant_weights: Some(
				parse_variant_weights("0.0,9.0,x,x,x,x,x,x,x,x")
					.map_err(|e| anyhow::anyhow!("{e}"))?,
			),
			..Default::default()
		};
		let definition = frontend.definition(crate::braid_grass::definition());
		assert_eq!(definition.cell_extent_xz, Vec2::new(4.0, 3.0));
		assert_eq!(definition.distribution.buckets[0].weight, 0.0);
		assert_eq!(definition.distribution.buckets[1].weight, 9.0);
		Ok(())
	}

	#[test]
	fn mismatched_weights_keep_authored_values() -> Result<()> {
		let frontend = GroveFrontend {
			variant_weights: Some(
				parse_variant_weights("1.0").map_err(|e| anyhow::anyhow!("{e}"))?,
			),
			..Default::default()
		};
		let authored = crate::braid_grass::definition();
		let definition = frontend.definition(authored.clone());
		assert_eq!(definition.distribution, authored.distribution);
		Ok(())
	}

	#[test]
	fn parse_vec_csv_accepts_whitespace_and_rejects_arity() -> Result<()> {
		assert_eq!(
			parse_vec2_csv("1.0, 2.0").map_err(|e| anyhow::anyhow!("{e}"))?,
			Vec2::new(1.0, 2.0)
		);
		assert!(parse_vec2_csv("1.0,2.0,3.0").is_err());
		assert_eq!(
			parse_vec3_csv("1,2,3").map_err(|e| anyhow::anyhow!("{e}"))?,
			Vec3::new(1.0, 2.0, 3.0)
		);
		assert!(parse_vec3_csv("1,nope,3").is_err());
		Ok(())
	}
}
