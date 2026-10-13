//! Construction and mutation helpers for [`PanelComplex`].

use bevy_math::Vec3;
use building_components::panels::PanelStyle;

use super::types::{
	PanelComplex, PanelComplexJointPolicy, PanelPoint, PanelPointId, PanelTriangle,
	DEFAULT_PANEL_THICKNESS,
};

impl PanelComplex {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			points: Vec::new(),
			triangles: Vec::new(),
			joint_policy: PanelComplexJointPolicy::default(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	pub fn with_style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn set_style(&mut self, style: PanelStyle) -> &mut Self {
		self.style = style;
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.joint_policy = joint_policy;
		self
	}

	/// Insert a point with [`DEFAULT_PANEL_THICKNESS`] (appends; id = next slot).
	pub fn insert_point(&mut self, position: Vec3) -> PanelPointId {
		self.insert_point_thick(position, DEFAULT_PANEL_THICKNESS)
	}

	/// Insert a point with explicit thickness (appends; id = next slot).
	pub fn insert_point_thick(&mut self, position: Vec3, thickness: f32) -> PanelPointId {
		let id = PanelPointId(self.points.len() as u32);
		self.points.push(Some(PanelPoint::new(position, thickness)));
		id
	}

	/// Place or replace a point at an explicit author id (grows the table with holes).
	pub fn put_point(&mut self, id: PanelPointId, point: impl Into<PanelPoint>) -> &mut Self {
		let idx = id.0 as usize;
		if idx >= self.points.len() {
			self.points.resize(idx + 1, None);
		}
		self.points[idx] = Some(point.into());
		self
	}

	/// Owned insert: returns `(self, id)`.
	pub fn with_point(mut self, position: Vec3) -> (Self, PanelPointId) {
		let id = self.insert_point(position);
		(self, id)
	}

	/// Owned insert with thickness.
	pub fn with_point_thick(mut self, position: Vec3, thickness: f32) -> (Self, PanelPointId) {
		let id = self.insert_point_thick(position, thickness);
		(self, id)
	}

	/// Insert many default-thickness points; returns their ids in order.
	pub fn insert_points<I>(&mut self, positions: I) -> Vec<PanelPointId>
	where
		I: IntoIterator<Item = Vec3>,
	{
		positions.into_iter().map(|p| self.insert_point(p)).collect()
	}

	/// Extend with [`PanelPoint`] values; returns their ids in order.
	pub fn extend_points<I>(&mut self, points: I) -> Vec<PanelPointId>
	where
		I: IntoIterator<Item = PanelPoint>,
	{
		points
			.into_iter()
			.map(|p| self.insert_point_thick(p.position, p.thickness))
			.collect()
	}

	pub fn set_point(&mut self, id: PanelPointId, point: PanelPoint) -> &mut Self {
		self.put_point(id, point)
	}

	pub fn set_position(&mut self, id: PanelPointId, position: Vec3) -> &mut Self {
		if let Some(p) = self.point_mut(id) {
			p.position = position;
		} else {
			debug_assert!(false, "set_position: unknown id {id:?}");
		}
		self
	}

	pub fn set_thickness(&mut self, id: PanelPointId, thickness: f32) -> &mut Self {
		if let Some(p) = self.point_mut(id) {
			p.thickness = thickness.max(1e-4);
		} else {
			debug_assert!(false, "set_thickness: unknown id {id:?}");
		}
		self
	}

	/// Remove a point and any triangles that reference it. Id slots are not reused.
	pub fn remove_point(&mut self, id: PanelPointId) -> &mut Self {
		if let Some(slot) = self.points.get_mut(id.0 as usize) {
			*slot = None;
			self.triangles.retain(|t| !t.contains(id));
		} else {
			debug_assert!(false, "remove_point: unknown id {id:?}");
		}
		self
	}

	pub fn clear_triangles(&mut self) -> &mut Self {
		self.triangles.clear();
		self
	}

	pub fn clear(&mut self) -> &mut Self {
		self.points.clear();
		self.triangles.clear();
		self
	}

	/// Append a triangle. Unknown ids or degeneracy → debug assert and skip.
	pub fn add_triangle(&mut self, a: PanelPointId, b: PanelPointId, c: PanelPointId) -> &mut Self {
		if self.point(a).is_none() || self.point(b).is_none() || self.point(c).is_none() {
			debug_assert!(false, "add_triangle: unknown point id");
			return self;
		}
		if a == b || b == c || c == a {
			debug_assert!(false, "add_triangle: degenerate vertex repeat");
			return self;
		}
		self.triangles.push(PanelTriangle::new(a, b, c));
		self
	}

	/// Alias of [`Self::add_triangle`].
	pub fn triangle(&mut self, a: PanelPointId, b: PanelPointId, c: PanelPointId) -> &mut Self {
		self.add_triangle(a, b, c)
	}

	pub fn add_triangles<I>(&mut self, tris: I) -> &mut Self
	where
		I: IntoIterator<Item = (PanelPointId, PanelPointId, PanelPointId)>,
	{
		for (a, b, c) in tris {
			self.add_triangle(a, b, c);
		}
		self
	}

	/// Append another complex's points and triangles, remapping ids by table offset.
	///
	/// Assumes `other`'s live point ids are dense indices into `other.points` (holes
	/// are preserved as empty slots so id arithmetic stays valid).
	pub fn append_complex(&mut self, other: PanelComplex) -> &mut Self {
		let offset = self.points.len() as u32;
		self.points.extend(other.points);
		for t in other.triangles {
			self.triangles.push(PanelTriangle::new(
				PanelPointId(t.a.0 + offset),
				PanelPointId(t.b.0 + offset),
				PanelPointId(t.c.0 + offset),
			));
		}
		self
	}
}
