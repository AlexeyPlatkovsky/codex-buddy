//! Approximate, local-only reporting for model-visible request context.

use crate::client_common::Prompt;
use crate::context_manager::estimate_item_token_count;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::ContextUsage;
use codex_utils_output_truncation::approx_token_count;
use codex_utils_output_truncation::approx_tokens_from_byte_count_i64;

/// Estimates the model-visible token cost of a fully constructed request.
///
/// The result intentionally mirrors the local context-window estimator rather than attempting
/// tokenizer-accurate or billable accounting. It is sent only to the local client and is never
/// included in a model request.
pub(crate) fn estimate_prompt_context_usage(prompt: &Prompt) -> ContextUsage {
    let mut context_usage = ContextUsage {
        base_instructions_tokens: approximate_text_tokens(&prompt.base_instructions.text),
        tool_definitions_tokens: approximate_tool_tokens(prompt),
        ..ContextUsage::default()
    };

    for item in &prompt.input {
        let tokens = estimate_item_token_count(item);
        match item {
            ResponseItem::AdditionalTools { .. } => {
                context_usage.tool_definitions_tokens =
                    context_usage.tool_definitions_tokens.saturating_add(tokens);
            }
            ResponseItem::Message { role, .. } if role == "assistant" => {
                context_usage.assistant_and_reasoning_tokens = context_usage
                    .assistant_and_reasoning_tokens
                    .saturating_add(tokens);
            }
            ResponseItem::Message { role, .. } if matches!(role.as_str(), "user" | "developer") => {
                context_usage.user_and_developer_tokens = context_usage
                    .user_and_developer_tokens
                    .saturating_add(tokens);
            }
            ResponseItem::AgentMessage { .. }
            | ResponseItem::Reasoning { .. }
            | ResponseItem::Compaction { .. }
            | ResponseItem::ContextCompaction { .. } => {
                context_usage.assistant_and_reasoning_tokens = context_usage
                    .assistant_and_reasoning_tokens
                    .saturating_add(tokens);
            }
            ResponseItem::LocalShellCall { .. }
            | ResponseItem::FunctionCall { .. }
            | ResponseItem::ToolSearchCall { .. }
            | ResponseItem::FunctionCallOutput { .. }
            | ResponseItem::CustomToolCall { .. }
            | ResponseItem::CustomToolCallOutput { .. }
            | ResponseItem::ToolSearchOutput { .. }
            | ResponseItem::WebSearchCall { .. }
            | ResponseItem::ImageGenerationCall { .. } => {
                context_usage.tool_activity_tokens =
                    context_usage.tool_activity_tokens.saturating_add(tokens);
            }
            ResponseItem::Message { .. }
            | ResponseItem::CompactionTrigger { .. }
            | ResponseItem::Other => {
                context_usage.other_tokens = context_usage.other_tokens.saturating_add(tokens);
            }
        }
    }

    context_usage.total_tokens = context_usage
        .base_instructions_tokens
        .saturating_add(context_usage.tool_definitions_tokens)
        .saturating_add(context_usage.user_and_developer_tokens)
        .saturating_add(context_usage.assistant_and_reasoning_tokens)
        .saturating_add(context_usage.tool_activity_tokens)
        .saturating_add(context_usage.other_tokens);
    context_usage
}

fn approximate_text_tokens(text: &str) -> i64 {
    i64::try_from(approx_token_count(text)).unwrap_or(i64::MAX)
}

fn approximate_tool_tokens(prompt: &Prompt) -> i64 {
    serde_json::to_vec(prompt.tools.as_ref())
        .map(|serialized| approx_tokens_from_byte_count_i64(serialized.len() as i64))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "context_usage_tests.rs"]
mod tests;
