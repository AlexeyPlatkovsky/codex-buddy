use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use codex_runtime_profile::RuntimeCompileCeiling;
use codex_runtime_profile::RuntimePreset;
use core_test_support::apps_test_server::AppsTestServer;
use core_test_support::responses;
use core_test_support::responses::namespace_child_tool;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;

const SERVER_NAME: &str = "explicit";
const MCP_PATH: &str = "/api/codex/ps/mcp";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn coding_explicit_mcp_starts_on_catalog_demand_and_reuses_connection() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = responses::start_mock_server().await;
    let (apps_server, startup) = AppsTestServer::mount_with_startup_control(&server).await?;
    let mcp_url = format!("{}{MCP_PATH}", apps_server.chatgpt_base_url);
    let config_toml = format!(
        r#"
[runtime.sources]
mcp = "explicit-only"

[mcp_servers.{SERVER_NAME}]
url = "{mcp_url}"
enabled_tools = ["calendar_create_event"]
startup_timeout_sec = 10
"#
    );
    let test = test_codex()
        .with_pre_build_hook(move |codex_home| {
            std::fs::write(codex_home.join("config.toml"), config_toml)
                .expect("write explicit MCP user config");
        })
        .with_config(|config| {
            config.runtime_profile = codex_runtime_profile::ResolvedRuntimeProfile::resolve(
                RuntimePreset::Coding,
                &RuntimeCompileCeiling::full(),
                config.runtime_profile_policy.restrictions(),
            );
        })
        .build_with_auto_env(&server)
        .await?;

    assert_eq!(startup.initialize_attempts(), 0);
    assert_eq!(mcp_request_count(&server).await, 0);

    let response = responses::mount_sse_sequence(
        &server,
        vec![
            tool_search_response("first-search-response", "first-search"),
            mcp_call_response("first-response", "first-call", "First"),
            completion_response("first-complete"),
            tool_search_response("second-search-response", "second-search"),
            mcp_call_response("second-response", "second-call", "Second"),
            completion_response("second-complete"),
        ],
    )
    .await;

    for (turn, expected_request_count) in [("first", 3), ("second", 6)] {
        test.codex
            .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
                text: format!("Use the explicit calendar tool for the {turn} event."),
                text_elements: Vec::new(),
            }]))
            .await?;
        wait_for_event(&test.codex, |event| {
            matches!(event, EventMsg::TurnComplete(_))
        })
        .await;

        assert_eq!(response.requests().len(), expected_request_count);
        assert_eq!(startup.initialize_attempts(), 1);
    }

    let requests = response.requests();
    for (request, search_call_id) in [
        (&requests[1], "first-search"),
        (&requests[4], "second-search"),
    ] {
        let search_output = request.tool_search_output(search_call_id);
        assert!(
            namespace_child_tool(&search_output, "mcp__explicit", "calendar_create_event")
                .is_some(),
            "configured MCP tool must be returned by model-visible discovery: {search_output}"
        );
    }
    assert!(requests[2].function_call_output("first-call").is_object());
    assert!(requests[5].function_call_output("second-call").is_object());

    Ok(())
}

fn tool_search_response(response_id: &str, call_id: &str) -> String {
    responses::sse(vec![
        responses::ev_response_created(response_id),
        responses::ev_tool_search_call(call_id, &json!({ "query": "calendar_create_event" })),
        responses::ev_completed(response_id),
    ])
}

fn mcp_call_response(response_id: &str, call_id: &str, title: &str) -> String {
    responses::sse(vec![
        responses::ev_response_created(response_id),
        responses::ev_function_call_with_namespace(
            call_id,
            "mcp__explicit",
            "calendar_create_event",
            &json!({
                "title": title,
                "starts_at": "2026-08-27T12:00:00Z",
            })
            .to_string(),
        ),
        responses::ev_completed(response_id),
    ])
}

fn completion_response(response_id: &str) -> String {
    responses::sse(vec![
        responses::ev_assistant_message(&format!("{response_id}-message"), "done"),
        responses::ev_completed(response_id),
    ])
}

async fn mcp_request_count(server: &wiremock::MockServer) -> usize {
    server
        .received_requests()
        .await
        .expect("mock server should record requests")
        .into_iter()
        .filter(|request| {
            request.url.path() == MCP_PATH && serde_json::from_slice::<Value>(&request.body).is_ok()
        })
        .count()
}
