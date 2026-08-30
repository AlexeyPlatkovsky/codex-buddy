use super::*;
use codex_protocol::models::BaseInstructions;
use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
use codex_protocol::models::WebSearchAction;
use pretty_assertions::assert_eq;

#[test]
fn categorizes_model_visible_response_items() {
    let prompt = Prompt {
        base_instructions: BaseInstructions {
            text: "base instructions".to_string(),
            provenance: None,
        },
        input: vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "user request".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::Message {
                id: None,
                role: "assistant".to_string(),
                content: vec![ContentItem::OutputText {
                    text: "assistant reply".to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::WebSearchCall {
                id: None,
                status: Some("completed".to_string()),
                action: Some(WebSearchAction::Search {
                    query: Some("Codex Buddy".to_string()),
                    queries: None,
                }),
                internal_chat_message_metadata_passthrough: None,
            },
        ],
        ..Prompt::default()
    };

    let context_usage = estimate_prompt_context_usage(&prompt);

    assert!(context_usage.base_instructions_tokens > 0);
    assert!(context_usage.user_and_developer_tokens > 0);
    assert!(context_usage.assistant_and_reasoning_tokens > 0);
    assert!(context_usage.tool_activity_tokens > 0);
    assert_eq!(
        context_usage.total_tokens,
        context_usage
            .base_instructions_tokens
            .saturating_add(context_usage.tool_definitions_tokens)
            .saturating_add(context_usage.user_and_developer_tokens)
            .saturating_add(context_usage.assistant_and_reasoning_tokens)
            .saturating_add(context_usage.tool_activity_tokens)
            .saturating_add(context_usage.other_tokens)
    );
}
