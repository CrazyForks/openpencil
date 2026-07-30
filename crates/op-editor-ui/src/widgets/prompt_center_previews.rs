//! Compile-time Prompt Center preview registry.
//!
//! Preview bytes stay inside the wasm bundle so card painting never performs
//! runtime file or network I/O.

const PREVIEW_IMAGE_ID_BASE: u64 = 0x5052_4d50_0000_0000;

macro_rules! preview {
    ($index:literal, $file:literal) => {
        Some((
            PREVIEW_IMAGE_ID_BASE | $index,
            include_bytes!(concat!(
                "../../assets/prompt_center_previews/",
                $file,
                ".jpg"
            ))
            .as_slice(),
        ))
    };
}

/// Return the stable cache id and embedded JPEG bytes for a built-in prompt.
///
/// Starter quick actions, user-defined prompts, and unknown ids intentionally
/// have no generated preview and therefore return `None`.
pub(crate) fn prompt_center_preview(prompt_id: &str) -> Option<(u64, &'static [u8])> {
    match prompt_id {
        "gallery-wander" => preview!(1, "gallery-wander"),
        "gallery-forage" => preview!(2, "gallery-forage"),
        "gallery-still" => preview!(3, "gallery-still"),
        "gallery-hearth" => preview!(4, "gallery-hearth"),
        "gallery-meteo" => preview!(5, "gallery-meteo"),
        "gallery-marginalia" => preview!(6, "gallery-marginalia"),
        "gallery-lingua" => preview!(7, "gallery-lingua"),
        "gallery-daybreak" => preview!(8, "gallery-daybreak"),
        "gallery-verdant" => preview!(9, "gallery-verdant"),
        "gallery-companion" => preview!(10, "gallery-companion"),
        "gallery-relic" => preview!(11, "gallery-relic"),
        "gallery-nocturne" => preview!(12, "gallery-nocturne"),
        "gallery-marquee" => preview!(13, "gallery-marquee"),
        "gallery-ritual" => preview!(14, "gallery-ritual"),
        "gallery-ember" => preview!(15, "gallery-ember"),
        "gallery-volt" => preview!(16, "gallery-volt"),
        "gallery-aloft" => preview!(17, "gallery-aloft"),
        "gallery-gallery" => preview!(18, "gallery-gallery"),
        "gallery-nightcap" => preview!(19, "gallery-nightcap"),
        "gallery-bloom" => preview!(20, "gallery-bloom"),
        "freeform-wander" => preview!(21, "freeform-wander"),
        "freeform-forage" => preview!(22, "freeform-forage"),
        "freeform-still" => preview!(23, "freeform-still"),
        "freeform-hearth" => preview!(24, "freeform-hearth"),
        "freeform-meteo" => preview!(25, "freeform-meteo"),
        "freeform-marginalia" => preview!(26, "freeform-marginalia"),
        "freeform-lingua" => preview!(27, "freeform-lingua"),
        "freeform-daybreak" => preview!(28, "freeform-daybreak"),
        "freeform-verdant" => preview!(29, "freeform-verdant"),
        "freeform-companion" => preview!(30, "freeform-companion"),
        "freeform-relic" => preview!(31, "freeform-relic"),
        "freeform-nocturne" => preview!(32, "freeform-nocturne"),
        "freeform-marquee" => preview!(33, "freeform-marquee"),
        "freeform-ritual" => preview!(34, "freeform-ritual"),
        "freeform-ember" => preview!(35, "freeform-ember"),
        "freeform-volt" => preview!(36, "freeform-volt"),
        "freeform-aloft" => preview!(37, "freeform-aloft"),
        "freeform-gallery" => preview!(38, "freeform-gallery"),
        "freeform-nightcap" => preview!(39, "freeform-nightcap"),
        "freeform-bloom" => preview!(40, "freeform-bloom"),
        "extreme-weather" => preview!(41, "extreme-weather"),
        "extreme-now-playing" => preview!(42, "extreme-now-playing"),
        "extreme-daily-app" => preview!(43, "extreme-daily-app"),
        "extreme-calendar" => preview!(44, "extreme-calendar"),
        "extreme-calm" => preview!(45, "extreme-calm"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use op_editor_core::prompt_center_catalog::{prompt_catalogue, PromptCategory};

    use super::prompt_center_preview;

    #[test]
    fn every_generated_catalogue_entry_has_one_unique_preview() {
        let generated: Vec<_> = prompt_catalogue()
            .iter()
            .filter(|prompt| prompt.category != PromptCategory::Starter)
            .collect();
        assert_eq!(generated.len(), 45);

        let mut image_ids = HashSet::new();
        for prompt in generated {
            let (image_id, bytes) = prompt_center_preview(&prompt.id)
                .unwrap_or_else(|| panic!("missing preview for `{}`", prompt.id));
            assert!(
                image_ids.insert(image_id),
                "duplicate image id {image_id:#x}"
            );
            assert_eq!(
                crate::image_runtime::encoded_image_dimensions(bytes),
                Some((640, 400)),
                "wrong preview dimensions for `{}`",
                prompt.id
            );
        }
        assert_eq!(image_ids.len(), 45);
    }

    #[test]
    fn starter_custom_and_unknown_ids_have_no_generated_preview() {
        for prompt in prompt_catalogue()
            .iter()
            .filter(|prompt| prompt.category == PromptCategory::Starter)
        {
            assert!(
                prompt_center_preview(&prompt.id).is_none(),
                "starter `{}` unexpectedly has a preview",
                prompt.id
            );
        }

        assert!(prompt_center_preview("custom-1").is_none());
        assert!(prompt_center_preview("unknown-prompt").is_none());
    }
}
