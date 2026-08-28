use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn goal_get_is_explicitly_unavailable_without_goal_extension() {
    let result = ThreadGoalRequestProcessor::new()
        .thread_goal_get(ThreadGoalGetParams {
            thread_id: "ignored".to_string(),
        })
        .await;

    assert_eq!(
        result.unwrap_err(),
        invalid_request("goals are unavailable in this Codex runtime")
    );
}
