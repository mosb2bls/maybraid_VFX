//! Clause-level composers: order and relative placement.

use maybraid_language_core::{RelativePlacement, SurfaceConstituent};

/// Six-way constituent order over aligned grammatical relations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordOrder {
	Svo,
	Sov,
	Vso,
	Vos,
	Ovs,
	Osv,
}

impl WordOrder {
	pub fn arrange(
		self,
		subject: Vec<SurfaceConstituent>,
		predicate: Vec<SurfaceConstituent>,
		object: Vec<SurfaceConstituent>,
		rest: Vec<SurfaceConstituent>,
	) -> Vec<SurfaceConstituent> {
		let mut out = Vec::new();
		match self {
			Self::Svo => {
				out.extend(subject);
				out.extend(predicate);
				out.extend(object);
				out.extend(rest);
			}
			Self::Sov => {
				out.extend(subject);
				out.extend(object);
				out.extend(rest);
				out.extend(predicate);
			}
			Self::Vso => {
				out.extend(predicate);
				out.extend(subject);
				out.extend(object);
				out.extend(rest);
			}
			Self::Vos => {
				out.extend(predicate);
				out.extend(object);
				out.extend(rest);
				out.extend(subject);
			}
			Self::Ovs => {
				out.extend(object);
				out.extend(rest);
				out.extend(predicate);
				out.extend(subject);
			}
			Self::Osv => {
				out.extend(object);
				out.extend(rest);
				out.extend(subject);
				out.extend(predicate);
			}
		}
		out
	}
}

pub fn wrap_relative(
	placement: RelativePlacement,
	mut head: Vec<SurfaceConstituent>,
	relatives: Vec<SurfaceConstituent>,
) -> Vec<SurfaceConstituent> {
	if relatives.is_empty() {
		return head;
	}
	let boundary =
		SurfaceConstituent::Boundary { kind: maybraid_language_core::BoundaryKind::Relative };
	match placement {
		RelativePlacement::AfterHead => {
			head.push(boundary);
			head.extend(relatives);
			head
		}
		RelativePlacement::BeforeHead => {
			let mut out = relatives;
			out.push(boundary);
			out.extend(head);
			out
		}
	}
}
