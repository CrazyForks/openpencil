use super::*;
use op_editor_core::{
    AuthenticatedCollabSession, CollabConnectionPathUi, CollabInviteCode, CollabPanelView,
    CollabRelayRegion,
};

#[test]
fn home_offers_create_and_join_paths() {
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;

    let model = CollabPanelModel::for_editor_ui(&ui);
    assert!(matches!(model.screen, CollabPanelScreen::Home));
    assert_eq!(
        model
            .actions
            .iter()
            .map(|action| action.action.clone())
            .collect::<Vec<_>>(),
        vec![CollabUiAction::OpenCreate, CollabUiAction::OpenJoin]
    );
    assert!(model.actions[0].primary);
    assert_eq!(model.actions[0].label, "创建会话");
    assert_eq!(model.actions[1].label, "加入会话");
}

#[test]
fn open_create_is_navigation_only_and_exposes_connection_choices() {
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.panel.open = true;

    assert!(apply_panel_hit(
        &mut ui,
        crate::widgets::collab_panel::CollabPanelHit::Action(CollabUiAction::OpenCreate)
    ));
    assert_eq!(ui.collab.panel.view, CollabPanelView::Create);
    assert!(!ui.collab.panel.join_address_focused);
    assert!(ui.collab.take_pending_action().is_none());

    let create = CollabPanelModel::for_editor_ui(&ui);
    assert!(matches!(create.screen, CollabPanelScreen::Create));
    assert_eq!(
        create
            .actions
            .iter()
            .map(|action| action.action.clone())
            .collect::<Vec<_>>(),
        vec![
            CollabUiAction::Start,
            CollabUiAction::StartLan,
            CollabUiAction::Cancel,
        ]
    );
    assert_eq!(create.actions[0].label, "公网中继");
    assert_eq!(create.actions[1].label, "局域网");

    assert!(apply_panel_hit(
        &mut ui,
        crate::widgets::collab_panel::CollabPanelHit::Action(CollabUiAction::Cancel)
    ));
    assert_eq!(ui.collab.panel.view, CollabPanelView::Home);
    assert!(ui.collab.take_pending_action().is_none());
}

#[test]
fn open_join_is_navigation_only_and_nearby_search_is_explicit() {
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.panel.open = true;

    let home = CollabPanelModel::for_editor_ui(&ui);
    assert!(home
        .actions
        .iter()
        .any(|action| action.action == CollabUiAction::OpenJoin));
    assert!(!home
        .actions
        .iter()
        .any(|action| action.action == CollabUiAction::BeginDiscovery));

    assert!(apply_panel_hit(
        &mut ui,
        crate::widgets::collab_panel::CollabPanelHit::Action(CollabUiAction::OpenJoin)
    ));
    assert_eq!(ui.collab.panel.view, CollabPanelView::Join);
    assert!(ui.collab.panel.join_address_focused);
    assert!(ui.collab.take_pending_action().is_none());

    let join = CollabPanelModel::for_editor_ui(&ui);
    assert!(join
        .actions
        .iter()
        .any(|action| action.action == CollabUiAction::BeginDiscovery));
    assert!(apply_panel_hit(
        &mut ui,
        crate::widgets::collab_panel::CollabPanelHit::Action(CollabUiAction::BeginDiscovery)
    ));
    assert_eq!(
        ui.collab.take_pending_action(),
        Some(CollabUiAction::BeginDiscovery)
    );
}

#[test]
fn invite_or_address_input_is_bounded_and_queues_one_join() {
    for target in ["opc1_Ab-9", "192.168.1.8:43120"] {
        let mut ui = EditorUiState::default();
        ui.collab.panel.join_address_focused = true;
        for character in target.chars() {
            assert_eq!(join_address_text(&mut ui, character), Some(true));
        }
        assert_eq!(join_address_text(&mut ui, ' '), Some(false));
        assert_eq!(join_address_submit(&mut ui), Some(true));
        assert_eq!(
            ui.collab.take_pending_action(),
            Some(CollabUiAction::JoinAddress {
                endpoint: target.into()
            })
        );
    }
}

#[test]
fn join_target_and_action_debug_are_redacted() {
    let raw_invite = "opc1_secret-join-route";
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.panel.view = CollabPanelView::Join;
    ui.collab.panel.join_address = raw_invite.into();

    let model = CollabPanelModel::for_editor_ui(&ui);
    let debug = format!("{model:?}");
    assert!(!debug.contains(raw_invite));
    assert!(debug.contains("[REDACTED]"));
}

#[test]
fn owner_model_projects_redacted_invite_and_relay_region() {
    let raw_invite = "opc1_secret-route-capability";
    let mut ui = EditorUiState::default();
    ui.collab.availability = CollabAvailability::Ready;
    ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "Design".into(),
            role: CollabUiRole::Owner,
            share_endpoint: CollabShareEndpoint::new("192.168.1.8:43120"),
        },
        Vec::new(),
    );
    ui.collab.set_public_session(
        CollabInviteCode::new(raw_invite),
        CollabConnectionPathUi::Relay {
            home_region: CollabRelayRegion::China,
        },
    );

    let model = CollabPanelModel::for_editor_ui(&ui);
    let CollabPanelScreen::Session {
        invite, connection, ..
    } = &model.screen
    else {
        panic!("expected owner session");
    };
    assert_eq!(
        invite.as_ref().map(CollabInviteCode::as_str),
        Some(raw_invite)
    );
    assert_eq!(
        *connection,
        Some(CollabConnectionPathUi::Relay {
            home_region: CollabRelayRegion::China
        })
    );
    assert!(!format!("{model:?}").contains(raw_invite));
    assert_eq!(
        connection_path_label(&ui, connection.unwrap()),
        "公网中继 · 中国"
    );
}

#[test]
fn guest_model_never_projects_owner_invite() {
    let mut ui = EditorUiState::default();
    ui.collab.set_authenticated_session(
        CollabConnectionPhase::Active,
        AuthenticatedCollabSession {
            session_name: "Design".into(),
            role: CollabUiRole::Editor,
            share_endpoint: None,
        },
        Vec::new(),
    );
    ui.collab.set_public_session(
        CollabInviteCode::new("opc1_owner-secret"),
        CollabConnectionPathUi::Relay {
            home_region: CollabRelayRegion::China,
        },
    );

    let model = CollabPanelModel::for_editor_ui(&ui);
    let CollabPanelScreen::Session {
        invite, connection, ..
    } = model.screen
    else {
        panic!("expected guest session");
    };
    assert!(invite.is_none());
    assert!(connection.is_some());
}
