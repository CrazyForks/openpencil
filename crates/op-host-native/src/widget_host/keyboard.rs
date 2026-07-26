//! Keyboard input handlers on `WidgetHostNative` — text input,
//! delete / duplicate / nudge, send, escape. Click routing +
//! marquee / layer-drag commit live in the sibling `click.rs`.
//!
//! `EditorState` is the host's source of truth: every focus / draft
//! / chat field is read + written on `editor_state`; mutations flag
//! the paint snapshot dirty.

use super::WidgetHostNative;
use op_editor_core::host_escape_transitions as escape;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::host_preset_name_draft as preset_name;

impl WidgetHostNative {
    /// Typed-char router: settings → rename → text-edit → variable
    /// row → property → chat.
    pub fn apply_text(&mut self, c: char) -> bool {
        // Preview (Play) mode owns the keyboard: printable chars go to
        // the live runtime's focused widget, never editor editing.
        if self.preview.is_some() {
            if c.is_control() {
                return false;
            }
            let mut s = [0u8; 4];
            return self.preview_dispatch_text(c.encode_utf8(&mut s));
        }
        // This popover is painted above every other editor input, so it wins
        // even if a lower surface retained stale focus.
        if self.apply_image_panel_text(c) {
            return true;
        }
        // Color-picker hex field owns the keyboard while focused.
        if self.editor_state.color_picker_hex_focused() {
            if c.is_control() {
                return false;
            }
            self.editor_state.color_picker_hex_char(c, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // Color-picker R/G/B numeric field owns the keyboard while focused.
        if self.editor_state.color_picker_rgb_focused() {
            if c.is_control() {
                return false;
            }
            self.editor_state.color_picker_rgb_char(c, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // Settings input owns the keyboard while focused.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text(c);
        }
        // The inline clone wizard owns the keyboard while it is open: a
        // focused URL / destination field takes the character (unless a
        // clone is already running), and every other key is swallowed so
        // nothing reaches the canvas.
        if self.git_clone_input_active() {
            if c.is_control() {
                return false;
            }
            let now = self.now_ms;
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    let mut s = [0u8; 4];
                    match form.focus {
                        Some(op_editor_core::CloneField::Url) => {
                            form.url_input.insert_str(c.encode_utf8(&mut s), now)
                        }
                        Some(op_editor_core::CloneField::Dest) => {
                            form.dest_input.insert_str(c.encode_utf8(&mut s), now)
                        }
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return true;
        }
        // Git panel's commit-message input owns the keyboard next.
        if self.git_commit_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel.commit_input.insert_str(c.encode_utf8(&mut s), now);
                panel.commit_no_changes = false;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's remote-URL input.
        if self.git_remote_focus_active() {
            if !c.is_control() {
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .remote_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's HTTPS-credential input.
        if self.git_https_focus_active() {
            if !c.is_control() {
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .https_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the commit-signature form's name / email inputs.
        if self.git_author_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                if panel.author_email_focused {
                    panel
                        .author_email_input
                        .insert_str(c.encode_utf8(&mut s), now);
                } else {
                    panel
                        .author_name_input
                        .insert_str(c.encode_utf8(&mut s), now);
                }
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.git_branch_create_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .branch_create_input
                    .insert_str(c.encode_utf8(&mut s), now);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if let Some(changed) = shared::rename_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) = shared::text_edit_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Variables-panel search filter — live append, no draft /
        // commit machinery (TS controlled `<input>`; same append/pop
        // discipline as the font-picker search).
        if shared::variables_search_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if shared::variables_header_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if preset_name::preset_name_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if let Some(changed) = shared::variable_row_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Effect-param value box + property-panel inputs share
        // `ui.property_input`; the gate is per-focus (numeric / hex /
        // free text) and lives in the shared router.
        if let Some(changed) = shared::property_input_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Font-family picker search box (font_picker_dispatch.rs).
        if self.apply_font_picker_text(c) {
            return true;
        }
        if self.editor_state.editor_ui.icon_picker.open && !c.is_control() {
            if self.editor_state.editor_ui.icon_picker_select_all {
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_select_all = false;
                self.editor_state.editor_ui.icon_picker.hover = None;
                self.editor_state.editor_ui.icon_picker.pressed = None;
            }
            self.editor_state.editor_ui.icon_picker_search.push(c);
            self.editor_state.editor_ui.icon_picker.hover = None;
            self.editor_state.editor_ui.icon_picker.pressed = None;
            // New filter → scroll the list back to the top.
            self.editor_state.editor_ui.icon_picker.scroll.offset = 0.0;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_text(c);
        }
        if self.editor_state.editor_ui.component_browser_open && !c.is_control() {
            if self.editor_state.editor_ui.component_browser_select_all {
                self.editor_state.editor_ui.component_browser_search.clear();
                self.editor_state.editor_ui.component_browser_select_all = false;
            }
            self.editor_state.editor_ui.component_browser_search.push(c);
            self.mark_dirty();
            return true;
        }
        if shared::chat_input_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Paste `text` into the focused chat input — appended at the
    /// caret (always the buffer end). Newlines are kept so a
    /// multi-line clipboard paste survives; the input widget wraps
    /// and honours `\n`. Returns `false` (no-op) when the chat input
    /// is not focused or `text` is empty. The desktop host calls
    /// this with the OS clipboard's contents on Cmd+V.
    pub fn chat_input_paste(&mut self, text: &str) -> bool {
        if !self.editor_state.chat.focused || text.is_empty() {
            return false;
        }
        self.editor_state.chat.insert_input_text(text, self.now_ms);
        self.mark_dirty();
        true
    }

    /// Paste clipboard `text` into whichever text input currently owns
    /// the keyboard — the clone-wizard URL / destination, the git commit
    /// message, the remote / HTTPS draft, or a settings field. Each
    /// character is routed through [`Self::apply_text`], so per-input
    /// filtering (e.g. digits-only for the MCP port, the clone field's
    /// `!cloning` lock) still applies; control characters / newlines are
    /// dropped since these inputs are single-line. Returns `true` if
    /// anything was inserted.
    pub fn apply_input_paste(&mut self, text: &str) -> bool {
        let mut inserted = false;
        for c in text.chars() {
            if c.is_control() {
                continue;
            }
            if self.apply_text(c) {
                inserted = true;
            }
        }
        inserted
    }

    /// Cut the focused chat input — returns its text and empties the
    /// buffer. `None` when the chat input is not focused or already
    /// empty. The desktop host writes the returned text to the OS
    /// clipboard on Cmd+X.
    pub fn chat_input_cut(&mut self) -> Option<String> {
        if !self.editor_state.chat.focused || self.editor_state.chat.input.text().is_empty() {
            return None;
        }
        if let Some(selected) = self
            .editor_state
            .chat
            .selected_input_text()
            .map(str::to_string)
        {
            self.editor_state.chat.delete_input_selection(self.now_ms);
            self.mark_dirty();
            return Some(selected);
        }
        let taken = self.editor_state.chat.input.text().to_owned();
        self.editor_state.chat.set_input_text("");
        self.editor_state.chat.input.touch(self.now_ms);
        self.mark_dirty();
        Some(taken)
    }

    /// Highlighted slice of whichever `TextInputState`-backed input
    /// currently owns the keyboard — settings / git (commit, remote,
    /// HTTPS, branch, author, clone) / rename / property / variables /
    /// model-picker / canvas text editor. `None` when no such input is
    /// focused or it has no selection. The desktop host writes the
    /// returned slice to the OS clipboard on Cmd+C; chat-input copy is
    /// handled separately (its own whole-buffer path). Routes through
    /// the shared `EditorState::active_text_input` resolver so every
    /// focused field is covered with one priority order.
    pub fn input_copy_text(&self) -> Option<String> {
        let state = self.editor_state.active_text_input()?;
        let (start, end) = state.highlight_range()?;
        Some(state.text().get(start..end)?.to_string())
    }

    /// Cut the highlighted slice of the focused `TextInputState` input:
    /// returns the slice and deletes it. `None` when no such input is
    /// focused or it has no selection. The delete reuses
    /// [`Self::apply_backspace`] so it follows each input's own backspace
    /// routing (per-input dirty / hint bookkeeping included); with a live
    /// selection `backspace` removes the whole highlighted range
    /// (`TextInputState::consume_pending`). Backs Cmd+X for every editor
    /// text field except the chat input (`chat_input_cut`).
    pub fn input_cut_text(&mut self) -> Option<String> {
        let text = self.input_copy_text()?;
        if text.is_empty() {
            return None;
        }
        self.apply_backspace();
        Some(text)
    }

    pub fn apply_backspace(&mut self) -> bool {
        // Preview mode: Backspace edits the focused runtime widget, not
        // the editor selection.
        if self.preview.is_some() {
            return self.preview_dispatch_key("Backspace", false);
        }
        if self.apply_image_panel_backspace() {
            return true;
        }
        if self.editor_state.color_picker_hex_focused() {
            self.editor_state.color_picker_hex_backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.color_picker_rgb_focused() {
            self.editor_state.color_picker_rgb_backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_backspace();
        }
        if self.git_clone_input_active() {
            // Swallow Backspace whenever the wizard is open so it can
            // never delete a selected node; pop a char only from a
            // focused field that isn't mid-clone.
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    match form.focus {
                        Some(op_editor_core::CloneField::Url) => {
                            form.url_input.backspace(self.now_ms)
                        }
                        Some(op_editor_core::CloneField::Dest) => {
                            form.dest_input.backspace(self.now_ms)
                        }
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return true;
        }
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.commit_input.backspace(self.now_ms);
            // Editing the message (delete or cut, not just typing) clears
            // the stale "no changes to commit" hint, matching `apply_text`.
            panel.commit_no_changes = false;
            self.mark_dirty();
            return true;
        }
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.remote_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.https_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if panel.author_email_focused {
                panel.author_email_input.backspace(self.now_ms);
            } else {
                panel.author_name_input.backspace(self.now_ms);
            }
            self.mark_dirty();
            return true;
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.branch_create_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if let Some(changed) = shared::rename_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) = shared::text_edit_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Variables-panel search filter — pop one char.
        if let Some(changed) =
            shared::variables_search_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            let changed = shared::variables_header_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) =
            preset_name::preset_name_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            let changed = shared::variable_row_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            let changed = shared::property_input_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Font-family picker search box (font_picker_dispatch.rs).
        if self.apply_font_picker_backspace() {
            return true;
        }
        if self.editor_state.editor_ui.icon_picker.open {
            if self.editor_state.editor_ui.icon_picker_select_all {
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_select_all = false;
                self.editor_state.editor_ui.icon_picker.hover = None;
                self.editor_state.editor_ui.icon_picker.pressed = None;
                // Filter changed → scroll the list back to the top.
                self.editor_state.editor_ui.icon_picker.scroll.offset = 0.0;
                self.mark_dirty();
                return true;
            }
            if self
                .editor_state
                .editor_ui
                .icon_picker_search
                .pop()
                .is_some()
            {
                self.editor_state.editor_ui.icon_picker.hover = None;
                self.editor_state.editor_ui.icon_picker.pressed = None;
                // Filter changed → scroll the list back to the top.
                self.editor_state.editor_ui.icon_picker.scroll.offset = 0.0;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_backspace();
        }
        if self.editor_state.editor_ui.component_browser_open {
            if self.editor_state.editor_ui.component_browser_select_all {
                self.editor_state.editor_ui.component_browser_search.clear();
                self.editor_state.editor_ui.component_browser_select_all = false;
                self.mark_dirty();
                return true;
            }
            if self
                .editor_state
                .editor_ui
                .component_browser_search
                .pop()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if let Some(changed) = shared::chat_input_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Pen authoring: Backspace pops the last anchor (`pen_press.rs`).
        if self.apply_pen_backspace() {
            return true;
        }
        if shared::delete_selection_with_history(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Delete — pops a char from rename / text-edit when active;
    /// otherwise deletes the selected node.
    pub fn apply_delete(&mut self) -> bool {
        // Preview mode: Delete edits the focused runtime widget, never
        // the editor selection.
        if self.preview.is_some() {
            return self.preview_dispatch_key("Delete", false);
        }
        if self.apply_image_panel_delete() {
            return true;
        }
        // The open font picker owns Delete. Its search draft handles
        // Backspace separately; forward-delete must never reach the canvas
        // selection behind the overlay.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            let changed =
                shared::variables_header_delete_forward(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) =
            preset_name::preset_name_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            let changed = shared::variable_row_delete_forward(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // The rename draft has no forward deletion — Delete pops the
        // char before the caret, same as Backspace.
        if let Some(changed) = shared::rename_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Delete is FORWARD deletion at the caret (or removes the
        // active selection) — textarea parity.
        if let Some(changed) = shared::text_edit_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            let changed =
                shared::property_input_delete_forward(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.chat.focused
            && self.editor_state.chat.delete_input_selection(self.now_ms)
        {
            self.mark_dirty();
            return true;
        }
        // Don't delete the selected node when a chrome text input or
        // search overlay owns the keyboard. Those branches ran above
        // (or deliberately swallow Delete); falling through here would
        // silently drop the node behind the focused field.
        if shared::delete_owned_by_chrome_input(&self.editor_state) {
            return false;
        }
        if shared::delete_selection_with_history(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd-D — duplicate selection as a sibling at +10 doc px.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        let dup = shared::duplicate_selection(&mut self.editor_state, &mut self.next_node_id);
        if dup {
            self.mark_dirty();
        }
        dup
    }

    /// Up / Down arrow on a focused numeric property input — steps
    /// the value by `delta` and commits it (like a `−` / `+`
    /// stepper). Returns `false` when no numeric property input is
    /// focused, so the caller falls back to nudging the selection.
    pub fn apply_property_step(&mut self, delta: f32) -> bool {
        // Effect-parameter focus: step the value, commit via
        // `SetEffectParam`, and reflect it back into the draft.
        if let Some(ef) = self.editor_state.editor_ui.effect_param_focus {
            let current: f32 = self
                .editor_state
                .ui
                .property_input
                .text()
                .trim()
                .parse()
                .unwrap_or(0.0);
            let next = current + delta;
            let id = self.editor_state.selection.anchor.clone();
            if id.is_real() {
                self.editor_state.commit_history();
                let _ = self
                    .editor_state
                    .apply(op_editor_core::EditorCommand::SetEffectParam {
                        node_id: id,
                        index: ef.effect as u32,
                        field: ef.field,
                        value: next,
                    });
            }
            let next_text = if next.fract() == 0.0 {
                format!("{}", next as i64)
            } else {
                format!("{next}")
            };
            self.editor_state
                .ui
                .property_input
                .set_text(next_text.clone());
            self.editor_state.ui.property_input.touch(self.now_ms);
            self.editor_state.ui.property_input_draft = next_text;
            self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        let Some(focus) = self.editor_state.ui.property_focus else {
            return false;
        };
        // Hex colour fields aren't numerically steppable.
        if focus.is_hex() {
            return false;
        }
        let current: f32 = self
            .editor_state
            .ui
            .property_input
            .text()
            .trim()
            .parse()
            .unwrap_or(0.0);
        let next = current + delta;
        // Instance-write redirect (GAP #10) — see property_dispatch
        // for the choke-point note.
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        let _ = self.editor_state.commit_property_edit(focus, next);
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        // Reflect the committed value back into the draft so the
        // field shows it and a further step builds on the new value.
        let next_text = if next.fract() == 0.0 {
            format!("{}", next as i64)
        } else {
            format!("{next}")
        };
        self.editor_state
            .ui
            .property_input
            .set_text(next_text.clone());
        self.editor_state.ui.property_input.touch(self.now_ms);
        self.editor_state.ui.property_input_draft = next_text;
        self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    /// Left / Right arrow during an inline rename — moves the rename
    /// caret one character. Returns `false` when no rename is active,
    /// so the caller falls back to the property caret / node-nudge.
    pub fn apply_rename_caret(&mut self, forward: bool) -> bool {
        let moved = shared::rename_caret(&mut self.editor_state, forward, self.now_ms);
        if moved {
            self.mark_dirty();
        }
        moved
    }

    /// Left / Right arrow on the focused chat input. Consumes the key
    /// even at text boundaries so it never falls through to canvas nudge.
    pub fn apply_chat_input_caret(&mut self, forward: bool) -> bool {
        if shared::chat_input_caret(&mut self.editor_state, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Left / Right arrow on a focused property input — moves the
    /// text caret one character. Returns `false` when no property
    /// input is focused, so the caller falls back to node-nudge.
    pub fn apply_property_caret(&mut self, forward: bool) -> bool {
        if shared::property_caret_move(&mut self.editor_state, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        // #20: the preset-name input rides the flat legacy draft, not a
        // `TextInputState`, so it has its own caret module. Consumed
        // even when the caret can't move — an arrow over a focused
        // input must never fall through to nudging the selected node.
        if let Some(moved) =
            preset_name::preset_name_caret_move(&mut self.editor_state, forward, self.now_ms)
        {
            if moved {
                self.mark_dirty();
            }
            return true;
        }
        false
    }

    /// Arrow-key nudge — translate selection by (dx, dy) doc px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if shared::nudge_selection(&mut self.editor_state, dx, dy) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_send(&mut self) -> bool {
        // Preview mode: Enter goes to the focused runtime widget
        // (textarea newline / activation), never chat send.
        if self.preview.is_some() {
            return self.preview_dispatch_key("Enter", false);
        }
        // The image popover is painted above every editor input. Submit or
        // swallow Enter before consulting any independently stale focus below.
        if self.apply_image_panel_send() {
            return true;
        }
        if self.exit_image_crop_edit() {
            return true;
        }
        if self.editor_state.color_picker_hex_focused() {
            self.editor_state.color_picker_blur_hex();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.color_picker_rgb_focused() {
            self.editor_state.color_picker_blur_rgb();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            self.commit_settings_focus_if_any();
            return true;
        }
        // Font-family picker: swallow Enter so it can't leak into
        // chat send / property commit while the overlay is open.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        // Enter is owned by the clone wizard whenever it is open: a
        // focused field (not mid-clone) requests the clone; otherwise the
        // key is simply swallowed so it can't fall through to chat send
        // or any other action.
        if self.git_clone_input_active() {
            let submit = self
                .editor_state
                .editor_ui
                .git_panel
                .clone_form
                .as_ref()
                .is_some_and(|f| f.focus.is_some() && !f.cloning);
            if submit {
                self.editor_state.editor_ui.git_panel.pending_action =
                    Some(op_editor_core::GitPanelAction::SubmitClone);
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git commit input requests a commit — needs a
        // message and a staged file (the commit is the staged set).
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.commit_input.text().trim().is_empty()
                && panel.changed_files.iter().any(|f| f.staged)
            {
                panel.pending_action = Some(op_editor_core::GitPanelAction::Commit);
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git remote-URL input sets `origin`.
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.remote_input.text().trim().is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SetRemote(
                    panel.remote_input.text().to_owned(),
                ));
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git HTTPS-credential input stores it.
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.https_input.text().trim().is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SetHttpsAuth(
                    panel.https_input.text().to_owned(),
                ));
            }
            self.mark_dirty();
            return true;
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let name = panel.branch_create_input.text().trim().to_string();
            if !name.is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::CreateBranch(name));
                panel.branch_picker_mode = op_editor_core::GitBranchPickerMode::List;
                panel.branch_create_input.set_text("");
                panel.branch_create_focused = false;
                panel.branch_picker_open = false;
                panel.branch_picker_menu.hover = None;
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the commit-signature form submits it when valid; swallowed
        // either way so it never falls through to the global chat send.
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.author_name_input.text().trim().is_empty()
                && panel.author_email_input.text().contains('@')
            {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SaveAuthor);
            }
            self.mark_dirty();
            return true;
        }
        // While a ready-state popover (branch picker / overflow menu) is
        // actually visible with no focused input, swallow Enter so it can't
        // fall through to the global chat send below. (Focused inputs already
        // submitted above; the helper requires the ready view so a stale flag
        // on a closed / merging / diff panel can't eat global Enter.)
        if self.git_ready_popover_open() {
            return true;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_commit();
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            // Enter INSERTS a newline (TS textarea parity) — only
            // Escape / outside click commit the session. Swallow the
            // key either way so it never falls through to chat send.
            if self.editor_state.text_edit_insert("\n", self.now_ms) {
                self.mark_dirty();
            }
            return true;
        }
        if let Some(ok) = self.apply_pen_enter() {
            return ok;
        }
        // #20: Enter in the preset-name input saves the preset
        // (variable-theme-manager.tsx:298).
        if self.commit_variables_preset_name_if_any() {
            return true;
        }
        // Enter in the variables search box just blurs it (the filter
        // is already live) — the same transition Escape runs.
        if self.editor_state.editor_ui.blur_variables_search() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            self.commit_variables_panel_header_focus_if_any();
            return true;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            self.commit_variable_row_focus_if_any();
            return true;
        }
        if self.editor_state.editor_ui.effect_param_focus.is_some() {
            self.commit_effect_param_focus_if_any();
            return true;
        }
        if self.editor_state.ui.property_focus.is_some() {
            self.commit_property_focus_if_any();
            return true;
        }
        if self.editor_state.chat.available_models.is_empty() {
            return false;
        }
        // `begin_send` itself gates on (text OR staged attachments) —
        // an attachment-only turn is valid, so don't short-circuit on
        // empty text here.
        // Real provider turn — raises `chat.pending_send`.
        let sent = self.editor_state.chat.begin_send();
        if sent {
            self.mark_dirty();
        }
        sent
    }

    /// Escape — priority cascade: rename → property → pickers →
    /// chat → selection. One layer per press.
    pub fn apply_escape(&mut self) -> bool {
        // Escape EXITS preview mode (top priority) — drops the runtime
        // and returns to the design surface.
        if self.preview.is_some() {
            self.exit_preview();
            return true;
        }
        if self.editor_state.color_picker_hex_focused() {
            self.editor_state.color_picker_blur_hex();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.color_picker_rgb_focused() {
            self.editor_state.color_picker_blur_rgb();
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .agent_settings
            .focus
            .take()
            .is_some()
        {
            self.clear_settings_caret();
            self.mark_dirty();
            return true;
        }
        // #20: Escape closes the preset-name input only — the
        // preset dropdown stays open (variable-theme-manager.tsx:299).
        if self.escape_variables_preset_name() {
            return true;
        }
        // Escape blurs the variables search box, keeping the filter.
        if self.editor_state.editor_ui.blur_variables_search() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_variables_row_menu() {
            self.mark_dirty();
            return true;
        }
        // Escape steps out of the clone wizard: first defocus the active
        // field, then (on a second press) close the wizard back to the
        // empty state.
        if self.git_clone_input_active() {
            let defocused = {
                let form = self
                    .editor_state
                    .editor_ui
                    .git_panel
                    .clone_form
                    .as_mut()
                    .unwrap();
                let url_caret = form.url_input.caret();
                form.url_input.set_caret(url_caret, self.now_ms);
                let dest_caret = form.dest_input.caret();
                form.dest_input.set_caret(dest_caret, self.now_ms);
                form.focus.take().is_some()
            };
            if !defocused {
                self.editor_state.editor_ui.git_panel.clone_form = None;
            }
            self.mark_dirty();
            return true;
        }
        // A branch-picker sub-mode (create / merge) takes Escape priority
        // OVER the Git input fields: step it back to the branch list (the
        // dropdown stays open). Driven off the mode, not input focus, so a
        // stale commit / remote / https focus can't intercept it, and merge
        // mode (which has no focused input) exits too.
        if self.editor_state.editor_ui.git_panel.branch_picker_open
            && self.editor_state.editor_ui.git_panel.branch_picker_mode
                != op_editor_core::GitBranchPickerMode::List
        {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.branch_picker_mode = op_editor_core::GitBranchPickerMode::List;
            panel.branch_picker_menu.hover = None;
            panel.branch_create_input.set_text("");
            panel.branch_create_focused = false;
            self.mark_dirty();
            return true;
        }
        // Escape dismisses the commit-signature form (TS form cancel) without
        // committing — checked before the input-focus handlers so a focused
        // name/email field doesn't swallow it.
        if self.editor_state.editor_ui.git_panel.author_prompt {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.author_prompt = false;
            panel.author_name_focused = false;
            panel.author_email_focused = false;
            let caret = panel.author_name_input.caret();
            panel.author_name_input.set_caret(caret, self.now_ms);
            let caret = panel.author_email_input.caret();
            panel.author_email_input.set_caret(caret, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // Escape defocuses the Git commit input (the panel stays open).
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.defocus_commit_input(self.now_ms);
            self.mark_dirty();
            return true;
        }
        // …and the Git remote-URL input.
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.remote_focused = false;
            let caret = panel.remote_input.caret();
            panel.remote_input.set_caret(caret, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // …and the Git HTTPS-credential input.
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.https_focused = false;
            let caret = panel.https_input.caret();
            panel.https_input.set_caret(caret, self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.font_picker.open
            && matches!(
                self.editor_state.editor_ui.font_picker_purpose,
                Some(op_editor_core::FontPickerPurpose::MissingFont { .. })
            )
        {
            self.close_font_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_agent_settings_modal() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_export_dialog() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            // Divergence kept on purpose: only the native host runs the
            // multi-page Figma picker, so only it has a Cancel
            // selection to post back before the shared close.
            if self.editor_state.editor_ui.figma_import_pages.len() > 1 {
                self.editor_state.editor_ui.pending_file_action = Some(
                    op_editor_core::editor_ui_state::FileAction::FinishFigmaImport(
                        op_editor_core::FigmaImportSelection::Cancel,
                    ),
                );
            }
            self.editor_state.editor_ui.escape_import_modal();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.import_menu_open {
            self.close_import_menu();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_file_menu() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_layer_context_menu() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.rename_cancel() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.text_edit_commit() {
            self.mark_dirty();
            return true;
        }
        // Anchor-menu close, then pen CANCEL (TS Escape discards).
        if self.apply_pen_escape() {
            return true;
        }
        if self.editor_state.editor_ui.close_corner_expand() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_effect_add_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.compositing_picker.open {
            self.close_compositing_picker();
            self.mark_dirty();
            return true;
        }
        if escape::escape_variable_row_focus(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        if escape::escape_effect_param_focus(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        if escape::escape_property_focus(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_locale_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_shape_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_icon_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_chat_model_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_component_browser() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            self.clear_image_input_selection_drag();
            self.editor_state.editor_ui.image_panel.close_popovers();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.font_picker.open {
            self.close_font_picker();
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .escape_instance_component_picker()
        {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_fill_type_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.interaction_menu_open {
            self.editor_state.editor_ui.close_interaction_menu();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_image_fill_popover() {
            self.mark_dirty();
            return true;
        }
        if self.exit_image_crop_edit() {
            return true;
        }
        if escape::escape_chat_focus(&mut self.editor_state, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if escape::escape_selection(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        // TS Escape order (use-tool-shortcuts.ts:38-49): clearing the
        // selection comes first; the NEXT Escape steps out of the
        // entered frame/group.
        if self
            .editor_state
            .editor_ui
            .entered_container
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        false
    }
}
