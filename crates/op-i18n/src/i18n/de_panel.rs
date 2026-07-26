//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `de_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Bilder suchen…",
        "imagePanel.searching" => "Suche läuft…",
        "imagePanel.noResults" => "Keine Ergebnisse",
        "imagePanel.searchPrompt" => "Nach Bildern suchen",
        "imagePanel.sourceNotice" => {
            "Bilder von {{source}}. Frei lizenziert — Lizenz vor Verwendung prüfen."
        }
        "imagePanel.genNotConfigured" => "Bildgenerierung nicht konfiguriert",
        "imagePanel.openSettings" => "Einstellungen öffnen",
        "imagePanel.promptPlaceholder" => "Beschreibe das Bild…",
        _ => return None,
    })
}
