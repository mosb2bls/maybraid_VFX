//! GPU device selection for `mistral.rs`.

use std::env;

use mistralrs::Device;

use crate::error::MistralLanguageError;

/// Darwin releases whose Metal compiler rejects Candle 0.11 `mlx_gemm.metal`.
///
/// GPUCompiler 32023+ requires `thread simdgroup_matrix`. Candle 0.11 (via
/// mistral.rs 0.9.4) still declares unqualified `simdgroup_matrix` members, so
/// Metal library load fails. Skip the GPU unless the operator forces it.
const METAL_GEMM_BROKEN_DARWIN_MAJOR: u32 = 25;

/// Where the loaded GGUF runs.
pub struct InferenceDevice;

impl InferenceDevice {
	/// Metal when that backend is compiled and usable, otherwise CUDA if present, else CPU.
	pub fn select(force_cpu: bool) -> Result<Device, MistralLanguageError> {
		let force_cpu = force_cpu || Self::must_use_cpu();
		mistralrs::best_device(force_cpu).map_err(|error| {
			MistralLanguageError::load("inference device", format!("failed to open GPU: {error}"))
		})
	}

	pub fn is_gpu(device: &Device) -> bool {
		!device.is_cpu()
	}

	/// Recent macOS Metal compilers cannot JIT Candle 0.11 GEMM until Candle
	/// qualifies `simdgroup_matrix` as `thread`. Override with
	/// `MAYBRAID_LANGUAGE_FORCE_METAL=1`.
	pub fn must_use_cpu() -> bool {
		if force_metal_from_env() {
			return false;
		}
		metal_gemm_incompatible()
	}
}

fn force_metal_from_env() -> bool {
	matches!(env::var("MAYBRAID_LANGUAGE_FORCE_METAL").as_deref(), Ok("1" | "true" | "TRUE"))
}

fn metal_gemm_incompatible() -> bool {
	cfg!(target_os = "macos")
		&& darwin_major().is_some_and(|major| major >= METAL_GEMM_BROKEN_DARWIN_MAJOR)
}

fn darwin_major() -> Option<u32> {
	darwin_major_from(env::consts::OS, optional_uname_release().as_deref())
}

fn optional_uname_release() -> Option<String> {
	let output = std::process::Command::new("uname").arg("-r").output().ok()?;
	if !output.status.success() {
		return None;
	}
	String::from_utf8(output.stdout).ok()
}

fn darwin_major_from(os: &str, release: Option<&str>) -> Option<u32> {
	if os != "macos" {
		return None;
	}
	release?.split('.').next()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
	use super::{darwin_major_from, METAL_GEMM_BROKEN_DARWIN_MAJOR};

	#[test]
	fn tahoe_and_newer_darwin_are_incompatible() {
		assert_eq!(darwin_major_from("macos", Some("27.0.0")), Some(27));
		assert!(darwin_major_from("macos", Some("27.0.0"))
			.is_some_and(|major| { major >= METAL_GEMM_BROKEN_DARWIN_MAJOR }));
		assert!(darwin_major_from("macos", Some("24.6.0"))
			.is_some_and(|major| { major < METAL_GEMM_BROKEN_DARWIN_MAJOR }));
		assert_eq!(darwin_major_from("linux", Some("27.0.0")), None);
	}
}
