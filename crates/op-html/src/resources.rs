use base64::Engine as _;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use std::collections::HashMap;
use url::Url;

#[path = "resources_css_imports.rs"]
mod css_imports;
#[path = "resources_css_urls.rs"]
mod css_urls;
pub(crate) use css_imports::expand_stylesheet_imports;

pub type ResourceFetcher<'a> = dyn Fn(&str) -> Option<Vec<u8>> + 'a;
pub type ImageTransform<'a> = dyn Fn(&[u8]) -> Option<Vec<u8>> + 'a;

const PLACEHOLDER_GRAY_PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

#[derive(Default)]
pub(crate) struct ResourceBudget;

impl ResourceBudget {
    pub(crate) fn take(&mut self, _warnings: &mut Vec<String>) -> bool {
        true
    }
}

pub fn resolve_url(base: Option<&str>, href: &str) -> Option<String> {
    let href = href.trim();
    if let Ok(url) = Url::parse(href) {
        return matches!(url.scheme(), "http" | "https" | "data").then(|| url.to_string());
    }
    let base = Url::parse(base?.trim()).ok()?;
    if !matches!(base.scheme(), "http" | "https") {
        return None;
    }
    base.join(href).ok().map(Into::into)
}

/// Resolves a resource URL while keeping virtual HTML projects self-contained.
///
/// Regular web imports may reference any HTTP(S) origin. The synthetic
/// `openpencil.local` origin is different: it represents files supplied by the
/// user, so an absolute `<base>`, stylesheet, import, or image URL must not turn
/// that virtual file lookup into an external request.
pub(crate) fn resolve_resource_url(base: Option<&str>, href: &str) -> Option<String> {
    let resolved = resolve_url(base, href)?;
    resource_url_allowed(base, &resolved).then_some(resolved)
}

pub(crate) fn select_document_base(
    document_url: Option<&str>,
    candidates: &[String],
    warnings: &mut Vec<String>,
) -> Option<String> {
    for href in candidates {
        let Some(resolved) = resolve_url(document_url, href) else {
            warnings.push(format!("invalid <base href> ignored: {href}"));
            continue;
        };
        let Ok(parsed) = Url::parse(&resolved) else {
            continue;
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            warnings.push(format!("invalid <base href> ignored: {href}"));
            continue;
        }
        if !resource_url_allowed(document_url, &resolved) {
            warnings.push(format!(
                "<base href> outside the HTML project origin ignored: {href}"
            ));
            continue;
        }
        return Some(resolved);
    }
    document_url.map(str::to_string)
}

fn resource_url_allowed(base: Option<&str>, resolved: &str) -> bool {
    let Some(base) = base.and_then(|value| Url::parse(value).ok()) else {
        return true;
    };
    if !is_virtual_project_url(&base) {
        return true;
    }
    Url::parse(resolved)
        .ok()
        .is_some_and(|url| same_origin(&base, &url))
}

fn is_virtual_project_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some("openpencil.local")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
        && right.username().is_empty()
        && right.password().is_none()
}

pub(crate) fn embed_images(
    nodes: &mut [PenNode],
    base_url: Option<&str>,
    fetcher: &ResourceFetcher<'_>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<String>,
) -> usize {
    let mut cache = HashMap::new();
    nodes
        .iter_mut()
        .map(|node| {
            embed_node_images(
                node, base_url, fetcher, transform, budget, warnings, &mut cache,
            )
        })
        .sum()
}

fn embed_node_images(
    node: &mut PenNode,
    base_url: Option<&str>,
    fetcher: &ResourceFetcher<'_>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<String>,
    cache: &mut HashMap<String, String>,
) -> usize {
    let mut count = 0;
    let children = match node {
        PenNode::Frame(frame) => {
            count += embed_fills(
                &mut frame.container.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            frame.children.as_mut()
        }
        PenNode::Group(group) => {
            count += embed_fills(
                &mut group.container.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            group.children.as_mut()
        }
        PenNode::Rectangle(rectangle) => {
            count += embed_fills(
                &mut rectangle.container.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            rectangle.children.as_mut()
        }
        PenNode::Ellipse(ellipse) => {
            count += embed_fills(
                &mut ellipse.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Polygon(polygon) => {
            count += embed_fills(
                &mut polygon.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Path(path) => {
            count += embed_fills(
                &mut path.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Text(text) => {
            count += embed_fills(
                &mut text.fill,
                base_url,
                fetcher,
                transform,
                budget,
                warnings,
                cache,
            );
            None
        }
        PenNode::Image(image) => {
            let src = image.src.as_str().to_string();
            if let Some(embedded) =
                embed_url(&src, base_url, fetcher, transform, budget, warnings, cache)
            {
                image.src = embedded.into();
                count += 1;
            }
            None
        }
        PenNode::Ref(reference) => reference.children.as_mut(),
        PenNode::Tabs(tabs) => tabs.children.as_mut(),
        _ => None,
    };
    if let Some(children) = children {
        for child in children {
            count +=
                embed_node_images(child, base_url, fetcher, transform, budget, warnings, cache);
        }
    }
    count
}

fn embed_fills(
    fills: &mut Option<Vec<PenFill>>,
    base_url: Option<&str>,
    fetcher: &ResourceFetcher<'_>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<String>,
    cache: &mut HashMap<String, String>,
) -> usize {
    let mut count = 0;
    if let Some(fills) = fills {
        for fill in fills {
            if let PenFill::Image(image) = fill {
                let url = image.url.as_str().to_string();
                if let Some(embedded) =
                    embed_url(&url, base_url, fetcher, transform, budget, warnings, cache)
                {
                    image.url = embedded.into();
                    count += 1;
                }
            }
        }
    }
    count
}

fn embed_url(
    url: &str,
    base_url: Option<&str>,
    fetcher: &ResourceFetcher<'_>,
    transform: Option<&ImageTransform<'_>>,
    budget: &mut ResourceBudget,
    warnings: &mut Vec<String>,
    cache: &mut HashMap<String, String>,
) -> Option<String> {
    if url.starts_with("data:") {
        return None;
    }
    let resolved = match resolve_resource_url(base_url, url) {
        Some(resolved) => resolved,
        None if base_url.is_some_and(is_virtual_project_base) => {
            warnings.push(format!(
                "image resource outside the HTML project origin, using placeholder: {url}"
            ));
            return Some(PLACEHOLDER_GRAY_PNG.to_string());
        }
        None => url.to_string(),
    };
    if let Some(cached) = cache.get(&resolved) {
        return Some(cached.clone());
    }
    if !budget.take(warnings) {
        return None;
    }
    let embedded = match fetcher(&resolved) {
        Some(bytes) => {
            let transformed = transform.and_then(|rewrite| rewrite(&bytes));
            blob_to_data_url(transformed.as_deref().unwrap_or(&bytes))
        }
        None => {
            warnings.push(format!(
                "image resource unavailable, using placeholder: {resolved}"
            ));
            PLACEHOLDER_GRAY_PNG.to_string()
        }
    };
    cache.insert(resolved, embedded.clone());
    Some(embedded)
}

fn is_virtual_project_base(base: &str) -> bool {
    Url::parse(base)
        .ok()
        .as_ref()
        .is_some_and(is_virtual_project_url)
}

fn blob_to_data_url(bytes: &[u8]) -> String {
    let mime = image_mime(bytes);
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    format!("data:{mime};base64,{encoded}")
}

fn image_mime(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [0xff, 0xd8, ..] => "image/jpeg",
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => "image/gif",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        [b'B', b'M', ..] => "image/bmp",
        [0, 0, 1 | 2, 0, ..] => "image/x-icon",
        [_, _, _, _, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f' | b's', ..] => "image/avif",
        _ if looks_like_svg(bytes) => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start_matches('\u{feff}').trim_start();
    text.starts_with("<svg")
        || text
            .strip_prefix("<?xml")
            .and_then(|rest| rest.find("?>").map(|end| &rest[end + 2..]))
            .is_some_and(|rest| rest.trim_start().starts_with("<svg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_url_forms() {
        assert_eq!(
            resolve_url(Some("https://a.dev/x/y.html"), "s.css").as_deref(),
            Some("https://a.dev/x/s.css")
        );
        assert_eq!(
            resolve_url(Some("https://a.dev/x/y.html"), "/s.css").as_deref(),
            Some("https://a.dev/s.css")
        );
        assert_eq!(
            resolve_url(Some("https://a.dev/x/y.html"), "../s.css").as_deref(),
            Some("https://a.dev/s.css")
        );
        assert_eq!(
            resolve_url(Some("https://a.dev/x/"), "//cdn.b.io/s.css").as_deref(),
            Some("https://cdn.b.io/s.css")
        );
        assert_eq!(
            resolve_url(None, "https://c.io/s.css").as_deref(),
            Some("https://c.io/s.css")
        );
        assert!(resolve_url(None, "s.css").is_none());
    }

    #[test]
    fn virtual_project_resource_resolution_cannot_change_origin() {
        let base = Some("https://openpencil.local/site/pages/index.html");
        assert_eq!(
            resolve_resource_url(base, "../../assets/site.css").as_deref(),
            Some("https://openpencil.local/assets/site.css")
        );
        assert!(resolve_resource_url(base, "https://example.test/site.css").is_none());
        assert!(resolve_resource_url(base, "//example.test/site.css").is_none());
    }

    #[test]
    fn document_base_uses_first_valid_candidate_and_confines_projects() {
        let mut warnings = Vec::new();
        let selected = select_document_base(
            Some("https://openpencil.local/site/index.html"),
            &[
                "javascript:alert(1)".into(),
                "https://example.test/".into(),
                "assets/".into(),
                "/ignored/".into(),
            ],
            &mut warnings,
        );
        assert_eq!(
            selected.as_deref(),
            Some("https://openpencil.local/site/assets/")
        );
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn sniffs_common_image_formats_instead_of_mislabeling_them_as_png() {
        let cases: &[(&[u8], &str)] = &[
            (b"\x89PNG\r\n\x1a\n", "image/png"),
            (b"\xff\xd8\xff", "image/jpeg"),
            (b"GIF89a", "image/gif"),
            (b"RIFF\0\0\0\0WEBP", "image/webp"),
            (b"BM\0\0", "image/bmp"),
            (b"\0\0\x01\0", "image/x-icon"),
            (b"\0\0\0\x18ftypavif", "image/avif"),
            (b"\0\0\0\x18ftypavis", "image/avif"),
            (
                b" \n<svg xmlns='http://www.w3.org/2000/svg'/>",
                "image/svg+xml",
            ),
            (b"not an image", "application/octet-stream"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(image_mime(bytes), *expected, "bytes: {bytes:?}");
            assert!(blob_to_data_url(bytes).starts_with(&format!("data:{expected};base64,")));
        }
    }
}
