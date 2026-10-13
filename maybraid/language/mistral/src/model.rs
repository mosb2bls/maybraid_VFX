//! Load Qwen once and reuse it for English responses.

use std::path::Path;

use mistralrs::{DeviceMapSetting, GgufModelBuilder, Model, RequestBuilder, TextMessageRole};
use serde_json::Value;

use crate::config::{MistralLanguageConfig, ResponseGenerationConfig};
use crate::device::InferenceDevice;
use crate::error::MistralLanguageError;
use crate::prompt::RespondPrompt;

/// Loaded `mistral.rs` model. Callers retain this across requests.
pub struct MistralLanguageModel {
	model: Model,
	respond: ResponseGenerationConfig,
}

/// English response request. Context is a bounded JSON blob, never ECS handles.
#[derive(Clone, Debug)]
pub struct ResponseRequest<'a> {
	pub input: &'a str,
	pub context: Value,
}

impl<'a> ResponseRequest<'a> {
	pub fn new(input: &'a str) -> Self {
		Self { input, context: RespondPrompt::empty_context() }
	}

	pub fn with_context(mut self, context: Value) -> Self {
		self.context = context;
		self
	}
}

impl MistralLanguageModel {
	pub async fn load(config: MistralLanguageConfig) -> Result<Self, MistralLanguageError> {
		let path = config.model_path;
		if !path.is_file() {
			return Err(MistralLanguageError::ModelMissing { path });
		}
		let file_name = path
			.file_name()
			.and_then(|name| name.to_str())
			.ok_or_else(|| {
				MistralLanguageError::load(&path, "model path is not a utf-8 file name")
			})?
			.to_owned();
		let dir = path.parent().unwrap_or_else(|| Path::new("."));
		// Auto device mapping treats macOS CPU as 0MB and refuses a 1.7B Q4.
		// Dummy mapping loads onto the selected GPU (or CPU) instead.
		// https://github.com/EricLBuehler/mistral.rs/issues/2078
		let device = InferenceDevice::select(config.force_cpu)?;
		if !InferenceDevice::is_gpu(&device) && !config.force_cpu && InferenceDevice::must_use_cpu()
		{
			eprintln!(
				"maybraid-language: Candle 0.11 Metal GEMM kernels fail on this macOS compiler; using CPU. Set MAYBRAID_LANGUAGE_FORCE_METAL=1 to try the GPU anyway."
			);
		}
		let model = GgufModelBuilder::new(dir.display().to_string(), vec![file_name])
			.with_device(device)
			.with_device_mapping(DeviceMapSetting::dummy())
			.with_logging()
			.with_throughput_logging()
			.build()
			.await
			.map_err(|error| MistralLanguageError::load(&path, error))?;
		Ok(Self { model, respond: config.respond })
	}

	pub async fn respond(
		&self,
		request: ResponseRequest<'_>,
	) -> Result<String, MistralLanguageError> {
		let builder = RequestBuilder::new()
			.add_message(TextMessageRole::System, RespondPrompt::SYSTEM)
			.add_message(
				TextMessageRole::User,
				RespondPrompt::user(request.input, &request.context),
			)
			.set_sampler_temperature(f64::from(self.respond.temperature))
			.set_sampler_topp(f64::from(self.respond.top_p))
			.set_sampler_max_len(self.respond.max_len)
			.enable_thinking(false);
		self.complete(builder).await
	}

	async fn complete(&self, request: RequestBuilder) -> Result<String, MistralLanguageError> {
		let response = self
			.model
			.send_chat_request(request)
			.await
			.map_err(MistralLanguageError::inference)?;
		response
			.choices
			.into_iter()
			.next()
			.and_then(|choice| choice.message.content)
			.filter(|content| !content.trim().is_empty())
			.ok_or_else(|| MistralLanguageError::inference("model returned no content"))
	}
}
