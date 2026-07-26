//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `pt_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Pesquisar imagens…",
        "imagePanel.searching" => "Pesquisando…",
        "imagePanel.noResults" => "Nenhum resultado",
        "imagePanel.searchPrompt" => "Pesquise imagens",
        "imagePanel.sourceNotice" => {
            "Imagens de {{source}}. Licença livre — verifique a licença antes de usar."
        }
        "imagePanel.genNotConfigured" => "Geração de imagens não configurada",
        "imagePanel.openSettings" => "Abrir configurações",
        "imagePanel.promptPlaceholder" => "Descreva a imagem…",
        _ => return None,
    })
}
