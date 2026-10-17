//! Read-only, deliberately versioned Houdini contract and bounded animation storage.
use bevy::{
	asset::RenderAssetUsages,
	image::{CompressedImageFormats, ImageSampler, ImageType},
	prelude::*,
	render::render_resource::{
		Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
	},
};
use serde_json::Value;
use std::{
	fs,
	path::{Path, PathBuf},
};

pub struct Bundle {
	pub root: PathBuf,
	pub manifest: Value,
}
impl Bundle {
	pub fn read(root: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
		let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
		let bundle = Self { root, manifest };
		bundle.validate()?;
		Ok(bundle)
	}
	pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
		let m = &self.manifest;
		for (actual, expected) in [
			(&m["C"]["resolution_xyz"], serde_json::json!([64, 32, 64])),
			(&m["space"]["bounds_min_m"], serde_json::json!([-3.2, 0, -3.2])),
			(&m["space"]["bounds_max_m"], serde_json::json!([3.2, 3.2, 3.2])),
			(&m["B"]["atlas_size_px"], serde_json::json!([1848, 1848])),
			(&m["B"]["tiles_xy"], serde_json::json!([7, 7])),
			(&m["B"]["padding_each_edge_px"], serde_json::json!(4)),
			(&m["C"]["consistent_scale"], serde_json::json!(1.0)),
		] {
			if actual != &expected {
				return Err(format!("Unsupported v1 contract: {actual} != {expected}").into());
			}
		}
		let timestamps =
			m["shared_source"]["timestamps_s"].as_array().ok_or("missing timestamps")?;
		if timestamps.len() != 49
			|| m["B"]["timestamps_s"] != m["C"]["timestamps_s"]
			|| m["B"]["timestamps_s"].as_array() != Some(timestamps)
		{
			return Err("B/C/source timestamps differ".into());
		}
		for (i, t) in timestamps.iter().enumerate() {
			if (t.as_f64().ok_or("invalid timestamp")? - i as f64 / 15.0).abs() > 1e-8 {
				return Err("unsupported nonuniform timestamps".into());
			}
		}
		Ok(())
	}
	pub fn atlas(&self) -> Result<Image, Box<dyn std::error::Error>> {
		let bytes =
			fs::read(self.root.join(self.manifest["B"]["file"].as_str().ok_or("atlas path")?))?;
		let image = Image::from_buffer(
			&bytes,
			ImageType::Extension("png"),
			CompressedImageFormats::NONE,
			true,
			ImageSampler::linear(),
			RenderAssetUsages::RENDER_WORLD,
		)?;
		if image.texture_descriptor.size.width != 1848
			|| image.texture_descriptor.size.height != 1848
			|| image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
		{
			return Err("unexpected atlas dimensions/format".into());
		}
		Ok(image)
	}
	/// Pack the *bounded* 49-frame sequence into 7x7 spatial cells of one 3D R16F texture.
	/// Each cell remains a true 64x32x64 volume. The shader clamps to cell voxel centres.
	/// This trades 12.25 MiB GPU residency for zero playback uploads/decodes/allocations.
	pub fn density(&self, markers: bool) -> Result<Image, Box<dyn std::error::Error>> {
		let mut packed = vec![0u8; 448 * 224 * 64 * 2];
		for i in 0..49 {
			let path = if markers {
				self.root.join("diagnostics/axis_markers.f16")
			} else {
				self.root.join(format!("C_density/frames/density_{i:03}.f16"))
			};
			let bytes = Self::read_frame(&path)?;
			for z in 0..64 {
				for y in 0..32 {
					let from = (z * 32 + y) * 64 * 2;
					let to = ((z * 224 + (i / 7) * 32 + y) * 448 + (i % 7) * 64) * 2;
					packed[to..to + 128].copy_from_slice(&bytes[from..from + 128]);
				}
			}
		}
		let mut image = Image::new(
			Extent3d { width: 448, height: 224, depth_or_array_layers: 64 },
			TextureDimension::D3,
			packed,
			TextureFormat::R16Float,
			RenderAssetUsages::RENDER_WORLD,
		);
		image.sampler = ImageSampler::linear();
		Ok(image)
	}
	pub fn read_frame(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
		let data = fs::read(path)?;
		if data.len() != 262144 {
			return Err(format!("wrong frame size: {}", path.display()).into());
		}
		for pair in data.chunks_exact(2) {
			let bits = u16::from_le_bytes([pair[0], pair[1]]);
			if bits & 0x8000 != 0 || bits & 0x7c00 == 0x7c00 {
				return Err("negative/nonfinite density".into());
			}
		}
		Ok(data)
	}
	pub fn basis(&self, key: &str) -> Result<Vec3, Box<dyn std::error::Error>> {
		let a = &self.manifest["B"]["capture"][key];
		Ok(Vec3::new(
			a[0].as_f64().ok_or("basis x")? as f32,
			a[1].as_f64().ok_or("basis y")? as f32,
			a[2].as_f64().ok_or("basis z")? as f32,
		))
	}
	pub fn frames(t: f32) -> (u32, u32, f32) {
		let f = t.clamp(0., 3.2) * 15.;
		let i = (f.floor() as u32).min(48);
		(i, (i + 1).min(48), f - i as f32)
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn matched_contract_and_axis_layout() -> Result<(), Box<dyn std::error::Error>> {
		let bundle =
			Bundle::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../wind_dust_demos"))?;
		let data = Bundle::read_frame(&bundle.root.join("diagnostics/axis_markers.f16"))?;
		for ((x, y, z), expected) in
			[((50, 6, 9), 0x3400u16), ((10, 24, 20), 0x3800), ((25, 12, 50), 0x3a00)]
		{
			let n = 2 * ((z * 32 + y) * 64 + x);
			assert_eq!(u16::from_le_bytes([data[n], data[n + 1]]), expected);
		}
		let image = bundle.density(false)?;
		assert_eq!(image.data.as_ref().ok_or("missing data")?.len(), 12845056);
		Ok(())
	}
	#[test]
	fn time_clamps_and_interpolates() {
		assert_eq!(Bundle::frames(-1.), (0, 1, 0.));
		assert_eq!(Bundle::frames(0.1), (1, 2, 0.5));
		assert_eq!(Bundle::frames(9.), (48, 48, 0.));
	}
}

/// The bounded eight-view v1 experiment: one 952px atlas per layer, never rewritten during playback.
pub struct MultiView;
impl MultiView {
	pub const GPU_BYTES: usize = 952 * 952 * 8 * 4;
	pub fn read(root: &Path, source: &Bundle) -> Result<Image, Box<dyn std::error::Error>> {
		let manifest: Value = serde_json::from_slice(&fs::read(root.join("manifest.json"))?)?;
		if manifest["contract"] != "Maybraid B+ eight-view baked dust v1"
			|| manifest["shared_source"]["timestamps_s"]
				!= source.manifest["shared_source"]["timestamps_s"]
			|| manifest["B_plus"]["timestamps_s"]
				!= source.manifest["shared_source"]["timestamps_s"]
			|| manifest["space"] != source.manifest["space"]
		{
			return Err("B+ source time/space contract differs from matched C".into());
		}
		for (key, expected) in [
			("view_count", serde_json::json!(8)),
			("atlas_size_px", serde_json::json!([952, 952])),
			("tiles_xy", serde_json::json!([7, 7])),
			("frame_tile_interior_px", serde_json::json!([128, 128])),
			("padding_each_edge_px", serde_json::json!(4)),
			("tile_pitch_px", serde_json::json!([136, 136])),
			("elevation_degrees", serde_json::json!(20)),
			("capture_width_height_m", serde_json::json!([7.8, 7.8])),
			("quad_centre_m", serde_json::json!([0., 0.55, 0.])),
			("pivot_m", serde_json::json!([0, 0, 0])),
		] {
			if manifest["B_plus"][key] != expected {
				return Err(format!("Unsupported B+ v1 {key}").into());
			}
		}
		let views = manifest["B_plus"]["views"].as_array().ok_or("Missing B+ camera views")?;
		if views.len() != 8 {
			return Err("Wrong view count".into());
		}
		for (i, view) in views.iter().enumerate() {
			if view["index"] != i
				|| view["azimuth_degrees"] != i * 45
				|| view["elevation_degrees"] != 20
			{
				return Err("Unsupported B+ azimuth ring".into());
			}
			let theta = i as f32 * std::f32::consts::FRAC_PI_4;
			let e = 20f32.to_radians();
			for (key, expected) in [
				("image_right", Vec3::new(theta.cos(), 0., -theta.sin())),
				("image_up", Vec3::new(-e.sin() * theta.sin(), e.cos(), -e.sin() * theta.cos())),
				("camera_target_m", Vec3::new(0., 0.55, 0.)),
			] {
				for axis in 0..3 {
					if (view[key][axis].as_f64().ok_or("Invalid B+ camera basis")? as f32
						- expected[axis])
						.abs() > 0.000001
					{
						return Err("B+ camera basis does not match shader".into());
					}
				}
			}
		}
		let mut packed = Vec::with_capacity(Self::GPU_BYTES);
		for view in 0..8 {
			let bytes = fs::read(root.join(format!("atlases/view_{view:02}.png")))?;
			let image = Image::from_buffer(
				&bytes,
				ImageType::Extension("png"),
				CompressedImageFormats::NONE,
				true,
				ImageSampler::linear(),
				RenderAssetUsages::RENDER_WORLD,
			)?;
			if image.texture_descriptor.size.width != 952
				|| image.texture_descriptor.size.height != 952
				|| image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
			{
				return Err("Unsupported B+ v1 atlas format/dimensions".into());
			}
			packed.extend(image.data.ok_or("missing multiview image data")?);
		}
		if packed.len() != Self::GPU_BYTES {
			return Err("B+ image byte count mismatch".into());
		}
		Ok(Self::array_image(952, 952, 8, packed))
	}
	pub fn fallback() -> Image {
		Self::array_image(1, 1, 1, vec![0, 0, 0, 0])
	}
	fn array_image(width: u32, height: u32, layers: u32, data: Vec<u8>) -> Image {
		let mut image = Image::new(
			Extent3d { width, height, depth_or_array_layers: layers },
			TextureDimension::D2,
			data,
			TextureFormat::Rgba8UnormSrgb,
			RenderAssetUsages::RENDER_WORLD,
		);
		image.texture_view_descriptor = Some(TextureViewDescriptor {
			dimension: Some(TextureViewDimension::D2Array),
			..Default::default()
		});
		image.sampler = ImageSampler::linear();
		image
	}
	/// Azimuth is atan2(camera.x,camera.z), with a cyclic seam at +Z.
	pub fn views(azimuth: f32, nearest: bool) -> (u32, u32, f32) {
		let x = azimuth.rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_4;
		if nearest {
			let i = (x.round() as u32) % 8;
			(i, i, 0.)
		} else {
			let i = x.floor() as u32 % 8;
			(i, (i + 1) % 8, x - x.floor())
		}
	}
}
#[cfg(test)]
mod multiview_tests {
	use super::*;
	#[test]
	fn actual_eight_atlases_match_source_and_pack_layers() -> Result<(), Box<dyn std::error::Error>>
	{
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
		let source = Bundle::read(root.join("wind_dust_demos"))?;
		let image = MultiView::read(&root.join("wind_dust_multiview"), &source)?;
		assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 8);
		assert_eq!(image.data.as_ref().ok_or("missing data")?.len(), MultiView::GPU_BYTES);
		assert_eq!(
			image.texture_view_descriptor.as_ref().ok_or("missing view")?.dimension,
			Some(TextureViewDimension::D2Array)
		);
		Ok(())
	}

	#[test]
	fn cyclic_selection_preserves_seam_and_nearest() {
		let (a, b, w) = MultiView::views(-std::f32::consts::FRAC_PI_4 * 0.25, false);
		assert_eq!((a, b), (7, 0));
		assert!((w - 0.75).abs() < 0.00001);
		assert_eq!(MultiView::views(std::f32::consts::TAU, false), (0, 1, 0.));
		assert_eq!(MultiView::views(std::f32::consts::FRAC_PI_4 * 0.6, true), (1, 1, 0.));
	}
	#[test]
	fn capture_bases_have_fixed_pivot_scale_and_orthonormal_ring() {
		let elevation = 20f32.to_radians();
		for i in 0..8 {
			let theta = i as f32 * std::f32::consts::FRAC_PI_4;
			let right = Vec3::new(theta.cos(), 0., -theta.sin());
			let up = Vec3::new(
				-elevation.sin() * theta.sin(),
				elevation.cos(),
				-elevation.sin() * theta.cos(),
			);
			assert!(right.dot(up).abs() < 0.000001);
			assert!((right.cross(up).length() - 1.).abs() < 0.000001);
		}
		assert_eq!(MultiView::GPU_BYTES, 29001728);
	}
}
