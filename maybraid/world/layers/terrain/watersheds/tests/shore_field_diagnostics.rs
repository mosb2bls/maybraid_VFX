//! Shoreline field diagnostics: is jaggedness in \(h(x,z)\) or only in the mesh?
//!
//! Run:
//! ```text
//! cargo test -p terrain-watersheds --test shore_field_diagnostics -- --nocapture
//! ```
//! Writes [`PLOT_REL`] next to this crate's `Cargo.toml`.
//!
//! Expected signal (pre-backfill):
//! - [`lake_iso_phi_rings_have_smooth_height`] — **pass**: continuous \(h\) is smooth on
//!   lake iso-\(\phi\) rings (jags are not an authored lake heightfield chatter).
//! - [`overlapping_reaches_per_node_band_vs_union_phi`] — **pass** on this synthetic
//!   crossing (ownership alone is not a slam-dunk for lake jags).
//! - [`reach_end_cap_bed_should_meet_shore`] — **`#[ignore]`**; run with `-- --ignored`
//!   to confirm reach `frame()` depth disagrees with capsule \(\phi\) at end-caps.

use bevy_math::Vec2;
use procedural_common::Bounds2;
use std::f32::consts::TAU;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use terrain_stamps::RegionNoise;
use terrain_watersheds::{
	CorrectionStage, Ellipse, HydroComplex, HydroElevation, HydroFootprint, HydroNode, HydroParams,
	HydroPrimitive, RadialBowl, ReachProfile, ReachSegment,
};

/// SVG written beside `maybraid/world/layers/terrain/watersheds/Cargo.toml`.
const PLOT_REL: &str = "shore_field_diagnostics.svg";

fn plot_path() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PLOT_REL)
}

fn lake_node() -> HydroNode {
	let mut params = HydroParams::default();
	params.rim.width = 25.0;
	params.apron.width = 40.0;
	params.rim.lift = 0.0;
	params.rim.shelf_anchor = Some(42.0);
	params.rim_height = RegionNoise::from_seed(0, 0.02, 0.0);
	params.boundary_noise = None;
	params.shore_blend = 5.0;
	params.rim_apron_blend = 5.0;
	HydroNode::new(
		HydroPrimitive {
			footprint: HydroFootprint::Ellipse(Ellipse {
				center: Vec2::new(100.0, 100.0),
				radii: Vec2::new(60.0, 40.0),
				rotation: 0.35,
			}),
			elevation: HydroElevation::Radial(RadialBowl { surface: 40.0, center_depth: 8.0 }),
			influence_pad: 80.0,
		},
		params,
		80.0,
	)
}

fn reach_node(a: Vec2, b: Vec2, half_width: f32) -> HydroNode {
	let mut params = HydroParams::default();
	params.rim.width = 8.0;
	params.apron.width = 12.0;
	params.rim.lift = 0.0;
	params.rim_height = RegionNoise::from_seed(0, 0.02, 0.0);
	params.boundary_noise = None;
	params.shore_blend = 4.0;
	params.rim_apron_blend = 4.0;
	HydroNode::new(
		HydroPrimitive {
			footprint: HydroFootprint::Reach(ReachSegment { a, b, half_width }),
			elevation: HydroElevation::Reach(ReachProfile {
				surface_a: 30.0,
				surface_b: 30.0,
				center_depth: 4.0,
			}),
			influence_pad: 30.0,
		},
		params,
		30.0,
	)
}

/// Sample a point on iso-\(\phi = target\) along ray `center + t * dir`.
fn iso_phi_on_ray(node: &HydroNode, center: Vec2, dir: Vec2, target: f32) -> anyhow::Result<Vec2> {
	let dir = dir.normalize_or_zero();
	anyhow::ensure!(dir.length_squared() > 0.0, "zero direction");
	let mut lo = 0.0f32;
	let mut hi = 1.0f32;
	// Expand until we bracket target (φ increases outward for these footprints).
	for _ in 0..48 {
		let phi = node.phi(center + dir * hi);
		if phi >= target {
			break;
		}
		hi *= 1.6;
		anyhow::ensure!(hi < 1.0e5, "failed to bracket φ={target} along {dir:?}");
	}
	for _ in 0..56 {
		let mid = 0.5 * (lo + hi);
		let phi = node.phi(center + dir * mid);
		if phi < target {
			lo = mid;
		} else {
			hi = mid;
		}
	}
	Ok(center + dir * (0.5 * (lo + hi)))
}

fn sample_iso_phi_ring(
	node: &HydroNode,
	center: Vec2,
	target: f32,
	n: usize,
) -> anyhow::Result<Vec<Vec2>> {
	let mut out = Vec::with_capacity(n);
	for i in 0..n {
		let ang = TAU * (i as f32) / (n as f32);
		let dir = Vec2::new(ang.cos(), ang.sin());
		out.push(iso_phi_on_ray(node, center, dir, target)?);
	}
	Ok(out)
}

fn ring_heights(node: &HydroNode, ring: &[Vec2], elevation: f32) -> Vec<f32> {
	ring.iter()
		.map(|p| HydroNode::blend_terrain_elevation(&[node], elevation, *p))
		.collect()
}

/// Max |h[i+1]-h[i]| on a closed ring.
fn max_first_diff(h: &[f32]) -> f32 {
	if h.len() < 2 {
		return 0.0;
	}
	let mut m = 0.0f32;
	for i in 0..h.len() {
		let a = h[i];
		let b = h[(i + 1) % h.len()];
		m = m.max((b - a).abs());
	}
	m
}

/// Max |Δ²h| on a closed ring (discrete curvature proxy).
fn max_second_diff(h: &[f32]) -> f32 {
	if h.len() < 3 {
		return 0.0;
	}
	let mut m = 0.0f32;
	let n = h.len();
	for i in 0..n {
		let a = h[(i + n - 1) % n];
		let b = h[i];
		let c = h[(i + 1) % n];
		m = m.max((a - 2.0 * b + c).abs());
	}
	m
}

/// Winning band using each node's own φ (current blend bucketing, simplified).
fn per_node_band(node: &HydroNode, p: Vec2) -> Option<CorrectionStage> {
	node.point_classification(p)
}

/// Band from union φ = min_i φ_i against the first node's widths.
fn union_band(nodes: &[&HydroNode], p: Vec2) -> Option<CorrectionStage> {
	let Some(first) = nodes.first() else {
		return None;
	};
	let mut phi = f32::INFINITY;
	for n in nodes {
		phi = phi.min(n.phi(p));
	}
	if !phi.is_finite() {
		return None;
	}
	let rim_w = first.params.rim.width.max(0.0);
	let apron_w = first.params.apron.width.max(0.0);
	if phi <= 0.0 {
		Some(CorrectionStage::Carve)
	} else if phi < rim_w {
		Some(CorrectionStage::Rim)
	} else if phi < rim_w + apron_w {
		Some(CorrectionStage::Apron)
	} else {
		None
	}
}

fn write_debug_plot(
	lake: &HydroNode,
	shore: &[Vec2],
	rim: &[Vec2],
	h_shore: &[f32],
	h_rim: &[f32],
	tip_xs: &[f32],
	tip_phi: &[f32],
	tip_bed: &[f32],
	tip_h: &[f32],
) -> anyhow::Result<()> {
	// Use rgb(...) fills so we never embed `#` inside Rust raw-string delimiters.
	const BG: &str = "rgb(26,29,35)";
	const PANEL: &str = "rgb(17,20,26)";
	const INK: &str = "rgb(232,230,227)";
	const MUTED: &str = "rgb(154,160,166)";
	const STROKE: &str = "rgb(60,64,72)";
	const SHORE: &str = "rgb(126,231,135)";
	const RIM: &str = "rgb(255,123,114)";
	const PHI: &str = "rgb(121,192,255)";
	const BED: &str = "rgb(255,166,87)";
	const BLEND: &str = "rgb(210,168,255)";

	let path = plot_path();
	let mut f = fs::File::create(&path)?;
	let mut out = String::new();
	let push = |s: &mut String, line: String| {
		s.push_str(&line);
		s.push('\n');
	};

	push(&mut out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>".into());
	push(
		&mut out,
		"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"920\" height=\"640\" viewBox=\"0 0 920 640\">"
			.into(),
	);
	push(&mut out, format!("<rect width=\"100%\" height=\"100%\" fill=\"{BG}\"/>"));
	push(
		&mut out,
		format!(
			"<text x=\"16\" y=\"24\" fill=\"{INK}\" font-family=\"monospace\" font-size=\"14\">Watershed shore field diagnostics</text>"
		),
	);
	push(
		&mut out,
		format!(
			"<text x=\"16\" y=\"44\" fill=\"{MUTED}\" font-family=\"monospace\" font-size=\"11\">Left: lake iso-phi rings colored by h. Right: h(theta). Bottom: reach tip transect.</text>"
		),
	);

	let plan = (16.0f32, 60.0, 420.0, 420.0);
	let center = match &lake.primitive.footprint {
		HydroFootprint::Ellipse(e) => e.center,
		_ => Vec2::ZERO,
	};
	let world_r = 120.0f32;
	let to_svg = |p: Vec2| -> (f32, f32) {
		let u = (p.x - center.x) / world_r;
		let v = (p.y - center.y) / world_r;
		(plan.0 + plan.2 * 0.5 + u * (plan.2 * 0.45), plan.1 + plan.3 * 0.5 + v * (plan.3 * 0.45))
	};
	push(
		&mut out,
		format!(
			"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{PANEL}\" stroke=\"{STROKE}\"/>",
			plan.0, plan.1, plan.2, plan.3
		),
	);

	let h_min = h_shore.iter().chain(h_rim.iter()).cloned().fold(f32::INFINITY, f32::min);
	let h_max = h_shore.iter().chain(h_rim.iter()).cloned().fold(f32::NEG_INFINITY, f32::max);
	let color = |hh: f32| -> String {
		let t = ((hh - h_min) / (h_max - h_min).max(1e-3)).clamp(0.0, 1.0);
		let r = (40.0 + 180.0 * t) as u8;
		let g = (80.0 + 100.0 * (1.0 - t)) as u8;
		let b = (160.0 + 40.0 * t) as u8;
		format!("rgb({r},{g},{b})")
	};

	for iy in 0..64 {
		for ix in 0..64 {
			let u = (ix as f32 + 0.5) / 64.0 * 2.0 - 1.0;
			let v = (iy as f32 + 0.5) / 64.0 * 2.0 - 1.0;
			let p = center + Vec2::new(u, v) * world_r;
			let hh = HydroNode::blend_terrain_elevation(&[lake], 50.0, p);
			let (sx, sy) = to_svg(p);
			let fill = color(hh);
			push(
				&mut out,
				format!(
					"<circle cx=\"{sx:.2}\" cy=\"{sy:.2}\" r=\"2.2\" fill=\"{fill}\" fill-opacity=\"0.85\"/>"
				),
			);
		}
	}

	let mut poly = |pts: &[Vec2], stroke: &str| {
		let mut line =
			format!("<polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\" points=\"");
		for p in pts {
			let (sx, sy) = to_svg(*p);
			line.push_str(&format!("{sx:.2},{sy:.2} "));
		}
		if let Some(p0) = pts.first() {
			let (sx, sy) = to_svg(*p0);
			line.push_str(&format!("{sx:.2},{sy:.2}"));
		}
		line.push_str("\"/>");
		push(&mut out, line);
	};
	poly(shore, SHORE);
	poly(rim, RIM);
	push(
		&mut out,
		format!(
			"<text x=\"{}\" y=\"{}\" fill=\"{SHORE}\" font-family=\"monospace\" font-size=\"11\">phi=0</text>",
			plan.0 + 8.0,
			plan.1 + plan.3 - 20.0
		),
	);
	push(
		&mut out,
		format!(
			"<text x=\"{}\" y=\"{}\" fill=\"{RIM}\" font-family=\"monospace\" font-size=\"11\">phi=rim</text>",
			plan.0 + 8.0,
			plan.1 + plan.3 - 6.0
		),
	);

	let chart = (460.0f32, 60.0, 440.0, 200.0);
	push(
		&mut out,
		format!(
			"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{PANEL}\" stroke=\"{STROKE}\"/>",
			chart.0, chart.1, chart.2, chart.3
		),
	);
	push(
		&mut out,
		format!(
			"<text x=\"{}\" y=\"{}\" fill=\"{INK}\" font-family=\"monospace\" font-size=\"12\">h(theta) on iso-phi rings</text>",
			chart.0 + 8.0,
			chart.1 + 16.0
		),
	);
	let mut plot_series = |vals: &[f32], stroke: &str| {
		if vals.is_empty() {
			return;
		}
		let vmin = vals.iter().cloned().fold(f32::INFINITY, f32::min);
		let vmax = vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
		let mut line =
			format!("<polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\" points=\"");
		for (i, hh) in vals.iter().enumerate() {
			let x = chart.0 + chart.2 * (i as f32) / (vals.len().saturating_sub(1).max(1) as f32);
			let t = (*hh - vmin) / (vmax - vmin).max(1e-3);
			let y = chart.1 + chart.3 - 20.0 - t * (chart.3 - 36.0);
			line.push_str(&format!("{x:.2},{y:.2} "));
		}
		line.push_str("\"/>");
		push(&mut out, line);
	};
	plot_series(h_shore, SHORE);
	plot_series(h_rim, RIM);
	push(
		&mut out,
		format!(
			"<text x=\"{}\" y=\"{}\" fill=\"{MUTED}\" font-family=\"monospace\" font-size=\"10\">d1 shore={:.4}  d2 shore={:.4}  d1 rim={:.4}  d2 rim={:.4}</text>",
			chart.0 + 8.0,
			chart.1 + chart.3 - 8.0,
			max_first_diff(h_shore),
			max_second_diff(h_shore),
			max_first_diff(h_rim),
			max_second_diff(h_rim),
		),
	);

	let tip = (460.0f32, 290.0, 440.0, 320.0);
	push(
		&mut out,
		format!(
			"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{PANEL}\" stroke=\"{STROKE}\"/>",
			tip.0, tip.1, tip.2, tip.3
		),
	);
	push(
		&mut out,
		format!(
			"<text x=\"{}\" y=\"{}\" fill=\"{INK}\" font-family=\"monospace\" font-size=\"12\">Reach tip transect (past endpoint along axis)</text>",
			tip.0 + 8.0,
			tip.1 + 16.0
		),
	);
	let series = [(tip_phi, PHI, "phi"), (tip_bed, BED, "geo bed"), (tip_h, BLEND, "blend h")];
	for (si, (vals, stroke, label)) in series.iter().enumerate() {
		if vals.is_empty() {
			continue;
		}
		let vmin = vals.iter().cloned().fold(f32::INFINITY, f32::min);
		let vmax = vals.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
		let mut line =
			format!("<polyline fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\" points=\"");
		for (i, v) in vals.iter().enumerate() {
			let x = tip.0 + tip.2 * (i as f32) / (vals.len().saturating_sub(1).max(1) as f32);
			let t = (*v - vmin) / (vmax - vmin).max(1e-3);
			let y = tip.1 + 36.0 + (si as f32) * 90.0 + (1.0 - t) * 70.0;
			line.push_str(&format!("{x:.2},{y:.2} "));
		}
		line.push_str("\"/>");
		push(&mut out, line);
		push(
			&mut out,
			format!(
				"<text x=\"{}\" y=\"{}\" fill=\"{stroke}\" font-family=\"monospace\" font-size=\"10\">{label} [{vmin:.2},{vmax:.2}]</text>",
				tip.0 + 8.0,
				tip.1 + 30.0 + (si as f32) * 90.0,
			),
		);
	}
	if let (Some(x0), Some(x1)) = (tip_xs.first(), tip_xs.last()) {
		push(
			&mut out,
			format!(
				"<text x=\"{}\" y=\"{}\" fill=\"{MUTED}\" font-family=\"monospace\" font-size=\"10\">x in [{x0:.1}, {x1:.1}] (endpoint at 0)</text>",
				tip.0 + 8.0,
				tip.1 + tip.3 - 8.0,
			),
		);
	}

	push(&mut out, "</svg>".into());
	f.write_all(out.as_bytes())?;
	eprintln!("wrote shore diagnostics plot: {}", path.display());
	Ok(())
}

#[test]
fn lake_iso_phi_rings_have_smooth_height() -> anyhow::Result<()> {
	let lake = lake_node();
	let center = match &lake.primitive.footprint {
		HydroFootprint::Ellipse(e) => e.center,
		_ => anyhow::bail!("expected ellipse"),
	};
	let n = 256;
	let shore = sample_iso_phi_ring(&lake, center, 0.0, n)?;
	let rim = sample_iso_phi_ring(&lake, center, lake.params.rim.width, n)?;
	let h_shore = ring_heights(&lake, &shore, 50.0);
	let h_rim = ring_heights(&lake, &rim, 50.0);

	// Closed smooth bank/shelf should not chatter along a geometric iso-φ ring.
	let d1_shore = max_first_diff(&h_shore);
	let d2_shore = max_second_diff(&h_shore);
	let d1_rim = max_first_diff(&h_rim);
	let d2_rim = max_second_diff(&h_rim);
	eprintln!(
		"lake ring smoothness: shore Δ1={d1_shore:.5} Δ2={d2_shore:.5} | rim Δ1={d1_rim:.5} Δ2={d2_rim:.5}"
	);

	// Tip transect for the plot (independent of this assert).
	let hw = 8.0;
	let tip_node = reach_node(Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0), hw);
	let mut tip_xs = Vec::new();
	let mut tip_phi = Vec::new();
	let mut tip_bed = Vec::new();
	let mut tip_h = Vec::new();
	for i in 0..120 {
		// From inside channel through endpoint into the end-cap / exterior.
		let x = -hw * 1.5 + (i as f32) * (hw * 3.0) / 119.0;
		let p = Vec2::new(x, 0.0);
		tip_xs.push(x);
		tip_phi.push(tip_node.phi(p));
		tip_bed.push(tip_node.bed_level(p));
		tip_h.push(HydroNode::blend_terrain_elevation(&[&tip_node], 40.0, p));
	}
	write_debug_plot(&lake, &shore, &rim, &h_shore, &h_rim, &tip_xs, &tip_phi, &tip_bed, &tip_h)?;

	// With flat bank (no rim height / boundary noise), consecutive samples on φ=0
	// should be nearly constant. Large Δ ⇒ jaggedness is in the heightfield.
	anyhow::ensure!(
		d1_shore < 0.05,
		"shore ring h chatters in XZ (max |Δh|={d1_shore}); jags are in the authored field"
	);
	anyhow::ensure!(d2_shore < 0.08, "shore ring h has high curvature (max |Δ²h|={d2_shore})");
	anyhow::ensure!(d1_rim < 0.08, "rim ring h chatters in XZ (max |Δh|={d1_rim})");
	Ok(())
}

#[test]
#[ignore = "documents known reach frame() vs capsule φ end-cap mismatch; run with -- --ignored"]
fn reach_end_cap_bed_should_meet_shore() -> anyhow::Result<()> {
	let hw = 8.0;
	let node = reach_node(Vec2::new(0.0, 0.0), Vec2::new(40.0, 0.0), hw);
	// On-axis past the start endpoint, still inside the circular cap (φ < 0).
	let p_inside_cap = Vec2::new(-hw * 0.5, 0.0);
	let phi = node.phi(p_inside_cap);
	anyhow::ensure!(phi < 0.0, "expected inside cap, φ={phi}");
	let geo_bed = node.bed_level(p_inside_cap);
	let w = node.surface_level(p_inside_cap);
	let depth = w - geo_bed;
	// Capsule SDF: halfway from tip to shore (φ ≈ -hw/2). A distance-consistent
	// bowl would be shallower than center; frame() still reports xn=0 → full depth.
	eprintln!("end-cap on-axis: φ={phi:.4} depth={depth:.4} (W={w:.2} bed={geo_bed:.2})");
	let center_depth = 4.0;
	anyhow::ensure!(
		depth < 0.55 * center_depth,
		"end-cap on-axis still has near-full bowl depth {depth} (center={center_depth}) while φ={phi} — frame vs SDF mismatch"
	);

	// On the geometric shore (leftmost point of the end-cap circle).
	let p_shore = Vec2::new(-hw, 0.0);
	let phi_s = node.phi(p_shore);
	anyhow::ensure!(phi_s.abs() < 0.05, "expected φ≈0 at end-cap shore, got {phi_s}");
	let depth_s = node.surface_level(p_shore) - node.bed_level(p_shore);
	eprintln!("end-cap shore: φ={phi_s:.4} depth={depth_s:.4}");
	anyhow::ensure!(depth_s < 0.25, "bed depth should vanish on φ=0 end-cap shore, got {depth_s}");
	Ok(())
}

#[test]
fn overlapping_reaches_per_node_band_vs_union_phi() -> anyhow::Result<()> {
	// Two overlapping corridors — classic multi-node ownership case.
	let a = reach_node(Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0), 6.0);
	let b = reach_node(Vec2::new(25.0, -8.0), Vec2::new(25.0, 30.0), 6.0);
	let nodes = [&a, &b];
	let complex = HydroComplex::new(Bounds2::from_xz(-20.0, -20.0, 70.0, 50.0), 1)
		.with_hydro(vec![a.clone(), b.clone()]);

	let mut mismatches = 0usize;
	let mut samples = 0usize;
	for iz in 0..40 {
		for ix in 0..40 {
			let p = Vec2::new(-10.0 + ix as f32 * 2.0, -10.0 + iz as f32 * 1.5);
			if complex.min_phi_at(p.x, p.y).is_none() && a.phi(p) > 40.0 && b.phi(p) > 40.0 {
				continue;
			}
			samples += 1;
			// Current terrain path: classify each node, then priority in blend.
			// Approximate "current" wet ownership: any node with φ<=0 is carve.
			let any_carve = nodes.iter().any(|n| n.phi(p) <= 0.0);
			let union = union_band(&nodes, p);
			let union_carve = matches!(union, Some(CorrectionStage::Carve));
			if any_carve != union_carve {
				mismatches += 1;
			}
			// Also compare hard point_classification of the nearest node vs union.
			let nearest = nodes
				.iter()
				.min_by(|x, y| x.phi(p).partial_cmp(&y.phi(p)).unwrap_or(std::cmp::Ordering::Equal))
				.copied();
			if let Some(n) = nearest {
				let per = per_node_band(n, p);
				if per != union {
					// Count band mismatches at the nearest-node vs union-φ.
					mismatches += 1;
				}
			}
		}
	}
	eprintln!("union-φ vs per-node: samples={samples} mismatch_events={mismatches}");
	// This is diagnostic: overlapping geometry often disagrees. Fail if a large
	// fraction of the support sees ownership/band disagreement.
	let frac = mismatches as f32 / samples.max(1) as f32;
	anyhow::ensure!(
		frac < 0.08,
		"per-node banding diverges from union phi on {:.1}% of samples ({}/{})",
		frac * 100.0,
		mismatches,
		samples
	);
	Ok(())
}
