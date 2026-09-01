//! Full-screen transcript state used while the pinned agent-tree board is visible.
//!
//! The normal inline TUI writes completed cells to terminal scrollback. A fixed side panel needs
//! an alternate screen instead, so this state reuses the existing transcript pager for the left
//! pane while preserving its scroll position and live-tail behavior.

use crate::chatwidget::ChatWidget;
use crate::history_cell::HistoryCell;
use crate::keymap::PagerKeymap;
use crate::pager_overlay::TranscriptOverlay;
use crate::tui::Tui;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::sync::Arc;

pub(super) struct PinnedTranscript {
    overlay: TranscriptOverlay,
    cells: Vec<Arc<dyn HistoryCell>>,
}

impl PinnedTranscript {
    pub(super) fn new(cells: Vec<Arc<dyn HistoryCell>>, keymap: PagerKeymap) -> Self {
        Self {
            overlay: TranscriptOverlay::new_pinned(cells.clone(), keymap),
            cells,
        }
    }

    pub(super) fn sync_and_render(
        &mut self,
        area: Rect,
        buffer: &mut Buffer,
        cells: &[Arc<dyn HistoryCell>],
        chat_widget: &ChatWidget,
    ) {
        if !same_cells(&self.cells, cells) {
            self.cells = cells.to_vec();
            self.overlay.replace_cells(self.cells.clone());
        }
        let width = area.width.max(1);
        self.overlay
            .sync_live_tail(width, chat_widget.active_cell_transcript_key(), |w| {
                chat_widget.active_cell_transcript_hyperlink_lines(w)
            });
        self.overlay.render(area, buffer);
    }

    pub(super) fn handle_navigation_key(&mut self, tui: &mut Tui, key_event: KeyEvent) -> bool {
        self.overlay.handle_navigation_key(tui, key_event)
    }
}

fn same_cells(current: &[Arc<dyn HistoryCell>], next: &[Arc<dyn HistoryCell>]) -> bool {
    current.len() == next.len()
        && current
            .iter()
            .zip(next)
            .all(|(current, next)| Arc::ptr_eq(current, next))
}
