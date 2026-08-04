//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `de_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

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
        "providerProbe.connectedViaCli" => "Über {{name}}-CLI verbunden",
        "providerProbe.cliExitedWithError" => "{{name}}-CLI wurde mit einem Fehler beendet",
        "providerProbe.cliNoVersionOutput" => "{{name}}-CLI hat keine Versionsausgabe geliefert",
        "providerProbe.modelQueryFailed" => "Modellabfrage für {{name}} fehlgeschlagen oder abgelaufen",
        "providerProbe.modelQueryFailedRunLogin" => "Modellabfrage für {{name}} fehlgeschlagen. Führe {{command}} einmal aus, um dich zu authentifizieren.",
        "providerProbe.modelQueryNeedsAuth" => "Die Modellabfrage für {{name}} erfordert eine Authentifizierung. Führe {{command}} einmal aus, um dich anzumelden.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} hat einen unbekannten Modellkatalog zurückgegeben",
        "promptCenter.title" => "Prompt-Bibliothek",
        "promptCenter.searchPlaceholder" => "Prompts durchsuchen…",
        "promptCenter.category.all" => "Alle",
        "promptCenter.category.starter" => "Schnellstart",
        "promptCenter.category.mobileApp" => "Mobile App",
        "promptCenter.category.webPage" => "Webseite",
        "promptCenter.category.dashboard" => "Dashboard",
        "promptCenter.category.component" => "Komponente",
        "promptCenter.category.modify" => "Überarbeiten",
        "promptCenter.category.custom" => "Meine",
        "promptCenter.empty" => "Keine passenden Prompts gefunden",
        "promptCenter.saveCurrent" => "Aktuelle Eingabe als Prompt speichern",
        "promptCenter.saveTitlePlaceholder" => "Titel des Prompts",
        "promptCenter.save" => "Speichern",
        "promptCenter.cancel" => "Abbrechen",
        "promptCenter.delete" => "Löschen",
        "promptCenter.screens" => "{{count}} Screens",
        "promptCenter.freeform" => "Freie Form",
        "promptCenter.item.wander.title" => "Wander · Reiseplanung",
        "promptCenter.item.forage.title" => "Forage · Saisonale Rezepte",
        "promptCenter.item.still.title" => "Still · Meditation und Einschlafen",
        "promptCenter.item.hearth.title" => "Hearth · Smart Home",
        "promptCenter.item.meteo.title" => "Meteo · Immersives Wetter",
        "promptCenter.item.marginalia.title" => "Marginalia · Lesen und Anmerkungen",
        "promptCenter.item.lingua.title" => "Lingua · Sprachen lernen",
        "promptCenter.item.daybreak.title" => "Daybreak · Kaffee bestellen",
        "promptCenter.item.verdant.title" => "Verdant · Pflanzenpflege",
        "promptCenter.item.companion.title" => "Companion · Leben mit Haustieren",
        "promptCenter.item.relic.title" => "Relic · Kuratierter Secondhand-Markt",
        "promptCenter.item.nocturne.title" => "Nocturne · Sternbeobachtung",
        "promptCenter.item.marquee.title" => "Marquee · Film-Merkliste",
        "promptCenter.item.ritual.title" => "Ritual · Gewohnheiten aufbauen",
        "promptCenter.item.ember.title" => "Ember · Stimmungstagebuch",
        "promptCenter.item.volt.title" => "Volt · Elektroauto-Begleiter",
        "promptCenter.item.aloft.title" => "Aloft · Flugverfolgung",
        "promptCenter.item.gallery.title" => "Gallery · Ausstellungen und Kultur",
        "promptCenter.item.nightcap.title" => "Nightcap · Cocktails zu Hause",
        "promptCenter.item.bloom.title" => "Bloom · Familienmomente und Entwicklung",
        "promptCenter.item.extremeWeather.title" => "Extrem · Wetter-App",
        "promptCenter.item.extremeNowPlaying.title" => "Extrem · Aktueller Titel",
        "promptCenter.item.extremeDailyApp.title" => "Extrem · Jeden Tag öffnen",
        "promptCenter.item.extremeCalendar.title" => "Extrem · Kalender neu erfinden",
        "promptCenter.item.extremeCalm.title" => "Extrem · Ein Screen voller Ruhe",
        "promptCenter.item.webOrbit.title" => "Orbit · Landingpage für den KI-Arbeitsbereich",
        "promptCenter.item.webAtelier.title" => "Atelier · Möbel-E-Commerce",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Wachstumsanalyse-Dashboard",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Logistikbetrieb",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Unternehmens-Datentabelle",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Formular-Komponentensystem",
        "promptCenter.item.modifyPolishCurrent.title" => "Aktuellen Screen verfeinern",
        "promptCenter.item.modifyCompleteStates.title" => "Komponentenzustände vervollständigen",
        "collab.ownerConfirm.title" => "Bestätige, wem du beitrittst",
        "collab.ownerConfirm.hint" => "Aus dieser Sitzung wurde noch nichts geladen.",
        "collab.ownerConfirm.account" => "Verifiziertes Konto",
        "collab.ownerConfirm.device" => "Verifiziertes Gerät",
        "collab.ownerConfirm.claimedName" => "Von diesem Konto gewählter Name (nicht verifiziert)",
        "collab.action.confirmOwner" => "Dieser Sitzung beitreten",
        "collab.action.rejectOwner" => "Nicht beitreten",
        "collab.error.ownerNotConfirmed" => "Du hast den Host nicht bestätigt, daher wurde nichts geladen.",
        "sceneTemplate.title" => "Szenenvorlagen",
        "sceneTemplate.searchPlaceholder" => "Szenen oder Vorlagen suchen…",
        "sceneTemplate.empty" => "Keine passenden Vorlagen gefunden",
        "sceneTemplate.frames" => "Seiten: {{count}}",
        "sceneTemplate.generate.placeholder" => "Thema beschreiben – die KI erzeugt die ganze Präsentation",
        "sceneTemplate.generate.button" => "Erzeugen",
        "sceneTemplate.generate.hint" => "Ein neues Dokument, aus deinem Thema als vollständige Präsentation erzeugt.",
        "sceneTemplate.generate.promptTemplate" => "Erstelle eine Präsentation (PPT) zum folgenden Thema: {{topic}}",
        "sceneTemplate.filter.all" => "Alle",
        "sceneTemplate.scene.tutorial" => "Tutorials",
        "sceneTemplate.scene.comparison" => "Vergleich",
        "sceneTemplate.scene.carousel" => "Karussell",
        "sceneTemplate.scene.slides" => "Folien",
        "sceneTemplate.scene.card" => "Karten",
        "sceneTemplate.item.screenshotTutorial.title" => "Screenshot-Tutorial · 3 Schritte",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Cover, drei Anleitungsschritte und ein abschließender Call-to-Action. Screenshots und Texte ersetzen – fertig zur Veröffentlichung."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Wissens- und Insights-Karussell",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Cover, drei Kernpunkte und eine Zusammenfassung – ideal, um einen Gedanken in wischbare Karten aufzuteilen."
        }
        "sceneTemplate.item.beforeAfter.title" => "Redesign-Vergleich: Vorher/Nachher",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Vorher und Nachher nebeneinander mit Hinweisen zu den Änderungen – ideal für Retrospektiven und Portfolios."
        }
        "sceneTemplate.item.slideDeck.title" => "Präsentation · 6 Folien",
        "sceneTemplate.item.slideDeck.summary" => {
            "Cover, Agenda, Kernpunkte, Daten, Diagramm und Abschluss im 16:9-Format. Texte ersetzen und präsentieren."
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "Wissenskarte · Hochformat",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Eine einzelne 3:4-Karte mit Überschrift, vier Kernpunkten und Signaturzeile. Texte ersetzen und posten.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Wissenskarte · Quadratisch",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Eine 1:1-Karte im gleichen Layout, kompakt genug für ein Beitragsbild oder einen Social-Post.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch-Deck · Dunkel",
        "sceneTemplate.item.pitchDeckDark.summary" => "Titel, Problem, Lösung, Zahlen, Roadmap und Kontaktseite. Große Schrift auf dunklem Grund, gebaut für Finanzierungsrunden und Launches.",
        "sceneTemplate.item.lectureDeckLight.title" => "Vorlesungsfolien · Hell",
        "sceneTemplate.item.lectureDeckLight.summary" => "Kursdeckblatt, Lernziele, Konzepterklärung, Rechenbeispiel, Vergleichstabelle und Zusammenfassung. Papierweiß, auch über eine ganze Stunde angenehm.",
        "sceneTemplate.item.minimalKeynote.title" => "Minimalistische Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => "Viel Weißraum, überdimensionale Schrift, ein Gedanke pro Seite — acht Seiten ganz ohne Karten. Für Launches und Keynotes.",
        "sceneTemplate.item.gradientTech.title" => "Gradient Tech",
        "sceneTemplate.item.gradientTech.summary" => "Dunkler Verlauf mit Milchglaskarten: Architektur, Benchmarks und eine Kundenwand. Für Entwickler-Produktlaunches.",
        "fileMenu.newFromTemplate" => "Neu aus Vorlage",
        "fileMenu.exportSlideshowHtml" => "Diashow als HTML exportieren...",
        "fileMenu.exportPptx" => "Als PowerPoint exportieren...",
        "dialog.slideshowHtmlTitle" => "Diashow exportieren",
        "dialog.slideshowHtmlSummary" => "{{count}} Folien exportiert nach:",
        "dialog.slideshowHtmlEmpty" => "Diese Präsentation hat keine sichtbaren Folien zum Exportieren.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Importierbare HTML-Inhalte sind nicht verfügbar.",
        "htmlImport.warn.content.empty_body" => {
            "Importierbare Inhalte im HTML-Rumpf sind nicht verfügbar."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML, das tiefer als {{max_depth}} Ebenen verschachtelt ist, wurde verworfen."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Knotenlimit erreicht; der restliche Seiteninhalt wurde ausgelassen."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Knotenlimit erreicht; ein Teil des HTML-Baums wurde ausgelassen."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Knotenlimit erreicht; eine Zeile für Inline-Formatierung wurde ausgelassen."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Knotenlimit erreicht; erzeugte Pseudo-Elemente wurden ausgelassen."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "CSS-Regeln, die tiefer als {{max_depth}} At-Regeln verschachtelt sind, wurden ignoriert."
        }
        "htmlImport.warn.css.unterminated_rule" => {
            "Eine nicht abgeschlossene CSS-Regel wurde ignoriert."
        }
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "CSS-::marker-Regeln wurden nicht importiert."
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "Verschachtelte CSS-Stilregeln wurden ignoriert."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "Der ungültige @layer-Name '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "Die nicht unterstützte @{{name}}-Anweisung wurde ignoriert."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "@media-Regeln ohne Anzeigebereich wurden ignoriert."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Der ungültige @layer-Blockname '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.unsupported_container_block" => {
            "Der @container-Block wurde ignoriert."
        }
        "htmlImport.warn.css.unsupported_block" => {
            "Der nicht unterstützte @{{name}}-Block wurde ignoriert."
        }
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "Die @font-face-Webschrift '{{family}}' ist nicht verfügbar."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Prozentuale Versätze eines absolut positionierten Elements wurden angenähert."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Prozentuale position:relative-Versätze wurden angenähert."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "CSS-aspect-ratio ohne festgelegte Achse wurde ignoriert."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "CSS-aspect-ratio in einem unbestimmten enthaltenden Block wurde ignoriert."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS-position:sticky wurde ignoriert.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Nicht unterstützte CSS-Rasterspuren wurden angenähert."
        }
        "htmlImport.warn.layout.float_ignored" => "CSS-float wurde ignoriert.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "CSS-mix-blend-mode auf Knotenebene wurde angenähert."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS-overflow: auto / scroll wurde angenähert."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "Negative CSS-Außenabstände wurden ignoriert."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "CSS-Außenabstände an einem sichtbaren Kasten wurden ignoriert."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Prozentuale Größenberechnung mit content-box wurde angenähert."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Durch ausdrückliche Startlinien entstandene leere CSS-Rasterzellen wurden angenähert."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Ein CSS-Rasterelement, dessen Bereich nicht zu seiner Startlinie passte, wurde angenähert."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Knotenlimit erreicht; Zeilencontainer des CSS-Rasters wurden ausgelassen."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "CSS-Rasterspurbreiten mit auto-fit / auto-fill wurden angenähert."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "Die Platzierung per CSS-grid-template-areas wurde nicht importiert."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "Die Platzierung per CSS-grid-row wurde nicht importiert."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS-grid-column `{{value}}` wurde angenähert."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Automatische CSS-Außenabstände in der Blockachse wurden nicht importiert."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Knotenlimit erreicht; die Ausrichtung über automatische CSS-Außenabstände wurde ausgelassen."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Ein CSS-Versatz im Fluss an einem Element ohne festgelegte Größe wurde verworfen."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Knotenlimit erreicht; ein CSS-Versatz im Fluss wurde ausgelassen."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS-Versätze im Fluss (position:relative-Abstände, transform-Verschiebung) wurden angenähert."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Ein CSS-Versatz im Fluss an einem Kasten, der keinen Versatzcontainer aufnehmen kann, wurde verworfen."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap an einem spaltenweisen Flex-Container wurde nicht importiert."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse wurde angenähert."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap an einem Container ohne festgelegte Breite wurde ignoriert."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "CSS-align-content an einem umbrechenden Flex-Container wurde nicht importiert."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap mit unbestimmten Hauptachsengrößen der Kindelemente wurde ignoriert."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Knotenlimit erreicht; flex-wrap-Zeilen wurden ausgelassen."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Nicht unterstützte CSS-transform-Syntax wurde ignoriert."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Nicht unterstützte CSS-transform-Funktionen (3D, matrix3d) wurden ignoriert."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Eine prozentuale CSS-transform-Verschiebung auf einer unbestimmten Achse wurde verworfen."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Eine CSS-Transformation, die eine nicht endliche Matrix ergab, wurde ignoriert."
        }
        "htmlImport.warn.transform.skew_dropped" => "CSS-transform-Scherung wurde verworfen.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Eine CSS-Transformation mit Skalierung null oder nicht endlichem Wert wurde angenähert."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "CSS-transform-Spiegelung wurde angenähert."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Der Z-Versatz von CSS-transform-origin wurde ignoriert."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Eine CSS-transform-Skalierung, die nicht in die Knotengröße übernommen werden konnte, wurde verworfen."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "In die Knotengröße übernommene CSS-transform-Skalierung wurde angenähert."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "CSS-transform-Skalierung an einem automatisch dimensionierten Element wurde ignoriert."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "Gerichtetes oder verteiltes CSS-background-repeat wurde angenähert."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Eine ausdrückliche CSS-Kachelgröße des Hintergrunds wurde ignoriert."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "CSS-background-size an einem automatisch dimensionierten Element wurde angenähert."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "CSS-background-size, das die eigene Bildgröße benötigt, wurde angenähert."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Eine nicht unterstützte CSS-background-position wurde ignoriert."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Eine leere URL eines CSS-Hintergrundbilds wurde ignoriert."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Konische CSS-Verläufe wurden ignoriert."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Eine nicht unterstützte CSS-background-image-Ebene wurde ignoriert."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Eine nicht aufgelöste CSS-Hintergrundfarbe wurde ignoriert."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS-background-position wurde ignoriert."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Seitenweise CSS-Rahmenfarben wurden angenähert."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Gemischte seitenweise CSS-Rahmenstile wurden angenähert."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Ein komplexer CSS-Rahmenstil wurde angenähert."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Ein nicht unterstützter CSS-Rahmenstil wurde angenähert."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Elliptische CSS-Eckenradien wurden angenähert."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Ein nicht unterstützter CSS-Eckenradius wurde ignoriert."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Eine nicht unterstützte CSS-box-shadow-Ebene wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "Die Farbinterpolationsmethode des CSS-Verlaufs wurde ignoriert."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Eine nicht unterstützte Richtung von CSS-linear-gradient wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Farbhinweise in CSS-Verläufen wurden ignoriert."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Ein nicht unterstützter Farbstopp eines CSS-Verlaufs wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Ein CSS-Verlauf mit weniger als zwei nutzbaren Farbstopps wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Ein sich wiederholender CSS-Verlauf wurde angenähert."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Farbstopps von CSS-Verläufen außerhalb des gültigen Bereichs wurden angenähert."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Ein nicht unterstützter CSS-Weichzeichnungsradius wurde ignoriert."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Ein nicht unterstütztes CSS-filter-drop-shadow() wurde ignoriert."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Eine nicht unterstützte CSS-Filterfunktion wurde ignoriert."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Eine nicht unterstützte CSS-backdrop-filter-Funktion wurde ignoriert."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Ein nicht unterstützter CSS-background-blend-mode wurde ignoriert."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "CSS-mix-blend-mode an einzelnen Füllungen wurde angenähert."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Ein nicht unterstützter CSS-mix-blend-mode wurde ignoriert."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS-{{property}} wurde ignoriert.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "CSS-background-size an einem Verlauf wurde ignoriert."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Eine nicht unterstützte Position von CSS-radial-gradient wurde ignoriert."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Ein elliptischer CSS-radial-gradient wurde angenähert."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Ein Ausdehnungsschlüsselwort von CSS-radial-gradient wurde angenähert."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Eine nicht unterstützte Größe von CSS-radial-gradient wurde ignoriert."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Eine nicht unterstützte CSS-text-shadow-Ebene wurde ignoriert."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "CSS-text-shadow-Ebenen nach der ersten wurden ignoriert."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "CSS-text-shadow an einem Inline-Element wurde ignoriert."
        }
        "htmlImport.warn.list.style_image_ignored" => {
            "CSS-list-style-image wurde nicht importiert."
        }
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Eine hängende Aufzählungsmarke mit `list-style-position: outside` wurde angenähert."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "Der nicht unterstützte CSS-list-style-type `{{value}}` wurde angenähert."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "CSS-object-fit:scale-down wurde angenähert."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS-object-fit:none wurde ignoriert.",
        "htmlImport.warn.media.object_position_ignored" => "CSS-object-position wurde ignoriert.",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Ein nicht unterstützter CSS-mix-blend-mode an einem Bild wurde ignoriert."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Ein eingebettetes <svg>-Element wurde als Platzhalter importiert."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Ein nicht unterstützter <input>-Typ wurde angenähert."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "Das <{{tag}}>-Element wurde als Platzhalter importiert."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Ein <picture> mit ausschließlich nicht dekodierbaren Quelltypen wurde angenähert."
        }
        "htmlImport.warn.table.rowspan_ignored" => {
            "Das HTML-Attribut rowspan wurde nicht importiert."
        }
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Spaltenbreiten einer Tabelle mit durch CSS entflachten Zeilengruppen wurden angenähert."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Spaltenbreiten einer CSS-Tabelle ohne festgelegte Breite wurden angenähert."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "Das ungültige <base href> {{href}} wurde ignoriert."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "Das <base href> {{href}} außerhalb des Projektursprungs wurde ignoriert."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "Die externe CSS-Stilvorlage {{url}} ist nicht verfügbar."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "Das Bild {{url}} außerhalb des Projektursprungs wurde als Platzhalter importiert."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "Das nicht verfügbare Bild {{url}} wurde als Platzhalter importiert."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "Der ungültige CSS-@import {{prelude}} wurde ignoriert."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "Der CSS-@import {{reference}} ist nicht verfügbar."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "Der zyklische CSS-@import {{url}} wurde ignoriert."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "Der CSS-@import {{url}} jenseits von Tiefe {{max_depth}} wurde ignoriert."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "Der CSS-@import {{url}} ist nicht verfügbar."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} HTML-Einstiegsdateien gefunden; {{entry}} wurde gewählt, der Rest wurde angenähert."
        }
        "htmlImport.warn.snapshot.truncated" => {
            "Ein Teil der Browser-Momentaufnahme wurde verworfen."
        }
        "htmlImport.warn.snapshot.node_limit" => {
            "Knotenlimit erreicht; der restliche Inhalt der Momentaufnahme wurde ausgelassen."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} durch CORS belastete Bilder wurden als entfernte URLs beibehalten und sind nicht verfügbar."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Ein Knoten der Momentaufnahme mit fehlendem oder ungültigem Rechteck wurde verworfen."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Ein Knoten der Momentaufnahme unbekannter Art wurde verworfen."
        }
        "htmlImport.warn.snapshot.rejected" => {
            "Die Browser-Momentaufnahme ({{reason}}) wurde verworfen."
        }
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Eine nicht unterstützte Transformation der Momentaufnahme wurde ignoriert."
        }
        "htmlImport.warn.css.media_empty_query" => "Eine leere @media-Abfrage wurde ignoriert.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Der nicht unterstützte @media-Typ '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Die nicht unterstützte @media-Bedingung '{{input}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Die ungültige @media-Ausrichtung '{{value}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Das nicht unterstützte @media-Merkmal '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Der nicht unterstützte @media-Bereich '({{input}})' wurde ignoriert."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Der ungültige @media-Bereich '({{input}})' wurde ignoriert."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Die ungültige @media-Länge '{{value}}' wurde ignoriert."
        }
        "htmlImport.diagnostics.title" => "HTML-Import abgeschlossen",
        "htmlImport.diagnostics.summary" => "Eingeschränkte Elemente: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Schließen",
        "htmlImport.diagnostics.expand" => "Details anzeigen",
        "htmlImport.diagnostics.collapse" => "Details ausblenden",
        "htmlImport.diagnostics.more" => "+{{count}} weitere",
        "dialog.pptxTitle" => "Als PowerPoint exportieren",
        "dialog.pptxSummary" => "{{count}} Folien exportiert nach:",
        "dialog.pptxEmpty" => "Diese Präsentation hat keine sichtbaren Folien zum Exportieren.",
        "settings.agents.acpQuickAdd" => "Schnell hinzufügen",
        "settings.agents.acpPresetAdd" => "Hinzufügen",
        "settings.agents.acpNotInstalled" => "Nicht installiert",
        "assetCenter.title" => "Asset-Center",
        "assetCenter.tab.templates" => "Vorlagen",
        "assetCenter.tab.styles" => "Stile",
        "assetCenter.style.empty" => "Keine passenden Stile",
        "assetCenter.style.pinned" => "Angeheftet",
        "assetCenter.style.searchPlaceholder" => "Stile oder Tags suchen",
        "assetCenter.style.generateHint" => "Ein neues Dokument aus deinem Thema, im angehefteten Stil.",
        "slidesPanel.tabSlides" => "Folien",
        "slidesPanel.tabCards" => "Karten",
        "slidesPanel.present" => "Präsentieren",
        _ => return super::de_collab::lookup(key),
    })
}
