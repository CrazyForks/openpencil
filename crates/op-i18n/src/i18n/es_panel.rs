//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `es_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Buscar imágenes…",
        "imagePanel.searching" => "Buscando…",
        "imagePanel.noResults" => "Sin resultados",
        "imagePanel.searchPrompt" => "Busca imágenes",
        "imagePanel.sourceNotice" => {
            "Imágenes de {{source}}. Licencia libre — verifica la licencia antes de usar."
        }
        "imagePanel.genNotConfigured" => "La generación de imágenes no está configurada",
        "imagePanel.openSettings" => "Abrir ajustes",
        "imagePanel.promptPlaceholder" => "Describe la imagen…",
        _ => return None,
    })
}
