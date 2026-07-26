//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `fr_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Rechercher des images…",
        "imagePanel.searching" => "Recherche…",
        "imagePanel.noResults" => "Aucun résultat",
        "imagePanel.searchPrompt" => "Recherchez des images",
        "imagePanel.sourceNotice" => {
            "Images de {{source}}. Licence libre — vérifiez la licence avant utilisation."
        }
        "imagePanel.genNotConfigured" => "Génération d'images non configurée",
        "imagePanel.openSettings" => "Ouvrir les réglages",
        "imagePanel.promptPlaceholder" => "Décrivez l'image…",
        _ => return None,
    })
}
