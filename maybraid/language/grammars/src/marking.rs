//! Case, adposition, TAM, and polarity marking.

use maybraid_language_core::{
	AffixPlacement, Aspect, Mood, ParticleDomain, SemanticNode, SemanticRole, SurfaceConstituent,
	Tense,
};

use crate::alignment::CaseLabel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marking {
	Zero,
	Particle { form: &'static str, before: bool },
	Prefix { form: &'static str },
	Suffix { form: &'static str },
}

impl Marking {
	pub fn apply(
		self,
		words: &mut Vec<SurfaceConstituent>,
		domain: ParticleDomain,
		host: Option<SemanticNode>,
	) {
		match self {
			Self::Zero => {}
			Self::Particle { form, before } => {
				let particle = SurfaceConstituent::Particle { form: form.to_owned(), domain, host };
				if before {
					words.insert(0, particle);
				} else {
					words.push(particle);
				}
			}
			Self::Prefix { form } => {
				if let Some(host) = host {
					words.insert(
						0,
						SurfaceConstituent::Affix {
							form: form.to_owned(),
							domain,
							host,
							placement: AffixPlacement::Prefix,
						},
					);
				}
			}
			Self::Suffix { form } => {
				if let Some(host) = host {
					words.push(SurfaceConstituent::Affix {
						form: form.to_owned(),
						domain,
						host,
						placement: AffixPlacement::Suffix,
					});
				}
			}
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaseStrategy {
	pub nominative: Marking,
	pub accusative: Marking,
	pub ergative: Marking,
	pub absolutive: Marking,
	pub dative: Marking,
	pub genitive: Marking,
	pub instrumental: Marking,
	pub locative: Marking,
	pub ablative: Marking,
	pub allative: Marking,
}

impl CaseStrategy {
	pub fn unmarked() -> Self {
		Self {
			nominative: Marking::Zero,
			accusative: Marking::Zero,
			ergative: Marking::Zero,
			absolutive: Marking::Zero,
			dative: Marking::Zero,
			genitive: Marking::Zero,
			instrumental: Marking::Zero,
			locative: Marking::Zero,
			ablative: Marking::Zero,
			allative: Marking::Zero,
		}
	}

	pub fn marking(self, label: CaseLabel) -> Marking {
		match label {
			CaseLabel::Nominative => self.nominative,
			CaseLabel::Accusative => self.accusative,
			CaseLabel::Ergative => self.ergative,
			CaseLabel::Absolutive => self.absolutive,
			CaseLabel::Dative => self.dative,
			CaseLabel::Genitive => self.genitive,
			CaseLabel::Instrumental => self.instrumental,
			CaseLabel::Locative => self.locative,
			CaseLabel::Ablative => self.ablative,
			CaseLabel::Allative => self.allative,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdpositionStrategy {
	None,
	Preposition,
	Postposition,
	Circumposition,
}

impl AdpositionStrategy {
	pub fn apply(
		self,
		words: &mut Vec<SurfaceConstituent>,
		role: SemanticRole,
		host: Option<SemanticNode>,
	) {
		let Some(form) = role_adposition(role) else {
			return;
		};
		match self {
			Self::None => {}
			Self::Preposition => {
				Marking::Particle { form, before: true }.apply(
					words,
					ParticleDomain::Adposition,
					host,
				);
			}
			Self::Postposition => {
				Marking::Particle { form, before: false }.apply(
					words,
					ParticleDomain::Adposition,
					host,
				);
			}
			Self::Circumposition => {
				Marking::Particle { form, before: true }.apply(
					words,
					ParticleDomain::Adposition,
					host,
				);
				Marking::Particle { form: "ni", before: false }.apply(
					words,
					ParticleDomain::Adposition,
					host,
				);
			}
		}
	}
}

fn role_adposition(role: SemanticRole) -> Option<&'static str> {
	match role {
		SemanticRole::Goal => Some("to"),
		SemanticRole::Source => Some("from"),
		SemanticRole::Location => Some("at"),
		SemanticRole::Instrument => Some("with"),
		SemanticRole::Beneficiary | SemanticRole::Recipient => Some("for"),
		_ => None,
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TamStrategy {
	pub past: Marking,
	pub future: Marking,
	pub progressive: Marking,
	pub perfect: Marking,
	pub imperative: Marking,
}

impl TamStrategy {
	pub fn unmarked() -> Self {
		Self {
			past: Marking::Zero,
			future: Marking::Zero,
			progressive: Marking::Zero,
			perfect: Marking::Zero,
			imperative: Marking::Zero,
		}
	}

	pub fn apply(
		self,
		predicate: &mut Vec<SurfaceConstituent>,
		tense: Tense,
		aspect: Aspect,
		mood: Mood,
		host: SemanticNode,
	) {
		match tense {
			Tense::Past => self.past.apply(predicate, ParticleDomain::Tense, Some(host)),
			Tense::Future => self.future.apply(predicate, ParticleDomain::Tense, Some(host)),
			_ => {}
		}
		match aspect {
			Aspect::Progressive => {
				self.progressive.apply(predicate, ParticleDomain::Aspect, Some(host));
			}
			Aspect::Perfect => self.perfect.apply(predicate, ParticleDomain::Aspect, Some(host)),
			_ => {}
		}
		if mood == Mood::Imperative {
			self.imperative.apply(predicate, ParticleDomain::Mood, Some(host));
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolarityStrategy {
	pub pre: Marking,
	pub post: Marking,
}

impl PolarityStrategy {
	pub fn preverbal(form: &'static str) -> Self {
		Self { pre: Marking::Particle { form, before: true }, post: Marking::Zero }
	}

	pub fn postverbal(form: &'static str) -> Self {
		Self { pre: Marking::Zero, post: Marking::Particle { form, before: false } }
	}

	pub fn circumfix(pre: &'static str, post: &'static str) -> Self {
		Self {
			pre: Marking::Particle { form: pre, before: true },
			post: Marking::Particle { form: post, before: false },
		}
	}

	pub fn unmarked() -> Self {
		Self { pre: Marking::Zero, post: Marking::Zero }
	}
}
