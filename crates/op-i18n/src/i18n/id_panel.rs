//! Image-panel popover strings for this locale.
//!
//! Overflow shard: the main table sits at the repo's 800-line
//! file cap, so `id_git` falls through here for the
//! `imagePanel.*` keys.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Cari gambar…",
        "imagePanel.searching" => "Mencari…",
        "imagePanel.noResults" => "Tidak ada hasil",
        "imagePanel.searchPrompt" => "Cari gambar",
        "imagePanel.sourceNotice" => {
            "Gambar dari {{source}}. Berlisensi bebas — periksa lisensi sebelum digunakan."
        }
        "imagePanel.genNotConfigured" => "Pembuatan gambar belum dikonfigurasi",
        "imagePanel.openSettings" => "Buka Pengaturan",
        "imagePanel.promptPlaceholder" => "Deskripsikan gambar…",
        _ => return None,
    })
}
