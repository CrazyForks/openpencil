//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `zh_tw_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜尋圖片…",
        "imagePanel.searching" => "搜尋中…",
        "imagePanel.noResults" => "未找到結果",
        "imagePanel.searchPrompt" => "搜尋圖片",
        "imagePanel.sourceNotice" => "圖片來自 {{source}}。自由授權 — 使用前請確認授權條款。",
        "imagePanel.genNotConfigured" => "圖片生成尚未設定",
        "imagePanel.openSettings" => "開啟設定",
        "imagePanel.promptPlaceholder" => "描述要生成的圖片…",
        _ => return None,
    })
}
