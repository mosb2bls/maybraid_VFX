//! PNG encode / decode helpers used by the disk cache.

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use std::io::Write;

use crate::error::CaptureError;
use crate::request::CaptureResolution;

/// Transparent RGBA placeholder the caller holds until capture or cache load.
pub fn placeholder_image(resolution: CaptureResolution) -> Image {
	let len = (resolution.width as usize)
		.saturating_mul(resolution.height as usize)
		.saturating_mul(4);
	Image::new(
		Extent3d {
			width: resolution.width.max(1),
			height: resolution.height.max(1),
			depth_or_array_layers: 1,
		},
		TextureDimension::D2,
		vec![0; len.max(4)],
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::default(),
	)
}

/// GPU render target used only by the isolated capture camera.
pub fn render_target_image(resolution: CaptureResolution) -> Image {
	let mut image = Image::new_uninit(
		Extent3d {
			width: resolution.width.max(1),
			height: resolution.height.max(1),
			depth_or_array_layers: 1,
		},
		TextureDimension::D2,
		TextureFormat::Bgra8UnormSrgb,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
		| TextureUsages::COPY_SRC
		| TextureUsages::COPY_DST
		| TextureUsages::RENDER_ATTACHMENT;
	image
}

/// Encode tightly packed 8-bit RGBA into a PNG.
pub fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
	let expected = (width as usize)
		.checked_mul(height as usize)
		.and_then(|pixels| pixels.checked_mul(4))
		.ok_or_else(|| String::from("png dimensions overflow"))?;
	if rgba.len() != expected {
		return Err(format!("rgba len {} != {expected}", rgba.len()));
	}
	let mut out = Vec::new();
	{
		let mut encoder = png::Encoder::new(&mut out, width, height);
		encoder.set_color(png::ColorType::Rgba);
		encoder.set_depth(png::BitDepth::Eight);
		let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
		writer.write_image_data(rgba).map_err(|error| error.to_string())?;
		writer.finish().map_err(|error| error.to_string())?;
	}
	Ok(out)
}

/// Decode a PNG into a Bevy image (sRGB, sample-default).
pub fn image_from_png(bytes: &[u8]) -> Result<Image, CaptureError> {
	Image::from_buffer(
		bytes,
		ImageType::Extension("png"),
		CompressedImageFormats::NONE,
		true,
		ImageSampler::Default,
		RenderAssetUsages::default(),
	)
	.map_err(|_| CaptureError::Readback)
}

/// BGRA (optionally row-padded) → tightly packed RGBA.
pub fn bgra_to_rgba(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
	if width == 0 || height == 0 {
		return Err(String::from("empty capture"));
	}
	let row_bytes = (width as usize).saturating_mul(4);
	if data.len() < row_bytes.saturating_mul(height as usize) {
		return Err(format!(
			"readback too small: {} < {}",
			data.len(),
			row_bytes * height as usize
		));
	}
	let stride = data.len() / height as usize;
	if stride < row_bytes {
		return Err(String::from("readback stride shorter than a row"));
	}
	let mut rgba = Vec::with_capacity(row_bytes * height as usize);
	for y in 0..height as usize {
		let start = y * stride;
		let row = &data[start..start + row_bytes];
		for pixel in row.chunks_exact(4) {
			rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
		}
	}
	Ok(rgba)
}

pub fn write_all_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
	}
	let tmp = path.with_extension(format!("png.tmp-{}", std::process::id()));
	(|| {
		let mut file = std::fs::File::create(&tmp).map_err(|error| error.to_string())?;
		file.write_all(bytes).map_err(|error| error.to_string())?;
		file.sync_all().map_err(|error| error.to_string())?;
		std::fs::rename(&tmp, path).map_err(|error| error.to_string())?;
		Ok(())
	})()
	.inspect_err(|_| {
		let _ = std::fs::remove_file(&tmp);
	})
}
