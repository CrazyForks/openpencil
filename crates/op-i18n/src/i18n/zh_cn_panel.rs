//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `zh_cn_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜索图片…",
        "imagePanel.searching" => "搜索中…",
        "imagePanel.noResults" => "未找到结果",
        "imagePanel.searchPrompt" => "搜索图片",
        "imagePanel.sourceNotice" => "图片来自 {{source}}。自由许可 — 使用前请核实许可协议。",
        "imagePanel.genNotConfigured" => "图片生成未配置",
        "imagePanel.openSettings" => "打开设置",
        "imagePanel.promptPlaceholder" => "描述要生成的图片…",
        _ => return None,
    })
}
