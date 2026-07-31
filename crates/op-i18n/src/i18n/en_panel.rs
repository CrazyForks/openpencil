//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `en_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Search images...",
        "imagePanel.searching" => "Searching...",
        "imagePanel.noResults" => "No results found",
        "imagePanel.searchPrompt" => "Search for images",
        "imagePanel.sourceNotice" => {
            "Images from {{source}}. Freely licensed — verify license before use."
        }
        "imagePanel.genNotConfigured" => "Image generation not configured",
        "imagePanel.openSettings" => "Open Settings",
        "imagePanel.promptPlaceholder" => "Describe the image...",
        "providerProbe.connectedViaCli" => "Connected via {{name}} CLI",
        "providerProbe.cliExitedWithError" => "{{name}} CLI exited with an error",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI produced no version output",
        "providerProbe.modelQueryFailed" => "{{name}} model query failed or timed out",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} model query failed. Run {{command}} once to authenticate."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} model query requires authentication. Run {{command}} once to sign in."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}} returned an unrecognized model catalog"
        }
        "providerProbe.connectedAs" => "Connected as @{{login}}{{method}}",
        "providerProbe.connectedViaGithub" => "Connected via GitHub",
        "importProgress.figmaTitle" => "Parsing Figma file…",
        "importProgress.htmlTitle" => "Parsing HTML and page resources…",
        "importProgress.htmlSubtitle" => "Loading styles and images. Please wait.",
        "importProgress.largeFileSubtitle" => "Large files take a few seconds. Please wait.",
        "account.signedOutHint" => "Sign in to sync your settings and preferences",
        "code.noUsableCode" => "The AI returned no usable code. Retry or switch AI models.",
        "code.previousResultKept" => "The previous generated result is still available",
        "promptCenter.title" => "Prompt Center",
        "promptCenter.searchPlaceholder" => "Search prompts…",
        "promptCenter.category.all" => "All",
        "promptCenter.category.starter" => "Starter",
        "promptCenter.category.mobileApp" => "Mobile Apps",
        "promptCenter.category.webPage" => "Web Pages",
        "promptCenter.category.dashboard" => "Dashboards",
        "promptCenter.category.component" => "Components",
        "promptCenter.category.modify" => "Modify",
        "promptCenter.category.custom" => "Mine",
        "promptCenter.empty" => "No matching prompts",
        "promptCenter.saveCurrent" => "Save current input",
        "promptCenter.saveTitlePlaceholder" => "Prompt title",
        "promptCenter.save" => "Save",
        "promptCenter.cancel" => "Cancel",
        "promptCenter.delete" => "Delete",
        "promptCenter.screens" => "{{count}} screens",
        "promptCenter.freeform" => "Freeform",
        "promptCenter.item.wander.title" => "Wander · Travel Itinerary",
        "promptCenter.item.forage.title" => "Forage · Seasonal Recipes",
        "promptCenter.item.still.title" => "Still · Meditation & Sleep",
        "promptCenter.item.hearth.title" => "Hearth · Smart Home",
        "promptCenter.item.meteo.title" => "Meteo · Immersive Weather",
        "promptCenter.item.marginalia.title" => "Marginalia · Reading & Annotation",
        "promptCenter.item.lingua.title" => "Lingua · Language Learning",
        "promptCenter.item.daybreak.title" => "Daybreak · Coffee Ordering",
        "promptCenter.item.verdant.title" => "Verdant · Plant Care",
        "promptCenter.item.companion.title" => "Companion · Pet Life",
        "promptCenter.item.relic.title" => "Relic · Curated Resale",
        "promptCenter.item.nocturne.title" => "Nocturne · Stargazing Guide",
        "promptCenter.item.marquee.title" => "Marquee · Movie Watchlist",
        "promptCenter.item.ritual.title" => "Ritual · Habit Building",
        "promptCenter.item.ember.title" => "Ember · Mood Journal",
        "promptCenter.item.volt.title" => "Volt · EV Companion",
        "promptCenter.item.aloft.title" => "Aloft · Flight Tracking",
        "promptCenter.item.gallery.title" => "Gallery · Exhibitions & Culture",
        "promptCenter.item.nightcap.title" => "Nightcap · Home Bartending",
        "promptCenter.item.bloom.title" => "Bloom · Family Growth Tracker",
        "promptCenter.item.extremeWeather.title" => "Extreme · Weather App",
        "promptCenter.item.extremeNowPlaying.title" => "Extreme · Now Playing",
        "promptCenter.item.extremeDailyApp.title" => "Extreme · Everyday App",
        "promptCenter.item.extremeCalendar.title" => "Extreme · Calendar",
        "promptCenter.item.extremeCalm.title" => "Extreme · Calm",
        "promptCenter.item.webOrbit.title" => "Orbit · AI Workbench Landing Page",
        "promptCenter.item.webAtelier.title" => "Atelier · Furniture Commerce",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Growth Analytics",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Logistics Operations",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Enterprise Data Table",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Form System",
        "promptCenter.item.modifyPolishCurrent.title" => "Polish the Current Screen",
        "promptCenter.item.modifyCompleteStates.title" => "Complete Component States",
        _ => return super::en_collab::lookup(key),
    })
}
