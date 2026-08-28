use std::time::Duration;

use anyhow::Result;
use app_test_support::TestAppServer;
use codex_app_server_protocol::JSONRPCError;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::ReviewDelivery;
use codex_app_server_protocol::ReviewStartParams;
use codex_app_server_protocol::ReviewTarget;
use pretty_assertions::assert_eq;
use serde_json::json;
use tokio::time::timeout;

const READ_TIMEOUT: Duration = Duration::from_secs(/*secs*/ 10);
const INVALID_REQUEST_ERROR_CODE: i64 = -32600;

#[cfg(not(feature = "queue"))]
#[tokio::test]
async fn queue_methods_remain_known_and_report_unavailable() -> Result<()> {
    let mut app = TestAppServer::builder().build_initialized().await?;
    let thread_id = "00000000-0000-0000-0000-000000000000";
    let requests = [
        (
            "thread/queue/add",
            json!({
                "threadId": thread_id,
                "input": [],
                "clientUserMessageId": "queued-message",
            }),
        ),
        (
            "thread/queue/list",
            json!({"threadId": thread_id, "cursor": null, "limit": null}),
        ),
        (
            "thread/queue/update",
            json!({
                "threadId": thread_id,
                "queuedSubmissionId": "queued-message",
                "input": [],
            }),
        ),
        (
            "thread/queue/delete",
            json!({"threadId": thread_id, "queuedSubmissionId": "queued-message"}),
        ),
        (
            "thread/queue/reorder",
            json!({"threadId": thread_id, "queuedSubmissionIds": []}),
        ),
        (
            "thread/queue/start",
            json!({"threadId": thread_id, "queuedSubmissionId": "queued-message"}),
        ),
    ];

    for (method, params) in requests {
        let request_id = app.send_raw_request(method, Some(params)).await?;
        let error: JSONRPCError = timeout(
            READ_TIMEOUT,
            app.read_stream_until_error_message(RequestId::Integer(request_id)),
        )
        .await??;
        assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
        assert_eq!(error.error.message, "user message queue is unavailable");
    }

    Ok(())
}

#[cfg(not(feature = "detached-review"))]
#[tokio::test]
async fn detached_review_method_remains_known_and_reports_unavailable() -> Result<()> {
    let mut app = TestAppServer::builder().build_initialized().await?;
    let request_id = app
        .send_review_start_request(ReviewStartParams {
            thread_id: "00000000-0000-0000-0000-000000000000".to_string(),
            target: ReviewTarget::Custom {
                instructions: "review this change".to_string(),
            },
            delivery: Some(ReviewDelivery::Detached),
        })
        .await?;
    let error: JSONRPCError = timeout(
        READ_TIMEOUT,
        app.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;
    assert_eq!(error.error.code, INVALID_REQUEST_ERROR_CODE);
    assert_eq!(
        error.error.message,
        "detached review is unavailable in this build"
    );

    Ok(())
}
