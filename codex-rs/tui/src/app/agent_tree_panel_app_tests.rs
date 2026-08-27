use super::*;
use crate::app::test_support::make_test_app;
use pretty_assertions::assert_eq;

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
