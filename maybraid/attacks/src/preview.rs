//! Top-down SVG of placeholder telegraph and hazard poses.

use bevy::prelude::Vec3;

use crate::{HazardPose, HazardShape};

/// Draw a telegraph (gold) and hazard (red) in a simple plan view.
pub fn placeholder_svg(
	telegraph: (HazardPose, HazardShape),
	hazard: (HazardPose, HazardShape),
) -> String {
	let mut marks = String::new();
	append_shape(&mut marks, telegraph.0, telegraph.1, "#f0c12a", 0.45);
	append_shape(&mut marks, hazard.0, hazard.1, "#c42b24", 0.55);
	format!(
		concat!(
			"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"-8 -8 16 16\" ",
			"width=\"640\" height=\"640\" role=\"img\">",
			"<rect x=\"-8\" y=\"-8\" width=\"16\" height=\"16\" fill=\"#16181d\"/>",
			"<g transform=\"scale(1 -1)\">",
			"<circle r=\"0.12\" fill=\"#d7dde7\"/>",
			"{marks}",
			"</g></svg>\n"
		),
		marks = marks
	)
}

fn append_shape(out: &mut String, pose: HazardPose, shape: HazardShape, color: &str, opacity: f32) {
	let x = pose.origin.x;
	let z = pose.origin.z;
	match shape {
		HazardShape::Sphere { radius }
		| HazardShape::Wavefront { radius, .. }
		| HazardShape::Cylinder { radius, .. } => {
			out.push_str(&format!(
				"<circle cx=\"{x:.3}\" cy=\"{z:.3}\" r=\"{radius:.3}\" fill=\"{color}\" fill-opacity=\"{opacity}\"/>"
			));
		}
		HazardShape::Box { half_extents } => {
			out.push_str(&format!(
				"<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"{color}\" fill-opacity=\"{opacity}\"/>",
				x - half_extents.x,
				z - half_extents.z,
				half_extents.x * 2.0,
				half_extents.z * 2.0
			));
		}
		HazardShape::Line { length, half_width } => {
			let end = pose.origin + pose.rotation * Vec3::Z * length;
			out.push_str(&format!(
				"<line x1=\"{x:.3}\" y1=\"{z:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"{color}\" stroke-opacity=\"{opacity}\" stroke-width=\"{:.3}\" stroke-linecap=\"round\"/>",
				end.x,
				end.z,
				half_width.max(0.08) * 2.0
			));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AimSnapshot, HazardOrigin, HazardSpec};
	use bevy::prelude::Quat;

	#[test]
	fn placeholder_svg_includes_telegraph_and_hazard() {
		let snapshot = AimSnapshot { origin: Vec3::ZERO, direction: Vec3::Z, target: None };
		let telegraph = (
			HazardPose::new(Vec3::ZERO, Quat::IDENTITY),
			HazardShape::Line { length: 8.0, half_width: 0.4 },
		);
		let hazard = HazardSpec {
			shape: HazardShape::Wavefront { radius: 1.2, speed: 4.0, length: 8.0 },
			origin: HazardOrigin::AimPoint,
			damage: crate::DamageSpec::authored(),
		}
		.pose(snapshot, 0.5);
		let svg = placeholder_svg(
			telegraph,
			(hazard, HazardShape::Wavefront { radius: 1.2, speed: 4.0, length: 8.0 }),
		);
		assert!(svg.contains("stroke=\"#f0c12a\""));
		assert!(svg.contains("fill=\"#c42b24\""));
		assert!(svg.contains("<svg"));
	}
}
