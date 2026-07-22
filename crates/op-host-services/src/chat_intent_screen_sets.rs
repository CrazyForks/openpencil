//! Strong intent signal for continuation prompts that list sibling screens.
//!
//! Generic words such as `完成` and `界面` are individually ambiguous: they
//! can describe finishing the current screen or one of its sections.  A
//! delimited list before a shared whole-screen noun is much stronger, e.g.
//! `继续完成 explore/profile界面`.  Keep this parser narrow so ordinary
//! append/modify requests retain their existing behavior.

use super::EXISTING_SCREEN_CTX_CJK;

const LIST_VERBS_CJK: &[&str] = &["完成", "补齐", "补完", "生成", "设计", "画", "绘", "做"];
const SCREEN_NOUNS_CJK: &[&str] = &["界面", "页面", "屏幕", "页", "屏"];
const LIST_VERBS_EN: &[&str] = &[
    "generate",
    "generating",
    "design",
    "designing",
    "create",
    "creating",
    "build",
    "building",
    "draw",
    "drawing",
    "complete",
    "completing",
    "finish",
    "finishing",
];
const SCREEN_NOUNS_EN: &[&str] = &[
    "interface",
    "interfaces",
    "page",
    "pages",
    "screen",
    "screens",
];
const NEGATION_EN: &[&str] = &["not", "never", "don't", "dont", "without", "stop", "avoid"];
const EXISTING_SCREEN_CTX_EN: &[&str] = &[
    "current", "existing", "same", "this", "that", "these", "those",
];

fn has_name_chars(part: &str) -> bool {
    let letters: Vec<char> = part.chars().filter(|c| c.is_alphabetic()).collect();
    letters.iter().any(|c| !c.is_ascii()) || letters.len() >= 3
}

fn is_named_list_with(targets: &str, separator: char) -> bool {
    let parts: Vec<&str> = targets.split(separator).map(str::trim).collect();
    parts.len() >= 2 && parts.iter().all(|part| has_name_chars(part))
}

fn has_named_list(targets: &str) -> bool {
    is_named_list_with(targets, '／')
        || is_named_list_with(targets, '、')
        || (!targets.contains("://") && !targets.contains("//") && is_named_list_with(targets, '/'))
}

fn is_clause_end(tail: &str) -> bool {
    tail.trim_start().chars().next().is_none_or(|c| {
        matches!(
            c,
            ',' | '，' | '.' | '。' | ';' | '；' | '!' | '！' | '?' | '？' | '\n'
        )
    })
}

fn crosses_clause_boundary(targets: &str) -> bool {
    targets.chars().any(|c| {
        matches!(
            c,
            ',' | '，' | '.' | '。' | ';' | '；' | '!' | '！' | '?' | '？' | '\n'
        )
    })
}

fn completion_is_negated(prompt: &str, verb_pos: usize, verb: &str, after_verb: &str) -> bool {
    let prefix = prompt[..verb_pos].trim_end();
    let recent_start = prefix
        .char_indices()
        .rev()
        .nth(7)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let recent_prefix = &prefix[recent_start..];
    let explicit_negation = ["不要", "无需", "不用", "不必", "禁止", "别"]
        .iter()
        .any(|marker| recent_prefix.contains(marker));
    explicit_negation
        || (verb == "完成"
            && (prefix
                .chars()
                .next_back()
                .is_some_and(|c| matches!(c, '未' | '没' | '已'))
                || after_verb.starts_with('度')))
}

fn targets_reference_existing_screen(targets: &str) -> bool {
    EXISTING_SCREEN_CTX_CJK
        .iter()
        .any(|marker| targets.contains(marker))
}

fn is_ascii_word_match(text: &str, start: usize, word: &str) -> bool {
    let end = start + word.len();
    let is_word_char = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|c| !is_word_char(c))
        && text[end..].chars().next().is_none_or(|c| !is_word_char(c))
}

fn contains_ascii_word(text: &str, word: &str) -> bool {
    text.match_indices(word)
        .any(|(start, _)| is_ascii_word_match(text, start, word))
}

fn english_completion_is_negated(lower: &str, verb_pos: usize) -> bool {
    let prefix = &lower[..verb_pos];
    let clause = prefix
        .rsplit([',', '.', ';', '!', '?', '\n'])
        .next()
        .unwrap_or(prefix);
    NEGATION_EN
        .iter()
        .any(|marker| contains_ascii_word(clause, marker))
}

fn targets_reference_existing_screen_en(targets: &str) -> bool {
    EXISTING_SCREEN_CTX_EN
        .iter()
        .any(|marker| contains_ascii_word(targets, marker))
}

fn requests_listed_whole_screens_en(prompt: &str) -> bool {
    let lower = prompt.to_ascii_lowercase();
    for &verb in LIST_VERBS_EN {
        for (verb_pos, _) in lower.match_indices(verb) {
            if !is_ascii_word_match(&lower, verb_pos, verb)
                || english_completion_is_negated(&lower, verb_pos)
            {
                continue;
            }
            let after_verb = &lower[verb_pos + verb.len()..];
            for &noun in SCREEN_NOUNS_EN {
                for (noun_pos, _) in after_verb.match_indices(noun) {
                    if !is_ascii_word_match(after_verb, noun_pos, noun) {
                        continue;
                    }
                    let targets = after_verb[..noun_pos].trim();
                    let tail = &after_verb[noun_pos + noun.len()..];
                    if !crosses_clause_boundary(targets)
                        && !targets_reference_existing_screen_en(targets)
                        && has_named_list(targets)
                        && is_clause_end(tail)
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub(super) fn requests_listed_whole_screens(prompt: &str) -> bool {
    for &verb in LIST_VERBS_CJK {
        for (verb_pos, _) in prompt.match_indices(verb) {
            let after_verb = &prompt[verb_pos + verb.len()..];
            if completion_is_negated(prompt, verb_pos, verb, after_verb) {
                continue;
            }
            // Try every noun occurrence rather than the earliest one: screen
            // names themselves can contain 页/屏 ("首页/profile页面").
            for &noun in SCREEN_NOUNS_CJK {
                for (noun_pos, _) in after_verb.match_indices(noun) {
                    let targets = after_verb[..noun_pos].trim();
                    let tail = &after_verb[noun_pos + noun.len()..];
                    if !crosses_clause_boundary(targets)
                        && !targets_reference_existing_screen(targets)
                        && has_named_list(targets)
                        && is_clause_end(tail)
                    {
                        return true;
                    }
                }
            }
        }
    }
    requests_listed_whole_screens_en(prompt)
}

#[cfg(test)]
mod tests {
    use super::requests_listed_whole_screens;

    #[test]
    fn recognizes_listed_follow_on_screens_with_shared_cjk_noun() {
        for prompt in [
            "继续完成 explore/profile界面",
            "继续完成 Explore ／ Profile 界面",
            "接着补齐 Search、Orders 两个页面",
            "继续生成 cart/checkout屏幕，保持首页的视觉风格",
            "基于当前首页继续完成 explore/profile界面",
            "继续完成 explore/profile界面，沿用当前设计",
            "继续完成 首页/profile页面",
            "继续完成 首屏/profile界面",
        ] {
            assert!(
                requests_listed_whole_screens(prompt),
                "expected listed whole screens: {prompt}"
            );
        }
    }

    #[test]
    fn recognizes_listed_follow_on_screens_with_english_interface_noun() {
        for prompt in [
            "Continue generating the explore/profile interface",
            "continue generating Explore / Profile interfaces",
            "Continue designing search/checkout screens.",
        ] {
            assert!(
                requests_listed_whole_screens(prompt),
                "expected listed English whole screens: {prompt}"
            );
        }
    }

    #[test]
    fn rejects_current_screen_and_subobject_requests() {
        for prompt in [
            "继续完成这个界面",
            "继续完成当前界面的推荐区块",
            "继续完成 profile 界面的 header",
            "继续完成 explore/profile 界面之间的跳转",
            "继续把 explore/profile 两个界面改成深色",
            "继续在首页加入 explore/profile 入口",
            "继续修改未完成的 explore/profile界面",
            "不要完成 explore/profile界面",
            "无需完成 explore/profile界面",
            "请不要再完成 explore/profile界面",
            "继续完成 16/9页面",
            "继续完成 UI/UX界面",
            "Continue modifying the explore/profile interface",
            "Do not continue generating the explore/profile interface",
            "Continue generating the explore/profile interface transitions",
            "Continue generating the UI/UX interface",
            "Don't generate the explore/profile interfaces",
            "Stop generating explore/profile interfaces",
            "Continue generating the current explore/profile interfaces",
            "Continue generating the explore/profile interface header",
            "Continue generating the explore/profile interface's navigation",
            "Continue generating the profile interface",
            "Continue generating 16/9 interfaces",
            "Continue regenerating the explore/profile interface",
        ] {
            assert!(
                !requests_listed_whole_screens(prompt),
                "expected an in-place request: {prompt}"
            );
        }
    }
}
