//! Chat model selection and catalog rebuilds on [`EditorState`].

use crate::EditorState;

impl EditorState {
    /// Select a chat model and close the picker. Built-in dynamic rows also
    /// become the provider's persisted default, while each chat tab retains
    /// its own selected row identity for request-time routing.
    pub fn select_chat_model(&mut self, idx: usize) {
        let update = self.chat.available_models.get(idx).map(|entry| {
            (
                entry.provider,
                entry.builtin_provider_id.is_none() && entry.acp_agent_id().is_none(),
                entry
                    .builtin_provider_id
                    .as_deref()
                    .zip(entry.builtin_model_id())
                    .map(|(id, model)| (id.to_string(), model.to_string())),
            )
        });
        if let Some((provider, use_native_agent, builtin_model)) = update {
            self.chat.selected_model = idx;
            if let Some((id, model)) = builtin_model {
                if let Some(agent) = self
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .iter_mut()
                    .find(|agent| agent.id == id)
                {
                    agent.model = model;
                }
            }
            if use_native_agent {
                if let Some(pidx) = crate::AgentProvider::ALL
                    .iter()
                    .position(|candidate| *candidate == provider)
                {
                    self.editor_ui.chat_selected_agent = pidx;
                }
            }
        }
        self.editor_ui.close_chat_model_picker();
    }

    /// Recompute the active chat tab's selectable catalog. Every ready
    /// built-in contributes its configured model first, followed by normalized
    /// runtime-discovered options that are not duplicates of that model.
    pub fn rebuild_chat_models(&mut self) {
        self.editor_ui.agent_settings.prune_builtin_model_catalogs();
        let previous = self
            .chat
            .available_models
            .get(self.chat.selected_model)
            .cloned();
        let connected = self.editor_ui.agent_settings.verified_connected_mask();
        self.chat.rebuild_available_models(&connected);

        let builtin_entries = self
            .editor_ui
            .agent_settings
            .builtin_agents
            .iter()
            .filter(|agent| agent.discovery_ready())
            .flat_map(|agent| {
                let current = agent.model.trim();
                let mut models = Vec::with_capacity(
                    1 + self
                        .editor_ui
                        .agent_settings
                        .builtin_model_catalog_options(&agent.id)
                        .len(),
                );
                if !current.is_empty() {
                    models.push((current.to_string(), current.to_string()));
                }
                for option in self
                    .editor_ui
                    .agent_settings
                    .builtin_model_catalog_options(&agent.id)
                {
                    let id = option.id.trim();
                    if id.is_empty() || models.iter().any(|(known, _)| known == id) {
                        continue;
                    }
                    let display = option.display_name.trim();
                    models.push((
                        id.to_string(),
                        if display.is_empty() {
                            id.to_string()
                        } else {
                            display.to_string()
                        },
                    ));
                }
                models.into_iter().map(|(model, display_name)| {
                    crate::ModelEntry::builtin_with_display_name(
                        agent.kind.model_provider(),
                        agent.id.clone(),
                        agent.display_name.clone(),
                        format!("builtin:{}:{model}", agent.id),
                        display_name,
                    )
                })
            })
            .collect::<Vec<_>>();
        self.chat.available_models.extend(builtin_entries);

        let settings = &self.editor_ui.agent_settings;
        self.chat.available_models.extend(
            settings
                .acp_agents
                .iter()
                .filter(|agent| agent.ready() && settings.acp_agent_verified_connected(&agent.id))
                .map(|agent| crate::ModelEntry::acp(agent.id.clone(), agent.display_name.clone())),
        );
        if let Some(previous) = previous {
            if let Some(index) = self.chat.available_models.iter().position(|entry| {
                entry.provider == previous.provider
                    && entry.value == previous.value
                    && entry.builtin_provider_id == previous.builtin_provider_id
            }) {
                self.chat.selected_model = index;
            }
        }
        if let Some(entry) = self.chat.selected_model_entry() {
            if entry.builtin_provider_id.is_none() && entry.acp_agent_id().is_none() {
                if let Some(index) = crate::AgentProvider::ALL
                    .iter()
                    .position(|candidate| *candidate == entry.provider)
                {
                    self.editor_ui.chat_selected_agent = index;
                }
            }
        }
    }
}
