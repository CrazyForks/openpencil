//! The Asset Center's Styles tab: style-guide catalogue → card view models.
//!
//! The registry stores whole markdown documents; a card needs a name, a few
//! colours, and one line of prose. Deriving that here — once, from the
//! canonical `op_ai_skills` registry rather than from a hand-copied table —
//! is what keeps the tab honest: a guide added to the corpus shows up in the
//! panel without anyone remembering to list it, and a guide whose palette
//! section changes shows the new swatches.

use op_ai_skills::style_guide::{
    extract_style_guide_values, style_guide_registry, ParsedStyleGuide, Platform,
};
use op_util::hex_color;

use crate::Color;

/// How many swatches a card's colour band paints. The registry's palettes
/// carry more entries than this, but a band is a glance, not a legend.
pub const STYLE_SWATCH_COUNT: usize = 5;

/// One style-guide card.
pub struct StyleGuideCard {
    /// The guide's `name` — also the value pinning writes to the document.
    pub name: &'static str,
    /// Which platform the guide was written for.
    pub platform: Platform,
    /// Palette band, left to right. Entries the guide does not declare are
    /// dropped rather than filled with a placeholder: an invented swatch
    /// would misrepresent the guide the user is about to commit to.
    pub swatches: Vec<Color>,
    /// One line under the name — the guide's own tags, in corpus order.
    pub summary: String,
}

impl StyleGuideCard {
    fn from_guide(guide: &'static ParsedStyleGuide) -> Self {
        let values = extract_style_guide_values(&guide.content);
        let colors = &values.colors;
        let swatches = [
            colors.background.as_deref(),
            colors.surface.as_deref(),
            colors.accent.as_deref(),
            colors.text_primary.as_deref(),
            colors.border.as_deref(),
        ]
        .into_iter()
        .flatten()
        .filter_map(parse_swatch)
        .take(STYLE_SWATCH_COUNT)
        .collect();

        Self {
            name: guide.name.as_str(),
            platform: guide.platform,
            swatches,
            summary: summary_for(guide, &values.typography.display_font),
        }
    }

    /// Whether this card is the pinned one.
    pub fn is_pinned(&self, pinned: Option<&str>) -> bool {
        pinned == Some(self.name)
    }
}

/// The full catalogue as cards, in registry order.
pub fn style_guide_cards() -> Vec<StyleGuideCard> {
    style_guide_registry()
        .iter()
        .map(StyleGuideCard::from_guide)
        .collect()
}

/// Cards surviving a search query, matched against name and tags.
pub fn filtered_style_guide_cards(query: &str) -> Vec<StyleGuideCard> {
    let query = query.trim().to_lowercase();
    style_guide_registry()
        .iter()
        .filter(|guide| {
            query.is_empty()
                || guide.name.to_lowercase().contains(&query)
                || guide.tags.iter().any(|tag| tag.contains(&query))
        })
        .map(StyleGuideCard::from_guide)
        .collect()
}

fn parse_swatch(raw: &str) -> Option<Color> {
    let [r, g, b, a] = hex_color::parse_hex_rgba8(raw, hex_color::HexOptions::LENIENT)?;
    Some(Color::rgba_u8(r, g, b, a as f32 / 255.0))
}

/// "swiss · minimal · light-mode — Inter" — the tags the guide declares,
/// with its display font when it names one. Tags are the vocabulary the
/// prompt ranker already scores against, so a user picking by tag is
/// picking the same way the automatic path does.
fn summary_for(guide: &ParsedStyleGuide, display_font: &Option<String>) -> String {
    let mut summary = guide
        .tags
        .iter()
        .take(4)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" · ");
    if let Some(font) = display_font {
        if !summary.is_empty() {
            summary.push_str(" — ");
        }
        summary.push_str(font);
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registry_guide_becomes_a_card() {
        let cards = style_guide_cards();
        assert_eq!(cards.len(), style_guide_registry().len());
        assert!(cards.len() > 20, "the shipped corpus is not this small");
    }

    #[test]
    fn cards_carry_a_name_swatches_and_a_summary() {
        // Not a spot check on one guide: a card with an empty band or an
        // empty summary paints as a blank tile, and the tab is only useful
        // if every entry is recognizable.
        for card in style_guide_cards() {
            assert!(!card.name.is_empty());
            assert!(
                !card.swatches.is_empty(),
                "{} has no parseable palette colours",
                card.name
            );
            assert!(
                card.swatches.len() <= STYLE_SWATCH_COUNT,
                "{} overflows the band",
                card.name
            );
            assert!(!card.summary.is_empty(), "{} has no summary", card.name);
        }
    }

    #[test]
    fn a_known_guide_reads_its_own_palette() {
        let card = style_guide_cards()
            .into_iter()
            .find(|c| c.name == "nordic-frost-light")
            .expect("the corpus ships this guide");
        assert_eq!(card.platform, Platform::Webapp);
        // The background swatch is the first entry, and a light guide's
        // background must not come back near-black — that would mean the
        // colour parse silently failed and painted a default.
        let background = card.swatches[0];
        assert!(
            background.r > 0.8 && background.g > 0.8 && background.b > 0.8,
            "expected a light background, got {background:?}"
        );
    }

    #[test]
    fn search_matches_names_and_tags() {
        assert!(filtered_style_guide_cards("terminal")
            .iter()
            .any(|c| c.name == "developer-terminal-dark"));
        // Tag-only match: "brutalist" is a tag, not a substring of the name.
        assert!(!filtered_style_guide_cards("brutalist").is_empty());
        assert_eq!(
            filtered_style_guide_cards("").len(),
            style_guide_registry().len(),
            "an empty query filters nothing"
        );
        assert!(filtered_style_guide_cards("zzz-no-such-style").is_empty());
    }

    #[test]
    fn pinning_matches_by_exact_name() {
        let card = style_guide_cards()
            .into_iter()
            .find(|c| c.name == "zen-paper-light")
            .expect("the corpus ships this guide");
        assert!(card.is_pinned(Some("zen-paper-light")));
        assert!(!card.is_pinned(Some("zen-paper")));
        assert!(!card.is_pinned(None));
    }
}
