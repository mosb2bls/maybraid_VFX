//! Compose independent grammatical strategies into one [`Grammar`].

use maybraid_language_core::{
	Clause, ClauseId, Grammar, GrammarInput, GrammaticalRelation, ModifierPlacement,
	ParticleDomain, Polarity, Referent, ReferentId, RelativePlacement, SemanticNode, SemanticRole,
	SemanticValue, SurfaceClause, SurfaceConstituent, SurfaceForm,
};

use crate::agreement::{AgreementFeatures, AgreementStrategy};
use crate::alignment::Alignment;
use crate::clause::{wrap_relative, WordOrder};
use crate::copula::CopulaStrategy;
use crate::information::InformationStrategy;
use crate::linking::ClauseLinkStrategy;
use crate::marking::{AdpositionStrategy, CaseStrategy, PolarityStrategy, TamStrategy};
use crate::nominal::{place_modifiers, DefinitenessStrategy, NumberStrategy};
use crate::predicate::{PredicateStrategy, SerialStrategy};
use crate::question::QuestionStrategy;

/// Typological grammar assembled from independent composers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositeGrammar {
	pub alignment: Alignment,
	pub order: WordOrder,
	pub relative: RelativePlacement,
	pub modifiers: ModifierPlacement,
	pub modifier_linker: Option<&'static str>,
	pub case: CaseStrategy,
	pub adpositions: AdpositionStrategy,
	pub number: NumberStrategy,
	pub definiteness: DefinitenessStrategy,
	pub tam: TamStrategy,
	pub polarity: PolarityStrategy,
	pub predicate: PredicateStrategy,
	pub serial: SerialStrategy,
	pub information: InformationStrategy,
	pub question: QuestionStrategy,
	pub copula: CopulaStrategy,
	pub agreement: AgreementStrategy,
	pub linking: ClauseLinkStrategy,
	pub coordinator: Option<&'static str>,
}

impl Grammar for CompositeGrammar {
	fn realize_surface(&self, input: &GrammarInput<'_>) -> SurfaceForm {
		let mut clauses = Vec::new();
		for (index, root) in input.utterance.roots.iter().enumerate() {
			let mut built = self.emit_clause(input, *root, None);
			if index > 0 {
				if let Some(form) = self.coordinator {
					built.insert(0, SurfaceConstituent::particle(form, ParticleDomain::Discourse));
				}
			}
			self.information.apply(&mut built, &input.utterance.information);
			clauses.push(SurfaceClause {
				clause: Some(*root),
				constituents: built,
				topic: input.utterance.information.topic,
				focus: match input.utterance.information.focus {
					Some(maybraid_language_core::FocusTarget::Referent(id)) => Some(id),
					_ => None,
				},
			});
		}
		SurfaceForm::new(clauses)
	}
}

impl CompositeGrammar {
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
		let mut object = Vec::new();
		let mut rest = Vec::new();
		let mut serial_instrument = Vec::new();
		let mut subject_features = None;
		let mut object_features = None;

		for argument in &clause.arguments {
			if is_bound(argument.value, bound) {
				continue;
			}
			if self.serial.lift_instrument(argument.role) {
				serial_instrument.extend(self.emit_value(input, argument.value, bound, None));
				continue;
			}
			let relation = self.alignment.relation(argument.role);
			let features = agreement_features(input, argument.value);
			match relation {
				GrammaticalRelation::Subject => subject_features = subject_features.or(features),
				GrammaticalRelation::Object => object_features = object_features.or(features),
				_ => {}
			}
			let mut words = self.emit_value(input, argument.value, bound, Some(relation));
			self.mark_argument(&mut words, clause, argument.role, relation);
			match relation {
				GrammaticalRelation::Subject => subject.extend(words),
				GrammaticalRelation::Object => object.extend(words),
				GrammaticalRelation::IndirectObject | GrammaticalRelation::Oblique => {
					rest.extend(words);
				}
			}
		}

		let node = SemanticNode::Predicate(clause_id);
		let form = input.ipa_for(node).unwrap_or("…");
		let (stem, separable) = self.predicate.stem(form, node);
		let mut predicate = self.copula.realize(clause, stem, node);
		if !predicate.is_empty() {
			self.tam.apply(&mut predicate, clause.tense, clause.aspect, clause.mood, node);
			if clause.polarity == Polarity::Negative {
				self.polarity.pre.apply(&mut predicate, ParticleDomain::Polarity, Some(node));
				self.polarity.post.apply(&mut predicate, ParticleDomain::Polarity, Some(node));
			}
			self.agreement.apply(&mut predicate, subject_features, object_features, node);
			self.question.apply_predicate(&mut predicate, clause.mood, node);
		}

		if let Some(particle) = separable {
			rest.insert(0, particle);
		}

		let order = self.question.order(self.order, clause.mood);
		let mut ordered = order.arrange(subject, predicate, object, rest);
		let prefix = self.serial.prefix(serial_instrument);
		if !prefix.is_empty() {
			let mut combined = prefix;
			combined.extend(ordered);
			ordered = combined;
		}
		self.question.apply_clause(&mut ordered, clause.mood);
		ordered
	}

	fn mark_argument(
		&self,
		words: &mut Vec<SurfaceConstituent>,
		_clause: &Clause,
		role: SemanticRole,
		relation: GrammaticalRelation,
	) {
		let host = words.iter().find_map(SurfaceConstituent::node);
		self.adpositions.apply(words, role, host);
		let label = self.alignment.case_for(relation, role);
		self.case.marking(label).apply(words, ParticleDomain::Case, host);
	}

	fn emit_value(
		&self,
		input: &GrammarInput<'_>,
		value: SemanticValue,
		bound: Option<ReferentId>,
		relation: Option<GrammaticalRelation>,
	) -> Vec<SurfaceConstituent> {
		match value {
			SemanticValue::Referent(referent) if Some(referent) == bound => Vec::new(),
			SemanticValue::Referent(referent) => self.emit_referent(input, referent, relation),
			SemanticValue::Clause(clause) => self.linking.wrap(
				self.emit_clause(input, clause, bound),
				Some(SemanticNode::Predicate(clause)),
			),
		}
	}

	fn emit_referent(
		&self,
		input: &GrammarInput<'_>,
		referent_id: ReferentId,
		relation: Option<GrammaticalRelation>,
	) -> Vec<SurfaceConstituent> {
		let Some(referent) = input.utterance.referents.get(referent_id) else {
			return Vec::new();
		};
		let head = self.emit_noun_phrase(input, referent_id, referent, relation);
		let mut relatives = Vec::new();
		for (index, clause) in referent.relative_clauses.iter().enumerate() {
			if index > 0 {
				relatives.push(SurfaceConstituent::Boundary {
					kind: maybraid_language_core::BoundaryKind::Relative,
				});
			}
			relatives.extend(self.emit_clause(input, *clause, Some(referent_id)));
		}
		wrap_relative(self.relative, head, relatives)
	}

	fn emit_noun_phrase(
		&self,
		input: &GrammarInput<'_>,
		referent_id: ReferentId,
		referent: &Referent,
		relation: Option<GrammaticalRelation>,
	) -> Vec<SurfaceConstituent> {
		let mut modifiers = Vec::new();
		for index in 0..referent.modifiers.len() {
			let node = SemanticNode::Modifier { referent: referent_id, index };
			if let Some(ipa) = input.ipa_for(node) {
				modifiers.push(SurfaceConstituent::lexical(ipa, node));
			}
		}
		let node = SemanticNode::Referent(referent_id);
		let mut noun = SurfaceConstituent::lexical(input.ipa_for(node).unwrap_or("…"), node);
		if let Some(relation) = relation {
			noun = noun.with_relation(relation);
		}
		let mut words = place_modifiers(self.modifiers, noun, modifiers, self.modifier_linker);
		self.definiteness.apply(&mut words, referent.definiteness, node);
		self.number.apply(&mut words, referent.number, node);
		words
	}
}

fn is_bound(value: SemanticValue, bound: Option<ReferentId>) -> bool {
	matches!(value, SemanticValue::Referent(referent) if Some(referent) == bound)
}

fn agreement_features(input: &GrammarInput<'_>, value: SemanticValue) -> Option<AgreementFeatures> {
	let SemanticValue::Referent(id) = value else {
		return None;
	};
	let referent = input.utterance.referents.get(id)?;
	Some(AgreementFeatures { person: referent.person, number: referent.number })
}
