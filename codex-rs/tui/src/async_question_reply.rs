//! Recognize the desktop's existing async-question reply envelope for display and dismissal.
//! Only complete envelopes, optionally following the standard IDE context prefix, are interpreted.

use codex_app_server_protocol::UserInput;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AsyncQuestionReply {
    pub(crate) question_item_id: String,
    question: String,
    answer: String,
}

pub(crate) fn parse(text: &str) -> Option<Vec<AsyncQuestionReply>> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Replies {
        Many(Vec<AsyncQuestionReply>),
        One(AsyncQuestionReply),
    }
    let text = text.trim();
    // JSON strings escape newlines, so this cannot match a delimiter inside an answer.
    let text = if text.starts_with("# Context from my IDE setup:\n") {
        text.rsplit_once("\n## My request for Codex:\n")?.1.trim()
    } else {
        text
    };
    let json = text
        .strip_prefix("<send_user_message_question_reply>")?
        .strip_suffix("</send_user_message_question_reply>")?;
    let replies = match serde_json::from_str::<Replies>(json).ok()? {
        Replies::Many(replies) => replies,
        Replies::One(reply) => vec![reply],
    };
    (!replies.is_empty()).then_some(replies)
}

pub(crate) fn display_text(text: &str) -> Option<String> {
    Some(
        parse(text)?
            .iter()
            .map(|reply| format!("> {}\n\n{}", reply.question, reply.answer))
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

/// Preserve attachment chip ranges when a reply envelope becomes an editable user draft.
pub(crate) fn display_with_elements(
    text: &str,
    elements: &[codex_protocol::user_input::TextElement],
) -> Option<(String, Vec<codex_protocol::user_input::TextElement>)> {
    let replies = parse(text)?;
    let mut displayed = String::new();
    let mut displayed_elements = Vec::new();
    let mut source_cursor = 0;
    for reply in replies {
        if !displayed.is_empty() {
            displayed.push_str("\n\n");
        }
        displayed.push_str(&format!("> {}\n\n", reply.question));
        let offset = displayed.len();
        displayed.push_str(&reply.answer);
        let encoded = serde_json::to_string(&reply.answer).ok()?;
        let needle = format!("\"answer\":{encoded}");
        let Some(start) = text[source_cursor..]
            .find(&needle)
            .map(|start| source_cursor + start + "\"answer\":\"".len())
        else {
            continue;
        };
        let end = start + encoded.len() - 2;
        for element in elements {
            if element.byte_range.start < start
                || element.byte_range.end > end
                || element.byte_range.start >= element.byte_range.end
            {
                continue;
            }
            let decode_position = |position| {
                serde_json::from_str::<String>(&format!("\"{}\"", text.get(start..position)?))
                    .ok()
                    .map(|prefix| offset + prefix.len())
            };
            if let (Some(start), Some(end)) = (
                decode_position(element.byte_range.start),
                decode_position(element.byte_range.end),
            ) {
                displayed_elements.push(element.clone().map_range(|_| (start..end).into()));
            }
        }
        source_cursor = end;
    }
    Some((displayed, displayed_elements))
}

pub(crate) fn parse_input(input: &[UserInput]) -> Option<Vec<AsyncQuestionReply>> {
    let mut content = input.iter().filter(|item| {
        !matches!(
            item,
            UserInput::Skill { .. }
                | UserInput::Mention { .. }
                | UserInput::LocalImage { .. }
                | UserInput::Image { .. }
        )
    });
    let UserInput::Text { text, .. } = content.next()? else {
        return None;
    };
    if content.next().is_some() {
        return None;
    }
    parse(text)
}

#[cfg(test)]
#[path = "async_question_reply_tests.rs"]
mod tests;
