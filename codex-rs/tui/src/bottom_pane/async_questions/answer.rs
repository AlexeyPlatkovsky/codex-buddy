//! Build ordinary user-message submissions without losing image attachments or chip ranges.

use super::*;
use codex_context_fragments::AnsweredQuestion;
use codex_context_fragments::ContextualUserFragment;

impl AsyncQuestions {
    #[allow(
        clippy::expect_used,
        reason = "AnsweredQuestion emits the answer field for bounded IDs, and serializing a string cannot fail"
    )]
    pub(super) fn go_next_or_submit(&mut self) {
        self.save_current_draft();
        if !self.delivery_enabled {
            return;
        }
        let Some(answer) = self.current_answer() else {
            return;
        };
        let selected = answer
            .options_state
            .selected_idx
            .and_then(|index| answer.question.options.as_ref()?.get(index))
            .map(String::as_str)
            .unwrap_or_default();
        // Only a fully displayed model-authored option may become user authorization.
        let (first, count) = self.visible_options.get();
        let index = self.selected_option_index().unwrap_or(0);
        if !self.focus_is_notes() && !(first..first + count).contains(&index) {
            self.composer.show_footer_flash(
                "Expand terminal to read the entire option".into(),
                Duration::from_secs(5),
            );
            return;
        }
        let notes = self.focus_is_notes();
        let (raw_text, elements) = if notes {
            ChatComposer::expand_pending_pastes(
                &self.composer.current_text(),
                self.composer.text_elements(),
                &self.composer.pending_pastes(),
            )
        } else {
            (selected.to_string(), Vec::new())
        };
        let text = raw_text.trim();
        if text.is_empty() {
            return;
        }
        let reply =
            AnsweredQuestion::new(&answer.question_id, &answer.question.title, text).render();
        if reply.chars().count() > codex_protocol::user_input::MAX_USER_INPUT_TEXT_CHARS {
            self.composer.show_footer_flash(
                "Answer too long; shorten it before sending".into(),
                Duration::from_secs(5),
            );
        } else {
            let mut message = crate::chatwidget::UserMessage::from(reply);
            if notes {
                message.local_images = self.composer.local_images();
                message.remote_image_urls = self.composer.remote_image_urls();
                // The reply envelope JSON-escapes the answer. Keep chip ranges anchored to the
                // encoded answer so queue combination can still renumber actual attachments.
                let encoded = answer.question_id.len() <= 512;
                let offset = if encoded {
                    message.text.find("\"answer\":").expect("answer envelope")
                        + "\"answer\":\"".len()
                } else {
                    message.text.len() - text.len()
                };
                let trimmed = raw_text.len() - raw_text.trim_start().len();
                message.text_elements = elements
                    .into_iter()
                    .filter_map(|element| {
                        let start = element.byte_range.start.checked_sub(trimmed)?;
                        let end = element.byte_range.end.checked_sub(trimmed)?;
                        if end > text.len() {
                            return None;
                        }
                        let encoded_position = |position| {
                            if encoded {
                                serde_json::to_string(&text[..position])
                                    .expect("serialize answer")
                                    .len()
                                    - 2
                            } else {
                                position
                            }
                        };
                        Some(element.map_range(|_| {
                            (offset + encoded_position(start)..offset + encoded_position(end))
                                .into()
                        }))
                    })
                    .collect();
            }
            self.submission = Some(QuestionSubmission::Submit(message));
        }
    }
}
