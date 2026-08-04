//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `tr_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Görsel ara…",
        "imagePanel.searching" => "Aranıyor…",
        "imagePanel.noResults" => "Sonuç bulunamadı",
        "imagePanel.searchPrompt" => "Görsel arayın",
        "imagePanel.sourceNotice" => "Görseller {{source}} kaynağından. Özgür lisanslı — kullanmadan önce lisansı doğrulayın.",
        "imagePanel.genNotConfigured" => "Görsel oluşturma yapılandırılmamış",
        "imagePanel.openSettings" => "Ayarları Aç",
        "imagePanel.promptPlaceholder" => "Görseli tanımlayın…",
        "providerProbe.connectedViaCli" => "{{name}} CLI üzerinden bağlanıldı",
        "providerProbe.cliExitedWithError" => "{{name}} CLI bir hatayla sonlandı",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI sürüm bilgisi üretmedi",
        "providerProbe.modelQueryFailed" => "{{name}} model sorgusu başarısız oldu veya zaman aşımına uğradı",
        "providerProbe.modelQueryFailedRunLogin" => "{{name}} model sorgusu başarısız oldu. Kimlik doğrulamak için {{command}} komutunu bir kez çalıştırın.",
        "providerProbe.modelQueryNeedsAuth" => "{{name}} model sorgusu kimlik doğrulama gerektiriyor. Oturum açmak için {{command}} komutunu bir kez çalıştırın.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} tanınmayan bir model kataloğu döndürdü",
        "promptCenter.title" => "İstem Merkezi",
        "promptCenter.searchPlaceholder" => "İstemlerde ara…",
        "promptCenter.category.all" => "Tümü",
        "promptCenter.category.starter" => "Hızlı başlangıç",
        "promptCenter.category.mobileApp" => "Mobil uygulama",
        "promptCenter.category.webPage" => "Web sayfası",
        "promptCenter.category.dashboard" => "Panolar",
        "promptCenter.category.component" => "Bileşenler",
        "promptCenter.category.modify" => "Düzenleme",
        "promptCenter.category.custom" => "Benimkiler",
        "promptCenter.empty" => "Eşleşen istem bulunamadı",
        "promptCenter.saveCurrent" => "Mevcut girdiyi istem olarak kaydet",
        "promptCenter.saveTitlePlaceholder" => "İstem başlığını girin",
        "promptCenter.save" => "Kaydet",
        "promptCenter.cancel" => "İptal",
        "promptCenter.delete" => "Sil",
        "promptCenter.screens" => "{{count}} ekran",
        "promptCenter.freeform" => "Serbest biçim",
        "promptCenter.item.wander.title" => "Wander · Seyahat planı",
        "promptCenter.item.forage.title" => "Forage · Mevsimlik tarifler",
        "promptCenter.item.still.title" => "Still · Meditasyon ve uyku",
        "promptCenter.item.hearth.title" => "Hearth · Akıllı ev",
        "promptCenter.item.meteo.title" => "Meteo · Sürükleyici hava durumu",
        "promptCenter.item.marginalia.title" => "Marginalia · Okuma ve not alma",
        "promptCenter.item.lingua.title" => "Lingua · Dil öğrenme",
        "promptCenter.item.daybreak.title" => "Daybreak · Kahve siparişi",
        "promptCenter.item.verdant.title" => "Verdant · Bitki bakımı",
        "promptCenter.item.companion.title" => "Companion · Evcil hayvan yaşamı",
        "promptCenter.item.relic.title" => "Relic · Seçkin ikinci el pazarı",
        "promptCenter.item.nocturne.title" => "Nocturne · Yıldız gözlem rehberi",
        "promptCenter.item.marquee.title" => "Marquee · Film izleme listesi",
        "promptCenter.item.ritual.title" => "Ritual · Alışkanlık kazanma",
        "promptCenter.item.ember.title" => "Ember · Duygu günlüğü",
        "promptCenter.item.volt.title" => "Volt · Elektrikli araç asistanı",
        "promptCenter.item.aloft.title" => "Aloft · Uçuş takibi",
        "promptCenter.item.gallery.title" => "Gallery · Sergiler ve kültür",
        "promptCenter.item.nightcap.title" => "Nightcap · Evde kokteyl",
        "promptCenter.item.bloom.title" => "Bloom · Çocuk gelişim günlüğü",
        "promptCenter.item.extremeWeather.title" => "Hava durumu uygulaması · Beni şaşırt",
        "promptCenter.item.extremeNowPlaying.title" => "Şimdi çalıyor · Yayına hazır güzellikte",
        "promptCenter.item.extremeDailyApp.title" => "Her gün açmak isteyeceğin uygulama",
        "promptCenter.item.extremeCalendar.title" => "Takvimi baştan tasarla",
        "promptCenter.item.extremeCalm.title" => "Tek ekranda huzur",
        "promptCenter.item.webOrbit.title" => "Orbit · Yapay zekâ çalışma alanı açılış sayfası",
        "promptCenter.item.webAtelier.title" => "Atelier · Mobilya e-ticareti",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Büyüme analitiği panosu",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Lojistik operasyonları",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Kurumsal veri tablosu",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Form bileşeni sistemi",
        "promptCenter.item.modifyPolishCurrent.title" => "Mevcut ekranı iyileştir",
        "promptCenter.item.modifyCompleteStates.title" => "Bileşen durumlarını tamamla",
        "sceneTemplate.title" => "Sahne Şablonları",
        "sceneTemplate.searchPlaceholder" => "Sahne veya şablon ara…",
        "sceneTemplate.empty" => "Eşleşen şablon bulunamadı",
        "sceneTemplate.frames" => "{{count}} sayfa",
        "sceneTemplate.filter.all" => "Tümü",
        "sceneTemplate.scene.tutorial" => "Eğitim görseli",
        "sceneTemplate.scene.comparison" => "Karşılaştırma görseli",
        "sceneTemplate.scene.carousel" => "Bilgi kartları",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "Üç adımlı ekran görüntülü eğitim kartı"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Kapak, üç işlem adımı ve son harekete geçirici mesaj; ekran görüntülerini ve metni değiştirip yayımlayın."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Bilgi ve içgörü karuseli",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Kapak, üç ana fikir ve özet sayfası; tek bir fikri kaydırılabilir ardışık kartlara bölmek için ideal."
        }
        "sceneTemplate.item.beforeAfter.title" => "Yeniden tasarım öncesi ve sonrası",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Değişiklik notlarıyla yan yana önce/sonra karşılaştırması; geriye dönük değerlendirmeler ve portföy sunumları için ideal."
        }
        "sceneTemplate.item.slideDeck.title" => "Sunum · Altı slayt",
        "sceneTemplate.item.slideDeck.summary" => {
            "Kapak, gündem, ana noktalar, veriler, grafik ve kapanış; 16:9 sunum oranında, metni değiştirip sunmaya hazır."
        }
        "fileMenu.newFromTemplate" => "Şablondan yeni oluştur",
        "collab.ownerConfirm.title" => "Kime katıldığınızı onaylayın",
        "collab.ownerConfirm.hint" => "Bu oturumdan henüz hiçbir şey yüklenmedi.",
        "collab.ownerConfirm.account" => "Doğrulanmış hesap",
        "collab.ownerConfirm.device" => "Doğrulanmış cihaz",
        "collab.ownerConfirm.claimedName" => "Bu hesabın seçtiği ad (doğrulanmadı)",
        "collab.action.confirmOwner" => "Bu oturuma katıl",
        "collab.action.rejectOwner" => "Katılma",
        "collab.error.ownerNotConfirmed" => "Sunucuyu onaylamadınız, bu yüzden hiçbir şey yüklenmedi.",
        "fileMenu.exportSlideshowHtml" => "Slayt gösterisini HTML olarak dışa aktar...",
        "dialog.slideshowHtmlTitle" => "Slayt gösterisini dışa aktar",
        "dialog.slideshowHtmlSummary" => "{{count}} slayt şuraya aktarıldı:",
        "dialog.slideshowHtmlEmpty" => "Bu sunuda dışa aktarılacak görünür slayt yok.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "İçe aktarılabilir HTML içeriği kullanılamıyor.",
        "htmlImport.warn.content.empty_body" => {
            "HTML gövdesindeki içe aktarılabilir içerik kullanılamıyor."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "{{max_depth}} düzeyden daha derin iç içe geçmiş HTML atıldı."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Düğüm sınırına ulaşıldı; sayfanın kalan içeriği çıkarıldı."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Düğüm sınırına ulaşıldı; HTML ağacının bir bölümü çıkarıldı."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Düğüm sınırına ulaşıldı; bir satır içi biçimlendirme satırı çıkarıldı."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Düğüm sınırına ulaşıldı; oluşturulan sözde öğeler çıkarıldı."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "{{max_depth}} at-rule düzeyinden daha derin iç içe geçmiş CSS kuralları yok sayıldı."
        }
        "htmlImport.warn.css.unterminated_rule" => "Sonlandırılmamış bir CSS kuralı yok sayıldı.",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker kuralları içe aktarılmadı.",
        "htmlImport.warn.css.nesting_unsupported" => {
            "İç içe geçmiş CSS stil kuralları yok sayıldı."
        }
        "htmlImport.warn.css.invalid_layer_name" => "Geçersiz @layer adı '{{name}}' yok sayıldı.",
        "htmlImport.warn.css.unsupported_statement" => {
            "Desteklenmeyen @{{name}} ifadesi yok sayıldı."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "Görünüm alanı olmayan @media kuralları yok sayıldı."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Geçersiz @layer blok adı '{{name}}' yok sayıldı."
        }
        "htmlImport.warn.css.unsupported_container_block" => "@container bloğu yok sayıldı.",
        "htmlImport.warn.css.unsupported_block" => "Desteklenmeyen @{{name}} bloğu yok sayıldı.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "'{{family}}' adlı @font-face web yazı tipi kullanılamıyor."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Mutlak konumlu bir öğenin yüzde ofsetleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Yüzde cinsinden position:relative ofsetleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "Belirli bir ekseni olmayan CSS aspect-ratio yok sayıldı."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "Belirsiz bir kapsayıcı blok içindeki CSS aspect-ratio yok sayıldı."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky yok sayıldı.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Desteklenmeyen CSS grid izleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.float_ignored" => "CSS float yok sayıldı.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "Düğüm düzeyindeki CSS mix-blend-mode yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "Negatif CSS kenar boşlukları yok sayıldı."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "Görsel bir kutudaki CSS kenar boşlukları yok sayıldı."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box yüzde boyutlandırması yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Açık başlangıç çizgilerinin bıraktığı boş CSS grid hücreleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Yayılımı başlangıç çizgisine sığmayan bir CSS grid öğesi yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Düğüm sınırına ulaşıldı; CSS grid satır sarmalayıcıları çıkarıldı."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "auto-fit / auto-fill kullanan CSS grid iz genişlikleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas yerleşimi içe aktarılmadı."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "CSS grid-row yerleşimi içe aktarılmadı."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Blok eksenindeki otomatik CSS kenar boşlukları içe aktarılmadı."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Düğüm sınırına ulaşıldı; CSS otomatik kenar boşluğu hizalaması çıkarıldı."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Belirli bir boyutu olmayan bir öğedeki akış içi CSS ofseti atıldı."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Düğüm sınırına ulaşıldı; akış içi bir CSS ofseti çıkarıldı."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Akış içi CSS ofsetleri (position:relative iç konumları, transform ötelemesi) yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Ofset sarmalayıcı barındıramayan bir kutudaki akış içi CSS ofseti atıldı."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "Sütun yönlü bir flex kapsayıcısındaki flex-wrap içe aktarılmadı."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "Belirli bir genişliği olmayan bir kapsayıcıdaki flex-wrap yok sayıldı."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "Satır kaydıran bir flex kapsayıcısındaki CSS align-content içe aktarılmadı."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "Alt öğelerin ana eksen boyutları belirsiz olduğu için flex-wrap yok sayıldı."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Düğüm sınırına ulaşıldı; flex-wrap satırları çıkarıldı."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Desteklenmeyen CSS transform söz dizimi yok sayıldı."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Desteklenmeyen CSS transform işlevleri (3D, matrix3d) yok sayıldı."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Belirsiz bir eksende yüzde cinsinden CSS transform ötelemesi atıldı."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Sonlu olmayan bir matris üreten CSS transform yok sayıldı."
        }
        "htmlImport.warn.transform.skew_dropped" => "CSS transform eğriltmesi atıldı.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Sıfır veya sonlu olmayan ölçekli bir CSS transform yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "CSS transform aynalaması yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "CSS transform-origin Z ofseti yok sayıldı."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Düğüm boyutuna işlenemeyen bir CSS transform ölçeği atıldı."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "Düğüm boyutuna işlenen CSS transform ölçeği yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "Otomatik boyutlu bir öğedeki CSS transform ölçeği yok sayıldı."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "Yönlü veya aralıklı CSS background-repeat yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Açıkça belirtilen bir CSS arka plan döşeme boyutu yok sayıldı."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "Otomatik boyutlu bir öğedeki CSS background-size yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "Görüntünün asıl boyutunu gerektiren CSS background-size yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Desteklenmeyen bir CSS background-position yok sayıldı."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Boş bir CSS arka plan görüntüsü URL adresi yok sayıldı."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS konik gradyanları yok sayıldı.",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Desteklenmeyen bir CSS background-image katmanı yok sayıldı."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Çözümlenemeyen bir CSS arka plan rengi yok sayıldı."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS background-position yok sayıldı."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Kenar başına farklı CSS kenarlık renkleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Kenar başına karışık CSS kenarlık stilleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Karmaşık bir CSS kenarlık stili yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Desteklenmeyen bir CSS kenarlık stili yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Eliptik CSS kenarlık yarıçapları yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Desteklenmeyen bir CSS kenarlık yarıçapı yok sayıldı."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Desteklenmeyen bir CSS box-shadow katmanı yok sayıldı."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "CSS gradyan rengi ara değerleme yöntemi yok sayıldı."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Desteklenmeyen bir CSS linear-gradient yönü yok sayıldı."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "CSS gradyan renk ipuçları yok sayıldı."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Desteklenmeyen bir CSS gradyan renk durağı yok sayıldı."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "İkiden az kullanılabilir durağı olan bir CSS gradyanı yok sayıldı."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Yinelenen bir CSS gradyanı yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Aralık dışındaki CSS gradyan durakları yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Desteklenmeyen bir CSS bulanıklık yarıçapı yok sayıldı."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Desteklenmeyen bir CSS filtre drop-shadow() değeri yok sayıldı."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Desteklenmeyen bir CSS filtre işlevi yok sayıldı."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Desteklenmeyen bir CSS backdrop-filter işlevi yok sayıldı."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Desteklenmeyen bir CSS background-blend-mode yok sayıldı."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "Tek tek dolgulardaki CSS mix-blend-mode yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Desteklenmeyen bir CSS mix-blend-mode yok sayıldı."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} yok sayıldı.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "Bir gradyandaki CSS background-size yok sayıldı."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Desteklenmeyen bir CSS radial-gradient konumu yok sayıldı."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Eliptik bir CSS radial-gradient yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Bir CSS radial-gradient kapsam anahtar sözcüğü yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Desteklenmeyen bir CSS radial-gradient boyutu yok sayıldı."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Desteklenmeyen bir CSS text-shadow katmanı yok sayıldı."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "İlkinden sonraki CSS text-shadow katmanları yok sayıldı."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "Satır içi bir öğedeki CSS text-shadow yok sayıldı."
        }
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image içe aktarılmadı.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` ile asılı duran bir madde imi yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "Desteklenmeyen CSS list-style-type `{{value}}` yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "CSS object-fit:scale-down yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none yok sayıldı.",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position yok sayıldı.",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Bir görüntüdeki desteklenmeyen CSS mix-blend-mode yok sayıldı."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Satır içi bir <svg> öğesi yer tutucu olarak içe aktarıldı."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Desteklenmeyen bir <input> türü yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "<{{tag}}> öğesi yer tutucu olarak içe aktarıldı."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Yalnızca çözülemeyen kaynak türleri içeren bir <picture> yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML rowspan özniteliği içe aktarılmadı.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Satır grupları CSS ile düzleştirilmeyen bir tablonun sütun genişlikleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Belirli bir genişliği olmayan bir CSS tablosunun sütun genişlikleri yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "Geçersiz <base href> {{href}} yok sayıldı."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "Proje kaynağı dışındaki <base href> {{href}} yok sayıldı."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "{{url}} adresindeki dış stil sayfası kullanılamıyor."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "Proje kaynağı dışındaki {{url}} görüntüsü yer tutucu olarak içe aktarıldı."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "Kullanılamayan {{url}} görüntüsü yer tutucu olarak içe aktarıldı."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "Geçersiz CSS @import {{prelude}} yok sayıldı."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} kullanılamıyor."
        }
        "htmlImport.warn.resource.css_import_cycle" => "Döngüsel CSS @import {{url}} yok sayıldı.",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "{{max_depth}} derinliğini aşan CSS @import {{url}} yok sayıldı."
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} kullanılamıyor.",
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} adet HTML giriş dosyası bulundu; {{entry}} seçildi, geri kalanlar yaklaşık olarak uygulandı."
        }
        "htmlImport.warn.snapshot.truncated" => "Tarayıcı anlık görüntüsünün bir bölümü atıldı.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Düğüm sınırına ulaşıldı; anlık görüntünün kalan içeriği çıkarıldı."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "Uzak URL olarak tutulan {{count}} adet CORS kirlenmiş görüntü kullanılamıyor."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Dikdörtgeni eksik veya geçersiz olan bir anlık görüntü düğümü atıldı."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Bilinmeyen türde bir anlık görüntü düğümü atıldı."
        }
        "htmlImport.warn.snapshot.rejected" => "Tarayıcı anlık görüntüsü ({{reason}}) atıldı.",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Desteklenmeyen bir anlık görüntü dönüşümü yok sayıldı."
        }
        "htmlImport.warn.css.media_empty_query" => "Boş bir @media sorgusu yok sayıldı.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Desteklenmeyen @media türü '{{name}}' yok sayıldı."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Desteklenmeyen @media koşulu '{{input}}' yok sayıldı."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Geçersiz @media yönü '{{value}}' yok sayıldı."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Desteklenmeyen @media özelliği '{{name}}' yok sayıldı."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Desteklenmeyen @media aralığı '({{input}})' yok sayıldı."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Geçersiz @media aralığı '({{input}})' yok sayıldı."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Geçersiz @media uzunluğu '{{value}}' yok sayıldı."
        }
        "htmlImport.diagnostics.title" => "HTML içe aktarma tamamlandı",
        "htmlImport.diagnostics.summary" => "Bozulan öğeler: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Kapat",
        "htmlImport.diagnostics.expand" => "Ayrıntıları göster",
        "htmlImport.diagnostics.collapse" => "Ayrıntıları gizle",
        "htmlImport.diagnostics.more" => "+{{count}} daha",
        _ => return super::tr_collab::lookup(key),
    })
}
