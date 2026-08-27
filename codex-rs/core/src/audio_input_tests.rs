use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;
use pretty_assertions::assert_eq;

use super::UNSUPPORTED_AUDIO_PLACEHOLDER;
use super::estimate_audio_token_count;
use super::prepare_response_items;

#[test]
fn slim_build_replaces_message_and_tool_audio() {
    let mut items = vec![
        ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputAudio {
                audio_url: "data:audio/wav;base64,YXVkaW8=".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        },
        ResponseItem::FunctionCallOutput {
            id: None,
            call_id: Some("call-1".to_string()),
            name: None,
            namespace: None,
            output: FunctionCallOutputPayload::from_content_items(vec![
                FunctionCallOutputContentItem::InputAudio {
                    audio_url: "data:audio/ogg;base64,YXVkaW8=".to_string(),
                },
            ]),
            internal_chat_message_metadata_passthrough: None,
        },
    ];

    prepare_response_items(&mut items);

    assert_eq!(
        items,
        vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: UNSUPPORTED_AUDIO_PLACEHOLDER.to_string(),
                }],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: Some("call-1".to_string()),
                name: None,
                namespace: None,
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputText {
                        text: UNSUPPORTED_AUDIO_PLACEHOLDER.to_string(),
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
        ]
    );
}

#[test]
fn slim_build_estimates_audio_from_bounded_text_size() {
    let audio_url = "data:audio/wav;base64,YXVkaW8=";

    assert_eq!(
        estimate_audio_token_count(audio_url),
        codex_utils_string::approx_token_count(audio_url)
    );
}
