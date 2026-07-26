//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `hi_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "छवियां खोजें…",
        "imagePanel.searching" => "खोज रहे हैं…",
        "imagePanel.noResults" => "कोई परिणाम नहीं मिला",
        "imagePanel.searchPrompt" => "छवियां खोजें",
        "imagePanel.sourceNotice" => "{{source}} से छवियां। मुक्त लाइसेंस — उपयोग से पहले लाइसेंस जांचें।",
        "imagePanel.genNotConfigured" => "छवि निर्माण कॉन्फ़िगर नहीं है",
        "imagePanel.openSettings" => "सेटिंग्स खोलें",
        "imagePanel.promptPlaceholder" => "छवि का वर्णन करें…",
        _ => return None,
    })
}
