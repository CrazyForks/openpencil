//! Scene Template card previews.
//!
//! Baked from the full-resolution renders by
//! `templates/step0/_generators/scene_preview_cards.py`; see that script for
//! why a deck is tiled into a grid rather than fitted as a strip.
//!
//! ~388 KB of JPEG. Embedded on desktop, fetched from the daemon on `wasm32`
//! for the same reason the Prompt Center's are — see
//! `prompt_center_previews` for the full rationale and
//! `op_editor_core::web_assets` for the loader.

/// Cache ids are hand-assigned and must stay stable: the renderer keys its
/// decoded-raster cache on them, so reusing an id for different bytes would
/// serve the wrong image. They start above the Prompt Center's range so the
/// two catalogues can never collide in that shared cache.
const CACHE_ID_BASE: u64 = 10_000;

// Test-only: `concat!` cannot interpolate a const, so the route literals in
// the macro below spell the directory out. This is the value the route
// tests rebuild the expected path from, which is what keeps the spelled-out
// literal and the staged bundle layout (`tools/stage-web-assets.sh`) from
// drifting into a silent per-asset 404.
#[cfg(test)]
const PREVIEW_DIR: &str = "scene_template_previews";

/// One card's preview, resolved for the current platform.
pub(crate) struct TemplatePreview {
    /// Stable renderer cache id, known before any bytes are.
    pub image_id: u64,
    /// `None` on wasm until the fetch lands; always `Some` on native.
    pub bytes: Option<&'static [u8]>,
    /// Daemon route to fetch. Unused on native.
    pub route: &'static str,
}

macro_rules! preview {
    ($offset:expr, $name:literal) => {{
        const ROUTE: &str = concat!(
            "/pkg/assets/",
            "scene_template_previews",
            "/",
            $name,
            ".jpg"
        );
        #[cfg(not(target_arch = "wasm32"))]
        let bytes = Some(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/scene_template_previews/",
                $name,
                ".jpg"
            ))
            .as_slice(),
        );
        #[cfg(target_arch = "wasm32")]
        let bytes = op_editor_core::web_assets::installed_bytes(ROUTE);
        Some(TemplatePreview {
            image_id: CACHE_ID_BASE + $offset,
            bytes,
            route: ROUTE,
        })
    }};
}

/// Return the preview for a template id.
///
/// Every shipped template has one — `scene_template_catalog` rejects a
/// catalogue entry without a document, and the preview baker is driven by the
/// same id list — so `None` means an unknown id, not a missing asset. On web a
/// `Some` with `bytes: None` means "not fetched yet", which is not the same
/// answer.
pub(crate) fn scene_template_preview(template_id: &str) -> Option<TemplatePreview> {
    match template_id {
        "screenshot-tutorial" => preview!(1, "screenshot-tutorial"),
        "knowledge-carousel" => preview!(2, "knowledge-carousel"),
        "before-after" => preview!(3, "before-after"),
        "slide-deck" => preview!(4, "slide-deck"),
        "knowledge-card-vertical" => preview!(5, "knowledge-card-vertical"),
        "knowledge-card-square" => preview!(6, "knowledge-card-square"),
        "pitch-deck-dark" => preview!(7, "pitch-deck-dark"),
        "lecture-deck-light" => preview!(8, "lecture-deck-light"),
        "minimal-keynote" => preview!(9, "minimal-keynote"),
        "gradient-tech" => preview!(10, "gradient-tech"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::scene_template_catalog::scene_template_catalogue;
    use std::collections::HashSet;

    #[test]
    fn every_shipped_template_has_a_preview_with_a_unique_cache_id() {
        let mut ids = HashSet::new();
        for template in scene_template_catalogue() {
            let preview = scene_template_preview(&template.id)
                .unwrap_or_else(|| panic!("{} has no card preview", template.id));
            let bytes = preview.bytes.expect("native embeds every preview");
            assert!(!bytes.is_empty(), "{} preview is empty", template.id);
            assert!(
                ids.insert(preview.image_id),
                "{} reuses cache id {}",
                template.id,
                preview.image_id
            );
        }
        assert!(scene_template_preview("no-such-template").is_none());
    }

    #[test]
    fn previews_are_jpeg_so_the_raster_decoder_accepts_them() {
        for template in scene_template_catalogue() {
            let bytes = scene_template_preview(&template.id)
                .expect("preview")
                .bytes
                .expect("native embeds every preview");
            assert_eq!(&bytes[..2], &[0xFF, 0xD8], "{} is not a JPEG", template.id);
        }
    }

    #[test]
    fn every_route_is_distinct_and_uses_the_shared_prefix() {
        // The route is the web host's identity for the asset, exactly as the
        // cache id is the renderer's. `concat!` cannot interpolate the prefix
        // constant, so this is what keeps the spelled-out literal honest.
        let mut routes = HashSet::new();
        for template in scene_template_catalogue() {
            let preview = scene_template_preview(&template.id).expect("preview");
            assert_eq!(
                preview.route,
                format!(
                    "{}{}/{}.jpg",
                    op_editor_core::web_assets::WEB_ASSET_ROUTE_PREFIX,
                    PREVIEW_DIR,
                    template.id
                )
            );
            assert!(routes.insert(preview.route), "duplicate {}", preview.route);
        }
    }
}
