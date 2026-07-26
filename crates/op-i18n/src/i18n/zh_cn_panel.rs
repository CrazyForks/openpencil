//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `zh_cn_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

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
        "providerProbe.connectedViaCli" => "已通过 {{name}} CLI 连接",
        "providerProbe.cliExitedWithError" => "{{name}} CLI 退出并报错",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI 未输出版本信息",
        "providerProbe.modelQueryFailed" => "{{name}} 模型查询失败或超时",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 模型查询失败。请先运行 {{command}} 完成认证。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 模型查询需要认证。请先运行 {{command}} 登录。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} 返回了无法识别的模型列表",
        _ => return None,
    })
}
