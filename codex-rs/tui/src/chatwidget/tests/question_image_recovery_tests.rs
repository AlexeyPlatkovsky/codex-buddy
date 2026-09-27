//! Recover image-bearing answers through the live turn-completion notification route.

use super::*;
use codex_protocol::items::AsyncUserInputQuestion;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn completed_turn_recovers_question_images_beside_existing_attachments() {
    let (mut chat, _rx, mut ops) = make_chatwidget_manual(/*model_override*/ None).await;
    chat.thread_id = Some(ThreadId::new());
    handle_turn_started(&mut chat, "turn");
    let dir = tempfile::tempdir().unwrap();
    let main_image = dir.path().join("main.png");
    let answer_image = dir.path().join("answer.png");
    for path in [&main_image, &answer_image] {
        image::RgbImage::new(/*width*/ 2, /*height*/ 2)
            .save(path)
            .unwrap();
    }
    chat.handle_paste(main_image.display().to_string());
    chat.add_async_questions(
        "question",
        &[AsyncUserInputQuestion {
            title: "Which image?".into(),
            options: None,
        }],
    );
    chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
    chat.handle_paste(answer_image.display().to_string());
    handle_turn_completed(&mut chat, "turn", /*duration_ms*/ None);

    assert_eq!(
        (
            chat.bottom_pane.composer_text(),
            chat.bottom_pane.composer_local_images(),
            chat.bottom_pane.question_editor().unanswered_count(),
        ),
        (
            "[Image #1] \n[Image #2]".into(),
            vec![
                LocalImageAttachment {
                    placeholder: "[Image #1]".into(),
                    path: main_image,
                },
                LocalImageAttachment {
                    placeholder: "[Image #2]".into(),
                    path: answer_image,
                },
            ],
            0,
        )
    );
    insta::assert_snapshot!(
        chat.bottom_pane.composer_text(),
        @"[Image #1] \n[Image #2]"
    );
    assert!(ops.try_recv().is_err());
}
