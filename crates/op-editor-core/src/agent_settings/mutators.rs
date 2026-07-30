//! `impl AgentSettings` — the modal's mutators: add / draft / save /
//! cancel for built-in + ACP agents, image-generation profile CRUD, and
//! the browser-snapshot ownership transfers. Carved off
//! `agent_settings.rs` to keep every file under the 800-line cap.

use std::collections::BTreeMap;

use super::{
    AcpAgentConfig, AcpConnectionType, AgentSettings, BuiltinAgentConfig, BuiltinAgentKind,
    ImageGenProfile, ImageGenProvider, ImageTestStatus,
};
use crate::agent_settings_builtin_presets::{
    builtin_agent_preset, infer_builtin_agent_preset, BuiltinAgentPresetKey, BUILTIN_AGENT_PRESETS,
};

impl AgentSettings {
    /// Turn a browser-snapshot built-in into a daemon/operator-owned entry
    /// before native settings mutate it. Browser snapshots identify ownership
    /// through the scoped id, so changing the id is the ownership transfer.
    pub fn take_over_browser_builtin_agent(&mut self, index: usize) -> bool {
        let Some(old_id) = self
            .builtin_agents
            .get(index)
            .map(|agent| agent.id.clone())
            .filter(|id| id.starts_with("web-credential:builtin:"))
        else {
            return false;
        };
        let next = next_free_numeric_id(
            self.next_builtin_agent_id,
            "builtin-",
            self.builtin_agents.iter().map(|agent| agent.id.as_str()),
        );
        self.builtin_agents[index].id = format!("builtin-{next}");
        self.next_builtin_agent_id = next.saturating_add(1);
        debug_assert_ne!(self.builtin_agents[index].id, old_id);
        true
    }

    /// Operator ownership transfer for a browser-snapshot image profile. Keep
    /// the active-profile pointer aligned with the newly allocated local id.
    pub fn take_over_browser_image_profile(&mut self, index: usize) -> bool {
        let Some(old_id) = self
            .image_gen_profiles
            .get(index)
            .map(|profile| profile.id.clone())
            .filter(|id| id.starts_with("web-credential:image:"))
        else {
            return false;
        };
        let next = next_free_numeric_id(
            self.next_image_gen_profile_id,
            "igp-",
            self.image_gen_profiles
                .iter()
                .map(|profile| profile.id.as_str()),
        );
        let new_id = format!("igp-{next}");
        self.image_gen_profiles[index].id = new_id.clone();
        self.next_image_gen_profile_id = next.saturating_add(1);
        if self.active_image_gen_profile_id.as_deref() == Some(old_id.as_str()) {
            self.active_image_gen_profile_id = Some(new_id);
        }
        true
    }

    pub fn add_builtin_agent(&mut self) -> String {
        if let Some(preset) = BUILTIN_AGENT_PRESETS.iter().find(|preset| {
            !self
                .builtin_agents
                .iter()
                .any(|agent| agent.display_name == preset.display_name)
        }) {
            return self.add_builtin_agent_config(
                preset.display_name,
                "",
                preset.model,
                preset.kind,
                preset.base_url,
            );
        }
        let n = self.next_builtin_agent_id.max(1);
        let name = format!("Built-in Agent {n}");
        self.add_builtin_agent_with_defaults(
            &name,
            "",
            crate::agent_settings_builtin_presets::DEFAULT_ANTHROPIC_MODEL,
        )
    }

    pub fn begin_builtin_agent_draft(&mut self) {
        if self.builtin_agent_draft.is_some() {
            return;
        }
        let preset = builtin_agent_preset(BuiltinAgentPresetKey::Anthropic);
        self.builtin_agent_draft = Some(BuiltinAgentConfig {
            id: String::new(),
            preset: preset.key,
            display_name: preset.display_name.into(),
            kind: preset.kind,
            api_key: String::new(),
            model: preset.model.into(),
            base_url: preset.base_url.into(),
            enabled: true,
        });
    }

    pub fn set_builtin_agent_preset(&mut self, index: usize, preset: BuiltinAgentPresetKey) {
        if let Some(agent) = self.builtin_agents.get_mut(index) {
            let api_key = agent.api_key.clone();
            let enabled = agent.enabled;
            agent.apply_preset(preset);
            agent.api_key = api_key;
            agent.enabled = enabled;
        }
    }

    pub fn set_builtin_agent_draft_preset(&mut self, preset: BuiltinAgentPresetKey) {
        if let Some(agent) = self.builtin_agent_draft.as_mut() {
            let api_key = agent.api_key.clone();
            agent.apply_preset(preset);
            agent.api_key = api_key;
        }
    }

    pub fn save_builtin_agent_draft(&mut self) -> Option<String> {
        if !self
            .builtin_agent_draft
            .as_ref()
            .is_some_and(|draft| draft.ready())
        {
            return None;
        }
        let draft = self.builtin_agent_draft.take()?;
        Some(self.add_builtin_agent_config(
            draft.display_name,
            draft.api_key,
            draft.model,
            draft.kind,
            draft.base_url,
        ))
    }

    pub fn cancel_builtin_agent_draft(&mut self) {
        self.builtin_agent_draft = None;
        self.builtin_preset_menu_open = None;
        self.builtin_preset_menu_scroll.offset = 0.0;
        self.builtin_preset_menu_hover = None;
    }

    pub fn add_builtin_agent_with_defaults(
        &mut self,
        display_name: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> String {
        self.add_builtin_agent_config(
            display_name,
            api_key,
            model,
            BuiltinAgentKind::Anthropic,
            BuiltinAgentKind::Anthropic.default_base_url(),
        )
    }

    pub fn add_builtin_agent_config(
        &mut self,
        display_name: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        kind: BuiltinAgentKind,
        base_url: impl Into<String>,
    ) -> String {
        let display_name = display_name.into();
        let api_key = api_key.into();
        let model = model.into();
        let base_url = base_url.into();
        if let Some(existing) = self.builtin_agents.iter().find(|agent| {
            agent.matches_add_candidate(&display_name, &api_key, &model, kind, &base_url)
        }) {
            return existing.id.clone();
        }
        let id = format!("builtin-{}", self.next_builtin_agent_id.max(1));
        self.next_builtin_agent_id = self.next_builtin_agent_id.max(1).saturating_add(1);
        self.builtin_agents.push(BuiltinAgentConfig {
            id: id.clone(),
            preset: infer_builtin_agent_preset(kind, &base_url, &model),
            display_name,
            kind,
            api_key,
            model,
            base_url,
            enabled: true,
        });
        id
    }

    pub fn add_acp_agent(&mut self) -> String {
        let n = self.next_acp_agent_id.max(1);
        self.add_acp_agent_config(
            format!("ACP Agent {n}"),
            AcpConnectionType::Local,
            "",
            Vec::new(),
            BTreeMap::new(),
            None,
            true,
        )
    }

    pub fn begin_acp_agent_draft(&mut self) {
        if self.acp_agent_draft.is_some() {
            return;
        }
        let n = self.next_acp_agent_id.max(1);
        self.acp_agent_draft = Some(AcpAgentConfig {
            id: String::new(),
            display_name: format!("ACP Agent {n}"),
            connection_type: AcpConnectionType::Local,
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            url: None,
            enabled: true,
            connected: false,
        });
    }

    pub fn save_acp_agent_draft(&mut self) -> Option<String> {
        if !self
            .acp_agent_draft
            .as_ref()
            .is_some_and(|draft| draft.ready())
        {
            return None;
        }
        let draft = self.acp_agent_draft.take()?;
        Some(self.add_acp_agent_config(
            draft.display_name,
            draft.connection_type,
            draft.command,
            draft.args,
            draft.env,
            draft.url,
            draft.enabled,
        ))
    }

    pub fn cancel_acp_agent_draft(&mut self) {
        self.acp_agent_draft = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_acp_agent_config(
        &mut self,
        display_name: impl Into<String>,
        connection_type: AcpConnectionType,
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        url: Option<String>,
        enabled: bool,
    ) -> String {
        let id = format!("acp-{}", self.next_acp_agent_id.max(1));
        self.next_acp_agent_id = self.next_acp_agent_id.max(1).saturating_add(1);
        self.acp_agents.push(AcpAgentConfig {
            id: id.clone(),
            display_name: display_name.into(),
            connection_type,
            command: command.into(),
            args,
            env,
            url,
            enabled,
            connected: false,
        });
        id
    }

    pub fn remove_acp_agent(&mut self, id: &str) -> bool {
        let before = self.acp_agents.len();
        self.invalidate_acp_agent_connection(id);
        self.acp_agents.retain(|agent| agent.id != id);
        self.acp_agents.len() != before
    }

    /// Active image-generation profile, matching the product fallback of the
    /// configured id first and the first row second.
    pub fn active_image_gen_profile(&self) -> Option<&ImageGenProfile> {
        self.image_gen_profiles
            .iter()
            .find(|profile| Some(&profile.id) == self.active_image_gen_profile_id.as_ref())
            .or_else(|| self.image_gen_profiles.first())
    }

    pub fn image_generation_configured(&self) -> bool {
        self.active_image_gen_profile()
            .is_some_and(|profile| !profile.api_key.trim().is_empty())
    }

    pub fn add_image_gen_profile(&mut self) -> String {
        let n = self.next_image_gen_profile_id.max(1);
        let id = format!("igp-{n}");
        self.next_image_gen_profile_id = n.saturating_add(1);
        self.image_gen_profiles.push(ImageGenProfile {
            id: id.clone(),
            name: format!("Config {n}"),
            provider: ImageGenProvider::OpenAi,
            api_key: String::new(),
            model: String::new(),
            base_url: None,
            test_status: ImageTestStatus::Idle,
        });
        if self.active_image_gen_profile_id.is_none() {
            self.active_image_gen_profile_id = Some(id.clone());
        }
        id
    }

    pub fn set_active_image_gen_profile(&mut self, id: &str) -> bool {
        if self.image_gen_profiles.iter().any(|p| p.id == id) {
            self.active_image_gen_profile_id = Some(id.to_string());
            true
        } else {
            false
        }
    }

    pub fn remove_image_gen_profile(&mut self, id: &str) -> bool {
        let before = self.image_gen_profiles.len();
        self.image_gen_profiles.retain(|p| p.id != id);
        if self.image_gen_profiles.len() == before {
            return false;
        }
        self.image_gen_provider_menu_open = None;
        self.hover_image_gen_provider_option = None;
        self.hover_image_gen_profile_header = None;
        self.hover_image_gen_profile_remove = None;
        self.hover_image_gen_profile_provider = None;
        self.hover_image_gen_profile_test = None;
        if self.active_image_gen_profile_id.as_deref() == Some(id) {
            self.active_image_gen_profile_id =
                self.image_gen_profiles.first().map(|p| p.id.clone());
        }
        true
    }
}

fn next_free_numeric_id<'a>(
    requested: u64,
    prefix: &str,
    ids: impl Iterator<Item = &'a str>,
) -> u64 {
    let used: std::collections::HashSet<u64> = ids
        .filter_map(|id| id.strip_prefix(prefix)?.parse().ok())
        .collect();
    let mut next = requested.max(1);
    while used.contains(&next) {
        let advanced = next.saturating_add(1);
        if advanced == next {
            break;
        }
        next = advanced;
    }
    next
}
