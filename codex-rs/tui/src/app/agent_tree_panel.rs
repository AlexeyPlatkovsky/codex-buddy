//! Responsive read-only rendering for the sub-agent tree beside the transcript.
//!
//! Layout selection is intentionally pure so the chat widget can measure itself against the
//! exact reduced rectangle before terminal reflow. The panel consumes only cached navigation
//! state and does not perform background work or alter the existing picker fallback.

use super::agent_tree::AgentTreeRow;
use super::agent_tree::AgentTreeSnapshot;
use super::agent_tree::AgentTreeStatus;
use super::agent_tree_viewport::AgentTreeViewport;
use crate::text_formatting::truncate_text;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

const MIN_PANEL_TERMINAL_WIDTH: u16 = 100;
const MIN_PANEL_WIDTH: u16 = 28;
const MAX_PANEL_WIDTH: u16 = 36;

/// The two drawing rectangles chosen for one app frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AgentTreePanelLayout {
    pub(crate) chat_area: Rect,
    pub(crate) panel_area: Option<Rect>,
}

impl AgentTreePanelLayout {
    /// Returns the number of content rows in the panel after its title border.
    pub(crate) fn panel_content_height(self) -> usize {
        self.panel_area
            .map(|area| usize::from(area.height.saturating_sub(1)))
            .unwrap_or_default()
    }
}

/// Splits a frame into chat and tree rectangles when there is enough room for both.
pub(crate) fn layout_agent_tree_panel(area: Rect, has_subagents: bool) -> AgentTreePanelLayout {
    if !has_subagents || area.width < MIN_PANEL_TERMINAL_WIDTH {
        return AgentTreePanelLayout {
            chat_area: area,
            panel_area: None,
        };
    }

    let panel_width = (area.width / 3).clamp(MIN_PANEL_WIDTH, MAX_PANEL_WIDTH);
    let [chat_area, _divider, panel_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(panel_width),
    ])
    .areas(area);
    AgentTreePanelLayout {
        chat_area,
        panel_area: Some(panel_area),
    }
}

/// Renders the cached tree using the viewport selected by [`AgentTreeViewport`].
pub(crate) fn render_agent_tree_panel(
    area: Rect,
    tree: &AgentTreeSnapshot,
    viewport: &AgentTreeViewport,
    buffer: &mut Buffer,
) {
    if area.is_empty() {
        return;
    }

    let visible = viewport.visible(tree);
    let title = if visible.truncated {
        " Subagents … "
    } else if visible.above || visible.below {
        " Subagents ↕ "
    } else {
        " Subagents "
    };
    let block = Block::default()
        .borders(Borders::TOP | Borders::LEFT)
        .title(title.bold());
    let content_area = block.inner(area);
    block.render(area, buffer);

    if content_area.is_empty() {
        return;
    }
    let lines = visible
        .rows
        .iter()
        .map(|row| render_row(row, content_area.width))
        .collect::<Vec<_>>();
    Paragraph::new(lines).render(content_area, buffer);
}

fn render_row(row: &AgentTreeRow, width: u16) -> Line<'static> {
    let indent = "  ".repeat(row.depth.min(12));
    let available = usize::from(width)
        .saturating_sub(indent.len())
        .saturating_sub(4);
    let label = truncate_text(&row.label, available);
    let selection = if row.is_selected || row.is_current {
        "› ".cyan()
    } else {
        "  ".into()
    };
    let label = if row.is_selected || row.is_current {
        label.cyan()
    } else if row.status.is_terminal() {
        label.dim()
    } else {
        label.into()
    };
    Line::from(vec![
        selection,
        indent.into(),
        status_symbol(row.status),
        " ".into(),
        label,
    ])
}

fn status_symbol(status: AgentTreeStatus) -> Span<'static> {
    match status {
        AgentTreeStatus::Running => "●".cyan(),
        AgentTreeStatus::Waiting => "○".dim(),
        AgentTreeStatus::NeedsApproval => "!".red(),
        AgentTreeStatus::NeedsInput => "?".cyan(),
        AgentTreeStatus::Completed => "✓".green(),
        AgentTreeStatus::Failed => "×".red(),
        AgentTreeStatus::Interrupted => "–".dim(),
    }
}

#[cfg(test)]
#[path = "agent_tree_panel_tests.rs"]
mod tests;
