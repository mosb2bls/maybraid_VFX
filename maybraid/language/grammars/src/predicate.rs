//! Predicate realization: simple, separable, and serial strategies.

use maybraid_language_core::{
	BoundaryKind, LexicalPart, ParticleDomain, SemanticNode, SemanticRole, SurfaceConstituent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicateStrategy {
	Simple,
	/// Stem stays in the verb slot; particle is placed after the object.
	Separable {
		particle: &'static str,
	},
}

impl PredicateStrategy {
	pub fn stem(
		self,
		form: &str,
		node: SemanticNode,
	) -> (Vec<SurfaceConstituent>, Option<SurfaceConstituent>) {
		match self {
			Self::Simple => (vec![SurfaceConstituent::lexical(form, node)], None),
			Self::Separable { particle } => (
				vec![SurfaceConstituent::Lexical {
					form: form.to_owned(),
					node,
					relation: None,
					part: LexicalPart::Stem,
				}],
				Some(SurfaceConstituent::Lexical {
					form: particle.to_owned(),
					node,
					relation: None,
					part: LexicalPart::SeparableParticle,
				}),
			),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialStrategy {
	None,
	/// Instrument is realized as an extra TAKE predicate: take knife cut meat.
	InstrumentAsTake {
		take: &'static str,
	},
}

impl SerialStrategy {
	pub fn lift_instrument(self, role: SemanticRole) -> bool {
		matches!(self, Self::InstrumentAsTake { .. }) && role == SemanticRole::Instrument
	}

	pub fn prefix(self, instrument: Vec<SurfaceConstituent>) -> Vec<SurfaceConstituent> {
		let Self::InstrumentAsTake { take } = self else {
			return Vec::new();
		};
		if instrument.is_empty() {
			return Vec::new();
		}
		let mut out = vec![SurfaceConstituent::particle(take, ParticleDomain::Predicate)];
		out.extend(instrument);
		out.push(SurfaceConstituent::Boundary { kind: BoundaryKind::Serial });
		out
	}
}
