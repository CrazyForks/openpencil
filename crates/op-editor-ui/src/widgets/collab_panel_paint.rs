//! Small paint primitives shared by the collaboration panel body.

use super::*;
use crate::widgets::collab_ui::{role_label, CollabAvatarModel};
use crate::{Color, TextLayout};

impl CollabPanel<'_> {
    pub fn rect_at(&self, anchor: Rect, viewport: Rect) -> Rect {
        let width = COLLAB_PANEL_WIDTH.min((viewport.size.x - 16.0).max(180.0));
        let right = anchor.origin.x + anchor.size.x;
        let x = (right - width)
            .max(viewport.origin.x + 8.0)
            .min(viewport.origin.x + viewport.size.x - width - 8.0);
        let preferred_y = anchor.origin.y + anchor.size.y + 6.0;
        let height = self
            .panel_height()
            .min((viewport.origin.y + viewport.size.y - preferred_y - 8.0).max(120.0));
        Rect::xywh(x, preferred_y, width, height)
    }

    pub fn panel_height(&self) -> f32 {
        let notice = if self.model.notice.is_some() {
            NOTICE_HEIGHT + 8.0
        } else {
            0.0
        };
        let body = match &self.model.screen {
            CollabPanelScreen::Unavailable | CollabPanelScreen::SignInRequired => 82.0,
            CollabPanelScreen::Home | CollabPanelScreen::Create => 66.0,
            CollabPanelScreen::Progress { .. } => 70.0,
            CollabPanelScreen::ConfirmOwner(confirm) => {
                CONFIRM_OWNER_HEAD_HEIGHT
                    + (confirm.authoritative.len() + usize::from(confirm.claimed_name.is_some()))
                        as f32
                        * CONFIRM_OWNER_ROW_HEIGHT
            }
            CollabPanelScreen::Join { discovered, .. } => {
                let visible_rows = discovered.len().clamp(1, MAX_VISIBLE_ENDPOINTS);
                106.0 + visible_rows as f32 * ROW_HEIGHT
            }
            CollabPanelScreen::Session {
                invite,
                connection,
                share_endpoint,
                participants,
                admission_request,
                ..
            } => {
                58.0 + if connection.is_some() {
                    CONNECTION_PATH_HEIGHT
                } else {
                    0.0
                } + if invite.is_some() { INVITE_HEIGHT } else { 0.0 }
                    + if share_endpoint.is_some() {
                        SHARE_ENDPOINT_HEIGHT
                    } else {
                        0.0
                    }
                    + if admission_request.is_some() {
                        ADMISSION_HEIGHT
                    } else {
                        0.0
                    }
                    + participants.len().min(MAX_VISIBLE_PARTICIPANTS) as f32 * ROW_HEIGHT
            }
        };
        HEADER_HEIGHT + notice + body + self.actions_height() + PAD
    }

    pub(super) fn paint_message(
        &self,
        cx: &mut PaintCx<'_>,
        rect: Rect,
        body_top: f32,
        key: &'static str,
    ) {
        paint_text(
            cx,
            op_i18n::translate(self.ui.locale, key),
            12.0,
            self.theme.muted_foreground,
            Point2D::new(rect.origin.x + PAD, body_top + 29.0),
            400,
        );
    }
}

pub(super) fn paint_participant(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    participant: &CollabAvatarModel,
    x: f32,
    y: f32,
    width: f32,
) {
    let avatar = Rect::xywh(x, y + 6.0, 22.0, 22.0);
    crate::widgets::collab_avatar_paint::paint_collab_avatar(
        cx,
        participant,
        avatar,
        9.0,
        y + 20.0,
    );
    paint_text(
        cx,
        &participant.display_name,
        12.0,
        theme.foreground,
        Point2D::new(x + 31.0, y + 21.0),
        if participant.is_self { 600 } else { 400 },
    );
    let role = role_label(ui, participant.role);
    let role_w = cx.backend.measure_text(role, 10.0);
    paint_text(
        cx,
        role,
        10.0,
        theme.muted_foreground,
        Point2D::new(x + width - role_w, y + 20.0),
        400,
    );
}

pub(super) fn paint_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    label: &str,
    primary: bool,
    enabled: bool,
    hovered: bool,
) {
    let background = if primary {
        theme.primary
    } else {
        theme.secondary
    };
    cx.backend.fill_round_rect(
        rect,
        6.0,
        background.with_alpha(if enabled { 1.0 } else { 0.5 }),
    );
    if hovered && enabled {
        cx.backend.fill_round_rect(rect, 6.0, theme.button_hover);
        cx.backend
            .stroke_round_rect(rect, 6.0, theme.foreground.with_alpha(0.12), 1.0);
    }
    let color = if primary {
        theme.primary_foreground
    } else {
        theme.secondary_foreground
    };
    let width = cx.backend.measure_text(label, 11.0);
    paint_text(
        cx,
        label,
        11.0,
        color.with_alpha(if enabled { 1.0 } else { 0.6 }),
        Point2D::new(
            rect.origin.x + (rect.size.x - width) / 2.0,
            rect.origin.y + 21.0,
        ),
        500,
    );
}

pub(super) fn paint_text(
    cx: &mut PaintCx<'_>,
    text: &str,
    size: f32,
    color: Color,
    origin: Point2D,
    weight: u16,
) {
    let layout = TextLayout::single_run(text, "system-ui", size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}
