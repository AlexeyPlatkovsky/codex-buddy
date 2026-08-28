use std::time::Duration;

use anyhow::Result;
use app_test_support::TestAppServer;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::timeout;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const EXTERNAL_AGENT_CONFIG_MIGRATION_UNAVAILABLE: &str =
    "external agent config migration is unavailable in this runtime profile";

#[tokio::test]
async fn external_agent_config_requests_are_unavailable_without_full_runtime_extensions()
-> Result<()> {
    let mut app_server = TestAppServer::builder()
        .without_auto_env()
        .build_initialized_with_timeout(DEFAULT_TIMEOUT)
        .await?;

    for (method, params) in [
        ("externalAgentConfig/detect", Some(json!({}))),
        (
            "externalAgentConfig/import",
            Some(json!({ "migrationItems": [] })),
        ),
        (
            "externalAgentConfig/import/recordHistory",
            Some(json!({ "providerId": "test", "itemTypeResults": [] })),
        ),
        ("externalAgentConfig/import/readHistories", None),
    ] {
        let request_id = app_server.send_raw_request(method, params).await?;
        let error: JSONRPCError = timeout(
            DEFAULT_TIMEOUT,
            app_server.read_stream_until_error_message(RequestId::Integer(request_id)),
        )
        .await??;

        assert_eq!(error.error.code, -32600);
        assert_eq!(
            error.error.message,
            EXTERNAL_AGENT_CONFIG_MIGRATION_UNAVAILABLE
        );
    }

    Ok(())
}
