//! Opening plan helpers and construct-time mapped contact geometry.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::arc_kit::arc_ring_dir_deg;

use crate::openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId,
	OpeningLabel, Openings,
};

use super::ring::{aabb3d_intersects, ring_dir_at, EPS, SECTORS, SEG_DEG};
use super::walls::SectorCuts;
use super::{ArcFloor, ArcFloorParams};

impl ArcFloor {
	/// Authoring helper: thin passage/aperture AABB on the ring at normalized \(t\).
	pub fn plan_opening_at_t(
		id: impl Into<OpeningId>,
		label: OpeningLabel,
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		t: f32,
	) -> (OpeningId, Opening) {
		let id = id.into();
		let dir = ring_dir_at(t);
		let radius = radius.max(1e-4);
		let storey_height = storey_height.max(1e-4);
		let center = Vec3::new(center_xz.x, center_xz.y, center_xz.z);
		let on_ring = Vec3::new(center.x + dir.x * radius, center.y, center.z + dir.y * radius);
		let right = Vec3::new(-dir.y, 0.0, dir.x);
		let half_w = radius * (SEG_DEG.to_radians() * 0.5).sin().max(0.15);
		let half_d = 0.35;
		// Ground-to-0.8H passage/aperture silhouette (footer omitted, header left for strips).
		let h = 0.8 * storey_height;
		let min = on_ring - right * half_w - Vec3::new(dir.x, 0.0, dir.y) * half_d;
		let max = on_ring + right * half_w + Vec3::new(dir.x, 0.0, dir.y) * half_d + Vec3::Y * h;
		(id, Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label))
	}
}

impl MapsOpenings for ArcFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}

impl ArcFloorParams {
	/// Map connectable openings onto cut sectors as contact quads.
	pub(super) fn map_connectable_openings(
		&self,
		sectors: &[SectorCuts; SECTORS as usize],
	) -> (Openings, MappedOpenings) {
		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			let mut hit_sectors = Vec::new();
			for i in 0..SECTORS {
				if sectors[i as usize].is_solid() {
					continue;
				}
				let sector = self.sector_aabb(i);
				if aabb3d_intersects(&opening.bounds, &sector) {
					hit_sectors.push(i);
				}
			}
			if hit_sectors.is_empty() {
				continue;
			}
			openings.insert(id.clone(), opening.clone());
			mapped.insert(id.clone(), self.mapped_from_opening(opening, &hit_sectors));
		}
		(openings, mapped)
	}

	fn mapped_from_opening(&self, opening: &Opening, hit: &[u32]) -> MappedOpening {
		let lo = *hit.iter().min().unwrap_or(&0);
		let hi = *hit.iter().max().unwrap_or(&0);
		let deg_start = hi as f32 * SEG_DEG;
		let deg_end = lo as f32 * SEG_DEG - SEG_DEG;
		let deg_mid = 0.5 * (deg_start + deg_end);
		let y0 = opening.bounds.min.y.max(self.center_xz.y);
		let y1 = opening.bounds.max.y.min(self.center_xz.y + self.storey_height);
		let h = (y1 - y0).max(EPS);
		let bl = self.ring_point_deg(deg_end).with_y(y0);
		let br = self.ring_point_deg(deg_start).with_y(y0);
		let tl = bl + Vec3::Y * h;
		let tr = br + Vec3::Y * h;
		let orientation = arc_ring_dir_deg(deg_mid);
		let right = Vec3::new(-orientation.y, 0.0, orientation.x);
		let (bl, br, tl, tr) =
			if (br - bl).dot(right) < 0.0 { (br, bl, tr, tl) } else { (bl, br, tl, tr) };
		MappedOpening::new(MappedOpeningQuad::new(bl, br, tl, tr), orientation)
	}
}
