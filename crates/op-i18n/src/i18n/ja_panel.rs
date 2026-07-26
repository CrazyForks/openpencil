//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `ja_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "画像を検索…",
        "imagePanel.searching" => "検索中…",
        "imagePanel.noResults" => "結果が見つかりません",
        "imagePanel.searchPrompt" => "画像を検索",
        "imagePanel.sourceNotice" => {
            "画像の提供元: {{source}}。自由ライセンス — 使用前にライセンスをご確認ください。"
        }
        "imagePanel.genNotConfigured" => "画像生成が未設定です",
        "imagePanel.openSettings" => "設定を開く",
        "imagePanel.promptPlaceholder" => "画像の内容を入力…",
        _ => return None,
    })
}
