//! MCP tools `list_scene_templates` and `use_scene_template` — the shipped
//! scene-template catalogue, including the 16:9 deck templates.
//!
//! The catalogue has been in the editor since `v0.8.3` behind File ▸ New from
//! template, so an agent could only ever start from a blank frame. Listing it
//! is what lets one start from a real layout, and the deck templates are the
//! entry point to the whole presentation workflow.
//!
//! `use_scene_template` returns a command rather than mutating: the boards and
//! the palette they resolve against have to land in one transaction, which is
//! exactly what `EditorCommand::AdoptSceneTemplate` gives. On an untouched
//! starter page the template takes the page over; anywhere else it appends to
//! the right of what is already there.

use std::collections::BTreeMap;

use op_editor_core::scene_template_catalog::{scene_template_by_id, scene_template_catalogue};
use op_editor_core::EditorCommand;
use op_mcp::{McpTool, ToolErrorCode, ToolOutcome};

pub struct ListSceneTemplates;

impl McpTool for ListSceneTemplates {
    fn name(&self) -> &str {
        "list_scene_templates"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let scene_filter = args
            .get("scene")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());
        let tag_filter = args
            .get("tag")
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());

        let templates: Vec<serde_json::Value> = scene_template_catalogue()
            .iter()
            .filter(|template| {
                scene_filter.is_none_or(|scene| scene.eq_ignore_ascii_case(template.scene.as_str()))
            })
            .filter(|template| {
                tag_filter.is_none_or(|tag| {
                    template
                        .tags
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(tag))
                })
            })
            .map(|template| {
                serde_json::json!({
                    "id": template.id,
                    "scene": template.scene.as_str(),
                    "title": template.title_fallback,
                    "summary": template.summary_fallback,
                    "tags": template.tags,
                    "frames": template.frames,
                    "frameWidth": template.frame_width,
                    "frameHeight": template.frame_height,
                    // Absent rather than null when a template carries no
                    // guide: that is the gate on generating from it, not a
                    // missing field.
                    "styleGuide": template.style_guide,
                })
            })
            .collect();

        ToolOutcome::OkJson(serde_json::json!({ "templates": templates }).to_string())
    }
}

pub struct UseSceneTemplate;

impl McpTool for UseSceneTemplate {
    fn name(&self) -> &str {
        "use_scene_template"
    }

    fn call(&self, args: &BTreeMap<String, String>) -> ToolOutcome {
        let template_id = match args.get("templateId").map(|value| value.trim()) {
            Some(id) if !id.is_empty() => id,
            _ => {
                return ToolOutcome::Err(
                    ToolErrorCode::MissingArgument,
                    "templateId is required — call list_scene_templates for the catalogue".into(),
                );
            }
        };
        // Validate here so an unknown id is a named argument error rather
        // than a command the host silently applies as a no-op.
        let Some(template) = scene_template_by_id(template_id) else {
            return ToolOutcome::Err(
                ToolErrorCode::InvalidArgument,
                format!("unknown template id {template_id:?}"),
            );
        };
        ToolOutcome::OkJsonWithCommand(
            serde_json::json!({
                "id": template.id,
                "title": template.title_fallback,
                "frames": template.frames,
                "frameWidth": template.frame_width,
                "frameHeight": template.frame_height,
            })
            .to_string(),
            EditorCommand::AdoptSceneTemplate {
                template_id: template.id.clone(),
            },
        )
    }
}

pub fn list_scene_templates_snapshot() -> ListSceneTemplates {
    ListSceneTemplates
}

pub fn use_scene_template_snapshot() -> UseSceneTemplate {
    UseSceneTemplate
}
