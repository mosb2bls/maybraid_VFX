//! Copula realization. Classification and locative predications need not be verbs.

use maybraid_language_core::{
	Clause, ParticleDomain, SemanticNode, SemanticRole, SurfaceConstituent,
};

/// How a grammar realizes non-verbal predication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopulaStrategy {
	/// Keep the lexicalized predicate (stative-verb-like).
	Stative,
	/// Replace the predicate with a dedicated copula form.
	Obligatory { form: &'static str },
	/// Drop the predicate and juxtapose the arguments.
	Zero,
	/// Dedicated form for classification; other copular clauses are zero.
	Nominal { form: &'static str },
	/// Dedicated form for locative predication; classification is zero.
	Locative { form: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PredicationKind {
	Verbal,
	Classificational,
	Locative,
}

impl PredicationKind {
	pub fn of(clause: &Clause) -> Self {
		if clause.has_role(SemanticRole::Classification) {
			Self::Classificational
		} else if clause.has_role(SemanticRole::Location)
			&& !clause.has_role(SemanticRole::Agent)
			&& !clause.has_role(SemanticRole::Patient)
		{
			Self::Locative
		} else {
			Self::Verbal
		}
	}

	pub fn is_copular(self) -> bool {
		!matches!(self, Self::Verbal)
	}
}

impl CopulaStrategy {
	pub fn stative() -> Self {
		Self::Stative
	}

	pub fn realize(
		self,
		clause: &Clause,
		stem: Vec<SurfaceConstituent>,
		host: SemanticNode,
	) -> Vec<SurfaceConstituent> {
		let kind = PredicationKind::of(clause);
		if !kind.is_copular() {
			return stem;
		}
		match (self, kind) {
			(Self::Stative, _) => stem,
			(Self::Zero, _) => Vec::new(),
			(Self::Obligatory { form }, _) => copula_form(form, host),
			(Self::Nominal { form }, PredicationKind::Classificational) => copula_form(form, host),
			(Self::Nominal { .. }, _) => Vec::new(),
			(Self::Locative { form }, PredicationKind::Locative) => copula_form(form, host),
			(Self::Locative { .. }, _) => Vec::new(),
		}
	}
}

fn copula_form(form: &'static str, host: SemanticNode) -> Vec<SurfaceConstituent> {
	vec![SurfaceConstituent::Particle {
		form: form.to_owned(),
		domain: ParticleDomain::Predicate,
		host: Some(host),
	}]
}
