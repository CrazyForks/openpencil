//! Browser image-search route for the web daemon.
//!
//! `POST /api/ai/image/search` mirrors the desktop image panel's Search
//! popover backend (`op-host-desktop/src/image_panel_host.rs`: Openverse →
//! two-keyword retry → Wikimedia, thumbnails embedded as `data:` URLs) so
//! the wasm shell can drain its `search_epoch` through the daemon instead
//! of leaving the popover loading forever. Openverse credentials come from
//! the request body (browser-held) or fall back to the daemon's persisted
//! agent settings. Openverse / Wikimedia are product-constant public hosts
//! — the same operator-trust tier as the desktop path — so they dial with
//! a plain client; nothing in this route dials a browser-supplied URL.
//!
//! Unlike the desktop, fetched thumbnails are NOT re-encoded/down-scaled
//! here: `image_downscale` needs skia and this crate must stay GL-free for
//! `op-host-web-server`. The 4 MiB per-image cap still bounds what can be
//! embedded.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;

/// Cap on concurrently running image jobs (search + generate combined).
/// Each job blocks one connection thread for up to minutes of provider
/// network; without a ceiling a page could exhaust the daemon's threads.
const MAX_IN_FLIGHT_IMAGE_JOBS: usize = 4;

static IN_FLIGHT_IMAGE_JOBS: AtomicUsize = AtomicUsize::new(0);

/// RAII slot for one running image job. `acquire` fails once
/// [`MAX_IN_FLIGHT_IMAGE_JOBS`] jobs are running (route answers 429).
pub struct ImageJobSlot(());

impl ImageJobSlot {
    pub fn acquire() -> Option<Self> {
        IN_FLIGHT_IMAGE_JOBS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < MAX_IN_FLIGHT_IMAGE_JOBS).then_some(n + 1)
            })
            .ok()
            .map(|_| Self(()))
    }
}

impl Drop for ImageJobSlot {
    fn drop(&mut self) {
        IN_FLIGHT_IMAGE_JOBS.fetch_sub(1, Ordering::AcqRel);
    }
}

/// TS popover requests `count: 5` (desktop parity).
const SEARCH_RESULT_COUNT: usize = 5;
/// Fetch a wider catalogue window before relevance ranking. The public route
/// still materializes at most [`SEARCH_RESULT_COUNT`] thumbnails.
const SEARCH_CANDIDATE_COUNT: usize = 20;
pub const MAX_EMBEDDED_IMAGE_BYTES: usize = 4 * 1024 * 1024;

/// Design-artifact words that are pure noise against a photo corpus (see
/// the desktop `image_search_session.rs` for the measurement notes).
const IMAGE_SEARCH_ARTIFACT_WORDS: &[&str] = &[
    "album",
    "cover",
    "playlist",
    "artwork",
    "poster",
    "thumbnail",
    "logo",
    "icon",
    "banner",
    "mockup",
    "screenshot",
    "wallpaper",
];

const IMAGE_SEARCH_STOP_WORDS: &[&str] = &[
    "a",
    "an",
    "the",
    "and",
    "or",
    "but",
    "in",
    "on",
    "at",
    "to",
    "for",
    "of",
    "with",
    "by",
    "from",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "do",
    "does",
    "did",
    "will",
    "would",
    "could",
    "should",
    "may",
    "might",
    "shall",
    "can",
    "that",
    "this",
    "these",
    "those",
    "it",
    "its",
    "very",
    "really",
    "just",
    "also",
    "about",
    "above",
    "after",
    "before",
    "between",
    "into",
    "through",
    "during",
    "each",
    "some",
    "such",
    "no",
    "not",
    "only",
    "same",
    "so",
    "than",
    "too",
    "up",
    "out",
    "if",
    "then",
    "once",
    "here",
    "there",
    "when",
    "where",
    "how",
    "all",
    "both",
    "few",
    "more",
    "most",
    "other",
    "any",
    "as",
    "while",
    "using",
    "showing",
    "featuring",
    "looking",
    "style",
    "styled",
    "inspired",
    "based",
];

/// Presentation adjectives and staging words that make a zero-result retry
/// less searchable. They stay in the primary query; only the shortened retry
/// removes them so the provider receives the concrete subject phrase.
const IMAGE_SEARCH_DESCRIPTORS: &[&str] = &[
    "minimal",
    "minimalist",
    "warm",
    "modern",
    "neutral",
    "tone",
    "surface",
    "beige",
    "background",
    "shelf",
    "product",
    "photo",
    "photography",
    "isolated",
    "studio",
];

/// Metadata markers that make a catalogue hit an explicitly non-photographic
/// result. They are only a fence when the authored query itself asks for a
/// photo/studio result; illustration searches must keep working unchanged.
const NON_PHOTO_RESULT_WORDS: &[&str] = &[
    "illustration",
    "illustrated",
    "drawing",
    "engraving",
    "painting",
    "diagram",
    "sketch",
    "poster",
    "catalog",
    "catalogue",
];

/// Result themes that are usually adjacent catalogue noise rather than the
/// requested product. They remain valid when the authored query explicitly
/// asks for that theme (for example "plush toy studio photo").
const OFF_SUBJECT_RESULT_GROUPS: &[&[&str]] = &[
    &["toy", "plush", "teddy", "doll", "figurine", "stuffed"],
    &["collage", "montage", "puzzle", "pattern"],
];

/// Metadata that usually describes a staged room rather than an isolated
/// catalogue subject. Product-photo prompts reject these results unless the
/// authored query explicitly asks for a room/interior scene.
const SCENE_HEAVY_RESULT_WORDS: &[&str] = &[
    "room", "house", "interior", "hotel", "bedroom", "kitchen", "dining", "hallway", "lounge",
];

/// Query words that explicitly opt into a staged scene. `lounge` is omitted:
/// it is also a product descriptor in phrases such as "lounge chair". A
/// genuine scene request still opts in through `room`, `interior`, or the
/// explicit `hotel lounge` phrase handled by [`query_requests_scene`].
const SCENE_QUERY_OPT_IN_WORDS: &[&str] = &[
    "room", "house", "interior", "bedroom", "kitchen", "dining", "hallway",
];

/// Positive catalogue evidence that a result is presented independently from
/// a room scene. An authored `isolated` query is stricter than a generic
/// `studio photo` query and requires one of these signals (or an equivalent
/// white/plain/transparent-background phrase).
const ISOLATION_RESULT_WORDS: &[&str] = &["isolated", "isolate", "isolation", "cutout"];

#[derive(Clone, PartialEq, Eq)]
pub struct WebOpenverseCredentials {
    pub client_id: String,
    pub client_secret: String,
}

impl WebOpenverseCredentials {
    /// `None` unless both parts are non-empty after trimming.
    pub fn from_parts(client_id: &str, client_secret: &str) -> Option<Self> {
        let client_id = client_id.trim();
        let client_secret = client_secret.trim();
        if client_id.is_empty() || client_secret.is_empty() {
            None
        } else {
            Some(Self {
                client_id: client_id.to_string(),
                client_secret: client_secret.to_string(),
            })
        }
    }
}

/// One search hit ready for the JSON reply / the desktop popover.
pub struct WebImageSearchHit {
    pub id: String,
    pub thumb_data_url: String,
    pub attribution: String,
}

pub struct WebImageSearchOutcome {
    pub results: Vec<WebImageSearchHit>,
    /// `"openverse"` / `"wikimedia"`, `None` when nothing landed.
    pub source: Option<&'static str>,
}

/// Why a `POST /api/ai/image/search` body was refused. Both variants answer
/// HTTP 400; the enum exists so the route reports WHICH client mistake was
/// made instead of matching on prose, and `Display` reproduces the exact
/// sentence the JSON reply already carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchRequestError {
    /// The body is not JSON, or is JSON but not an object.
    InvalidBody,
    /// The body is a valid object but carries no non-blank `query`.
    MissingQuery,
}

impl std::fmt::Display for SearchRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchRequestError::InvalidBody => f.write_str("invalid request body"),
            SearchRequestError::MissingQuery => f.write_str("missing query"),
        }
    }
}

impl std::error::Error for SearchRequestError {}

/// Parse the request body and snapshot the daemon-side credential fallback.
/// Returns `(query, credentials)` or the reason for the 400 reply.
pub fn parse_search_request(
    body: &str,
    state: &op_editor_core::EditorState,
) -> Result<(String, Option<WebOpenverseCredentials>), SearchRequestError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| SearchRequestError::InvalidBody)?;
    let obj = value.as_object().ok_or(SearchRequestError::InvalidBody)?;
    let query = obj
        .get("query")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .ok_or(SearchRequestError::MissingQuery)?;
    // Browser-held credential wins; the daemon's persisted settings are the
    // fallback (both are optional — anonymous Openverse works, rate-limited).
    let request_credentials = obj
        .get("openverse")
        .and_then(serde_json::Value::as_object)
        .and_then(|cred| {
            WebOpenverseCredentials::from_parts(
                cred.get("client_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                cred.get("client_secret")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            )
        });
    let credentials = request_credentials.or_else(|| {
        let settings = &state.editor_ui.agent_settings;
        WebOpenverseCredentials::from_parts(
            &settings.openverse_client_id,
            &settings.openverse_client_secret,
        )
    });
    Ok((query.to_string(), credentials))
}

/// JSON reply body for a finished search.
pub fn search_outcome_to_json(outcome: &WebImageSearchOutcome) -> String {
    let results: Vec<serde_json::Value> = outcome
        .results
        .iter()
        .map(|hit| {
            serde_json::json!({
                "id": hit.id,
                "thumb_data_url": hit.thumb_data_url,
                "attribution": hit.attribution,
            })
        })
        .collect();
    serde_json::json!({
        "ok": true,
        "results": results,
        "source": outcome.source,
    })
    .to_string()
}

/// Run the full search ladder on the calling thread (the connection's own
/// thread — the caller must NOT hold the state lock).
pub fn run_search_blocking(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> WebImageSearchOutcome {
    // A private runtime here would panic the moment this sync helper is
    // reached from a tokio worker; `block_on_anywhere` runs the ladder on the
    // shared (enable_all) runtime instead — same IO/timer drivers, no
    // runtime-in-runtime hazard.
    crate::net::block_on_image_runtime(run_search(query, credentials))
}

/// Run the provider ladder for a single usable thumbnail, bounding the whole
/// async ladder (catalog requests, retries, and thumbnail download together)
/// by `remaining`. This is the MCP enrichment path: unlike the Web UI search,
/// it needs only one image and must return before its outer transport timeout.
pub fn run_first_search_blocking_with_timeout(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
    remaining: Duration,
) -> WebImageSearchOutcome {
    run_with_timeout(remaining, run_first_search(query, credentials))
        .unwrap_or_else(empty_search_outcome)
}

fn run_with_timeout<F>(remaining: Duration, future: F) -> Option<F::Output>
where
    F: std::future::Future,
{
    if remaining.is_zero() {
        return None;
    }
    crate::net::block_on_image_runtime(
        async move { tokio::time::timeout(remaining, future).await.ok() },
    )
}

fn empty_search_outcome() -> WebImageSearchOutcome {
    WebImageSearchOutcome {
        results: Vec::new(),
        source: None,
    }
}

async fn run_search(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> WebImageSearchOutcome {
    let Ok(client) = reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-web-daemon/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return WebImageSearchOutcome {
            results: Vec::new(),
            source: None,
        };
    };
    run_search_with_fetcher(&client, query, credentials, |url: String| {
        let client = client.clone();
        async move { fetch_image_data_url(&client, &url).await }
    })
    .await
}

/// Single-result variant of [`run_search`]. Provider list lookup keeps the
/// same Openverse → retry → Wikimedia order, but thumbnail materialization
/// stops after the first successful download instead of fetching the whole
/// five-result Web UI page.
async fn run_first_search(
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> WebImageSearchOutcome {
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("openpencil-web-daemon/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        return empty_search_outcome();
    };
    run_first_search_with_fetcher(&client, query, credentials, |url: String| {
        let client = client.clone();
        async move { fetch_image_data_url(&client, &url).await }
    })
    .await
}

/// The full search ladder over a caller-supplied client + thumbnail
/// materializer. Shared by this daemon route (plain embed) and the desktop
/// popover (its own user-agent + skia down-scale pass on each thumbnail).
///
/// `fetch_data_url` downloads one thumbnail URL into a `data:` URL; hits
/// whose thumbnails fail to download are dropped.
pub async fn run_search_with_fetcher<F, Fut>(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
    fetch_data_url: F,
) -> WebImageSearchOutcome
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    // Simplify verbose prompts into keywords (TS simplifySearchQuery).
    let query = simplify_search_query(query);

    // Openverse first; either a zero-result answer or an answer fully removed
    // by the relevance/photo fence retries once with a short concrete subject
    // phrase before falling through to Wikimedia.
    let hits = fetch_relevant_openverse_list(client, &query, credentials).await;
    if let Some(urls) = hits.filter(|h| !h.is_empty()) {
        let results = materialize_thumbs(urls, &fetch_data_url).await;
        if !results.is_empty() {
            return WebImageSearchOutcome {
                results,
                source: Some("openverse"),
            };
        }
    }
    let wiki = fetch_relevant_wikimedia_list(client, &query).await;
    let results = materialize_thumbs(wiki, &fetch_data_url).await;
    let source = (!results.is_empty()).then_some("wikimedia");
    WebImageSearchOutcome { results, source }
}

async fn run_first_search_with_fetcher<F, Fut>(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
    fetch_data_url: F,
) -> WebImageSearchOutcome
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let query = simplify_search_query(query);

    let hits = fetch_relevant_openverse_list(client, &query, credentials).await;
    if let Some(urls) = hits.filter(|h| !h.is_empty()) {
        if let Some(result) = materialize_first_thumb(urls, &fetch_data_url).await {
            return WebImageSearchOutcome {
                results: vec![result],
                source: Some("openverse"),
            };
        }
    }

    let wiki = fetch_relevant_wikimedia_list(client, &query).await;
    let Some(result) = materialize_first_thumb(wiki, &fetch_data_url).await else {
        return empty_search_outcome();
    };
    WebImageSearchOutcome {
        results: vec![result],
        source: Some("wikimedia"),
    }
}

/// Fetch and relevance-filter the Openverse catalogue, retrying at most once
/// with the concrete subject phrase. `None` remains a request-level failure:
/// it falls through to Wikimedia without turning a network error into another
/// Openverse request. `Some([])` and non-empty-but-fully-filtered replies share
/// the same single retry, so the two conditions can never trigger duplicate
/// catalogue requests.
async fn fetch_relevant_openverse_list(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<Vec<RawHit>> {
    fetch_relevant_openverse_list_with(query, |candidate| {
        let client = client.clone();
        let credentials = credentials.cloned();
        async move { fetch_openverse_list(&client, &candidate, credentials.as_ref()).await }
    })
    .await
}

async fn fetch_relevant_openverse_list_with<F, Fut>(
    query: &str,
    mut fetch_list: F,
) -> Option<Vec<RawHit>>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Option<Vec<RawHit>>>,
{
    let hits = fetch_list(query.to_string()).await?;
    let relevant = retain_relevant_hits(hits, query);
    if !relevant.is_empty() {
        return Some(relevant);
    }

    let Some(retry_query) = two_keyword_retry(query) else {
        return Some(relevant);
    };
    let retry = fetch_list(retry_query).await?;
    // The provider receives a shorter concrete query, but relevance still
    // follows the authored query so photo/studio/isolated intent is not lost.
    Some(retain_relevant_hits(retry, query))
}

fn two_keyword_retry(query: &str) -> Option<String> {
    let core = concrete_query_words(query);
    // Product prompts commonly put the searchable subject noun near the end
    // (for example "... table lamp terracotta"). Keeping the concrete tail
    // avoids truncating that noun while preserving the existing behavior for
    // short two- and three-word subject phrases.
    let core_tail = &core[core.len().saturating_sub(3)..];
    let retry = core_tail.join(" ");
    let primary = lexical_words(query).join(" ");
    (!retry.is_empty() && retry != primary).then_some(retry)
}

fn core_query_words(query: &str) -> Vec<String> {
    concrete_query_words(query)
        .into_iter()
        .map(|word| canonicalize_word(&word))
        .collect()
}

fn concrete_query_words(query: &str) -> Vec<String> {
    lexical_words(query)
        .into_iter()
        .filter(|word| {
            let canonical = canonicalize_word(word);
            !IMAGE_SEARCH_DESCRIPTORS.contains(&canonical.as_str())
        })
        .collect()
}

fn lexical_words(value: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(value.len());
    for ch in value.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }
    normalized.split_whitespace().map(str::to_string).collect()
}

fn normalized_words(value: &str) -> Vec<String> {
    lexical_words(value)
        .into_iter()
        .map(|word| canonicalize_word(&word))
        .collect()
}

fn canonicalize_word(word: &str) -> String {
    match word {
        "knit" | "knitted" | "knitting" => return "knit".to_string(),
        "wood" | "wooden" => return "wood".to_string(),
        _ => {}
    }

    if word.len() > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..word.len() - 3]);
    }
    if word.len() > 4
        && ["ches", "shes", "xes", "zes"]
            .iter()
            .any(|suffix| word.ends_with(suffix))
    {
        return word[..word.len() - 2].to_string();
    }
    if word.len() > 3
        && word.ends_with('s')
        && !word.ends_with("ss")
        && !word.ends_with("us")
        && !word.ends_with("is")
    {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// Keep only provider hits whose title/tags mention at least one concrete
/// subject word. When a query contains no concrete words (for example an
/// all-descriptor prompt), preserve the provider response rather than making
/// an unprovable relevance decision.
fn retain_relevant_hits(hits: Vec<RawHit>, query: &str) -> Vec<RawHit> {
    let strict = retain_relevant_hits_enforcing(hits.clone(), query, true);
    if !strict.is_empty() {
        return strict;
    }
    // Isolation evidence ("isolated", "white background", …) is sparse in
    // provider metadata, and treating it as a hard fence empties the result
    // set for perfectly good product queries — the slot then publishes as a
    // gray placeholder. When the strict pass keeps nothing, degrade the
    // isolation requirement to a preference and keep the subject fence.
    retain_relevant_hits_enforcing(hits, query, false)
}

fn retain_relevant_hits_enforcing(
    hits: Vec<RawHit>,
    query: &str,
    enforce_isolation: bool,
) -> Vec<RawHit> {
    let core = core_query_words(query);
    let requires_photo = query_requests_photo(query);
    let requires_isolation = enforce_isolation && query_requests_isolation(query);
    if core.is_empty() {
        return hits
            .into_iter()
            .filter(|hit| {
                !metadata_is_off_subject(&hit.relevance_metadata, query)
                    && (!requires_photo
                        || !metadata_is_explicitly_non_photo(&hit.relevance_metadata))
                    && (!requires_isolation
                        || metadata_has_isolation_evidence(&hit.relevance_metadata))
            })
            .collect();
    }
    // Multi-word product subjects need more than one token of evidence. A
    // single generic overlap such as "lamp" previously accepted
    // "Photography lamp setup" for "ceramic table lamp studio photo", which
    // is technically an image but visibly the wrong product. Single-word
    // subjects still use one match so common queries such as "armchair" keep
    // their useful recall.
    let minimum_overlap = if core.len() >= 2 { 2 } else { 1 };
    let mut ranked: Vec<(usize, usize, usize, usize, RawHit)> = hits
        .into_iter()
        .filter_map(|hit| {
            if metadata_is_off_subject(&hit.relevance_metadata, query)
                || (requires_photo && metadata_is_explicitly_non_photo(&hit.relevance_metadata))
                || (requires_isolation && !metadata_has_isolation_evidence(&hit.relevance_metadata))
                || (requires_photo
                    && metadata_is_scene_heavy(&hit.relevance_metadata)
                    && !query_requests_scene(query))
            {
                return None;
            }
            let title = normalized_words(&hit.title);
            let metadata = normalized_words(&hit.relevance_metadata);
            let title_overlap = overlap_count(&core, &title);
            if requires_photo && title_overlap == 0 {
                return None;
            }
            let total_overlap = overlap_count(&core, &metadata);
            if total_overlap < minimum_overlap {
                return None;
            }
            let title_extra_tokens = title
                .iter()
                .filter(|word| {
                    !core.contains(word)
                        && !IMAGE_SEARCH_STOP_WORDS.contains(&word.as_str())
                        && !IMAGE_SEARCH_DESCRIPTORS.contains(&word.as_str())
                })
                .count();
            let title_subject_tokens = title_overlap + title_extra_tokens;
            Some((
                title_overlap,
                title_subject_tokens,
                title_extra_tokens,
                total_overlap,
                hit,
            ))
        })
        .collect();
    // Prefer a subject-dense title before raw overlap: a concise "Ceramic
    // vase" is a safer product match than a long archaeological title that
    // happens to contain both words. Then prefer title evidence, concision,
    // and finally title + tag evidence. `sort_by` is stable, so exact ties
    // retain provider order.
    ranked.sort_by(|left, right| {
        let density = (right.0 * left.1).cmp(&(left.0 * right.1));
        density
            .then_with(|| right.0.cmp(&left.0))
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| right.3.cmp(&left.3))
    });
    ranked.into_iter().map(|(_, _, _, _, hit)| hit).collect()
}

fn overlap_count(core: &[String], candidate: &[String]) -> usize {
    core.iter().filter(|word| candidate.contains(word)).count()
}

fn query_requests_photo(query: &str) -> bool {
    normalized_words(query).iter().any(|word| {
        matches!(
            word.as_str(),
            "photo" | "photograph" | "photography" | "studio" | "isolated"
        )
    })
}

fn metadata_is_explicitly_non_photo(metadata: &str) -> bool {
    normalized_words(metadata)
        .iter()
        .any(|word| NON_PHOTO_RESULT_WORDS.contains(&word.as_str()))
}

fn metadata_is_off_subject(metadata: &str, query: &str) -> bool {
    let metadata = normalized_words(metadata);
    let query = normalized_words(query);
    OFF_SUBJECT_RESULT_GROUPS.iter().any(|group| {
        let query_requests_group = query.iter().any(|word| group.contains(&word.as_str()));
        !query_requests_group && metadata.iter().any(|word| group.contains(&word.as_str()))
    })
}

fn query_requests_scene(query: &str) -> bool {
    let words = normalized_words(query);
    words
        .iter()
        .any(|word| SCENE_QUERY_OPT_IN_WORDS.contains(&word.as_str()))
        || contains_adjacent_words(&words, "hotel", "lounge")
}

fn metadata_is_scene_heavy(metadata: &str) -> bool {
    let words = normalized_words(metadata);
    words
        .iter()
        .any(|word| word != "lounge" && SCENE_HEAVY_RESULT_WORDS.contains(&word.as_str()))
        || words.iter().enumerate().any(|(index, word)| {
            word == "lounge" && words.get(index + 1).is_none_or(|next| next != "chair")
        })
}

fn query_requests_isolation(query: &str) -> bool {
    normalized_words(query).iter().any(|word| {
        matches!(
            word.as_str(),
            "isolated" | "isolate" | "isolation" | "cutout"
        )
    })
}

fn metadata_has_isolation_evidence(metadata: &str) -> bool {
    let words = normalized_words(metadata);
    words
        .iter()
        .any(|word| ISOLATION_RESULT_WORDS.contains(&word.as_str()))
        || contains_adjacent_words(&words, "cut", "out")
        || contains_adjacent_words(&words, "white", "background")
        || contains_adjacent_words(&words, "white", "backdrop")
        || contains_adjacent_words(&words, "plain", "background")
        || contains_adjacent_words(&words, "transparent", "background")
        || contains_adjacent_words(&words, "on", "white")
}

fn contains_adjacent_words(words: &[String], first: &str, second: &str) -> bool {
    words
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
}

#[derive(Clone)]
pub struct RawHit {
    id: String,
    thumb_url: String,
    attribution: String,
    title: String,
    relevance_metadata: String,
}

/// `None` = request-level failure (429 / network), `Some([])` = the
/// catalogue answered with zero hits (the ladder distinguishes the two).
async fn fetch_openverse_list(
    client: &reqwest::Client,
    query: &str,
    credentials: Option<&WebOpenverseCredentials>,
) -> Option<Vec<RawHit>> {
    let url = reqwest::Url::parse_with_params(
        "https://api.openverse.org/v1/images/",
        &[
            ("q", query),
            ("page_size", &SEARCH_CANDIDATE_COUNT.to_string()),
        ],
    )
    .ok()?;
    let mut request = client.get(url);
    if let Some(credentials) = credentials {
        if let Some(token) = fetch_openverse_token(client, credentials).await {
            request = request.bearer_auth(token);
        }
    }
    let resp = request.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json = read_json_capped(resp).await?;
    Some(parse_openverse_results(&json))
}

/// Catalogue-list bodies are small JSON; 4 MiB bounds a misbehaving reply.
async fn read_json_capped(resp: reqwest::Response) -> Option<serde_json::Value> {
    let bytes = read_capped(resp, MAX_EMBEDDED_IMAGE_BYTES).await?;
    serde_json::from_slice(&bytes).ok()
}

pub fn parse_openverse_results(json: &serde_json::Value) -> Vec<RawHit> {
    let Some(results) = json.get("results").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    results
        .iter()
        .filter_map(|r| {
            let thumb = r
                .get("thumbnail")
                .and_then(serde_json::Value::as_str)
                .or_else(|| r.get("url").and_then(serde_json::Value::as_str))?;
            let license = format!(
                "{} {}",
                r.get("license")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
                r.get("license_version")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(""),
            );
            Some(RawHit {
                id: r
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                thumb_url: thumb.to_string(),
                attribution: r
                    .get("attribution")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| license.trim().to_string()),
                title: r
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                relevance_metadata: openverse_relevance_metadata(r),
            })
        })
        .take(SEARCH_CANDIDATE_COUNT)
        .collect()
}

fn openverse_relevance_metadata(result: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    if let Some(title) = result.get("title").and_then(serde_json::Value::as_str) {
        parts.push(title.to_string());
    }
    if let Some(tags) = result.get("tags") {
        match tags {
            serde_json::Value::Array(items) => {
                for item in items {
                    if let Some(tag) = item
                        .as_str()
                        .or_else(|| item.get("name").and_then(serde_json::Value::as_str))
                    {
                        parts.push(tag.to_string());
                    }
                }
            }
            serde_json::Value::String(tags) => parts.push(tags.clone()),
            _ => {}
        }
    }
    parts.join(" ")
}

async fn fetch_wikimedia_list(client: &reqwest::Client, query: &str) -> Vec<RawHit> {
    let Ok(url) = reqwest::Url::parse_with_params(
        "https://commons.wikimedia.org/w/api.php",
        &[
            ("action", "query"),
            ("generator", "search"),
            ("gsrsearch", query),
            ("gsrnamespace", "6"),
            ("gsrlimit", &SEARCH_CANDIDATE_COUNT.to_string()),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime|extmetadata"),
            ("iiurlwidth", "800"),
            ("format", "json"),
            ("origin", "*"),
        ],
    ) else {
        return Vec::new();
    };
    let Ok(resp) = client.get(url).send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    let Some(json) = read_json_capped(resp).await else {
        return Vec::new();
    };
    parse_wikimedia_results(&json)
}

async fn fetch_relevant_wikimedia_list(client: &reqwest::Client, query: &str) -> Vec<RawHit> {
    fetch_relevant_wikimedia_list_with(query, |candidate| {
        let client = client.clone();
        async move { fetch_wikimedia_list(&client, &candidate).await }
    })
    .await
}

async fn fetch_relevant_wikimedia_list_with<F, Fut>(query: &str, mut fetch_list: F) -> Vec<RawHit>
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = Vec<RawHit>>,
{
    let hits = fetch_list(query.to_string()).await;
    let relevant = retain_relevant_hits(hits, query);
    if !relevant.is_empty() {
        return relevant;
    }

    let Some(retry_query) = two_keyword_retry(query) else {
        return relevant;
    };
    let retry = fetch_list(retry_query).await;
    // Keep the original photo/studio/isolated contract for concrete retries.
    retain_relevant_hits(retry, query)
}

pub fn parse_wikimedia_results(json: &serde_json::Value) -> Vec<RawHit> {
    let Some(pages) = json
        .get("query")
        .and_then(|q| q.get("pages"))
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };
    pages
        .values()
        .filter_map(|page| {
            let info = page.get("imageinfo")?.as_array()?.first()?;
            if !wikimedia_info_is_image(page, info) {
                return None;
            }
            let thumb = info
                .get("thumburl")
                .and_then(serde_json::Value::as_str)
                .or_else(|| info.get("url").and_then(serde_json::Value::as_str))?;
            Some(RawHit {
                id: page
                    .get("pageid")
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                thumb_url: thumb.to_string(),
                attribution: info
                    .get("extmetadata")
                    .and_then(|m| m.get("LicenseShortName"))
                    .and_then(|l| l.get("value"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                title: page
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                relevance_metadata: page
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .take(SEARCH_CANDIDATE_COUNT)
        .collect()
}

/// Wikimedia can return page-one JPEG thumbnails for PDFs, audio, and video
/// files. Those thumbnails are renderable bytes but are not image-search
/// results, so accepting them turns an archival cover page into a product
/// photo. Trust the source MIME when it is present and retain a title-extension
/// fence for older/test payloads that omit `mime`.
pub(crate) fn wikimedia_info_is_image(page: &serde_json::Value, info: &serde_json::Value) -> bool {
    if let Some(mime) = info.get("mime").and_then(serde_json::Value::as_str) {
        return mime.trim().to_ascii_lowercase().starts_with("image/");
    }

    let title = page
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    ![".pdf", ".ogg", ".oga", ".ogv", ".webm", ".mp3", ".mp4"]
        .iter()
        .any(|extension| title.ends_with(extension))
}

/// Download each hit's thumbnail into a `data:` URL through the caller's
/// materializer. Hits whose thumbnails fail to download are dropped.
async fn materialize_thumbs<F, Fut>(hits: Vec<RawHit>, fetch_data_url: &F) -> Vec<WebImageSearchHit>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    let mut out = Vec::with_capacity(hits.len().min(SEARCH_RESULT_COUNT));
    for hit in hits {
        if let Some(data_url) = fetch_data_url(hit.thumb_url.clone()).await {
            out.push(WebImageSearchHit {
                id: hit.id,
                thumb_data_url: data_url,
                attribution: hit.attribution,
            });
            if out.len() == SEARCH_RESULT_COUNT {
                break;
            }
        }
    }
    out
}

/// Materialize only the first usable thumbnail. Failed downloads advance to
/// the next provider hit, while the first success ends the loop immediately.
async fn materialize_first_thumb<F, Fut>(
    hits: Vec<RawHit>,
    fetch_data_url: &F,
) -> Option<WebImageSearchHit>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = Option<String>>,
{
    for hit in hits {
        if let Some(data_url) = fetch_data_url(hit.thumb_url).await {
            return Some(WebImageSearchHit {
                id: hit.id,
                thumb_data_url: data_url,
                attribution: hit.attribution,
            });
        }
    }
    None
}

/// Simplify a verbose prompt into provider keywords. Shared by the desktop
/// image pipeline and the web daemon route.
pub fn simplify_search_query(prompt: &str) -> String {
    let mut normalized = String::with_capacity(prompt.len());
    for ch in prompt.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '-' {
            normalized.push(ch);
        } else {
            normalized.push(' ');
        }
    }
    let keywords: Vec<&str> = normalized
        .split_whitespace()
        .filter(|word| word.len() > 2 && !IMAGE_SEARCH_STOP_WORDS.contains(word))
        .take(6)
        .collect();
    // Drop artifact words ONLY when aesthetic words remain — "logo" alone
    // must not become an empty query.
    let non_artifact: Vec<&str> = keywords
        .iter()
        .copied()
        .filter(|word| !IMAGE_SEARCH_ARTIFACT_WORDS.contains(word))
        .collect();
    let keywords: Vec<&str> = if non_artifact.is_empty() {
        keywords
    } else {
        non_artifact
    }
    .into_iter()
    .take(4)
    .collect();
    if keywords.is_empty() {
        prompt.chars().take(30).collect()
    } else {
        keywords.join(" ")
    }
}

pub async fn fetch_openverse_token(
    client: &reqwest::Client,
    credentials: &WebOpenverseCredentials,
) -> Option<String> {
    let resp = client
        .post("https://api.openverse.org/v1/auth_tokens/token/")
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
        ])
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json = read_json_capped(resp).await?;
    json.get("access_token")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

/// Download `url` and embed it as a `data:` URL, subject to the 4 MiB cap.
/// Embeds only payloads the exact renderer can decode: PNG/JPEG go in as-is
/// (down-scaled when oversized), everything else must transcode or the
/// candidate is rejected — see `fetch::renderable_image_data_url`.
pub async fn fetch_image_data_url(client: &reqwest::Client, url: &str) -> Option<String> {
    let (mime, bytes) = fetch_image_bytes(client, url, MAX_EMBEDDED_IMAGE_BYTES).await?;
    crate::net::fetch::renderable_image_data_url(&mime, &bytes)
}

/// Download `url` and return its normalized image mime + raw bytes, subject
/// to `cap` (streaming abort). `None` for failures, empty bodies, and
/// non-embeddable mimes. Shared with the desktop, which layers its skia
/// down-scale pass on the bytes before embedding.
pub async fn fetch_image_bytes(
    client: &reqwest::Client,
    url: &str,
    cap: usize,
) -> Option<(String, Vec<u8>)> {
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let header_mime = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(normalize_image_mime_header);
    let bytes = read_capped(resp, cap).await?;
    if bytes.is_empty() {
        return None;
    }
    let mime = header_mime.or_else(|| sniff_image_mime(&bytes).map(str::to_string))?;
    Some((mime, bytes))
}

/// Read a response body, aborting as soon as it exceeds `cap` — the cap must
/// hold with or without a Content-Length header, and an over-cap body must
/// never be fully buffered first (a chunked response could otherwise stream
/// gigabytes into memory before a post-hoc length check).
pub async fn read_capped(mut resp: reqwest::Response, cap: usize) -> Option<Vec<u8>> {
    if resp.content_length().is_some_and(|len| len > cap as u64) {
        return None;
    }
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.ok()? {
        if bytes.len() + chunk.len() > cap {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    Some(bytes)
}

/// Normalize a Content-Type header into an embeddable `image/*` mime
/// (`image/jpg` alias folded, SVG rejected).
pub fn normalize_image_mime_header(value: &str) -> Option<String> {
    let mime = value.split(';').next()?.trim().to_ascii_lowercase();
    if mime == "image/jpg" {
        return Some("image/jpeg".to_string());
    }
    if mime.starts_with("image/") && mime != "image/svg+xml" {
        Some(mime)
    } else {
        None
    }
}

/// Magic-byte sniff for the embeddable raster formats.
pub fn sniff_image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1A\n") {
        return Some("image/png");
    }
    if bytes.starts_with(b"\xFF\xD8\xFF") {
        return Some("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    None
}

#[cfg(test)]
#[path = "web_image_search_tests.rs"]
mod tests;
