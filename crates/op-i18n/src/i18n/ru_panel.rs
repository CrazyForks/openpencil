//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `ru_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Поиск изображений…",
        "imagePanel.searching" => "Поиск…",
        "imagePanel.noResults" => "Ничего не найдено",
        "imagePanel.searchPrompt" => "Найдите изображения",
        "imagePanel.sourceNotice" => "Изображения из {{source}}. Свободная лицензия — проверьте лицензию перед использованием.",
        "imagePanel.genNotConfigured" => "Генерация изображений не настроена",
        "imagePanel.openSettings" => "Открыть настройки",
        "imagePanel.promptPlaceholder" => "Опишите изображение…",
        _ => return None,
    })
}
