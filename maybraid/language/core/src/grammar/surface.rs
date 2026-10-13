//! Structured surface IR. Grammar composers manipulate this, not strings.

use crate::marshall::SemanticNode;
use crate::utterance::{ClauseId, ReferentId};

use super::{GrammaticalOutput, IpaUtterance};

/// One realized utterance before phonology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceForm {
	pub clauses: Vec<SurfaceClause>,
}

impl SurfaceForm {
	pub fn new(clauses: Vec<SurfaceClause>) -> Self {
		Self { clauses }
	}

	pub fn linearize(&self) -> GrammaticalOutput {
		let mut words = Vec::new();
		for (index, clause) in self.clauses.iter().enumerate() {
			if index > 0 {
				words.push(BoundaryKind::Root.as_str().to_owned());
			}
			for constituent in &clause.constituents {
				if let Some(form) = constituent.form() {
					if !form.is_empty() {
						words.push(form.to_owned());
					}
				}
			}
		}
		GrammaticalOutput::new(words)
	}

	pub fn ipa(&self) -> IpaUtterance {
		self.linearize().ipa()
	}

	pub fn forms(&self) -> Vec<&str> {
		self.clauses
			.iter()
			.flat_map(|clause| clause.constituents.iter())
			.filter_map(SurfaceConstituent::form)
			.collect()
	}
}

/// One clause's ordered constituents, with optional topic/focus hooks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceClause {
	pub clause: Option<ClauseId>,
	pub constituents: Vec<SurfaceConstituent>,
	pub topic: Option<ReferentId>,
	pub focus: Option<ReferentId>,
}

impl SurfaceClause {
	pub fn new(clause: ClauseId, constituents: Vec<SurfaceConstituent>) -> Self {
		Self { clause: Some(clause), constituents, topic: None, focus: None }
	}
}

/// A grammatical atom that still knows where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceConstituent {
	Lexical {
		form: String,
		node: SemanticNode,
		relation: Option<GrammaticalRelation>,
		part: LexicalPart,
	},
	Particle {
		form: String,
		domain: ParticleDomain,
		host: Option<SemanticNode>,
	},
	Affix {
		form: String,
		domain: ParticleDomain,
		host: SemanticNode,
		placement: AffixPlacement,
	},
	Boundary {
		kind: BoundaryKind,
	},
}

impl SurfaceConstituent {
	pub fn lexical(form: impl Into<String>, node: SemanticNode) -> Self {
		Self::Lexical { form: form.into(), node, relation: None, part: LexicalPart::Whole }
	}

	pub fn particle(form: impl Into<String>, domain: ParticleDomain) -> Self {
		Self::Particle { form: form.into(), domain, host: None }
	}

	pub fn form(&self) -> Option<&str> {
		match self {
			Self::Lexical { form, .. } | Self::Particle { form, .. } | Self::Affix { form, .. } => {
				Some(form.as_str())
			}
			Self::Boundary { kind } => Some(kind.as_str()),
		}
	}

	pub fn node(&self) -> Option<SemanticNode> {
		match self {
			Self::Lexical { node, .. } => Some(*node),
			Self::Particle { host, .. } => *host,
			Self::Affix { host, .. } => Some(*host),
			Self::Boundary { .. } => None,
		}
	}

	pub fn with_relation(mut self, relation: GrammaticalRelation) -> Self {
		if let Self::Lexical { relation: slot, .. } = &mut self {
			*slot = Some(relation);
		}
		self
	}
}

/// Grammatical relation after alignment. Not a semantic role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrammaticalRelation {
	Subject,
	Object,
	IndirectObject,
	Oblique,
}

/// Which piece of a possibly discontinuous lexicalization this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexicalPart {
	Whole,
	Stem,
	SeparableParticle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffixPlacement {
	Prefix,
	Suffix,
}

/// Reusable particle/affix domains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleDomain {
	Clause,
	Predicate,
	NounPhrase,
	Focus,
	Topic,
	Aspect,
	Mood,
	Tense,
	Evidentiality,
	Discourse,
	Polarity,
	Case,
	Number,
	Definiteness,
	Adposition,
	Relativizer,
	Complementizer,
	Agreement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryKind {
	Root,
	Relative,
	Serial,
}

impl BoundaryKind {
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Root => "‖",
			Self::Relative => "|",
			Self::Serial => "¦",
		}
	}
}
