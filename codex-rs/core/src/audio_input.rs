#[cfg(not(feature = "audio-input"))]
use codex_protocol::models::ContentItem;
#[cfg(not(feature = "audio-input"))]
use codex_protocol::models::FunctionCallOutputContentItem;
#[cfg(not(feature = "audio-input"))]
use codex_protocol::models::ResponseItem;

#[cfg(feature = "audio-input")]
pub(crate) use codex_utils_audio::estimate_audio_token_count;
#[cfg(feature = "audio-input")]
pub(crate) use codex_utils_audio::prepare_response_items;

#[cfg(not(feature = "audio-input"))]
const UNSUPPORTED_AUDIO_PLACEHOLDER: &str =
    "audio content omitted because this build does not support audio input";

#[cfg(not(feature = "audio-input"))]
pub(crate) fn estimate_audio_token_count(audio_url: &str) -> usize {
    codex_utils_string::approx_token_count(audio_url)
}

#[cfg(not(feature = "audio-input"))]
pub(crate) fn prepare_response_items(items: &mut [ResponseItem]) {
    for item in items {
        match item {
            ResponseItem::Message { content, .. } => {
                for item in content {
                    if matches!(item, ContentItem::InputAudio { .. }) {
                        *item = ContentItem::InputText {
                            text: UNSUPPORTED_AUDIO_PLACEHOLDER.to_string(),
                        };
                    }
                }
            }
            ResponseItem::FunctionCallOutput { output, .. }
            | ResponseItem::CustomToolCallOutput { output, .. } => {
                if let Some(content) = output.content_items_mut() {
                    for item in content {
                        if matches!(item, FunctionCallOutputContentItem::InputAudio { .. }) {
                            *item = FunctionCallOutputContentItem::InputText {
                                text: UNSUPPORTED_AUDIO_PLACEHOLDER.to_string(),
                            };
                        }
                    }
                }
            }
            ResponseItem::AdditionalTools { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::AgentMessage { .. }
            | ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::ContextCompaction { .. }
            | ResponseItem::Other => {}
        }
    }
}

#[cfg(all(test, not(feature = "audio-input")))]
#[path = "audio_input_tests.rs"]
mod tests;
