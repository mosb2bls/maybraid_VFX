//! First-run Visual Style default from cheap host signals.
//!
//! # Heuristic
//!
//! Applied only when the player has no saved `visual_style` (first run, or a
//! migrated legacy `render_scale: auto`). Explicit saved choices are never
//! overridden. [`MAYBRAID_VISUAL_STYLE`](super::VISUAL_STYLE_ENV) still wins
//! for the live 3D path.
//!
//! 1. [`DEVICE_TIER_ENV`] `low` → Sharp, `high` → Native.
//! 2. Software / CPU adapters (`lavapipe`, `llvmpipe`, SwiftShader, …) → Sharp.
//! 3. Integrated / unified-memory GPUs (including Apple Silicon) with **total
//!    system RAM below 24 GiB** → Sharp. 24 GiB or more does not force Native;
//!    other signals still apply.
//! 4. Apple M-series **base / Air-class** (no Pro / Max / Ultra) → Sharp.
//!    Apple M-series **Pro / Max / Ultra** → Native.
//! 5. Discrete GPUs → Native (dedicated VRAM; system RAM is not a class signal).
//! 6. Integrated / virtual / unknown GPUs **above 1080p** → Sharp.
//! 7. Those same weaker GPUs with few CPU cores (≤4) → Sharp.
//! 8. Otherwise Native.
//!
//! # Limits
//!
//! - Adapter names vary by backend and driver. A new Apple chip that is not
//!   `M` + digits, or a discrete GPU reported as integrated, will misclassify.
//! - Base M-series in a MacBook Pro (same GPU name as Air) is treated as weak.
//! - Unified-memory class uses **total system RAM**, not GPU VRAM. Firmware can
//!   reserve a slice, so a marketed 24 GB machine may report slightly less and
//!   pick Sharp.
//! - We do not measure thermals, battery, or available VRAM.
//! - Window size is the primary framebuffer at pick time; later resize is ignored.
//! - CPU cores are optional and ignored when the adapter is already strong.

use bevy::prelude::*;

use super::visual_style::VisualStyle;

/// `MAYBRAID_DEVICE_TIER=low|high` forces the first-run default.
pub const DEVICE_TIER_ENV: &str = "MAYBRAID_DEVICE_TIER";

const ABOVE_1080P_PIXELS: u32 = 1920 * 1080;
const LOW_CPU_CORES: u32 = 4;
/// Integrated / unified-memory hosts below this total system RAM default to Sharp.
pub const UNIFIED_MEMORY_NATIVE_BYTES: u64 = 24 * 1024 * 1024 * 1024;

/// Forced host tier from [`DEVICE_TIER_ENV`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceTier {
	Low,
	High,
}

/// wgpu-style adapter kind, copied so the picker stays free of wgpu types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterKind {
	DiscreteGpu,
	IntegratedGpu,
	VirtualGpu,
	Cpu,
	Other,
}

/// Cheap signals collected once at startup.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VisualStyleSignals {
	pub adapter_kind: AdapterKind,
	pub name: String,
	pub driver: String,
	pub driver_info: String,
	pub window: UVec2,
	pub cpu_cores: Option<u32>,
	pub memory_bytes: Option<u64>,
	pub device_tier: Option<DeviceTier>,
}

impl Default for AdapterKind {
	fn default() -> Self {
		Self::Other
	}
}

impl AdapterKind {
	/// Map wgpu / Bevy `DeviceType` debug names (`DiscreteGpu`, `IntegratedGpu`, …).
	pub fn from_wgpu_device_type_name(name: &str) -> Self {
		match name {
			"DiscreteGpu" => Self::DiscreteGpu,
			"IntegratedGpu" => Self::IntegratedGpu,
			"VirtualGpu" => Self::VirtualGpu,
			"Cpu" => Self::Cpu,
			_ => Self::Other,
		}
	}
}

/// Why a first-run style was chosen. Copy so the settings readout can store it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualStyleAutoReason {
	DeviceTierLow,
	DeviceTierHigh,
	SoftwareRenderer,
	AppleBase,
	ApplePro,
	DiscreteGpu,
	UnifiedMemoryBelow24GiB,
	IntegratedAbove1080p,
	LowCpuOrMemory,
	DefaultNative,
}

impl VisualStyleAutoReason {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::DeviceTierLow => "MAYBRAID_DEVICE_TIER=low",
			Self::DeviceTierHigh => "MAYBRAID_DEVICE_TIER=high",
			Self::SoftwareRenderer => "software renderer",
			Self::AppleBase => "Apple M-series base/Air",
			Self::ApplePro => "Apple M-series Pro/Max/Ultra",
			Self::DiscreteGpu => "discrete GPU",
			Self::UnifiedMemoryBelow24GiB => "unified memory below 24 GB",
			Self::IntegratedAbove1080p => "integrated GPU above 1080p",
			Self::LowCpuOrMemory => "low CPU cores",
			Self::DefaultNative => "default Native",
		}
	}
}

/// Picked first-run style plus a stable reason for logs and the settings readout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualStylePick {
	pub style: VisualStyle,
	pub reason: VisualStyleAutoReason,
}

/// Parse [`DEVICE_TIER_ENV`]. Empty / unknown → `None`.
pub fn parse_device_tier_env(raw: Option<&str>) -> Option<DeviceTier> {
	let Some(raw) = raw else {
		return None;
	};
	let raw = raw.trim();
	if raw.is_empty() {
		return None;
	}
	match raw.to_ascii_lowercase().as_str() {
		"low" | "weak" => Some(DeviceTier::Low),
		"high" | "strong" => Some(DeviceTier::High),
		other => {
			eprintln!("[{DEVICE_TIER_ENV}] unknown {other:?} (use low or high)");
			None
		}
	}
}

/// Whether the window is larger than 1080p (more pixels, or taller than 1080).
pub fn window_above_1080p(window: UVec2) -> bool {
	if window.x == 0 || window.y == 0 {
		return false;
	}
	window.y > 1080 || window.x.saturating_mul(window.y) > ABOVE_1080P_PIXELS
}

/// Pick Native or Sharp from [`VisualStyleSignals`]. Pure; no I/O.
pub fn pick_default_visual_style(signals: &VisualStyleSignals) -> VisualStylePick {
	if let Some(tier) = signals.device_tier {
		return match tier {
			DeviceTier::Low => pick(VisualStyle::Sharp, VisualStyleAutoReason::DeviceTierLow),
			DeviceTier::High => pick(VisualStyle::Native, VisualStyleAutoReason::DeviceTierHigh),
		};
	}

	if is_software_renderer(signals) {
		return pick(VisualStyle::Sharp, VisualStyleAutoReason::SoftwareRenderer);
	}

	if is_unified_memory_host(signals) && below_unified_memory_native(signals) {
		return pick(VisualStyle::Sharp, VisualStyleAutoReason::UnifiedMemoryBelow24GiB);
	}

	if is_apple_m_series(&signals.name) {
		return if apple_m_is_pro_class(&signals.name) {
			pick(VisualStyle::Native, VisualStyleAutoReason::ApplePro)
		} else {
			pick(VisualStyle::Sharp, VisualStyleAutoReason::AppleBase)
		};
	}

	if signals.adapter_kind == AdapterKind::DiscreteGpu {
		return pick(VisualStyle::Native, VisualStyleAutoReason::DiscreteGpu);
	}

	if is_weaker_adapter(signals.adapter_kind) && window_above_1080p(signals.window) {
		return pick(VisualStyle::Sharp, VisualStyleAutoReason::IntegratedAbove1080p);
	}

	if is_weaker_adapter(signals.adapter_kind) && is_low_cpu(signals) {
		return pick(VisualStyle::Sharp, VisualStyleAutoReason::LowCpuOrMemory);
	}

	pick(VisualStyle::Native, VisualStyleAutoReason::DefaultNative)
}

fn pick(style: VisualStyle, reason: VisualStyleAutoReason) -> VisualStylePick {
	VisualStylePick { style, reason }
}

fn is_weaker_adapter(kind: AdapterKind) -> bool {
	matches!(
		kind,
		AdapterKind::IntegratedGpu
			| AdapterKind::VirtualGpu
			| AdapterKind::Other
			| AdapterKind::Cpu
	)
}

fn is_unified_memory_host(signals: &VisualStyleSignals) -> bool {
	is_apple_m_series(&signals.name) || is_weaker_adapter(signals.adapter_kind)
}

fn below_unified_memory_native(signals: &VisualStyleSignals) -> bool {
	signals
		.memory_bytes
		.is_some_and(|bytes| bytes > 0 && bytes < UNIFIED_MEMORY_NATIVE_BYTES)
}

fn is_low_cpu(signals: &VisualStyleSignals) -> bool {
	signals.cpu_cores.is_some_and(|cores| cores > 0 && cores <= LOW_CPU_CORES)
}

/// Total system RAM in bytes from a cheap OS query, when one succeeds.
///
/// Linux reads `/proc/meminfo` `MemTotal`. macOS runs `sysctl -n hw.memsize`.
pub fn host_memory_bytes() -> Option<u64> {
	#[cfg(target_os = "linux")]
	{
		return parse_meminfo_total_bytes(&std::fs::read_to_string("/proc/meminfo").ok()?);
	}
	#[cfg(target_os = "macos")]
	{
		return sysctl_hw_memsize();
	}
	#[cfg(not(any(target_os = "linux", target_os = "macos")))]
	{
		None
	}
}

/// Parse Linux `/proc/meminfo` `MemTotal` into bytes.
pub fn parse_meminfo_total_bytes(meminfo: &str) -> Option<u64> {
	for line in meminfo.lines() {
		let Some(rest) = line.trim().strip_prefix("MemTotal:") else {
			continue;
		};
		let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
		return Some(kib.saturating_mul(1024));
	}
	None
}

#[cfg(target_os = "macos")]
fn sysctl_hw_memsize() -> Option<u64> {
	let output = std::process::Command::new("sysctl").args(["-n", "hw.memsize"]).output().ok()?;
	if !output.status.success() {
		return None;
	}
	String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

fn is_software_renderer(signals: &VisualStyleSignals) -> bool {
	if signals.adapter_kind == AdapterKind::Cpu {
		return true;
	}
	haystack(signals).contains("lavapipe")
		|| haystack(signals).contains("llvmpipe")
		|| haystack(signals).contains("swiftshader")
		|| haystack(signals).contains("softpipe")
		|| haystack(signals).contains("microsoft basic render")
}

fn haystack(signals: &VisualStyleSignals) -> String {
	format!("{} {} {}", signals.name, signals.driver, signals.driver_info).to_ascii_lowercase()
}

fn tokens(name: &str) -> impl Iterator<Item = &str> {
	name.split(|c: char| !c.is_ascii_alphanumeric())
		.filter(|token| !token.is_empty())
}

fn is_apple_m_series(name: &str) -> bool {
	let lower = name.to_ascii_lowercase();
	lower.contains("apple") && tokens(&lower).any(is_m_series_token)
}

fn is_m_series_token(token: &str) -> bool {
	let token = token.as_bytes();
	token.len() >= 2
		&& token[0].eq_ignore_ascii_case(&b'm')
		&& token[1..].iter().all(|b| b.is_ascii_digit())
}

fn apple_m_is_pro_class(name: &str) -> bool {
	tokens(name).any(|token| {
		token.eq_ignore_ascii_case("pro")
			|| token.eq_ignore_ascii_case("max")
			|| token.eq_ignore_ascii_case("ultra")
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn signals(kind: AdapterKind, name: &str, window: UVec2) -> VisualStyleSignals {
		VisualStyleSignals {
			adapter_kind: kind,
			name: name.into(),
			window,
			cpu_cores: Some(8),
			memory_bytes: Some(32 * 1024 * 1024 * 1024),
			..VisualStyleSignals::default()
		}
	}

	fn pick_eq(signals: &VisualStyleSignals, style: VisualStyle, reason: VisualStyleAutoReason) {
		assert_eq!(pick_default_visual_style(signals), VisualStylePick { style, reason });
	}

	#[test]
	fn device_tier_env_wins() {
		let mut discrete =
			signals(AdapterKind::DiscreteGpu, "NVIDIA GeForce RTX 4080", UVec2::new(3840, 2160));
		discrete.device_tier = Some(DeviceTier::Low);
		pick_eq(&discrete, VisualStyle::Sharp, VisualStyleAutoReason::DeviceTierLow);

		let mut pipe = signals(AdapterKind::Cpu, "llvmpipe", UVec2::new(1920, 1080));
		pipe.device_tier = Some(DeviceTier::High);
		pick_eq(&pipe, VisualStyle::Native, VisualStyleAutoReason::DeviceTierHigh);
	}

	#[test]
	fn parse_device_tier_env_accepts_aliases() {
		assert_eq!(parse_device_tier_env(None), None);
		assert_eq!(parse_device_tier_env(Some("")), None);
		assert_eq!(parse_device_tier_env(Some("low")), Some(DeviceTier::Low));
		assert_eq!(parse_device_tier_env(Some("WEAK")), Some(DeviceTier::Low));
		assert_eq!(parse_device_tier_env(Some("high")), Some(DeviceTier::High));
		assert_eq!(parse_device_tier_env(Some("strong")), Some(DeviceTier::High));
		assert_eq!(parse_device_tier_env(Some("nope")), None);
	}

	#[test]
	fn software_renderers_are_sharp() {
		for (kind, name, driver) in [
			(AdapterKind::Cpu, "llvmpipe (LLVM 15.0.7)", ""),
			(AdapterKind::Other, "lavapipe", "llvmpipe"),
			(AdapterKind::IntegratedGpu, "Google SwiftShader", ""),
			(AdapterKind::DiscreteGpu, "Microsoft Basic Render Driver", ""),
		] {
			let mut s = signals(kind, name, UVec2::new(1920, 1080));
			s.driver = driver.into();
			pick_eq(&s, VisualStyle::Sharp, VisualStyleAutoReason::SoftwareRenderer);
		}
	}

	#[test]
	fn apple_base_and_air_are_sharp() {
		for name in ["Apple M1", "Apple M2", "Apple M3", "Apple M4", "Apple M2 GPU"] {
			pick_eq(
				&signals(AdapterKind::IntegratedGpu, name, UVec2::new(2560, 1664)),
				VisualStyle::Sharp,
				VisualStyleAutoReason::AppleBase,
			);
		}
	}

	#[test]
	fn apple_pro_max_ultra_are_native() {
		for name in
			["Apple M1 Pro", "Apple M1 Max", "Apple M1 Ultra", "Apple M3 Pro", "Apple M4 Max"]
		{
			pick_eq(
				&signals(AdapterKind::IntegratedGpu, name, UVec2::new(3456, 2234)),
				VisualStyle::Native,
				VisualStyleAutoReason::ApplePro,
			);
		}
	}

	#[test]
	fn discrete_stays_native_at_4k() {
		pick_eq(
			&signals(AdapterKind::DiscreteGpu, "AMD Radeon RX 7900 XT", UVec2::new(3840, 2160)),
			VisualStyle::Native,
			VisualStyleAutoReason::DiscreteGpu,
		);
	}

	#[test]
	fn integrated_above_1080p_is_sharp() {
		pick_eq(
			&signals(AdapterKind::IntegratedGpu, "Intel Iris Xe Graphics", UVec2::new(2560, 1440)),
			VisualStyle::Sharp,
			VisualStyleAutoReason::IntegratedAbove1080p,
		);
	}

	#[test]
	fn integrated_1080p_with_enough_cpu_is_native() {
		pick_eq(
			&signals(AdapterKind::IntegratedGpu, "Intel UHD Graphics", UVec2::new(1920, 1080)),
			VisualStyle::Native,
			VisualStyleAutoReason::DefaultNative,
		);
	}

	#[test]
	fn integrated_1080p_with_few_cores_is_sharp() {
		let mut s =
			signals(AdapterKind::IntegratedGpu, "AMD Radeon Graphics", UVec2::new(1920, 1080));
		s.cpu_cores = Some(4);
		pick_eq(&s, VisualStyle::Sharp, VisualStyleAutoReason::LowCpuOrMemory);
	}

	#[test]
	fn unified_memory_16_24_32_gib_boundaries() {
		let window_1080 = UVec2::new(1920, 1080);
		let mut intel = signals(AdapterKind::IntegratedGpu, "Intel UHD Graphics", window_1080);
		intel.memory_bytes = Some(16 * 1024 * 1024 * 1024);
		pick_eq(&intel, VisualStyle::Sharp, VisualStyleAutoReason::UnifiedMemoryBelow24GiB);

		intel.memory_bytes = Some(UNIFIED_MEMORY_NATIVE_BYTES);
		pick_eq(&intel, VisualStyle::Native, VisualStyleAutoReason::DefaultNative);

		intel.memory_bytes = Some(32 * 1024 * 1024 * 1024);
		pick_eq(&intel, VisualStyle::Native, VisualStyleAutoReason::DefaultNative);

		let mut apple_pro =
			signals(AdapterKind::IntegratedGpu, "Apple M1 Pro", UVec2::new(3456, 2234));
		apple_pro.memory_bytes = Some(16 * 1024 * 1024 * 1024);
		pick_eq(&apple_pro, VisualStyle::Sharp, VisualStyleAutoReason::UnifiedMemoryBelow24GiB);

		apple_pro.memory_bytes = Some(UNIFIED_MEMORY_NATIVE_BYTES);
		pick_eq(&apple_pro, VisualStyle::Native, VisualStyleAutoReason::ApplePro);

		apple_pro.memory_bytes = Some(32 * 1024 * 1024 * 1024);
		pick_eq(&apple_pro, VisualStyle::Native, VisualStyleAutoReason::ApplePro);

		let mut apple_base =
			signals(AdapterKind::IntegratedGpu, "Apple M2", UVec2::new(2560, 1664));
		apple_base.memory_bytes = Some(32 * 1024 * 1024 * 1024);
		pick_eq(&apple_base, VisualStyle::Sharp, VisualStyleAutoReason::AppleBase);

		let mut discrete =
			signals(AdapterKind::DiscreteGpu, "NVIDIA GeForce RTX 4070", UVec2::new(3840, 2160));
		discrete.memory_bytes = Some(16 * 1024 * 1024 * 1024);
		pick_eq(&discrete, VisualStyle::Native, VisualStyleAutoReason::DiscreteGpu);
	}

	#[test]
	fn parse_meminfo_total_bytes_reads_memtotal() {
		let meminfo = "MemTotal:       16777216 kB\nMemFree:         1024 kB\n";
		assert_eq!(parse_meminfo_total_bytes(meminfo), Some(16 * 1024 * 1024 * 1024));
		assert_eq!(parse_meminfo_total_bytes("MemFree: 1 kB\n"), None);
		assert_eq!(parse_meminfo_total_bytes(""), None);
	}

	#[test]
	fn window_above_1080p_matches_spec() {
		assert!(!window_above_1080p(UVec2::ZERO));
		assert!(!window_above_1080p(UVec2::new(1920, 1080)));
		assert!(!window_above_1080p(UVec2::new(1280, 720)));
		assert!(window_above_1080p(UVec2::new(2560, 1440)));
		assert!(window_above_1080p(UVec2::new(2560, 1080)));
		assert!(window_above_1080p(UVec2::new(1920, 1200)));
	}

	#[test]
	fn unknown_host_defaults_to_native() {
		pick_eq(
			&signals(AdapterKind::Other, "", UVec2::new(1920, 1080)),
			VisualStyle::Native,
			VisualStyleAutoReason::DefaultNative,
		);
	}

	#[test]
	fn wgpu_device_type_names_map() {
		assert_eq!(
			AdapterKind::from_wgpu_device_type_name("DiscreteGpu"),
			AdapterKind::DiscreteGpu
		);
		assert_eq!(
			AdapterKind::from_wgpu_device_type_name("IntegratedGpu"),
			AdapterKind::IntegratedGpu
		);
		assert_eq!(AdapterKind::from_wgpu_device_type_name("VirtualGpu"), AdapterKind::VirtualGpu);
		assert_eq!(AdapterKind::from_wgpu_device_type_name("Cpu"), AdapterKind::Cpu);
		assert_eq!(AdapterKind::from_wgpu_device_type_name("Other"), AdapterKind::Other);
		assert_eq!(AdapterKind::from_wgpu_device_type_name(""), AdapterKind::Other);
	}

	#[test]
	fn reasons_are_nonempty() {
		for reason in [
			VisualStyleAutoReason::DeviceTierLow,
			VisualStyleAutoReason::DeviceTierHigh,
			VisualStyleAutoReason::SoftwareRenderer,
			VisualStyleAutoReason::AppleBase,
			VisualStyleAutoReason::ApplePro,
			VisualStyleAutoReason::DiscreteGpu,
			VisualStyleAutoReason::UnifiedMemoryBelow24GiB,
			VisualStyleAutoReason::IntegratedAbove1080p,
			VisualStyleAutoReason::LowCpuOrMemory,
			VisualStyleAutoReason::DefaultNative,
		] {
			assert!(!reason.as_str().is_empty());
		}
	}
}
