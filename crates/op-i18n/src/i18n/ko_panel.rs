//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `ko_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "이미지 검색…",
        "imagePanel.searching" => "검색 중…",
        "imagePanel.noResults" => "결과가 없습니다",
        "imagePanel.searchPrompt" => "이미지를 검색하세요",
        "imagePanel.sourceNotice" => {
            "{{source}} 제공 이미지. 자유 라이선스 — 사용 전에 라이선스를 확인하세요."
        }
        "imagePanel.genNotConfigured" => "이미지 생성이 설정되지 않았습니다",
        "imagePanel.openSettings" => "설정 열기",
        "imagePanel.promptPlaceholder" => "이미지를 설명하세요…",
        _ => return None,
    })
}
