//! Recover unsent answers without turning image chips into ordinary text.

use super::*;
use codex_protocol::models::local_image_label_text;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QuestionDraft {
    pub(in crate::bottom_pane) text: String,
    text_elements: Vec<TextElement>,
    local_image_paths: Vec<PathBuf>,
    remote_image_urls: Vec<String>,
}

impl ComposerDraft {
    pub(in crate::bottom_pane) fn question_draft(&self) -> Option<QuestionDraft> {
        let (text, elements) = ChatComposer::expand_pending_pastes(
            &self.text,
            self.text_elements.clone(),
            &self.pending_pastes,
        );
        let start = text.len() - text.trim_start().len();
        let end = text.trim_end().len();
        if start >= end && self.local_image_paths.is_empty() && self.remote_image_urls.is_empty() {
            return None;
        }
        let text_elements = elements
            .into_iter()
            .filter(|element| element.byte_range.start >= start && element.byte_range.end <= end)
            .map(|element| {
                element.map_range(|range| (range.start - start..range.end - start).into())
            })
            .collect();
        Some(QuestionDraft {
            text: text.trim().to_owned(),
            text_elements,
            local_image_paths: self.local_image_paths.clone(),
            remote_image_urls: self.remote_image_urls.clone(),
        })
    }
}

impl ChatComposer {
    pub(in crate::bottom_pane) fn append_recovered_question_drafts(
        &mut self,
        drafts: &[QuestionDraft],
    ) {
        if drafts
            .iter()
            .all(|draft| draft.local_image_paths.is_empty() && draft.remote_image_urls.is_empty())
        {
            self.append_recovered_drafts(
                &drafts
                    .iter()
                    .map(|draft| draft.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            return;
        }
        self.edit_recovered_draft(|composer| {
            for draft in drafts {
                let mut remote_images = composer.remote_image_urls();
                remote_images.extend(draft.remote_image_urls.iter().cloned());
                composer.set_remote_image_urls(remote_images);
                if !composer.current_text().is_empty() {
                    composer.insert_str("\n");
                }
                let mut offset = 0;
                for element in &draft.text_elements {
                    let range = element.byte_range.start..element.byte_range.end;
                    composer.insert_recovered_text(&draft.text[offset..range.start]);
                    let path =
                        draft
                            .local_image_paths
                            .iter()
                            .enumerate()
                            .find_map(|(index, path)| {
                                let label = local_image_label_text(
                                    draft.remote_image_urls.len() + index + 1,
                                );
                                (element.placeholder(&draft.text) == Some(label.as_str()))
                                    .then_some(path)
                            });
                    if let Some(path) = path {
                        composer
                            .attachments
                            .attach_image(&mut composer.draft.textarea, path.clone());
                    } else {
                        composer
                            .draft
                            .textarea
                            .insert_element(&draft.text[range.clone()]);
                    }
                    offset = range.end;
                }
                composer.insert_recovered_text(&draft.text[offset..]);
            }
            composer.sync_popups();
        });
    }
}
