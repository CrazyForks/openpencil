use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn simplify_search_query_mirrors_the_desktop_adapter() {
    assert_eq!(
        simplify_search_query("A beautiful sunset over the mountains"),
        "beautiful sunset over mountains"
    );
    // Artifact words drop only when aesthetic words remain.
    assert_eq!(
        simplify_search_query("synthwave album cover neon"),
        "synthwave neon"
    );
    assert_eq!(simplify_search_query("logo"), "logo");
    // Empty keyword set falls back to a 30-char prefix.
    assert_eq!(simplify_search_query("の"), "の");
}

#[test]
fn zero_result_retry_keeps_the_core_product_phrase() {
    assert_eq!(
        simplify_search_query("minimalist ceramic vase beige"),
        "minimalist ceramic vase beige",
        "descriptor filtering belongs only to the zero-result retry"
    );
    assert_eq!(
        two_keyword_retry("minimalist ceramic vase beige"),
        Some("ceramic vase".to_string())
    );
    assert_eq!(
        two_keyword_retry("linen cushion neutral tone"),
        Some("linen cushion".to_string())
    );
    assert_eq!(
        two_keyword_retry("oak desk lamp"),
        None,
        "an already-concrete query must not repeat the same request"
    );
    assert_eq!(
        two_keyword_retry("minimal knitted wool sweater"),
        Some("knitted wool sweater".to_string()),
        "the retry must keep the garment subject instead of wasting a slot on staging prose"
    );
    assert_eq!(
        two_keyword_retry("sculptural arc table lamp terracotta"),
        Some("table lamp terracotta".to_string()),
        "the retry must keep a trailing product subject instead of truncating it"
    );
    assert_eq!(
        two_keyword_retry("minimal sneakers product photography studio"),
        Some("sneakers".to_string()),
        "one concrete product noun is a stronger retry than giving up on descriptor-only noise"
    );
    assert_eq!(
        two_keyword_retry("armchair isolated"),
        Some("armchair".to_string()),
        "two-word photo prompts still need a concrete recovery query"
    );
    assert_eq!(
        core_query_words("wooden lamps studio photo"),
        ["wood", "lamp"],
        "wooden is product evidence, while simple plurals canonicalize"
    );
    assert_eq!(
        core_query_words("knitting cardigans isolated"),
        ["knit", "cardigan"]
    );
}

#[test]
fn parse_search_request_reads_query_and_prefers_request_credentials() {
    let mut state = op_editor_core::EditorState::default();
    state.editor_ui.agent_settings.openverse_client_id = "persisted-id".into();
    state.editor_ui.agent_settings.openverse_client_secret = "persisted-secret".into();
    let (query, cred) = parse_search_request(
        r#"{"query":"cat","openverse":{"client_id":"req-id","client_secret":"req-secret"}}"#,
        &state,
    )
    .expect("parses");
    assert_eq!(query, "cat");
    assert_eq!(cred.expect("cred").client_id, "req-id");
    // No request credential → daemon-persisted fallback.
    let (_, cred) = parse_search_request(r#"{"query":"cat"}"#, &state).expect("parses");
    assert_eq!(cred.expect("cred").client_id, "persisted-id");
    // Neither → anonymous.
    let empty = op_editor_core::EditorState::default();
    let (_, cred) = parse_search_request(r#"{"query":"cat"}"#, &empty).expect("parses");
    assert!(cred.is_none());
}

#[test]
fn parse_search_request_rejects_bad_bodies() {
    let state = op_editor_core::EditorState::default();
    assert!(parse_search_request("", &state).is_err());
    assert!(parse_search_request("{}", &state).is_err());
    assert!(parse_search_request(r#"{"query":"  "}"#, &state).is_err());
}

#[test]
fn parse_openverse_results_maps_thumbnail_license_and_candidate_cap() {
    let mut results = vec![
        serde_json::json!({"id": "a", "thumbnail": "https://x/a.jpg", "attribution": "By A"}),
        serde_json::json!({"id": "b", "url": "https://x/b.jpg", "license": "cc0", "license_version": "1.0"}),
        serde_json::json!({"id": "missing-thumbnail"}),
    ];
    results.extend((0..(SEARCH_CANDIDATE_COUNT + 3)).map(|index| {
        serde_json::json!({
            "id": format!("candidate-{index}"),
            "thumbnail": format!("https://x/candidate-{index}.jpg")
        })
    }));
    let json = serde_json::json!({"results": results});
    let hits = parse_openverse_results(&json);
    assert_eq!(hits.len(), SEARCH_CANDIDATE_COUNT);
    assert_eq!(hits[0].id, "a");
    assert_eq!(hits[0].attribution, "By A");
    assert_eq!(hits[1].thumb_url, "https://x/b.jpg");
    assert_eq!(hits[1].attribution, "cc0 1.0");
}

#[test]
fn relevance_fence_rejects_holiday_hit_and_accepts_armchair_title_or_tag() {
    let json = serde_json::json!({
        "results": [
            {
                "id": "holiday",
                "thumbnail": "https://x/holiday.jpg",
                "title": "Christmas technology celebration",
                "tags": [{"name": "computer"}, {"name": "holiday"}]
            },
            {
                "id": "title",
                "thumbnail": "https://x/title.jpg",
                "title": "Boucle armchair in a quiet room"
            },
            {
                "id": "tag",
                "thumbnail": "https://x/tag.jpg",
                "title": "Neutral furniture study",
                "tags": [{"name": "armchair"}]
            }
        ]
    });
    assert_eq!(
        core_query_words("warm modern armchair"),
        vec!["armchair".to_string()]
    );

    let hits = retain_relevant_hits(parse_openverse_results(&json), "warm modern armchair");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["title", "tag"]);
    assert!(!ids.contains(&"holiday"));
}

#[test]
fn relevance_fence_returns_empty_when_no_hit_mentions_the_subject() {
    let hits = vec![RawHit {
        id: "holiday".to_string(),
        thumb_url: "https://x/holiday.jpg".to_string(),
        attribution: "By X".to_string(),
        title: "Christmas technology celebration".to_string(),
        relevance_metadata: "Christmas technology celebration computer".to_string(),
    }];

    assert!(retain_relevant_hits(hits, "warm modern armchair").is_empty());
}

#[test]
fn relevance_fence_ranks_more_complete_matches_stably() {
    let hits = [
        ("material-only", "Natural wool textile"),
        ("complete-first", "Knitted wool sweater"),
        ("complete-second", "Wool sweater, hand knitted"),
        ("subject-only", "Winter sweater"),
    ]
    .into_iter()
    .map(|(id, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: metadata.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "minimal knitted wool sweater");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(
        ids,
        ["complete-first", "complete-second"],
        "multi-word products require more than one concrete subject match while equal overlap preserves provider order"
    );
}

#[test]
fn multi_word_product_query_rejects_one_generic_match() {
    let hits = [
        ("setup", "Photography lamp setup"),
        ("table-lamp", "Modern table lamp in a studio"),
        ("ceramic-lamp", "Ceramic lamp product photograph"),
    ]
    .into_iter()
    .map(|(id, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: metadata.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "ceramic table lamp studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["table-lamp", "ceramic-lamp"]);
}

#[test]
fn single_word_product_query_keeps_one_exact_subject_match() {
    let hits = vec![RawHit {
        id: "armchair".to_string(),
        thumb_url: "https://x/armchair.jpg".to_string(),
        attribution: String::new(),
        title: "Studio armchair photograph".to_string(),
        relevance_metadata: "Studio armchair photograph".to_string(),
    }];

    let ranked = retain_relevant_hits(hits, "armchair studio photo");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "armchair");
}

#[test]
fn product_photo_query_rejects_room_scenes_unless_the_query_requests_one() {
    let hits = [
        (
            "scene",
            "Cream armchair in a living room",
            "Cream armchair furniture in a living room interior",
        ),
        (
            "product",
            "Cream armchair",
            "Cream armchair isolated product photograph",
        ),
    ]
    .into_iter()
    .map(|(id, title, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "cream armchair studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();
    assert_eq!(ids, ["product"]);

    let requested_scene = vec![RawHit {
        id: "scene".to_string(),
        thumb_url: "https://x/scene.jpg".to_string(),
        attribution: String::new(),
        title: "Cream armchair in a living room".to_string(),
        relevance_metadata: "Cream armchair furniture in a living room interior".to_string(),
    }];
    assert_eq!(
        retain_relevant_hits(requested_scene, "cream armchair living room photo").len(),
        1,
        "an explicit room request opts into scene-heavy results"
    );
}

#[test]
fn lounge_chair_does_not_opt_into_scenes_but_explicit_scene_phrases_do() {
    assert!(!query_requests_scene("lounge chair studio photo"));
    assert!(query_requests_scene("lounge chair in a living room photo"));
    assert!(query_requests_scene("lounge chair in a hotel lounge photo"));
    assert!(query_requests_scene("lounge chair interior photo"));

    let hits = [
        (
            "hotel-scene",
            "Modern lounge chair",
            "Modern lounge chair in a hotel lounge interior",
        ),
        (
            "product",
            "Modern lounge chair",
            "Modern lounge chair isolated product photograph",
        ),
    ]
    .into_iter()
    .map(|(id, title, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "lounge chair studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();
    assert_eq!(ids, ["product"]);
}

#[test]
fn product_photo_scene_fence_covers_common_room_metadata() {
    for marker in SCENE_HEAVY_RESULT_WORDS {
        assert!(
            metadata_is_scene_heavy(&format!("table lamp {marker}")),
            "{marker} must be recognized as scene-heavy metadata"
        );
    }
}

#[test]
fn concise_subject_dense_vase_title_beats_long_kylix_metadata() {
    let hits = [
        (
            "kylix",
            "Ancient Greek kylix drinking cup ceramic vase studio collection",
            "Ancient Greek kylix drinking cup ceramic vase studio pottery collection",
        ),
        (
            "vase",
            "Vase",
            "Vase ceramic pottery isolated product photograph",
        ),
    ]
    .into_iter()
    .map(|(id, title, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: title.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "ceramic vase studio photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["vase", "kylix"]);
}

#[test]
fn photo_query_rejects_explicit_illustration_and_catalog_hits() {
    let hits = [
        (
            "studio",
            "Ceramic tableware isolated studio product photograph",
        ),
        ("engraving", "Vintage ceramic tableware engraving"),
        ("catalog", "Ceramic tableware catalogue page"),
    ]
    .into_iter()
    .map(|(id, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: metadata.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(
        hits,
        "ceramic tableware isolated studio product photography",
    );
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["studio"]);
}

#[test]
fn isolated_query_requires_positive_isolation_metadata() {
    let hits = [
        ("bare-studio", "Modern lounge chair studio photograph"),
        (
            "isolated",
            "Modern lounge chair isolated product photograph",
        ),
        ("cutout", "Modern lounge chair cutout product photograph"),
        (
            "white-background",
            "Modern lounge chair on a white background",
        ),
    ]
    .into_iter()
    .map(|(id, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: metadata.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "lounge chair isolated photo");
    let ids: Vec<&str> = ranked.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(ids, ["isolated", "white-background", "cutout"]);
}

#[test]
fn isolated_retry_preserves_the_original_isolation_contract() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "modern sofa isolated photo" => Some(Vec::new()),
                "sofa" => Some(vec![
                    RawHit {
                        id: "bare-studio".to_string(),
                        thumb_url: "https://x/bare-studio.jpg".to_string(),
                        attribution: String::new(),
                        title: "Modern sofa studio photograph".to_string(),
                        relevance_metadata: "Modern sofa studio photograph".to_string(),
                    },
                    RawHit {
                        id: "cutout".to_string(),
                        thumb_url: "https://x/cutout.jpg".to_string(),
                        attribution: String::new(),
                        title: "Modern sofa".to_string(),
                        relevance_metadata: "Modern sofa cut out on white background".to_string(),
                    },
                ]),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "modern sofa isolated photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["modern sofa isolated photo", "sofa"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "cutout");
}

#[test]
fn relevance_filtered_openverse_results_retry_the_concrete_subject_once() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "wool sweater studio photo" => Some(vec![RawHit {
                    id: "catalog".to_string(),
                    thumb_url: "https://x/catalog.jpg".to_string(),
                    attribution: String::new(),
                    title: "Knitted wool sweater catalogue illustration".to_string(),
                    relevance_metadata: "Knitted wool sweater catalogue illustration".to_string(),
                }]),
                "wool sweater" => Some(vec![
                    RawHit {
                        id: "holiday".to_string(),
                        thumb_url: "https://x/holiday.jpg".to_string(),
                        attribution: String::new(),
                        title: "Christmas holiday lights".to_string(),
                        relevance_metadata: "Christmas holiday lights".to_string(),
                    },
                    RawHit {
                        id: "sweater".to_string(),
                        thumb_url: "https://x/sweater.jpg".to_string(),
                        attribution: String::new(),
                        title: "Knitted wool sweater product photo".to_string(),
                        relevance_metadata: "Knitted wool sweater product photo".to_string(),
                    },
                ]),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "wool sweater studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["wool sweater studio photo", "wool sweater"],
        "a filtered non-empty reply gets exactly one concrete retry"
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "sweater");
}

#[test]
fn photo_primary_rejects_tag_only_armchair_then_retries_real_title() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            let json = match query.as_str() {
                "armchair studio photo" => serde_json::json!({"results": [{
                    "id": "table",
                    "thumbnail": "https://x/table.jpg",
                    "title": "Earth-Stripe Table",
                    "tags": [
                        {"name": "coffeetable"},
                        {"name": "woodentable"},
                        {"name": "armchair", "accuracy": 0.94696},
                        {"name": "table", "accuracy": 0.98991}
                    ]
                }]}),
                "armchair" => serde_json::json!({"results": [{
                    "id": "exact",
                    "thumbnail": "https://x/armchair.jpg",
                    "title": "The armchair",
                    "tags": [{"name": "armchair", "accuracy": null}]
                }]}),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            };
            Some(parse_openverse_results(&json))
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "armchair studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["armchair studio photo", "armchair"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "exact");
    assert_eq!(hits[0].title, "The armchair");
}

#[test]
fn cardigan_retry_canonicalizes_knit_and_plural_but_rejects_doll() {
    let fetcher = |query: String| async move {
        let json = match query.as_str() {
            "knitted cardigan studio photo" => serde_json::json!({"results": [{
                "id": "tag-only",
                "thumbnail": "https://x/tag-only.jpg",
                "title": "a better photo of miss henry",
                "tags": ["cardigan", "knit", "knitting", "studio", "sweater", "wool"]
            }]}),
            "knitted cardigan" => serde_json::json!({"results": [
                {
                    "id": "doll",
                    "thumbnail": "https://x/doll.jpg",
                    "title": "My first hand knitted cardigans for 11 cm Obitsu doll body",
                    "tags": ["cardigan", "doll", "dolls", "dollfashion", "handknitted"]
                },
                {
                    "id": "garment",
                    "thumbnail": "https://x/garment.jpg",
                    "title": "#BabyGap NWT knit cardigan. 18-24M. $8 plus ship.",
                    "tags": ["instagramapp"]
                }
            ]}),
            unexpected => panic!("unexpected Openverse query: {unexpected}"),
        };
        Some(parse_openverse_results(&json))
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "knitted cardigan studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "garment");
}

#[test]
fn wooden_lamp_retry_prefers_concise_product_title() {
    let fetcher = |query: String| async move {
        let json = match query.as_str() {
            "wooden lamp studio photo" => serde_json::json!({"results": [{
                "id": "toy",
                "thumbnail": "https://x/toy.jpg",
                "title": "Only on Bonsoni!",
                "tags": ["lamp", "figurine", "plush", "teddy", "toy", "wool"]
            }]}),
            "wooden lamp" => serde_json::json!({"results": [
                {
                    "id": "framed",
                    "thumbnail": "https://x/framed.jpg",
                    "title": "Zardozi embroidery artwork framed on wooden lamp"
                },
                {
                    "id": "exact",
                    "thumbnail": "https://x/lamp.jpg",
                    "title": "The Wooden Lamp",
                    "tags": ["lamp", "light", "wood"]
                },
                {
                    "id": "rope",
                    "thumbnail": "https://x/rope.jpg",
                    "title": "Rope with Wooden Lamp over a dining table"
                }
            ]}),
            unexpected => panic!("unexpected Openverse query: {unexpected}"),
        };
        Some(parse_openverse_results(&json))
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "wooden lamp studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");
    let ids: Vec<&str> = hits.iter().map(|hit| hit.id.as_str()).collect();

    assert_eq!(
        two_keyword_retry("wooden lamp studio photo").as_deref(),
        Some("wooden lamp")
    );
    assert_eq!(ids.first(), Some(&"exact"));
    assert!(ids.contains(&"framed"));
    assert!(
        !ids.contains(&"rope"),
        "the original studio-photo intent must reject dining-room retry noise"
    );
}

#[test]
fn explicit_toy_query_keeps_toy_results_but_product_query_rejects_them() {
    let toy = RawHit {
        id: "toy".to_string(),
        thumb_url: "https://x/toy.jpg".to_string(),
        attribution: String::new(),
        title: "Handmade plush toy".to_string(),
        relevance_metadata: "Handmade plush teddy toy stuffed doll".to_string(),
    };

    assert_eq!(
        retain_relevant_hits(vec![toy], "plush toy studio photo").len(),
        1,
        "an explicit toy request opts into the toy result group"
    );

    let toy = RawHit {
        id: "toy".to_string(),
        thumb_url: "https://x/toy.jpg".to_string(),
        attribution: String::new(),
        title: "Wooden lamp toy".to_string(),
        relevance_metadata: "Wooden lamp plush teddy toy".to_string(),
    };
    assert!(retain_relevant_hits(vec![toy], "wooden lamp").is_empty());
}

#[test]
fn wikimedia_filtered_primary_retries_concrete_subject() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "wooden lamp studio photo" => vec![RawHit {
                    id: "montage".to_string(),
                    thumb_url: "https://x/montage.jpg".to_string(),
                    attribution: String::new(),
                    title: "Wooden lamp montage".to_string(),
                    relevance_metadata: "Wooden lamp montage collage".to_string(),
                }],
                "wooden lamp" => vec![
                    RawHit {
                        id: "dining-room".to_string(),
                        thumb_url: "https://x/dining-room.jpg".to_string(),
                        attribution: String::new(),
                        title: "Wooden lamp over a dining table".to_string(),
                        relevance_metadata: "Wooden lamp in dining room interior".to_string(),
                    },
                    RawHit {
                        id: "lamp".to_string(),
                        thumb_url: "https://x/lamp.jpg".to_string(),
                        attribution: String::new(),
                        title: "The Wooden Lamp".to_string(),
                        relevance_metadata: "The Wooden Lamp".to_string(),
                    },
                ],
                unexpected => panic!("unexpected Wikimedia query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_wikimedia_list_with(
        "wooden lamp studio photo",
        fetcher,
    ));

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["wooden lamp studio photo", "wooden lamp"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "lamp");
}

#[test]
fn zero_openverse_results_use_the_same_single_retry_path() {
    let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let fetch_calls = calls.clone();
    let fetcher = move |query: String| {
        fetch_calls.lock().expect("calls lock").push(query.clone());
        async move {
            match query.as_str() {
                "ceramic vase studio photo" => Some(Vec::new()),
                "ceramic vase" => Some(vec![
                    RawHit {
                        id: "hotel".to_string(),
                        thumb_url: "https://x/hotel.jpg".to_string(),
                        attribution: String::new(),
                        title: "Ceramic vase in a hotel lounge".to_string(),
                        relevance_metadata: "Ceramic vase in a hotel lounge interior".to_string(),
                    },
                    RawHit {
                        id: "catalog".to_string(),
                        thumb_url: "https://x/catalog.jpg".to_string(),
                        attribution: String::new(),
                        title: "Ceramic vase catalogue illustration".to_string(),
                        relevance_metadata: "Ceramic vase catalogue illustration".to_string(),
                    },
                    RawHit {
                        id: "vase".to_string(),
                        thumb_url: "https://x/vase.jpg".to_string(),
                        attribution: String::new(),
                        title: "Ceramic vase isolated on white".to_string(),
                        relevance_metadata: "Ceramic vase isolated on white".to_string(),
                    },
                ]),
                unexpected => panic!("unexpected Openverse query: {unexpected}"),
            }
        }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "ceramic vase studio photo",
        fetcher,
    ))
    .expect("provider requests succeed");

    assert_eq!(
        calls.lock().expect("calls lock").as_slice(),
        ["ceramic vase studio photo", "ceramic vase"]
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "vase");
}

#[test]
fn openverse_network_failure_falls_through_without_retrying() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |_query: String| {
        fetch_calls.fetch_add(1, Ordering::SeqCst);
        async move { None }
    };

    let hits = crate::net::block_on_image_runtime(fetch_relevant_openverse_list_with(
        "ceramic vase studio photo",
        fetcher,
    ));

    assert!(
        hits.is_none(),
        "request failure must retain fallback semantics"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "network failure must not trigger another Openverse request"
    );
}

#[test]
fn non_photo_query_keeps_relevant_illustrations() {
    let hits = vec![RawHit {
        id: "drawing".to_string(),
        thumb_url: "https://x/drawing.jpg".to_string(),
        attribution: String::new(),
        title: "Ceramic tableware illustration".to_string(),
        relevance_metadata: "Ceramic tableware illustration".to_string(),
    }];

    let ranked = retain_relevant_hits(hits, "ceramic tableware illustration");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "drawing");
}

#[test]
fn descriptor_only_studio_photo_query_preserves_unclassified_hits() {
    let hits = vec![RawHit {
        id: "unclassified".to_string(),
        thumb_url: "https://x/unclassified.jpg".to_string(),
        attribution: String::new(),
        title: String::new(),
        relevance_metadata: String::new(),
    }];

    let ranked = retain_relevant_hits(hits, "studio product photo");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "unclassified");
}

#[test]
fn descriptor_only_isolated_query_still_requires_positive_evidence() {
    let hits = [
        ("unclassified", ""),
        ("isolated", "Product cutout on a white background"),
    ]
    .into_iter()
    .map(|(id, metadata)| RawHit {
        id: id.to_string(),
        thumb_url: format!("https://x/{id}.jpg"),
        attribution: String::new(),
        title: metadata.to_string(),
        relevance_metadata: metadata.to_string(),
    })
    .collect();

    let ranked = retain_relevant_hits(hits, "isolated studio product photo");

    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "isolated");
}

#[test]
fn failed_relevant_thumbnail_never_falls_through_to_downloadable_holiday_hit() {
    let hits = vec![
        RawHit {
            id: "chair".to_string(),
            thumb_url: "https://x/chair.jpg".to_string(),
            attribution: "By Chair".to_string(),
            title: "Modern armchair".to_string(),
            relevance_metadata: "Modern armchair".to_string(),
        },
        RawHit {
            id: "holiday".to_string(),
            thumb_url: "https://x/holiday.jpg".to_string(),
            attribution: "By Holiday".to_string(),
            title: "Christmas technology celebration".to_string(),
            relevance_metadata: "Christmas technology celebration".to_string(),
        },
    ];
    let relevant = retain_relevant_hits(hits, "warm modern armchair");
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |url: String| {
        let fetch_calls = fetch_calls.clone();
        async move {
            fetch_calls.fetch_add(1, Ordering::SeqCst);
            (url == "https://x/holiday.jpg").then(|| "data:image/jpeg;base64,HOLIDAY".to_string())
        }
    };

    let result = crate::net::block_on_image_runtime(materialize_first_thumb(relevant, &fetcher));

    assert!(
        result.is_none(),
        "unrelated downloadable hit must be fenced out"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1, "only armchair was tried");
}

#[test]
fn parse_wikimedia_results_maps_thumburl_and_license() {
    let json = serde_json::json!({
        "query": {"pages": {
            "1": {"pageid": 1, "title": "File:Forest armchair.jpg", "imageinfo": [{
                "thumburl": "https://c/w1.jpg",
                "extmetadata": {"LicenseShortName": {"value": "CC BY-SA 4.0"}}
            }]},
            "2": {"pageid": 2, "title": "File:Mountain lake.jpg", "imageinfo": [{"url": "https://c/w2.jpg"}]},
            "3": {"pageid": 3}
        }}
    });
    let mut hits = parse_wikimedia_results(&json);
    hits.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].thumb_url, "https://c/w1.jpg");
    assert_eq!(hits[0].attribution, "CC BY-SA 4.0");
    assert_eq!(hits[0].relevance_metadata, "File:Forest armchair.jpg");
    assert_eq!(hits[1].thumb_url, "https://c/w2.jpg");
}

#[test]
fn parse_wikimedia_results_keeps_a_twenty_hit_candidate_pool() {
    let mut pages = serde_json::Map::new();
    for index in 0..(SEARCH_CANDIDATE_COUNT + 4) {
        pages.insert(
            index.to_string(),
            serde_json::json!({
                "pageid": index,
                "title": format!("File:Ceramic vase {index}.jpg"),
                "imageinfo": [{"thumburl": format!("https://x/vase-{index}.jpg")}]
            }),
        );
    }
    let json = serde_json::json!({"query": {"pages": pages}});

    assert_eq!(parse_wikimedia_results(&json).len(), SEARCH_CANDIDATE_COUNT);
}

#[test]
fn parse_wikimedia_results_rejects_pdf_page_thumbnails() {
    let json = serde_json::json!({
        "query": {"pages": {
            "1": {
                "pageid": 1,
                "title": "File:Technical Wool Conference (IA report).pdf",
                "imageinfo": [{
                    "mime": "application/pdf",
                    "thumburl": "https://upload.wikimedia.org/report.pdf/page1-report.pdf.jpg"
                }]
            },
            "2": {
                "pageid": 2,
                "title": "File:Knitted wool sweater.jpg",
                "imageinfo": [{
                    "mime": "image/jpeg",
                    "thumburl": "https://upload.wikimedia.org/sweater.jpg"
                }]
            },
            "3": {
                "pageid": 3,
                "title": "File:Historic wool bulletin.pdf",
                "imageinfo": [{
                    "thumburl": "https://upload.wikimedia.org/bulletin.pdf/page1.jpg"
                }]
            }
        }}
    });

    let hits = parse_wikimedia_results(&json);

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "2");
    assert_eq!(
        hits[0].thumb_url,
        "https://upload.wikimedia.org/sweater.jpg"
    );

    let pdf_page = &json["query"]["pages"]["1"];
    assert!(
        crate::net::fetch::wikimedia_image_candidates(pdf_page).is_empty(),
        "the shared desktop provider must reject the same PDF thumbnail"
    );
}

#[test]
fn search_outcome_json_shape() {
    let outcome = WebImageSearchOutcome {
        results: vec![WebImageSearchHit {
            id: "a".into(),
            thumb_data_url: "data:image/png;base64,AA==".into(),
            attribution: "By A".into(),
        }],
        source: Some("openverse"),
    };
    let json: serde_json::Value =
        serde_json::from_str(&search_outcome_to_json(&outcome)).expect("valid json");
    assert_eq!(json["ok"], true);
    assert_eq!(json["source"], "openverse");
    assert_eq!(json["results"][0]["id"], "a");
    assert_eq!(
        json["results"][0]["thumb_data_url"],
        "data:image/png;base64,AA=="
    );
    let empty = WebImageSearchOutcome {
        results: Vec::new(),
        source: None,
    };
    let json: serde_json::Value =
        serde_json::from_str(&search_outcome_to_json(&empty)).expect("valid json");
    assert!(json["source"].is_null());
}

#[test]
fn image_job_slot_caps_concurrency_and_releases_on_drop() {
    let held: Vec<_> = (0..MAX_IN_FLIGHT_IMAGE_JOBS)
        .map(|_| ImageJobSlot::acquire().expect("slot under the cap"))
        .collect();
    assert!(
        ImageJobSlot::acquire().is_none(),
        "cap reached — acquire must fail"
    );
    drop(held);
    assert!(
        ImageJobSlot::acquire().is_some(),
        "drop must release the slots"
    );
}

#[test]
fn sniff_image_mime_recognizes_the_embeddable_formats() {
    assert_eq!(sniff_image_mime(b"\x89PNG\r\n\x1A\nxx"), Some("image/png"));
    assert_eq!(sniff_image_mime(b"\xFF\xD8\xFFxx"), Some("image/jpeg"));
    assert_eq!(sniff_image_mime(b"GIF89a"), Some("image/gif"));
    assert_eq!(
        sniff_image_mime(b"RIFF\0\0\0\0WEBPVP8 "),
        Some("image/webp")
    );
    assert_eq!(sniff_image_mime(b"<svg>"), None);
    assert_eq!(
        normalize_image_mime_header("image/jpg"),
        Some("image/jpeg".into())
    );
    assert_eq!(normalize_image_mime_header("image/svg+xml"), None);
    assert_eq!(normalize_image_mime_header("text/html"), None);
}

#[test]
fn provider_timeout_helper_cancels_the_whole_slow_future() {
    let result = run_with_timeout(Duration::from_millis(20), async {
        tokio::time::sleep(Duration::from_millis(250)).await;
        7_u8
    });

    assert_eq!(
        result, None,
        "the provider ladder must not outlive its budget"
    );
    assert_eq!(
        run_with_timeout(Duration::ZERO, async { 9_u8 }),
        None,
        "an exhausted overall deadline must not start the ladder"
    );
}

#[test]
fn first_thumbnail_materializer_stops_after_first_success() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |url: String| {
        let fetch_calls = fetch_calls.clone();
        async move {
            let call = fetch_calls.fetch_add(1, Ordering::SeqCst);
            (call == 1).then(|| format!("data:image/png;base64,{url}"))
        }
    };
    let hits = ["first", "second", "third"]
        .into_iter()
        .map(|id| RawHit {
            id: id.to_string(),
            thumb_url: id.to_string(),
            attribution: format!("by {id}"),
            title: id.to_string(),
            relevance_metadata: id.to_string(),
        })
        .collect();

    let hit = crate::net::block_on_image_runtime(materialize_first_thumb(hits, &fetcher))
        .expect("the second thumbnail succeeds");

    assert_eq!(hit.id, "second");
    assert_eq!(hit.thumb_data_url, "data:image/png;base64,second");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "third hit must not download"
    );
}

#[test]
fn multi_thumbnail_materializer_caps_public_results_at_five() {
    let calls = Arc::new(AtomicUsize::new(0));
    let fetch_calls = calls.clone();
    let fetcher = move |url: String| {
        let fetch_calls = fetch_calls.clone();
        async move {
            fetch_calls.fetch_add(1, Ordering::SeqCst);
            Some(format!("data:image/png;base64,{url}"))
        }
    };
    let hits = (0..SEARCH_CANDIDATE_COUNT)
        .map(|index| RawHit {
            id: index.to_string(),
            thumb_url: index.to_string(),
            attribution: String::new(),
            title: format!("Product {index}"),
            relevance_metadata: format!("Product {index}"),
        })
        .collect();

    let results = crate::net::block_on_image_runtime(materialize_thumbs(hits, &fetcher));

    assert_eq!(results.len(), SEARCH_RESULT_COUNT);
    assert_eq!(calls.load(Ordering::SeqCst), SEARCH_RESULT_COUNT);
}

#[test]
fn renderable_image_data_url_is_restricted_to_renderer_codecs() {
    use crate::net::fetch::renderable_image_data_url;
    use skia_safe::{surfaces, EncodedImageFormat};
    let mut surface = surfaces::raster_n32_premul((8, 8)).expect("raster surface");
    surface.canvas().clear(skia_safe::Color::BLUE);
    let snapshot = surface.image_snapshot();
    let png = snapshot
        .encode(None, EncodedImageFormat::PNG, 100)
        .expect("encode png")
        .as_bytes()
        .to_vec();

    // PNG within budget embeds untouched.
    let url = renderable_image_data_url("image/png", &png).expect("png embeds");
    assert!(url.starts_with("data:image/png;base64,"));

    // A WebP payload must never survive as image/webp — it is either
    // transcoded to a renderer codec or rejected so the caller can try
    // the next candidate URL. (The encoder may be absent from this skia
    // build; the invariant holds either way.)
    if let Some(webp) = snapshot.encode(None, EncodedImageFormat::WEBP, 100) {
        if let Some(url) = renderable_image_data_url("image/webp", webp.as_bytes()) {
            assert!(
                url.starts_with("data:image/png;base64,")
                    || url.starts_with("data:image/jpeg;base64,"),
                "webp transcodes to a renderer codec, got {}",
                &url[..40.min(url.len())]
            );
        }
    }
    assert!(
        renderable_image_data_url("image/webp", b"RIFF\0\0\0\0WEBPVP8 junk").is_none(),
        "undecodable webp is rejected"
    );
    assert!(
        renderable_image_data_url("image/gif", b"GIF89a junk").is_none(),
        "gif is rejected rather than flattened"
    );
}
