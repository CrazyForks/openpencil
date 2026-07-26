//! Chrome-string translation layer.
//!
//! 15 canonical, hand-maintained locale tables. Each per-locale module
//! exposes a single `lookup(key) -> Option<&'static str>`; this module
//! dispatches to the right one given a [`Locale`] variant. Integrity tests
//! keep every direct locale key set and placeholder set aligned with English.
//! Unknown keys fall through to the key itself for debug visibility.
//!
//! Key naming follows the TS app's dot.case convention
//! (`common.untitled`, `rightPanel.design`, `layout.flexLayout`,
//! …) so cross-walking strings between TS and Rust is mechanical.

use crate::Locale;

mod de;
mod de_git;
mod de_panel;
mod en;
mod en_git;
mod en_panel;
mod es;
mod es_git;
mod es_panel;
mod fr;
mod fr_git;
mod fr_panel;
mod hi;
mod hi_git;
mod hi_panel;
mod id;
mod id_git;
mod id_panel;
mod ja;
mod ja_git;
mod ja_panel;
mod ko;
mod ko_git;
mod ko_panel;
mod pt;
mod pt_git;
mod pt_panel;
mod ru;
mod ru_git;
mod ru_panel;
mod th;
mod th_git;
mod th_panel;
mod tr;
mod tr_git;
mod tr_panel;
mod vi;
mod vi_git;
mod vi_panel;
mod zh_cn;
mod zh_cn_git;
mod zh_cn_panel;
mod zh_tw;
mod zh_tw_git;
mod zh_tw_panel;

/// Translate `key` for `locale`.
///
/// A missing locale entry falls back to English; a key absent from English
/// too is returned unchanged. `'static` keeps widget builders from cloning a
/// `String` per frame.
pub fn translate(locale: Locale, key: &'static str) -> &'static str {
    let lookup = match locale {
        Locale::EnUs => en::lookup(key),
        Locale::ZhCn => zh_cn::lookup(key),
        Locale::ZhTw => zh_tw::lookup(key),
        Locale::Ja => ja::lookup(key),
        Locale::Ko => ko::lookup(key),
        Locale::Fr => fr::lookup(key),
        Locale::Es => es::lookup(key),
        Locale::De => de::lookup(key),
        Locale::Pt => pt::lookup(key),
        Locale::Ru => ru::lookup(key),
        Locale::Hi => hi::lookup(key),
        Locale::Tr => tr::lookup(key),
        Locale::Th => th::lookup(key),
        Locale::Vi => vi::lookup(key),
        Locale::Id => id::lookup(key),
    };
    lookup.or_else(|| en::lookup(key)).unwrap_or(key)
}

/// CLDR-style cardinal plural category.
///
/// The complete category set keeps the API stable as locales are added even
/// though the current 15 locales do not exercise `Zero` or `Two`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

/// Return the locale-aware cardinal plural category for an integer.
pub fn plural_category(locale: Locale, count: i64) -> PluralCategory {
    let n = count.unsigned_abs();
    match locale {
        Locale::Ru => {
            let mod_10 = n % 10;
            let mod_100 = n % 100;
            if mod_10 == 1 && mod_100 != 11 {
                PluralCategory::One
            } else if (2..=4).contains(&mod_10) && !(12..=14).contains(&mod_100) {
                PluralCategory::Few
            } else if mod_10 == 0 || (5..=9).contains(&mod_10) || (11..=14).contains(&mod_100) {
                PluralCategory::Many
            } else {
                PluralCategory::Other
            }
        }
        // French and Portuguese use the singular category for the integer
        // values zero and one, and use `many` for non-zero exact millions.
        Locale::Fr | Locale::Pt if n != 0 && n.is_multiple_of(1_000_000) => PluralCategory::Many,
        Locale::Fr | Locale::Pt if n <= 1 => PluralCategory::One,
        // Spanish also has the exact-million `many` category.
        Locale::Es if n != 0 && n.is_multiple_of(1_000_000) => PluralCategory::Many,
        // CLDR cardinal rules for integer Hindi values treat both zero and
        // one as the singular category.
        Locale::Hi if n <= 1 => PluralCategory::One,
        // Chinese, Japanese, Korean, Thai, Vietnamese and Indonesian do not
        // vary cardinal nouns by integer count. Turkish likewise has only
        // the `other` cardinal category.
        Locale::ZhCn
        | Locale::ZhTw
        | Locale::Ja
        | Locale::Ko
        | Locale::Tr
        | Locale::Th
        | Locale::Vi
        | Locale::Id => PluralCategory::Other,
        _ if n == 1 => PluralCategory::One,
        _ => PluralCategory::Other,
    }
}

/// Substitute named placeholders in a translated template.
///
/// Both the canonical `{{name}}` form and the legacy `{name}` form used by
/// missing-font strings are supported. Unknown placeholders remain visible.
pub fn interpolate(template: &str, variables: &[(&str, &str)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(open) = remaining.find('{') {
        output.push_str(&remaining[..open]);
        let token = &remaining[open..];
        let (name_start, closing) = if token.starts_with("{{") {
            (2, "}}")
        } else {
            (1, "}")
        };
        let Some(relative_end) = token[name_start..].find(closing) else {
            // Do not reinterpret the second `{` of an unterminated
            // canonical token as the start of a legacy token.
            output.push_str(token);
            return output;
        };
        let name_end = name_start + relative_end;
        let name = &token[name_start..name_end];
        let token_end = name_end + closing.len();
        let replacement = variables
            .iter()
            .find_map(|(candidate, value)| (*candidate == name).then_some(*value));
        if let Some(value) = replacement {
            output.push_str(value);
            remaining = &token[token_end..];
        } else {
            output.push_str(&token[..token_end]);
            remaining = &token[token_end..];
        }
    }
    output.push_str(remaining);
    output
}

/// Translate a key and substitute named placeholders in one call.
pub fn translate_with(locale: Locale, key: &'static str, variables: &[(&str, &str)]) -> String {
    interpolate(translate(locale, key), variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zh_cn_returns_chinese_chrome_strings() {
        assert_eq!(translate(Locale::ZhCn, "common.untitled"), "未命名");
    }

    #[test]
    fn en_us_returns_english_chrome_strings() {
        assert_eq!(translate(Locale::EnUs, "common.untitled"), "Untitled");
    }

    #[test]
    fn every_locale_has_a_direct_common_translation() {
        for locale in Locale::ALL {
            assert_ne!(translate(locale, "common.cancel"), "common.cancel");
        }
    }

    #[test]
    fn unknown_key_falls_through_to_key() {
        for locale in Locale::ALL {
            assert_eq!(
                translate(locale, "this.key.does.not.exist"),
                "this.key.does.not.exist"
            );
        }
    }

    #[test]
    fn interpolation_supports_canonical_and_legacy_placeholders() {
        assert_eq!(
            interpolate(
                "{{count}} items; {actual}, {missing}",
                &[("count", "3"), ("actual", "Inter")]
            ),
            "3 items; Inter, {missing}"
        );
        assert_eq!(
            translate_with(Locale::Fr, "common.selected", &[("count", "2")]),
            "2 sélectionné(s)"
        );
        assert_eq!(
            interpolate(
                "{{value}} and {other}",
                &[("value", "{other}"), ("other", "safe")]
            ),
            "{other} and safe",
            "replacement values must not be recursively interpolated"
        );
        assert_eq!(
            interpolate("before {{name} after", &[("name", "Alice")]),
            "before {{name} after",
            "unterminated canonical placeholders must remain verbatim"
        );
        assert_eq!(
            interpolate("before {name after", &[("name", "Alice")]),
            "before {name after",
            "unterminated legacy placeholders must remain verbatim"
        );
    }

    #[test]
    fn plural_categories_cover_english_french_and_russian() {
        assert_eq!(plural_category(Locale::EnUs, 1), PluralCategory::One);
        assert_eq!(plural_category(Locale::EnUs, 0), PluralCategory::Other);
        assert_eq!(plural_category(Locale::EnUs, 2), PluralCategory::Other);

        assert_eq!(plural_category(Locale::Fr, 0), PluralCategory::One);
        assert_eq!(plural_category(Locale::Fr, 1), PluralCategory::One);
        assert_eq!(plural_category(Locale::Fr, 2), PluralCategory::Other);
        for locale in [Locale::Fr, Locale::Pt, Locale::Es] {
            assert_eq!(plural_category(locale, 1_000_000), PluralCategory::Many);
            assert_eq!(plural_category(locale, 2_000_000), PluralCategory::Many);
        }
        assert_eq!(plural_category(Locale::Tr, 1), PluralCategory::Other);

        for count in [1, 21, 101, -1] {
            assert_eq!(plural_category(Locale::Ru, count), PluralCategory::One);
        }
        for count in [2, 3, 4, 22, 23, 24] {
            assert_eq!(plural_category(Locale::Ru, count), PluralCategory::Few);
        }
        for count in [0, 5, 10, 11, 12, 14, 20, 25] {
            assert_eq!(plural_category(Locale::Ru, count), PluralCategory::Many);
        }
    }

    #[test]
    fn russian_git_other_forms_are_safe_for_few_and_one_ending_counts() {
        for count in ["2", "21", "22"] {
            assert_eq!(
                translate_with(
                    Locale::Ru,
                    "git.history.diff.framesChanged_other",
                    &[("count", count)]
                ),
                format!("Фреймов изменено: {count}")
            );
        }
    }
}

#[cfg(test)]
mod catalog_integrity_tests {
    use std::collections::BTreeSet;

    type Lookup = fn(&str) -> Option<&'static str>;

    fn tables() -> [(&'static str, &'static str, &'static str, Lookup); 15] {
        [
            (
                "en",
                include_str!("en.rs"),
                concat!(include_str!("en_git.rs"), include_str!("en_panel.rs")),
                super::en::lookup,
            ),
            (
                "zh_cn",
                include_str!("zh_cn.rs"),
                concat!(include_str!("zh_cn_git.rs"), include_str!("zh_cn_panel.rs")),
                super::zh_cn::lookup,
            ),
            (
                "zh_tw",
                include_str!("zh_tw.rs"),
                concat!(include_str!("zh_tw_git.rs"), include_str!("zh_tw_panel.rs")),
                super::zh_tw::lookup,
            ),
            (
                "ja",
                include_str!("ja.rs"),
                concat!(include_str!("ja_git.rs"), include_str!("ja_panel.rs")),
                super::ja::lookup,
            ),
            (
                "ko",
                include_str!("ko.rs"),
                concat!(include_str!("ko_git.rs"), include_str!("ko_panel.rs")),
                super::ko::lookup,
            ),
            (
                "fr",
                include_str!("fr.rs"),
                concat!(include_str!("fr_git.rs"), include_str!("fr_panel.rs")),
                super::fr::lookup,
            ),
            (
                "es",
                include_str!("es.rs"),
                concat!(include_str!("es_git.rs"), include_str!("es_panel.rs")),
                super::es::lookup,
            ),
            (
                "de",
                include_str!("de.rs"),
                concat!(include_str!("de_git.rs"), include_str!("de_panel.rs")),
                super::de::lookup,
            ),
            (
                "pt",
                include_str!("pt.rs"),
                concat!(include_str!("pt_git.rs"), include_str!("pt_panel.rs")),
                super::pt::lookup,
            ),
            (
                "ru",
                include_str!("ru.rs"),
                concat!(include_str!("ru_git.rs"), include_str!("ru_panel.rs")),
                super::ru::lookup,
            ),
            (
                "hi",
                include_str!("hi.rs"),
                concat!(include_str!("hi_git.rs"), include_str!("hi_panel.rs")),
                super::hi::lookup,
            ),
            (
                "tr",
                include_str!("tr.rs"),
                concat!(include_str!("tr_git.rs"), include_str!("tr_panel.rs")),
                super::tr::lookup,
            ),
            (
                "th",
                include_str!("th.rs"),
                concat!(include_str!("th_git.rs"), include_str!("th_panel.rs")),
                super::th::lookup,
            ),
            (
                "vi",
                include_str!("vi.rs"),
                concat!(include_str!("vi_git.rs"), include_str!("vi_panel.rs")),
                super::vi::lookup,
            ),
            (
                "id",
                include_str!("id.rs"),
                concat!(include_str!("id_git.rs"), include_str!("id_panel.rs")),
                super::id::lookup,
            ),
        ]
    }

    fn source_keys<'a>(name: &str, source: &'a str) -> Vec<&'a str> {
        source
            .lines()
            .enumerate()
            .filter_map(|(index, line)| {
                let line = line.trim_start();
                let (pattern, value) = line.split_once("=>")?;
                let pattern = pattern.trim();
                if pattern == "_" {
                    return None;
                }
                let rest = pattern.strip_prefix('"').unwrap_or_else(|| {
                    panic!(
                        "locale table `{name}` has a non-string match pattern on line {}: `{}`",
                        index + 1,
                        line.trim()
                    )
                });
                let (key, suffix) = rest.split_once('"').unwrap_or_else(|| {
                    panic!(
                        "locale table `{name}` has an unterminated key on line {}: `{}`",
                        index + 1,
                        line.trim()
                    )
                });
                assert!(
                    suffix.trim().is_empty(),
                    "locale table `{name}` has an unsupported match pattern on line {}: `{}`; \
                     use exactly one quoted key per match arm",
                    index + 1,
                    line.trim()
                );
                assert!(
                    !value.trim().is_empty(),
                    "locale table `{name}` has an empty value on line {}",
                    index + 1
                );
                Some(key)
            })
            .collect()
    }

    fn table_keys(name: &str, main: &str, git: &str) -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        for key in source_keys(name, main)
            .into_iter()
            .chain(source_keys(name, git))
        {
            assert!(
                keys.insert(key.to_string()),
                "locale table `{name}` contains duplicate key `{key}`"
            );
        }
        keys
    }

    fn placeholders(value: &str) -> BTreeSet<String> {
        let bytes = value.as_bytes();
        let mut result = BTreeSet::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] != b'{' {
                index += 1;
                continue;
            }
            let doubled = bytes.get(index + 1) == Some(&b'{');
            let start = index + if doubled { 2 } else { 1 };
            let Some(relative_end) = bytes[start..].iter().position(|byte| *byte == b'}') else {
                break;
            };
            let end = start + relative_end;
            if doubled && bytes.get(end + 1) != Some(&b'}') {
                index += 2;
                continue;
            }
            let candidate = &value[start..end];
            if !candidate.is_empty()
                && candidate
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
            {
                result.insert(candidate.to_string());
            }
            index = end + if doubled { 2 } else { 1 };
        }
        result
    }

    #[test]
    fn every_locale_has_exactly_the_english_key_set() {
        let all_tables = tables();
        let expected = table_keys(all_tables[0].0, all_tables[0].1, all_tables[0].2);
        assert_eq!(expected.len(), 1105, "update the intentional catalog size");

        for (name, main, git, lookup) in all_tables {
            let actual = table_keys(name, main, git);
            let missing: Vec<_> = expected.difference(&actual).cloned().collect();
            let extra: Vec<_> = actual.difference(&expected).cloned().collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "locale `{name}` differs from English; missing={missing:?}, extra={extra:?}"
            );
            for key in &expected {
                let english = super::en::lookup(key).expect("English key came from its own source");
                assert!(
                    lookup(key).is_some_and(|value| english.is_empty() || !value.is_empty()),
                    "locale `{name}` has no direct value for `{key}`"
                );
            }
        }
    }

    #[test]
    fn every_translation_preserves_the_english_placeholder_set() {
        let all_tables = tables();
        let expected_keys = table_keys(all_tables[0].0, all_tables[0].1, all_tables[0].2);
        for key in expected_keys {
            let english = super::en::lookup(&key).expect("English key came from its own source");
            let expected = placeholders(english);
            for (name, _, _, lookup) in all_tables {
                let translated =
                    lookup(&key).unwrap_or_else(|| panic!("locale `{name}` is missing `{key}`"));
                assert_eq!(
                    placeholders(translated),
                    expected,
                    "locale `{name}` changes placeholders for `{key}`"
                );
            }
        }
    }

    #[test]
    fn previously_missing_callsite_keys_are_catalogued() {
        for key in [
            "rightPanel.interact",
            "widget.title",
            "widget.checked",
            "fill.mesh",
            "fill.shader",
        ] {
            for (name, _, _, lookup) in tables() {
                assert!(
                    lookup(key).is_some(),
                    "locale `{name}` is missing call-site key `{key}`"
                );
            }
        }
    }
}

#[cfg(test)]
mod preview_device_key_tests {
    /// Every locale table must carry a DIRECT entry for the preview
    /// device-switcher keys — `translate`'s EN fallback must not mask
    /// a missing translation.
    #[test]
    fn preview_device_keys_exist_in_every_locale_table() {
        const KEYS: [&str; 3] = [
            "preview.device.phone",
            "preview.device.desktop",
            "preview.device.canvas",
        ];
        type Lookup = fn(&str) -> Option<&'static str>;
        let tables: [(&str, Lookup); 15] = [
            ("en", super::en::lookup),
            ("zh_cn", super::zh_cn::lookup),
            ("zh_tw", super::zh_tw::lookup),
            ("ja", super::ja::lookup),
            ("ko", super::ko::lookup),
            ("fr", super::fr::lookup),
            ("es", super::es::lookup),
            ("de", super::de::lookup),
            ("pt", super::pt::lookup),
            ("ru", super::ru::lookup),
            ("hi", super::hi::lookup),
            ("tr", super::tr::lookup),
            ("th", super::th::lookup),
            ("vi", super::vi::lookup),
            ("id", super::id::lookup),
        ];
        for (name, lookup) in tables {
            for key in KEYS {
                assert!(
                    lookup(key).is_some(),
                    "locale table `{name}` is missing `{key}`"
                );
            }
        }
    }
}

#[cfg(test)]
mod missing_fonts_key_tests {
    #[test]
    fn missing_fonts_keys_exist_in_every_locale_table() {
        const KEYS: [&str; 11] = [
            "settings.tab.fonts",
            "missingFonts.title",
            "missingFonts.subtitle",
            "missingFonts.usage",
            "missingFonts.chooseFile",
            "missingFonts.chooseFont",
            "missingFonts.resolved",
            "missingFonts.dismiss",
            "missingFonts.mismatch",
            "missingFonts.importedSection",
            "missingFonts.noneMissing",
        ];
        type Lookup = fn(&str) -> Option<&'static str>;
        let tables: [(&str, Lookup); 15] = [
            ("en", super::en::lookup),
            ("zh_cn", super::zh_cn::lookup),
            ("zh_tw", super::zh_tw::lookup),
            ("ja", super::ja::lookup),
            ("ko", super::ko::lookup),
            ("fr", super::fr::lookup),
            ("es", super::es::lookup),
            ("de", super::de::lookup),
            ("pt", super::pt::lookup),
            ("ru", super::ru::lookup),
            ("hi", super::hi::lookup),
            ("tr", super::tr::lookup),
            ("th", super::th::lookup),
            ("vi", super::vi::lookup),
            ("id", super::id::lookup),
        ];
        for (name, lookup) in tables {
            for key in KEYS {
                assert!(
                    lookup(key).is_some(),
                    "locale table `{name}` is missing `{key}`"
                );
            }
        }
    }
}

#[cfg(test)]
mod vector_fidelity_property_keys {
    #[test]
    fn task9_keys_exist_in_every_locale_table() {
        const KEYS: [&str; 7] = [
            "property.cornerPerCorner",
            "property.mixed",
            "fill.ruleNonzero",
            "fill.ruleEvenodd",
            "effects.addShadow",
            "effects.addLayerBlur",
            "effects.addBackgroundBlur",
        ];
        type Lookup = fn(&str) -> Option<&'static str>;
        let tables: [(&str, Lookup); 15] = [
            ("en", super::en::lookup),
            ("zh_cn", super::zh_cn::lookup),
            ("zh_tw", super::zh_tw::lookup),
            ("ja", super::ja::lookup),
            ("ko", super::ko::lookup),
            ("fr", super::fr::lookup),
            ("es", super::es::lookup),
            ("de", super::de::lookup),
            ("pt", super::pt::lookup),
            ("ru", super::ru::lookup),
            ("hi", super::hi::lookup),
            ("tr", super::tr::lookup),
            ("th", super::th::lookup),
            ("vi", super::vi::lookup),
            ("id", super::id::lookup),
        ];
        for (name, lookup) in tables {
            for key in KEYS {
                assert!(
                    lookup(key).is_some(),
                    "locale table `{name}` is missing `{key}`"
                );
            }
        }
    }
}

#[cfg(test)]
mod html_import_key_tests {
    type Lookup = fn(&str) -> Option<&'static str>;

    fn tables() -> [(&'static str, Lookup); 15] {
        [
            ("en", super::en::lookup),
            ("zh_cn", super::zh_cn::lookup),
            ("zh_tw", super::zh_tw::lookup),
            ("ja", super::ja::lookup),
            ("ko", super::ko::lookup),
            ("fr", super::fr::lookup),
            ("es", super::es::lookup),
            ("de", super::de::lookup),
            ("pt", super::pt::lookup),
            ("ru", super::ru::lookup),
            ("hi", super::hi::lookup),
            ("tr", super::tr::lookup),
            ("th", super::th::lookup),
            ("vi", super::vi::lookup),
            ("id", super::id::lookup),
        ]
    }

    #[test]
    fn every_locale_advertises_html_and_zip_imports_directly() {
        for (name, lookup) in tables() {
            let drop = lookup("html.dropFile")
                .unwrap_or_else(|| panic!("locale table `{name}` is missing `html.dropFile`"));
            for extension in [".html", ".htm", ".zip"] {
                assert!(
                    drop.contains(extension),
                    "locale table `{name}` omits `{extension}` from `html.dropFile`"
                );
            }

            let tip = lookup("html.saveTip")
                .unwrap_or_else(|| panic!("locale table `{name}` is missing `html.saveTip`"));
            assert!(
                tip.contains(".zip"),
                "locale table `{name}` omits `.zip` from `html.saveTip`"
            );

            let overlay = lookup("dialog.dropToOpen")
                .unwrap_or_else(|| panic!("locale table `{name}` is missing `dialog.dropToOpen`"));
            for extension in [".html", ".zip"] {
                assert!(
                    overlay.contains(extension),
                    "locale table `{name}` omits `{extension}` from `dialog.dropToOpen`"
                );
            }
        }
    }

    #[test]
    fn primary_locale_copy_describes_page_and_project_imports_naturally() {
        assert_eq!(
            super::en::lookup("html.title"),
            Some("Import HTML or web project")
        );
        assert_eq!(
            super::zh_cn::lookup("html.title"),
            Some("导入 HTML 或网页项目")
        );
        assert_eq!(
            super::zh_tw::lookup("html.title"),
            Some("匯入 HTML 或網頁專案")
        );
    }
}

#[cfg(test)]
mod figma_property_panel_key_tests {
    type Lookup = fn(&str) -> Option<&'static str>;

    const KEYS: [&str; 29] = [
        "page.background",
        "page.background.clear",
        "property.swapComponent",
        "appearance.blendMode",
        "appearance.maskType",
        "layer.blendMode",
        "layer.maskType",
        "fill.blendMode",
        "image.tileScale",
        "blendMode.normal",
        "blendMode.darken",
        "blendMode.multiply",
        "blendMode.screen",
        "blendMode.overlay",
        "blendMode.lighten",
        "blendMode.difference",
        "blendMode.hue",
        "blendMode.saturation",
        "blendMode.color",
        "blendMode.luminosity",
        "blendMode.softLight",
        "blendMode.colorDodge",
        "blendMode.colorBurn",
        "blendMode.hardLight",
        "blendMode.exclusion",
        "maskType.none",
        "maskType.alpha",
        "maskType.vector",
        "maskType.luminance",
    ];

    #[test]
    fn figma_property_keys_exist_directly_in_every_locale_table() {
        let tables: [(&str, Lookup); 15] = [
            ("en", super::en::lookup),
            ("zh_cn", super::zh_cn::lookup),
            ("zh_tw", super::zh_tw::lookup),
            ("ja", super::ja::lookup),
            ("ko", super::ko::lookup),
            ("fr", super::fr::lookup),
            ("es", super::es::lookup),
            ("de", super::de::lookup),
            ("pt", super::pt::lookup),
            ("ru", super::ru::lookup),
            ("hi", super::hi::lookup),
            ("tr", super::tr::lookup),
            ("th", super::th::lookup),
            ("vi", super::vi::lookup),
            ("id", super::id::lookup),
        ];

        for (name, lookup) in tables {
            for key in KEYS {
                let value = lookup(key)
                    .unwrap_or_else(|| panic!("locale table `{name}` is missing `{key}`"));
                assert!(
                    !value.is_empty(),
                    "locale table `{name}` has an empty value for `{key}`"
                );
            }
        }
    }
}
