use std::time::Duration;

use pretty_assertions::assert_eq;

use super::CellId;
use super::RuntimeResponse;
use super::WaitOutcome;

fn result_response() -> RuntimeResponse {
    RuntimeResponse::Result {
        cell_id: CellId::new("cell-1".to_string()),
        content_items: Vec::new(),
        error_text: None,
        code_mode_host_duration: None,
    }
}

#[test]
fn runtime_response_records_host_duration() {
    let response = result_response();

    assert_eq!(response.code_mode_host_duration(), None);
    assert_eq!(
        response
            .with_code_mode_host_duration(Duration::from_millis(/*millis*/ 42))
            .code_mode_host_duration(),
        Some(Duration::from_millis(/*millis*/ 42))
    );
}

#[test]
fn wait_outcome_preserves_host_duration() {
    let outcome = WaitOutcome::MissingCell(result_response());

    assert_eq!(
        outcome
            .with_code_mode_host_duration(Duration::ZERO)
            .code_mode_host_duration(),
        Some(Duration::ZERO)
    );
}
