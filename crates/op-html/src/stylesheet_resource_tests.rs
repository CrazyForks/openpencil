use super::*;

fn imported_text_color(result: &HtmlImportResult) -> &str {
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!("the import root should be a frame")
    };
    let PenNode::Frame(container) = &root.children.as_ref().expect("root children")[0] else {
        panic!("the styled element should be a frame")
    };
    let PenNode::Text(text) = &container.children.as_ref().expect("element children")[0] else {
        panic!("the styled element should contain text")
    };
    let Some([PenFill::Solid(fill)]) = text.fill.as_deref() else {
        panic!("the text should have a solid fill")
    };
    fill.color.as_str()
}

#[test]
fn external_css_image_uses_the_stylesheet_url_as_its_base() {
    let html = r#"<link rel="stylesheet" href="/assets/css/site.css">
        <div class="hero"></div>"#;
    let requests = std::cell::RefCell::new(Vec::new());
    let fetcher = |url: &str| -> Option<Vec<u8>> {
        requests.borrow_mut().push(url.to_string());
        match url {
            "https://example.test/assets/css/site.css" => Some(
                br#"@media (min-width: 1px) {
                    .hero { width: 20px; height: 20px;
                        background-image: url('../images/hero icon.png'); }
                }"#
                .to_vec(),
            ),
            "https://example.test/assets/images/hero%20icon.png" => {
                Some(vec![0x89, 0x50, 0x4e, 0x47, 1, 2, 3])
            }
            _ => None,
        }
    };
    let options = HtmlImportOptions {
        base_url: Some("https://example.test/documents/page.html".to_string()),
        ..Default::default()
    };

    let result = import_html_with_resources(html, &options, Some(&fetcher), None);

    assert_eq!(
        requests.into_inner(),
        [
            "https://example.test/assets/css/site.css",
            "https://example.test/assets/images/hero%20icon.png",
        ]
    );
    let PenNode::Frame(root) = &result.nodes[0] else {
        panic!("the import root should be a frame")
    };
    let PenNode::Frame(hero) = &root.children.as_ref().expect("root children")[0] else {
        panic!("the styled div should be a frame")
    };
    assert!(matches!(
        hero.container.fill.as_deref(),
        Some([PenFill::Image(image)]) if image.url.as_str().starts_with("data:image/png;base64,")
    ));
}

#[test]
fn first_valid_base_href_controls_stylesheets_and_images() {
    let html = r#"<head>
        <base href="javascript:alert(1)"><base href="../assets/"><base href="/ignored/">
        <link rel="stylesheet" href="theme.css"></head>
        <body><p class="hot">x</p><img src="photo.png"></body>"#;
    let requests = std::cell::RefCell::new(Vec::new());
    let fetcher = |url: &str| -> Option<Vec<u8>> {
        requests.borrow_mut().push(url.to_string());
        match url {
            "https://example.test/assets/theme.css" => Some(b".hot { color: #ff0000 }".to_vec()),
            "https://example.test/assets/photo.png" => Some(b"\x89PNG".to_vec()),
            _ => None,
        }
    };
    let result = import_html_with_resources(
        html,
        &HtmlImportOptions {
            base_url: Some("https://example.test/pages/index.html".into()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );

    assert_eq!(imported_text_color(&result), "#ff0000");
    assert_eq!(
        requests.into_inner(),
        [
            "https://example.test/assets/theme.css",
            "https://example.test/assets/photo.png",
        ]
    );
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("invalid <base href>")));
}

#[test]
fn base_href_cannot_move_a_virtual_project_to_an_external_origin() {
    let html = r#"<head><base href="https://evil.test/"><base href="assets/">
        <link rel="stylesheet" href="theme.css"></head>
        <body><p class="hot">x</p><img src="https://evil.test/photo.png"></body>"#;
    let requests = std::cell::RefCell::new(Vec::new());
    let fetcher = |url: &str| -> Option<Vec<u8>> {
        requests.borrow_mut().push(url.to_string());
        (url == "https://openpencil.local/site/assets/theme.css")
            .then(|| b".hot { color: #00ff00 }".to_vec())
    };
    let result = import_html_with_resources(
        html,
        &HtmlImportOptions {
            base_url: Some("https://openpencil.local/site/index.html".into()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );

    assert_eq!(imported_text_color(&result), "#00ff00");
    assert_eq!(
        requests.into_inner(),
        ["https://openpencil.local/site/assets/theme.css"]
    );
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("outside the HTML project origin")));
}

#[test]
fn recursive_imports_follow_css_cascade_order() {
    let html = r#"<link rel="stylesheet" href="css/main.css"><p class="hot">x</p>"#;
    let fetcher = |url: &str| -> Option<Vec<u8>> {
        match url {
            "https://example.test/css/main.css" => {
                Some(b"@import 'nested/a.css'; .hot { color: #0000ff }".to_vec())
            }
            "https://example.test/css/nested/a.css" => {
                Some(b"@import '../b.css'; .hot { color: #ff0000 }".to_vec())
            }
            "https://example.test/css/b.css" => Some(b".hot { color: #00ff00 }".to_vec()),
            _ => None,
        }
    };
    let result = import_html_with_resources(
        html,
        &HtmlImportOptions {
            base_url: Some("https://example.test/index.html".into()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );

    assert_eq!(imported_text_color(&result), "#0000ff");
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
}

#[test]
fn linked_utf16_stylesheet_participates_in_the_cascade() {
    let html = r#"<link rel="stylesheet" href="theme.css"><p class="hot">x</p>"#;
    let fetcher = |url: &str| -> Option<Vec<u8>> {
        if url != "https://example.test/theme.css" {
            return None;
        }
        let mut bytes = b"\xFF\xFE".to_vec();
        bytes.extend(
            ".hot { color: #123456 }"
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        );
        Some(bytes)
    };
    let result = import_html_with_resources(
        html,
        &HtmlImportOptions {
            base_url: Some("https://example.test/index.html".into()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );

    assert_eq!(imported_text_color(&result), "#123456");
}

#[test]
fn recursive_import_honors_a_case_insensitive_gbk_charset() {
    let html = r#"<link rel="stylesheet" href="main.css"><p class="热">x</p>"#;
    let (nested, _, had_errors) =
        encoding_rs::GBK.encode("@ChArSeT \"gbk\";.热 { color: #234567 }");
    assert!(!had_errors);
    let nested = nested.into_owned();
    let fetcher = |url: &str| -> Option<Vec<u8>> {
        match url {
            "https://example.test/main.css" => Some(b"@import 'nested.css';".to_vec()),
            "https://example.test/nested.css" => Some(nested.clone()),
            _ => None,
        }
    };
    let result = import_html_with_resources(
        html,
        &HtmlImportOptions {
            base_url: Some("https://example.test/index.html".into()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );

    assert_eq!(imported_text_color(&result), "#234567");
}

#[test]
fn undeclared_legacy_stylesheet_uses_windows_1252_fallback() {
    let html = r#"<link rel="stylesheet" href="legacy.css"><p class="café">x</p>"#;
    let fetcher = |url: &str| {
        (url == "https://example.test/legacy.css").then(|| b".caf\xE9 { color: #345678 }".to_vec())
    };
    let result = import_html_with_resources(
        html,
        &HtmlImportOptions {
            base_url: Some("https://example.test/index.html".into()),
            ..Default::default()
        },
        Some(&fetcher),
        None,
    );

    assert_eq!(imported_text_color(&result), "#345678");
}

#[test]
fn repeated_import_probes_expose_nested_missing_resources() {
    use std::collections::HashMap;

    let html = r#"<link rel="stylesheet" href="css/main.css"><p class="hot">x</p>"#;
    let loaded = std::cell::RefCell::new(HashMap::<String, Vec<u8>>::new());
    let missing = std::cell::RefCell::new(Vec::new());
    let fetcher = |url: &str| {
        loaded.borrow().get(url).cloned().or_else(|| {
            missing.borrow_mut().push(url.to_string());
            None
        })
    };
    let options = HtmlImportOptions {
        base_url: Some("https://openpencil.local/index.html".into()),
        ..Default::default()
    };

    import_html_with_resources(html, &options, Some(&fetcher), None);
    assert_eq!(
        missing.replace(Vec::new()),
        ["https://openpencil.local/css/main.css"]
    );
    loaded.borrow_mut().insert(
        "https://openpencil.local/css/main.css".into(),
        b"@import 'nested.css'; .hot { color: blue }".to_vec(),
    );

    import_html_with_resources(html, &options, Some(&fetcher), None);
    assert_eq!(
        missing.replace(Vec::new()),
        ["https://openpencil.local/css/nested.css"]
    );
    loaded.borrow_mut().insert(
        "https://openpencil.local/css/nested.css".into(),
        b".hot { color: red }".to_vec(),
    );

    let result = import_html_with_resources(html, &options, Some(&fetcher), None);
    assert!(missing.borrow().is_empty());
    assert_eq!(imported_text_color(&result), "#0000ff");
}
