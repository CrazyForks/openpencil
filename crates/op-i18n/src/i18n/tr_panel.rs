//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `tr_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Görsel ara…",
        "imagePanel.searching" => "Aranıyor…",
        "imagePanel.noResults" => "Sonuç bulunamadı",
        "imagePanel.searchPrompt" => "Görsel arayın",
        "imagePanel.sourceNotice" => "Görseller {{source}} kaynağından. Özgür lisanslı — kullanmadan önce lisansı doğrulayın.",
        "imagePanel.genNotConfigured" => "Görsel oluşturma yapılandırılmamış",
        "imagePanel.openSettings" => "Ayarları Aç",
        "imagePanel.promptPlaceholder" => "Görseli tanımlayın…",
        _ => return None,
    })
}
