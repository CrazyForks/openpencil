//! The request text the Scene Template Center's generate row sends.
//!
//! The row takes a topic ("Q3 复盘"), not a prompt, so something has to say
//! what kind of document that topic becomes. That wrapping is not cosmetic:
//! the generation pipeline reads the design type straight off the prompt
//! (`op_orchestrator::detect_design_type`), and a bare topic classifies as a
//! landing page — the user would type a talk title into a slides entry point
//! and get a 1200-wide scrolling page back.
//!
//! So the template is a contract with the classifier, and the locale tables
//! that carry it keep an ASCII "PPT" token in every language for the same
//! reason: it is the one trigger word that survives translation. The
//! cross-locale guarantee is asserted in `op-orchestrator`, where the
//! classifier itself lives, so a translation that drops the token fails
//! there rather than shipping as a silently mis-sized deck.

use crate::Locale;

/// The i18n key carrying the wrapper, with a `{{topic}}` placeholder.
pub const SLIDES_PROMPT_KEY: &str = "sceneTemplate.generate.promptTemplate";

/// Used when the key is missing from a locale table (runtime lookup already
/// falls back through English, so this only fires for a catalogue gap).
const SLIDES_PROMPT_FALLBACK: &str = "为以下主题制作一份演示文稿（PPT）：{{topic}}";

/// Wrap a raw topic into a request the pipeline reads as a deck.
///
/// Whitespace-only input returns `None`: there is no topic to ask about, and
/// a wrapper around nothing would still classify as slides and generate a
/// deck about the empty string.
pub fn slides_generate_prompt(locale: Locale, topic: &str) -> Option<String> {
    let topic = topic.trim();
    if topic.is_empty() {
        return None;
    }
    let translated = op_i18n::translate_with(locale, SLIDES_PROMPT_KEY, &[("topic", topic)]);
    if translated == SLIDES_PROMPT_KEY {
        return Some(SLIDES_PROMPT_FALLBACK.replace("{{topic}}", topic));
    }
    Some(translated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_wraps_the_topic_verbatim() {
        for locale in Locale::ALL {
            let prompt = slides_generate_prompt(locale, "  Q3 复盘  ").expect("a topic wraps");
            assert!(
                prompt.contains("Q3 复盘"),
                "{locale:?} dropped the topic: {prompt}"
            );
            assert!(
                !prompt.contains("{{topic}}"),
                "{locale:?} left the placeholder unsubstituted: {prompt}"
            );
        }
    }

    #[test]
    fn a_blank_topic_is_not_a_request() {
        assert_eq!(slides_generate_prompt(Locale::ZhCn, "   \t "), None);
        assert_eq!(slides_generate_prompt(Locale::EnUs, ""), None);
    }
}
