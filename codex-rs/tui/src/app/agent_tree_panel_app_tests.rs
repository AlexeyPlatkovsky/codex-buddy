use super::*;
use crate::app::test_support::make_test_app;
use crate::history_cell::PlainHistoryCell;
use crate::tui::test_support::make_test_tui;
use pretty_assertions::assert_eq;
use std::sync::Arc;

#[tokio::test]
async fn panel_appears_and_reflows_the_chat_width_at_the_responsive_threshold() {
    let mut app = make_test_app().await;
    let primary_thread_id = ThreadId::new();
    let subagent_thread_id = ThreadId::new();
    app.primary_thread_id = Some(primary_thread_id);
    app.agent_navigation.upsert(
        primary_thread_id,
        /*agent_nickname*/ None,
        /*agent_role*/ None,
        /*is_closed*/ false,
    );
    app.agent_navigation.upsert(
        subagent_thread_id,
        Some("Ada".to_string()),
        Some("planner".to_string()),
        /*is_closed*/ false,
    );

    let narrow = app.agent_tree_panel_layout(Size::new(/*width*/ 99, /*height*/ 24));
    let wide = app.agent_tree_panel_layout(Size::new(/*width*/ 100, /*height*/ 24));

    assert_eq!(narrow.panel_area, None);
    assert_eq!(narrow.chat_area.width, 99);
    assert_eq!(wide.panel_area.map(|area| area.width), Some(33));
    assert_eq!(wide.chat_area.width, 66);
    assert!(
        app.history_wrap_width_for_screen(Size::new(/*width*/ 100, /*height*/ 24))
            < app.history_wrap_width_for_screen(Size::new(/*width*/ 99, /*height*/ 24))
    );
}

#[tokio::test]
async fn panel_uses_the_alternate_screen_and_restores_inline_mode_when_hidden() {
    let mut app = make_test_app().await;
    let mut tui = make_test_tui().expect("test tui");

    app.update_agent_tree_panel_screen_mode(&mut tui, /*panel_visible*/ true)
        .expect("panel should enter the alternate screen");
    assert!(tui.is_alt_screen_active());

    app.update_agent_tree_panel_screen_mode(&mut tui, /*panel_visible*/ false)
        .expect("panel should restore inline mode");
    assert!(!tui.is_alt_screen_active());
}

#[tokio::test]
async fn visible_panel_uses_a_full_screen_pinned_transcript() {
    let mut app = make_test_app().await;
    let mut tui = make_test_tui().expect("test tui");
    let primary_thread_id = ThreadId::new();
    app.primary_thread_id = Some(primary_thread_id);
    app.agent_navigation.upsert(
        primary_thread_id,
        /*agent_nickname*/ None,
        /*agent_role*/ None,
        /*is_closed*/ false,
    );
    app.agent_navigation.upsert(
        ThreadId::new(),
        Some("Ada".to_string()),
        Some("planner".to_string()),
        /*is_closed*/ false,
    );
    app.transcript_cells
        .push(Arc::new(PlainHistoryCell::new(vec![
            "completed transcript line".into(),
        ])));
    let screen_size = Size::new(/*width*/ 120, /*height*/ 40);
    tui.terminal
        .resize(screen_size)
        .expect("resize test terminal");

    app.render_chat_widget_frame(&mut tui, screen_size)
        .expect("render full-screen agent tree");

    assert!(tui.is_alt_screen_active());
    assert!(app.pinned_transcript.is_some());
    assert_eq!(
        app.agent_tree_panel_layout(screen_size)
            .panel_area
            .map(|area| area.height),
        Some(screen_size.height)
    );
}

#[tokio::test]
async fn pinned_transcript_leaves_popup_navigation_to_the_active_menu() {
    let mut app = make_test_app().await;
    let mut tui = make_test_tui().expect("test tui");
    let mut app_server = crate::start_embedded_app_server_for_picker(&app.config)
        .await
        .expect("embedded app server");
    app.pinned_transcript = Some(pinned_transcript::PinnedTranscript::new(
        Vec::new(),
        app.keymap.pager.clone(),
    ));
    app.chat_widget.show_selection_view(SelectionViewParams {
        view_id: Some("pinned-popup-navigation"),
        items: ["First", "Second"]
            .into_iter()
            .map(|name| SelectionItem {
                name: name.to_string(),
                dismiss_on_select: true,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    });

    assert_eq!(
        app.chat_widget
            .selected_index_for_present_view("pinned-popup-navigation"),
        Some(0)
    );

    app.handle_key_event(
        &mut tui,
        &mut app_server,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(
        app.chat_widget
            .selected_index_for_present_view("pinned-popup-navigation"),
        Some(1)
    );

    app.handle_key_event(
        &mut tui,
        &mut app_server,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .await;

    assert_eq!(
        app.chat_widget
            .selected_index_for_present_view("pinned-popup-navigation"),
        None
    );
}

#[tokio::test]
#[cfg(feature = "full-runtime-extensions")]
async fn ambient_pet_is_suppressed_while_the_panel_is_visible() {
    let mut app = make_test_app().await;
    let primary_thread_id = ThreadId::new();
    app.primary_thread_id = Some(primary_thread_id);
    app.agent_navigation.upsert(
        primary_thread_id,
        /*agent_nickname*/ None,
        /*agent_role*/ None,
        /*is_closed*/ false,
    );
    app.agent_navigation.upsert(
        ThreadId::new(),
        Some("Ada".to_string()),
        Some("planner".to_string()),
        /*is_closed*/ false,
    );
    app.chat_widget
        .set_pet_image_support_for_tests(crate::pets::PetImageSupport::Supported(
            crate::pets::ImageProtocol::Kitty,
        ));
    app.chat_widget
        .install_test_ambient_pet_for_tests(/*animations_enabled*/ false);

    assert!(app.should_render_ambient_pet(Size::new(/*width*/ 99, /*height*/ 24)));
    assert!(!app.should_render_ambient_pet(Size::new(/*width*/ 100, /*height*/ 24)));
}
