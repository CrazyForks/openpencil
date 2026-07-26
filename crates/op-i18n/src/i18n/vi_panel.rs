//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `vi_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Tìm hình ảnh…",
        "imagePanel.searching" => "Đang tìm…",
        "imagePanel.noResults" => "Không có kết quả",
        "imagePanel.searchPrompt" => "Tìm kiếm hình ảnh",
        "imagePanel.sourceNotice" => {
            "Hình ảnh từ {{source}}. Giấy phép tự do — hãy kiểm tra giấy phép trước khi dùng."
        }
        "imagePanel.genNotConfigured" => "Chưa cấu hình tạo hình ảnh",
        "imagePanel.openSettings" => "Mở cài đặt",
        "imagePanel.promptPlaceholder" => "Mô tả hình ảnh…",
        _ => return None,
    })
}
