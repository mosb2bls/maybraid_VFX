//! Grammar abstraction and the basic POC linearizer.
//!
//! Concrete typological strategies live in `maybraid-grammars`. This module
//! defines the input, the surface IR, and [`SurfaceGrammar`] as the simplest
//! realization that still implements [`Grammar`].

use crate::marshall::SemanticNode;
use crate::output::LexicalOutput;
use crate::profile::Profile;
use crate::utterance::{
	Clause, ClauseId, Number, Polarity, Referent, ReferentId, SemanticRole, SemanticValue,
	Utterance,
};

mod surface;

pub use surface::{
	AffixPlacement, BoundaryKind, GrammaticalRelation, LexicalPart, ParticleDomain, SurfaceClause,
	SurfaceConstituent, SurfaceForm,
};

/// Linearized grammatical form. Phonology reads this, not the surface IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammaticalOutput {
	pub words: Vec<String>,
}

impl GrammaticalOutput {
	pub fn new(words: Vec<String>) -> Self {
		Self { words }
	}

	pub fn ipa(&self) -> IpaUtterance {
		IpaUtterance::from(self)
	}
}

impl std::fmt::Display for GrammaticalOutput {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.words.join(" "))
	}
}

/// `/word word word/` view of a [`GrammaticalOutput`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IpaUtterance {
	pub words: Vec<String>,
}

impl IpaUtterance {
	pub fn new(words: Vec<String>) -> Self {
		Self { words }
	}
}

impl From<&GrammaticalOutput> for IpaUtterance {
	fn from(output: &GrammaticalOutput) -> Self {
		Self { words: output.words.clone() }
	}
}

impl std::fmt::Display for IpaUtterance {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "/{}/", self.words.join(" "))
	}
}

/// Semantic graph plus lexical material. Grammar may split or insert forms.
pub struct GrammarInput<'a> {
	pub utterance: &'a Utterance,
	pub lexicalizations: &'a LexicalOutput,
	pub profile: &'a Profile,
}

impl<'a> GrammarInput<'a> {
	pub fn new(lexicalizations: &'a LexicalOutput, profile: &'a Profile) -> Self {
		Self { utterance: &lexicalizations.utterance, lexicalizations, profile }
	}

	pub fn from_output(lexicalizations: &'a LexicalOutput) -> Self {
		Self::new(lexicalizations, &Profile::NEUTRAL)
	}

	pub fn ipa_for(&self, node: SemanticNode) -> Option<&'a str> {
		self.lexicalizations.ipa_for(node)
	}
}

/// Realizes an utterance from semantics and lexicalizations, not from a word list.
pub trait Grammar {
	fn realize_surface(&self, input: &GrammarInput<'_>) -> SurfaceForm;

	fn realize(&self, input: &GrammarInput<'_>) -> GrammaticalOutput {
		self.realize_surface(input).linearize()
	}
}

/// Role-bearing function words used by the basic POC linearizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleParticles {
	pub recipient: &'static str,
	pub source: &'static str,
	pub goal: &'static str,
	pub plural: &'static str,
	pub negative: &'static str,
}

impl RoleParticles {
	pub fn compositional() -> Self {
		Self { recipient: "tʰə", source: "kə", goal: "ŋə", plural: "ɲi", negative: "ma" }
	}

	pub fn root_heavy() -> Self {
		Self { recipient: "ʔu", source: "ħe", goal: "ɡo", plural: "riː", negative: "nu" }
	}
}

/// Where the predicate sits relative to its arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClauseOrder {
	/// Agent — predicate — complements (SVO-like).
	VerbMedial,
	/// Agent — complements — predicate (SOV-like).
	VerbFinal,
}

/// Where a relative clause sits relative to its head noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativePlacement {
	AfterHead,
	BeforeHead,
}

/// Where modifiers sit relative to the noun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierPlacement {
	BeforeNoun,
	AfterNoun,
}

/// Simplest POC realization. Seeds the composers in `maybraid-grammars`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceGrammar {
	pub clause_order: ClauseOrder,
	pub relative: RelativePlacement,
	pub modifier: ModifierPlacement,
	pub particles: RoleParticles,
}

impl SurfaceGrammar {
	pub fn compositional() -> Self {
		Self {
			clause_order: ClauseOrder::VerbMedial,
			relative: RelativePlacement::AfterHead,
			modifier: ModifierPlacement::BeforeNoun,
			particles: RoleParticles::compositional(),
		}
	}

	pub fn root_heavy() -> Self {
		Self {
			clause_order: ClauseOrder::VerbFinal,
			relative: RelativePlacement::BeforeHead,
			modifier: ModifierPlacement::BeforeNoun,
			particles: RoleParticles::root_heavy(),
		}
	}

	pub fn realize(self, output: &LexicalOutput) -> GrammaticalOutput {
		<Self as Grammar>::realize(&self, &GrammarInput::from_output(output))
	}
}

impl Grammar for SurfaceGrammar {
	fn realize_surface(&self, input: &GrammarInput<'_>) -> SurfaceForm {
		let mut clauses = Vec::new();
		for root in &input.utterance.roots {
			let constituents = self.emit_clause(input, *root, None);
			clauses.push(SurfaceClause::new(*root, constituents));
		}
		SurfaceForm::new(clauses)
	}
}

impl SurfaceGrammar {
	fn emit_clause(
		&self,
		input: &GrammarInput<'_>,
		clause_id: ClauseId,
		bound: Option<ReferentId>,
	) -> Vec<SurfaceConstituent> {
		let Some(clause) = input.utterance.clauses.get(clause_id) else {
			return Vec::new();
		};
		let mut subject = Vec::new();
		let mut complements = Vec::new();
		self.collect_arguments(&mut subject, &mut complements, input, clause, bound);

		let predicate = SurfaceConstituent::lexical(
			input.ipa_for(SemanticNode::Predicate(clause_id)).unwrap_or("…"),
			SemanticNode::Predicate(clause_id),
		);
		let negative = (clause.polarity == Polarity::Negative).then(|| {
			SurfaceConstituent::particle(self.particles.negative, ParticleDomain::Polarity)
		});

		let mut words = Vec::new();
		match self.clause_order {
			ClauseOrder::VerbMedial => {
				words.extend(subject);
				words.extend(negative);
				words.push(predicate);
				words.extend(complements);
			}
			ClauseOrder::VerbFinal => {
				words.extend(subject);
				words.extend(complements);
				words.extend(negative);
				words.push(predicate);
			}
		}
		words
	}

	fn collect_arguments(
		&self,
		subject: &mut Vec<SurfaceConstituent>,
		complements: &mut Vec<SurfaceConstituent>,
		input: &GrammarInput<'_>,
		clause: &Clause,
		bound: Option<ReferentId>,
	) {
		for role in [SemanticRole::Agent, SemanticRole::Experiencer, SemanticRole::Possessor] {
			if let Some(value) = clause_value(clause, role) {
				self.emit_value(subject, input, value, bound, Some(GrammaticalRelation::Subject));
			}
		}

		for (role, particle) in [
			(SemanticRole::Patient, None),
			(SemanticRole::Theme, None),
			(SemanticRole::Classification, None),
			(SemanticRole::Content, None),
			(SemanticRole::Recipient, Some(self.particles.recipient)),
			(SemanticRole::Beneficiary, Some(self.particles.recipient)),
			(SemanticRole::Source, Some(self.particles.source)),
			(SemanticRole::Goal, Some(self.particles.goal)),
			(SemanticRole::Instrument, None),
			(SemanticRole::Location, None),
			(SemanticRole::Manner, None),
			(SemanticRole::Rate, None),
			(SemanticRole::Time, None),
			(SemanticRole::Cause, None),
		] {
			if let Some(value) = clause_value(clause, role) {
				if let Some(particle) = particle {
					if !is_bound(value, bound) {
						complements.push(SurfaceConstituent::particle(
							particle,
							ParticleDomain::Adposition,
						));
					}
				}
				self.emit_value(
					complements,
					input,
					value,
					bound,
					Some(GrammaticalRelation::Object),
				);
			}
		}
	}

	fn emit_value(
		&self,
		words: &mut Vec<SurfaceConstituent>,
		input: &GrammarInput<'_>,
		value: SemanticValue,
		bound: Option<ReferentId>,
		relation: Option<GrammaticalRelation>,
	) {
		match value {
			SemanticValue::Referent(referent) if Some(referent) == bound => {}
			SemanticValue::Referent(referent) => {
				self.emit_referent(words, input, referent, relation);
			}
			SemanticValue::Clause(clause) => {
				words.extend(self.emit_clause(input, clause, bound));
			}
		}
	}

	fn emit_referent(
		&self,
		words: &mut Vec<SurfaceConstituent>,
		input: &GrammarInput<'_>,
		referent_id: ReferentId,
		relation: Option<GrammaticalRelation>,
	) {
		let Some(referent) = input.utterance.referents.get(referent_id) else {
			return;
		};
		let mut head = Vec::new();
		self.emit_noun_phrase(&mut head, input, referent_id, referent, relation);

		let mut relatives = Vec::new();
		for (index, clause) in referent.relative_clauses.iter().enumerate() {
			if index > 0 || !relatives.is_empty() {
				relatives.push(SurfaceConstituent::Boundary { kind: BoundaryKind::Relative });
			}
			relatives.extend(self.emit_clause(input, *clause, Some(referent_id)));
		}

		match self.relative {
			RelativePlacement::AfterHead => {
				words.extend(head);
				if !relatives.is_empty() {
					words.push(SurfaceConstituent::Boundary { kind: BoundaryKind::Relative });
					words.extend(relatives);
				}
			}
			RelativePlacement::BeforeHead => {
				if !relatives.is_empty() {
					words.extend(relatives);
					words.push(SurfaceConstituent::Boundary { kind: BoundaryKind::Relative });
				}
				words.extend(head);
			}
		}
	}

	fn emit_noun_phrase(
		&self,
		words: &mut Vec<SurfaceConstituent>,
		input: &GrammarInput<'_>,
		referent_id: ReferentId,
		referent: &Referent,
		relation: Option<GrammaticalRelation>,
	) {
		let mut modifiers = Vec::new();
		for index in 0..referent.modifiers.len() {
			let node = SemanticNode::Modifier { referent: referent_id, index };
			if let Some(ipa) = input.ipa_for(node) {
				modifiers.push(SurfaceConstituent::lexical(ipa, node));
			}
		}
		let mut noun = SurfaceConstituent::lexical(
			input.ipa_for(SemanticNode::Referent(referent_id)).unwrap_or("…"),
			SemanticNode::Referent(referent_id),
		);
		if let Some(relation) = relation {
			noun = noun.with_relation(relation);
		}
		match self.modifier {
			ModifierPlacement::BeforeNoun => {
				words.extend(modifiers);
				words.push(noun);
			}
			ModifierPlacement::AfterNoun => {
				words.push(noun);
				words.extend(modifiers);
			}
		}
		if matches!(referent.number, Number::Plural | Number::Many) {
			words.push(SurfaceConstituent::Particle {
				form: self.particles.plural.to_owned(),
				domain: ParticleDomain::Number,
				host: Some(SemanticNode::Referent(referent_id)),
			});
		}
	}
}

impl LexicalOutput {
	pub fn realize(&self, grammar: impl Grammar) -> GrammaticalOutput {
		grammar.realize(&GrammarInput::from_output(self))
	}

	pub fn realize_surface(&self, grammar: impl Grammar) -> SurfaceForm {
		grammar.realize_surface(&GrammarInput::from_output(self))
	}

	pub fn ipa_for(&self, node: SemanticNode) -> Option<&str> {
		self.lexicalizations
			.iter()
			.find(|item| item.source.source == node)
			.map(|item| item.realization.term.ipa.as_str())
	}
}

fn clause_value(clause: &Clause, role: SemanticRole) -> Option<SemanticValue> {
	clause
		.arguments
		.iter()
		.find(|argument| argument.role == role)
		.map(|argument| argument.value)
}

fn is_bound(value: SemanticValue, bound: Option<ReferentId>) -> bool {
	matches!(value, SemanticValue::Referent(referent) if Some(referent) == bound)
}
