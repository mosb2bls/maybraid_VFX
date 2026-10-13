//! Noun-phrase composers: modifiers, number, definiteness.

use maybraid_language_core::{
	AffixPlacement, Definiteness, ModifierPlacement, Number, ParticleDomain, SemanticNode,
	SurfaceConstituent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberStrategy {
	Zero,
	Particle { form: &'static str },
	Prefix { form: &'static str },
	Suffix { form: &'static str },
	Reduplicate,
}

impl NumberStrategy {
	pub fn apply(self, words: &mut Vec<SurfaceConstituent>, number: Number, host: SemanticNode) {
		if !matches!(number, Number::Plural | Number::Many) {
			return;
		}
		match self {
			Self::Zero => {}
			Self::Particle { form } => words.push(SurfaceConstituent::Particle {
				form: form.to_owned(),
				domain: ParticleDomain::Number,
				host: Some(host),
			}),
			Self::Prefix { form } => words.insert(
				0,
				SurfaceConstituent::Affix {
					form: form.to_owned(),
					domain: ParticleDomain::Number,
					host,
					placement: AffixPlacement::Prefix,
				},
			),
			Self::Suffix { form } => words.push(SurfaceConstituent::Affix {
				form: form.to_owned(),
				domain: ParticleDomain::Number,
				host,
				placement: AffixPlacement::Suffix,
			}),
			Self::Reduplicate => {
				if let Some(noun) = words.iter().rev().find_map(|item| match item {
					SurfaceConstituent::Lexical { form, node, .. } if *node == host => {
						Some(form.clone())
					}
					_ => None,
				}) {
					words.push(SurfaceConstituent::Particle {
						form: noun,
						domain: ParticleDomain::Number,
						host: Some(host),
					});
				}
			}
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DefinitenessStrategy {
	Zero,
	Article { definite: &'static str, indefinite: &'static str },
}

impl DefinitenessStrategy {
	pub fn apply(
		self,
		words: &mut Vec<SurfaceConstituent>,
		definiteness: Definiteness,
		host: SemanticNode,
	) {
		let Self::Article { definite, indefinite } = self else {
			return;
		};
		let form = match definiteness {
			Definiteness::Definite => definite,
			Definiteness::Indefinite | Definiteness::Generic => indefinite,
			Definiteness::Proper => return,
		};
		if form.is_empty() {
			return;
		}
		words.insert(
			0,
			SurfaceConstituent::Particle {
				form: form.to_owned(),
				domain: ParticleDomain::Definiteness,
				host: Some(host),
			},
		);
	}
}

pub fn place_modifiers(
	placement: ModifierPlacement,
	noun: SurfaceConstituent,
	modifiers: Vec<SurfaceConstituent>,
	linker: Option<&'static str>,
) -> Vec<SurfaceConstituent> {
	let mut words = Vec::new();
	let link = linker
		.filter(|_| !modifiers.is_empty())
		.map(|form| SurfaceConstituent::particle(form, ParticleDomain::NounPhrase));
	match placement {
		ModifierPlacement::BeforeNoun => {
			words.extend(modifiers);
			words.extend(link);
			words.push(noun);
		}
		ModifierPlacement::AfterNoun => {
			words.push(noun);
			words.extend(link);
			words.extend(modifiers);
		}
	}
	words
}
