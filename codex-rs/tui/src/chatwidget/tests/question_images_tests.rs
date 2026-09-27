//! Image pastes in async answers retain their attachment through normal input delivery.
use super::*;
use codex_protocol::items::AsyncUserInputQuestion;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn pasted_question_image_is_rendered_and_delivered_with_the_answer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("screenshot with spaces.png");
    image::RgbImage::new(/*width*/ 2, /*height*/ 2)
        .save(&path)
        .unwrap();
    for prefix in ["  See \"café\":\n", ""] {
        for queued in [false, true] {
            let (mut chat, _rx, mut ops) = make_chatwidget_manual(/*model_override*/ None).await;
            chat.thread_id = Some(ThreadId::new());
            chat.input_queue.suppress_queue_autosend = queued;
            chat.bottom_pane
                .set_composer_text("main draft".into(), Vec::new(), Vec::new());
            chat.add_async_questions(
                "image-question",
                &[AsyncUserInputQuestion {
                    title: "Show the problem".into(),
                    options: None,
                }],
            );
            chat.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT));
            chat.handle_paste(prefix.into());
            let pasted = if cfg!(unix) {
                path.to_string_lossy().replace(' ', "\\ ")
            } else {
                format!("\"{}\"", path.display())
            };
            chat.handle_paste(pasted);
            if !queued && !prefix.is_empty() {
                insta::assert_snapshot!(
                    "async_question_image_attachment",
                    render_bottom_popup(&chat, /*width*/ 80)
                );
            }
            chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
            assert_eq!(chat.bottom_pane.composer_text(), "main draft");
            assert_eq!(
                chat.bottom_pane
                    .questions
                    .as_ref()
                    .unwrap()
                    .unanswered_count(),
                0
            );
            if queued {
                let message = &chat
                    .input_queue
                    .queued_user_messages
                    .front()
                    .unwrap()
                    .user_message;
                assert_eq!(
                    message.local_images,
                    vec![crate::bottom_pane::LocalImageAttachment {
                        placeholder: "[Image #1]".into(),
                        path: path.clone(),
                    }]
                );
                let element = &message.text_elements[0];
                assert_eq!(
                    &message.text[element.byte_range.start..element.byte_range.end],
                    "[Image #1]"
                );
                let display = ChatWidget::user_message_display_from_parts(
                    message.text.clone(),
                    message.text_elements.clone(),
                    vec![path.clone()],
                    Vec::new(),
                );
                let element = &display.text_elements[0];
                assert_eq!(
                    &display.message[element.byte_range.start..element.byte_range.end],
                    "[Image #1]"
                );
                let restored = chat.pop_latest_queued_composer_state().unwrap();
                assert_eq!(restored.local_images[0].path, path);
                let element = &restored.text_elements[0];
                assert_eq!(
                    &restored.text[element.byte_range.start..element.byte_range.end],
                    "[Image #1]"
                );
                chat.restore_composer_state(restored);
                chat.input_queue.suppress_queue_autosend = false;
                chat.handle_key_event(KeyEvent::from(KeyCode::Enter));
                let Op::UserTurn { items, .. } = ops.try_recv().unwrap() else {
                    panic!("resubmitted image")
                };
                assert!(items.iter().any(|item| matches!(item, UserInput::LocalImage { path: attached, .. } if attached == &path)));
            } else {
                let Op::UserTurn { items, .. } = ops.try_recv().unwrap() else {
                    panic!("user turn")
                };
                assert!(
                    matches!(&items[0], UserInput::LocalImage { path: attached, .. } if attached == &path)
                );
                let UserInput::Text {
                    text,
                    text_elements,
                } = &items[1]
                else {
                    panic!("answer")
                };
                assert_eq!(
                    crate::async_question_reply::display_text(text).unwrap(),
                    format!("> Show the problem\n\n{}[Image #1]", prefix.trim_start())
                );
                let element = &text_elements[0];
                assert_eq!(
                    &text[element.byte_range.start..element.byte_range.end],
                    "[Image #1]"
                );
                assert!(crate::async_question_reply::parse_input(&items).is_some());
            }
        }
    }
}
