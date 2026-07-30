//! Injectable system-clipboard snapshot used by desktop paste routing.

/// Every clipboard flavor relevant to Cmd/Ctrl+V.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ClipboardPayload {
    pub(crate) text: Option<String>,
    pub(crate) html: Option<String>,
    pub(crate) image: Option<crate::clipboard::ClipboardImage>,
}

impl ClipboardPayload {
    pub(super) fn read_system() -> Self {
        let (text, html, image) = crate::clipboard::read_paste_flavours();
        Self { text, html, image }
    }
}
