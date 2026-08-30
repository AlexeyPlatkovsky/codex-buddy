//! Local rendering for the latest model-visible context estimate.

use super::*;

impl ChatWidget {
    pub(super) fn add_context_usage_output(&mut self) {
        let Some(context_usage) = self.context_usage.as_ref() else {
            self.add_info_message(
                "Context usage is not available until Buddy sends its first model request."
                    .to_string(),
                /*hint*/ None,
            );
            return;
        };

        self.add_plain_history_lines(context_usage_lines(context_usage));
    }
}

fn context_usage_lines(context_usage: &ThreadContextUsage) -> Vec<Line<'static>> {
    let rows = [
        ("Base instructions", context_usage.base_instructions_tokens),
        ("Tool definitions", context_usage.tool_definitions_tokens),
        (
            "User and developer messages",
            context_usage.user_and_developer_tokens,
        ),
        (
            "Assistant answers and reasoning",
            context_usage.assistant_and_reasoning_tokens,
        ),
        ("Tool calls and results", context_usage.tool_activity_tokens),
        ("Other", context_usage.other_tokens),
    ];
    let category_width = rows
        .iter()
        .map(|(category, _)| category.len())
        .max()
        .unwrap_or_default()
        .max("Category".len());
    let token_values = rows.map(|(_, tokens)| format_rounded_tokens(tokens));
    let token_width = token_values
        .iter()
        .map(String::len)
        .max()
        .unwrap_or_default()
        .max("Tokens".len());
    let border = |left: char, middle: char, right: char| {
        format!(
            "{left}{}{middle}{}{right}",
            "─".repeat(category_width + 2),
            "─".repeat(token_width + 2)
        )
    };
    let row = |category: &str, tokens: &str| {
        format!("│ {category:<category_width$} │ {tokens:>token_width$} │")
    };
    let mut lines = vec![
        "Context usage (approximate)".bold().into(),
        border('┌', '┬', '┐').dim().into(),
        row("Category", "Tokens").bold().into(),
        border('├', '┼', '┤').dim().into(),
    ];
    lines.extend(
        rows.iter()
            .zip(token_values.iter())
            .map(|((category, _), tokens)| row(category, tokens).into()),
    );
    lines.push(border('├', '┼', '┤').dim().into());
    let total = format_rounded_tokens(context_usage.total_tokens);
    lines.push(row("Total", &total).bold().into());
    lines.push(border('└', '┴', '┘').dim().into());
    lines.push(
        "Rounded to the nearest thousand tokens; not tokenizer-accurate or billable usage."
            .dim()
            .into(),
    );
    lines
}

fn format_rounded_tokens(tokens: i64) -> String {
    let rounded_thousands = tokens.saturating_add(500).div_euclid(1_000);
    if tokens > 0 && rounded_thousands == 0 {
        "<1K".to_string()
    } else {
        format!("{rounded_thousands}K")
    }
}

#[cfg(test)]
#[path = "context_usage_tests.rs"]
mod tests;
