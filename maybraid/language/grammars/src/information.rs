//! Topic and focus realization hooks.

use maybraid_language_core::{
	FocusTarget, InformationStructure, ParticleDomain, SemanticNode, SurfaceConstituent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InformationStrategy {
	pub topic_particle: Option<&'static str>,
	pub focus_particle: Option<&'static str>,
	pub front_topic: bool,
}

impl InformationStrategy {
	pub fn unmarked() -> Self {
		Self { topic_particle: None, focus_particle: None, front_topic: false }
	}

	pub fn apply(self, words: &mut Vec<SurfaceConstituent>, information: &InformationStructure) {
		if let Some(topic) = information.topic {
			let host = SemanticNode::Referent(topic);
			if let Some(form) = self.topic_particle {
				let particle = SurfaceConstituent::Particle {
					form: form.to_owned(),
					domain: ParticleDomain::Topic,
					host: Some(host),
				};
				if let Some(index) = index_of_node(words, host) {
					words.insert(index + 1, particle);
					if self.front_topic {
						front_span(words, index, index + 2);
					}
				} else {
					words.insert(0, particle);
				}
			} else if self.front_topic {
				if let Some(index) = index_of_node(words, host) {
					front_span(words, index, index + 1);
				}
			}
		}
		if let Some(FocusTarget::Referent(focus)) = information.focus {
			if let Some(form) = self.focus_particle {
				let host = SemanticNode::Referent(focus);
				if let Some(index) = index_of_node(words, host) {
					words.insert(
						index + 1,
						SurfaceConstituent::Particle {
							form: form.to_owned(),
							domain: ParticleDomain::Focus,
							host: Some(host),
						},
					);
				}
			}
		}
	}
}

fn index_of_node(words: &[SurfaceConstituent], node: SemanticNode) -> Option<usize> {
	words.iter().position(|item| item.node() == Some(node))
}

fn front_span(words: &mut Vec<SurfaceConstituent>, start: usize, end: usize) {
	if start == 0 || start >= words.len() || end > words.len() || start >= end {
		return;
	}
	let drained: Vec<_> = words.drain(start..end).collect();
	for (offset, item) in drained.into_iter().enumerate() {
		words.insert(offset, item);
	}
}
