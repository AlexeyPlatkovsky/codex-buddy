use anyhow::Result;
use codex_core::StartThreadOptions;
use codex_core::TurnInputRequest;
use codex_features::Feature;
use codex_protocol::dynamic_tools::DynamicToolFunctionSpec;
use codex_protocol::dynamic_tools::DynamicToolSpec;
use codex_protocol::openai_models::ApplyPatchToolType;
use codex_protocol::openai_models::ConfigShellToolType;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use codex_runtime_profile::ResolvedRuntimeProfile;
use codex_runtime_profile::RuntimeCompileCeiling;
use codex_runtime_profile::RuntimePolicyPatch;
use core_test_support::responses;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coding_runtime_profile_sends_only_the_coding_tool_inventory() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let response = responses::mount_sse_once(
        &server,
        responses::sse(vec![ev_response_created("resp-1"), ev_completed("resp-1")]),
    )
    .await;
    let test = test_codex()
        .with_model_info_override("gpt-5.5", |model| {
            model.shell_type = ConfigShellToolType::UnifiedExec;
            model.apply_patch_tool_type = Some(ApplyPatchToolType::Freeform);
            model.supports_search_tool = false;
        })
        .with_config(|config| {
            config.runtime_profile = ResolvedRuntimeProfile::coding(
                &RuntimeCompileCeiling::full(),
                &RuntimePolicyPatch::default(),
            );
            for feature in [
                Feature::ShellTool,
                Feature::RequestPermissionsTool,
                Feature::ViewImage,
                Feature::Collab,
            ] {
                config
                    .features
                    .enable(feature)
                    .expect("coding baseline feature should be enableable");
            }
            config
                .features
                .disable(Feature::MultiAgentV2)
                .expect("v1 collaboration should be selectable for this test");
        })
        .build_with_auto_env(&server)
        .await?;

    let thread = test
        .thread_manager
        .start_thread(StartThreadOptions {
            dynamic_tools: vec![DynamicToolSpec::Function(DynamicToolFunctionSpec {
                name: "client_echo".to_string(),
                description: "A session-supplied client tool.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false,
                }),
                defer_loading: false,
            })],
            ..StartThreadOptions::new(test.config.clone())
        })
        .await?
        .thread;

    thread
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Inspect the workspace.".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&thread, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let request = response.single_request();
    let request_body = request.body_json();
    let tools = request_body["tools"]
        .as_array()
        .expect("outbound request tools should be an array");
    let tool_inventory = tools
        .iter()
        .map(|tool| {
            (
                tool["type"].as_str().expect("tool type").to_string(),
                tool["name"].as_str().unwrap_or_default().to_string(),
                tool["parameters"]
                    .get("type")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        tool_inventory,
        vec![
            (
                "function".to_string(),
                "exec_command".to_string(),
                Some("object".to_string())
            ),
            (
                "function".to_string(),
                "write_stdin".to_string(),
                Some("object".to_string())
            ),
            (
                "function".to_string(),
                "request_user_input".to_string(),
                Some("object".to_string()),
            ),
            (
                "function".to_string(),
                "request_permissions".to_string(),
                Some("object".to_string()),
            ),
            ("custom".to_string(), "apply_patch".to_string(), None),
            (
                "function".to_string(),
                "view_image".to_string(),
                Some("object".to_string())
            ),
            ("namespace".to_string(), "multi_agent_v1".to_string(), None),
            ("web_search".to_string(), "".to_string(), None),
        ]
    );

    Ok(())
}
