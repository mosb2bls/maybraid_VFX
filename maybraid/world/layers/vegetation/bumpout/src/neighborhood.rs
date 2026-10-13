use bevy::prelude::Color;
use material_ref::{MaterialRef, MATERIAL_RASTER_SAMPLES, MATERIAL_RASTER_WIDTH};
use vegetation_shaders::BUMP_OUT_MATERIAL;

pub use vegetation_shaders::{
	RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE, RASTER_BITE_SIZE_DEVIATION, RASTER_DENSITY,
	RASTER_HEIGHT_DEVIATION,
};

pub const BUMP_OUT_NEIGHBORHOOD_WIDTH: usize = MATERIAL_RASTER_WIDTH;
pub const BUMP_OUT_NEIGHBORHOOD_SAMPLES: usize = MATERIAL_RASTER_SAMPLES;

/// Row-major 3×3 vegetation-field samples centered on the presented chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BumpOutNeighborhood {
	pub densities: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	/// Characteristic world-space diameter of fragment bites.
	pub bite_sizes: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	/// Symmetric bite-size variation measured in binary scale octaves.
	pub bite_size_deviations: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	pub average_heights: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	/// Symmetric world-space displacement around `average_heights`.
	pub height_deviations: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
}

impl BumpOutNeighborhood {
	pub const fn new(
		densities: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		bite_sizes: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		bite_size_deviations: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		average_heights: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		height_deviations: [f32; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
	) -> Self {
		Self { densities, bite_sizes, bite_size_deviations, average_heights, height_deviations }
	}

	pub fn uniform(
		density: f32,
		bite_size: f32,
		bite_size_deviation: f32,
		average_height: f32,
		height_deviation: f32,
	) -> Self {
		Self {
			densities: [density; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
			bite_sizes: [bite_size; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
			bite_size_deviations: [bite_size_deviation; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
			average_heights: [average_height; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
			height_deviations: [height_deviation; BUMP_OUT_NEIGHBORHOOD_SAMPLES],
		}
	}

	/// Build the deferred material recipe consumed by [`crate::BumpOutMaterialLib`].
	pub fn material_ref(
		self,
		palette: impl IntoIterator<Item = Color>,
		noise: procedural_common::NoiseParams,
	) -> MaterialRef {
		MaterialRef::named(BUMP_OUT_MATERIAL)
			.with_palette(palette)
			.with_noise(noise)
			.with_raster(RASTER_DENSITY, self.densities)
			.with_raster(RASTER_BITE_SIZE, self.bite_sizes)
			.with_raster(RASTER_BITE_SIZE_DEVIATION, self.bite_size_deviations)
			.with_raster(RASTER_AVERAGE_HEIGHT, self.average_heights)
			.with_raster(RASTER_HEIGHT_DEVIATION, self.height_deviations)
			.with_scalars(BumpOutStyle::default().as_values())
	}

	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self {
			densities: material_ref.rasters.get_or(RASTER_DENSITY, 1.0),
			bite_sizes: material_ref.rasters.get_or(RASTER_BITE_SIZE, 12.0),
			bite_size_deviations: material_ref.rasters.get_or(RASTER_BITE_SIZE_DEVIATION, 0.0),
			average_heights: material_ref.rasters.get_or(RASTER_AVERAGE_HEIGHT, 0.0),
			height_deviations: material_ref.rasters.get_or(RASTER_HEIGHT_DEVIATION, 0.0),
		}
	}

	pub fn apply_to(self, material_ref: MaterialRef) -> MaterialRef {
		material_ref
			.with_raster(RASTER_DENSITY, self.densities)
			.with_raster(RASTER_BITE_SIZE, self.bite_sizes)
			.with_raster(RASTER_BITE_SIZE_DEVIATION, self.bite_size_deviations)
			.with_raster(RASTER_AVERAGE_HEIGHT, self.average_heights)
			.with_raster(RASTER_HEIGHT_DEVIATION, self.height_deviations)
	}

	pub fn min_displacement(self) -> f32 {
		self.average_heights
			.into_iter()
			.zip(self.height_deviations)
			.map(|(height, deviation)| height - deviation.abs())
			.fold(f32::INFINITY, f32::min)
	}

	pub fn max_displacement(self) -> f32 {
		self.average_heights
			.into_iter()
			.zip(self.height_deviations)
			.map(|(height, deviation)| height + deviation.abs())
			.fold(f32::NEG_INFINITY, f32::max)
	}

	pub fn set_density(&mut self, index: usize, density: f32) {
		if let Some(value) = self.densities.get_mut(index) {
			*value = density.clamp(0.0, 1.0);
		}
	}

	pub fn set_bite_size(&mut self, index: usize, bite_size: f32) {
		if let Some(value) = self.bite_sizes.get_mut(index) {
			*value = bite_size.max(0.01);
		}
	}

	pub fn set_bite_size_deviation(&mut self, index: usize, deviation: f32) {
		if let Some(value) = self.bite_size_deviations.get_mut(index) {
			*value = deviation.max(0.0);
		}
	}

	pub fn set_average_height(&mut self, index: usize, height: f32) {
		if let Some(value) = self.average_heights.get_mut(index) {
			*value = height;
		}
	}

	pub fn set_height_deviation(&mut self, index: usize, deviation: f32) {
		if let Some(value) = self.height_deviations.get_mut(index) {
			*value = deviation.max(0.0);
		}
	}
}

impl Default for BumpOutNeighborhood {
	fn default() -> Self {
		Self::uniform(1.0, 12.0, 0.0, 0.0, 0.0)
	}
}

/// Material-level controls not already represented by [`procedural_common::NoiseParams`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BumpOutStyle {
	/// Width of the fragment-coverage transition around the density threshold.
	pub coverage_softness: f32,
	/// PBR perceptual roughness.
	pub roughness: f32,
	/// Blend displaced geometric normals toward up.
	pub normal_soften: f32,
	/// Strength of the multi-scale bites relative to the broad density mask.
	pub cheese_amount: f32,
	/// Frequency multiplier for the multi-scale bite field.
	pub cheese_scale: f32,
	/// Frequency multiplier for static fragment-scale apparent height.
	pub fragment_height_frequency: f32,
	/// Apparent fragment-scale height amplitude in world units.
	pub fragment_height_amplitude: f32,
}

impl BumpOutStyle {
	pub const fn new(coverage_softness: f32, roughness: f32, normal_soften: f32) -> Self {
		Self {
			coverage_softness,
			roughness,
			normal_soften,
			cheese_amount: 0.75,
			cheese_scale: 1.0,
			fragment_height_frequency: 3.5,
			fragment_height_amplitude: 0.18,
		}
	}

	pub const fn with_cheese(mut self, amount: f32, scale: f32) -> Self {
		self.cheese_amount = amount;
		self.cheese_scale = scale;
		self
	}

	pub const fn with_fragment_height(mut self, frequency: f32, amplitude: f32) -> Self {
		self.fragment_height_frequency = frequency;
		self.fragment_height_amplitude = amplitude;
		self
	}

	pub const fn as_values(self) -> [f32; 7] {
		[
			self.coverage_softness,
			self.roughness,
			self.normal_soften,
			self.cheese_amount,
			self.cheese_scale,
			self.fragment_height_frequency,
			self.fragment_height_amplitude,
		]
	}

	pub fn apply_to(self, material_ref: MaterialRef) -> MaterialRef {
		material_ref.with_scalars(self.as_values())
	}

	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let values = material_ref.scalar_values();
		let defaults = Self::default();
		Self {
			coverage_softness: values.first().copied().unwrap_or(defaults.coverage_softness),
			roughness: values.get(1).copied().unwrap_or(defaults.roughness),
			normal_soften: values.get(2).copied().unwrap_or(defaults.normal_soften),
			cheese_amount: values.get(3).copied().unwrap_or(defaults.cheese_amount),
			cheese_scale: values.get(4).copied().unwrap_or(defaults.cheese_scale),
			fragment_height_frequency: values
				.get(5)
				.copied()
				.unwrap_or(defaults.fragment_height_frequency),
			fragment_height_amplitude: values
				.get(6)
				.copied()
				.unwrap_or(defaults.fragment_height_amplitude),
		}
	}
}

impl Default for BumpOutStyle {
	fn default() -> Self {
		Self::new(0.04, 0.92, 0.25)
	}
}
