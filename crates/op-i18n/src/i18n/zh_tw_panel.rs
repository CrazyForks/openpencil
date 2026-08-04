//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `zh_tw_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜尋圖片…",
        "imagePanel.searching" => "搜尋中…",
        "imagePanel.noResults" => "未找到結果",
        "imagePanel.searchPrompt" => "搜尋圖片",
        "imagePanel.sourceNotice" => "圖片來自 {{source}}。自由授權 — 使用前請確認授權條款。",
        "imagePanel.genNotConfigured" => "圖片生成尚未設定",
        "imagePanel.openSettings" => "開啟設定",
        "imagePanel.promptPlaceholder" => "描述要生成的圖片…",
        "providerProbe.connectedViaCli" => "已透過 {{name}} CLI 連線",
        "providerProbe.cliExitedWithError" => "{{name}} CLI 結束並回報錯誤",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI 未輸出版本資訊",
        "providerProbe.modelQueryFailed" => "{{name}} 模型查詢失敗或逾時",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 模型查詢失敗。請先執行 {{command}} 完成驗證。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 模型查詢需要驗證。請先執行 {{command}} 登入。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} 傳回無法辨識的模型清單",
        "promptCenter.title" => "提示詞中心",
        "promptCenter.searchPlaceholder" => "搜尋提示詞…",
        "promptCenter.category.all" => "全部",
        "promptCenter.category.starter" => "快速上手",
        "promptCenter.category.mobileApp" => "行動 App",
        "promptCenter.category.webPage" => "網頁",
        "promptCenter.category.dashboard" => "儀表板",
        "promptCenter.category.component" => "元件",
        "promptCenter.category.modify" => "改稿",
        "promptCenter.category.custom" => "我的",
        "promptCenter.empty" => "沒有符合的提示詞",
        "promptCenter.saveCurrent" => "儲存目前輸入",
        "promptCenter.saveTitlePlaceholder" => "提示詞標題",
        "promptCenter.save" => "儲存",
        "promptCenter.cancel" => "取消",
        "promptCenter.delete" => "刪除",
        "promptCenter.screens" => "{{count}} 個畫面",
        "promptCenter.freeform" => "自由發揮",
        "promptCenter.item.wander.title" => "Wander · 旅行行程規劃",
        "promptCenter.item.forage.title" => "Forage · 時令食譜",
        "promptCenter.item.still.title" => "Still · 冥想與睡前",
        "promptCenter.item.hearth.title" => "Hearth · 智慧家庭",
        "promptCenter.item.meteo.title" => "Meteo · 沉浸式天氣",
        "promptCenter.item.marginalia.title" => "Marginalia · 閱讀與註記",
        "promptCenter.item.lingua.title" => "Lingua · 語言學習",
        "promptCenter.item.daybreak.title" => "Daybreak · 咖啡預訂",
        "promptCenter.item.verdant.title" => "Verdant · 植物照護",
        "promptCenter.item.companion.title" => "Companion · 寵物生活",
        "promptCenter.item.relic.title" => "Relic · 精品二手市集",
        "promptCenter.item.nocturne.title" => "Nocturne · 觀星指南",
        "promptCenter.item.marquee.title" => "Marquee · 觀影清單",
        "promptCenter.item.ritual.title" => "Ritual · 習慣養成",
        "promptCenter.item.ember.title" => "Ember · 心情日記",
        "promptCenter.item.volt.title" => "Volt · 電動車夥伴",
        "promptCenter.item.aloft.title" => "Aloft · 航班追蹤",
        "promptCenter.item.gallery.title" => "Gallery · 展覽與文化活動",
        "promptCenter.item.nightcap.title" => "Nightcap · 家庭調酒",
        "promptCenter.item.bloom.title" => "Bloom · 親子成長記錄",
        "promptCenter.item.extremeWeather.title" => "極限 · 天氣 App",
        "promptCenter.item.extremeNowPlaying.title" => "極限 · 正在播放",
        "promptCenter.item.extremeDailyApp.title" => "極限 · 每日必開 App",
        "promptCenter.item.extremeCalendar.title" => "極限 · 行事曆",
        "promptCenter.item.extremeCalm.title" => "極限 · 寧靜",
        "promptCenter.item.webOrbit.title" => "Orbit · AI 工作台官網",
        "promptCenter.item.webAtelier.title" => "Atelier · 家居品牌電商",
        "promptCenter.item.dashboardPulse.title" => "Pulse · 成長分析台",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 物流維運中心",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · 企業資料表",
        "promptCenter.item.componentFormLab.title" => "Form Lab · 表單元件系統",
        "promptCenter.item.modifyPolishCurrent.title" => "精修目前介面",
        "promptCenter.item.modifyCompleteStates.title" => "補齊元件狀態",
        "collab.ownerConfirm.title" => "確認你要加入誰的工作階段",
        "collab.ownerConfirm.hint" => "此工作階段的任何內容都尚未載入。",
        "collab.ownerConfirm.account" => "已驗證帳戶",
        "collab.ownerConfirm.device" => "已驗證裝置",
        "collab.ownerConfirm.claimedName" => "該帳戶自選的名稱（未經驗證）",
        "collab.action.confirmOwner" => "加入此工作階段",
        "collab.action.rejectOwner" => "不加入",
        "collab.error.ownerNotConfirmed" => "你未確認主持人，因此未載入任何內容。",
        "sceneTemplate.title" => "場景範本",
        "sceneTemplate.searchPlaceholder" => "搜尋場景或範本",
        "sceneTemplate.empty" => "沒有符合的範本",
        "sceneTemplate.frames" => "{{count}} 頁",
        "sceneTemplate.filter.all" => "全部",
        "sceneTemplate.scene.tutorial" => "教學圖",
        "sceneTemplate.scene.comparison" => "對比圖",
        "sceneTemplate.scene.carousel" => "知識卡片",
        "sceneTemplate.scene.slides" => "簡報",
        "sceneTemplate.item.screenshotTutorial.title" => "三步截圖教學卡",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "封面、三個操作步驟和結尾行動呼籲，替換截圖與說明即可發布。"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "知識觀點輪播",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "封面、三個論點和總結頁，適合將一個觀點拆成可滑動的連續卡片。"
        }
        "sceneTemplate.item.beforeAfter.title" => "改版前後對比",
        "sceneTemplate.item.beforeAfter.summary" => {
            "左右並置的前後對比，搭配改動說明，適合回顧與作品展示。"
        }
        "sceneTemplate.item.slideDeck.title" => "簡報 · 六頁",
        "sceneTemplate.item.slideDeck.summary" => {
            "封面、目錄、要點、資料、圖表和結尾，16:9 投影比例，替換文案即可上台。"
        }
        "fileMenu.newFromTemplate" => "從範本新增",
        "fileMenu.exportSlideshowHtml" => "匯出放映 HTML...",
        "dialog.slideshowHtmlTitle" => "匯出放映",
        "dialog.slideshowHtmlSummary" => "已匯出 {{count}} 張投影片到：",
        "dialog.slideshowHtmlEmpty" => "目前簡報沒有可匯出的投影片。",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "可匯入的 HTML 內容無法使用。",
        "htmlImport.warn.content.empty_body" => "HTML 主體中可匯入的內容無法使用。",
        "htmlImport.warn.content.dom_depth_truncated" => {
            "巢狀層數超過 {{max_depth}} 層的 HTML 已捨棄。"
        }
        "htmlImport.warn.content.node_limit_truncated" => "已達節點上限，其餘頁面內容已略過。",
        "htmlImport.warn.content.node_limit_mapping" => "已達節點上限，部分 HTML 樹狀結構已略過。",
        "htmlImport.warn.content.node_limit_inline_row" => "已達節點上限，某個行內排版列已略過。",
        "htmlImport.warn.content.node_limit_pseudo" => "已達節點上限，產生的虛擬元素已略過。",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "巢狀 at-rule 超過 {{max_depth}} 層的 CSS 規則已忽略。"
        }
        "htmlImport.warn.css.unterminated_rule" => "未結束的 CSS 規則已忽略。",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 規則未匯入。",
        "htmlImport.warn.css.nesting_unsupported" => "巢狀的 CSS 樣式規則已忽略。",
        "htmlImport.warn.css.invalid_layer_name" => "無效的 @layer 名稱 '{{name}}' 已忽略。",
        "htmlImport.warn.css.unsupported_statement" => "不支援的 @{{name}} 陳述式已忽略。",
        "htmlImport.warn.css.media_without_viewport" => "未指定可視區域的 @media 規則已忽略。",
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "無效的 @layer 區塊名稱 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.unsupported_container_block" => "@container 區塊已忽略。",
        "htmlImport.warn.css.unsupported_block" => "不支援的 @{{name}} 區塊已忽略。",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face 網頁字型 '{{family}}' 無法使用。"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "絕對定位元素的百分比位移已近似處理。"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "百分比的 position:relative 位移已近似處理。"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "沒有確定軸向的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "位於不確定包含區塊內的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky 已忽略。",
        "htmlImport.warn.layout.grid_tracks_approximated" => "不支援的 CSS 格線軌道已近似處理。",
        "htmlImport.warn.layout.float_ignored" => "CSS float 已忽略。",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "節點層級的 CSS mix-blend-mode 已近似處理。"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll 已近似處理。"
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "負值的 CSS 邊界已忽略。",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "視覺方塊上的 CSS 邊界已忽略。",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box 的百分比尺寸已近似處理。"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "明確起始線所留下的空白 CSS 格線儲存格已近似處理。"
        }
        "htmlImport.warn.layout.grid_span_reflowed" => "跨距不符起始線的 CSS 格線項目已近似處理。",
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "已達節點上限，CSS 格線列的包裝元素已略過。"
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "使用 auto-fit / auto-fill 的 CSS 格線軌道寬度已近似處理。"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas 的配置未匯入。"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row 的配置未匯入。",
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` 已近似處理。"
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => "區塊軸向的 CSS 自動邊界未匯入。",
        "htmlImport.warn.layout.auto_margin_node_limit" => "已達節點上限，CSS 自動邊界對齊已略過。",
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "沒有確定尺寸之元素上的 CSS 流內位移已捨棄。"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "已達節點上限，某個 CSS 流內位移已略過。"
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS 流內位移（position:relative 內縮值、transform 平移）已近似處理。"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "無法容納位移包裝元素之方塊上的 CSS 流內位移已捨棄。"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "直向 flex 容器上的 flex-wrap 未匯入。"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse 已近似處理。",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "沒有確定寬度之容器上的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "換行 flex 容器上的 CSS align-content 未匯入。"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "子項主軸尺寸不確定的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => "已達節點上限，flex-wrap 的換行列已略過。",
        "htmlImport.warn.transform.unsupported_syntax" => "不支援的 CSS transform 語法已忽略。",
        "htmlImport.warn.transform.unsupported_function" => {
            "不支援的 CSS transform 函式（3D、matrix3d）已忽略。"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "位於不確定軸向上的百分比 CSS transform 平移已捨棄。"
        }
        "htmlImport.warn.transform.non_finite_matrix" => "產生非有限矩陣的 CSS transform 已忽略。",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform 的傾斜已捨棄。",
        "htmlImport.warn.transform.degenerate_scale" => {
            "縮放值為零或非有限值的 CSS transform 已近似處理。"
        }
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform 的鏡像已近似處理。",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin 的 Z 軸位移已忽略。",
        "htmlImport.warn.transform.scale_not_baked" => {
            "無法併入節點尺寸的 CSS transform 縮放已捨棄。"
        }
        "htmlImport.warn.transform.scale_baked" => {
            "已併入節點尺寸的 CSS transform 縮放已近似處理。"
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "自動尺寸元素上的 CSS transform 縮放已忽略。"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "具方向性或帶間隔的 CSS background-repeat 已近似處理。"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "明確指定的 CSS 背景拼貼尺寸已忽略。"
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "自動尺寸元素上的 CSS background-size 已近似處理。"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "需要圖片內在尺寸的 CSS background-size 已近似處理。"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "不支援的 CSS background-position 已忽略。"
        }
        "htmlImport.warn.visual.background_image_url_empty" => "空白的 CSS 背景圖片 URL 已忽略。",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS 圓錐漸層已忽略。",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "不支援的 CSS background-image 圖層已忽略。"
        }
        "htmlImport.warn.visual.background_color_unresolved" => "無法解析的 CSS 背景色已忽略。",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position 已忽略。",
        "htmlImport.warn.visual.border_colors_approximated" => {
            "各邊獨立的 CSS 框線顏色已近似處理。"
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "各邊混用的 CSS 框線樣式已近似處理。"
        }
        "htmlImport.warn.visual.border_style_complex" => "複雜的 CSS 框線樣式已近似處理。",
        "htmlImport.warn.visual.border_style_unsupported" => "不支援的 CSS 框線樣式已近似處理。",
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "橢圓形的 CSS 框線圓角半徑已近似處理。"
        }
        "htmlImport.warn.visual.border_radius_unsupported" => "不支援的 CSS 框線圓角半徑已忽略。",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "不支援的 CSS box-shadow 圖層已忽略。"
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS 漸層的色彩內插方式已忽略。",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "不支援的 CSS linear-gradient 方向已忽略。"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS 漸層的色彩提示點已忽略。",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "不支援的 CSS 漸層色彩停駐點已忽略。"
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => "可用停駐點少於兩個的 CSS 漸層已忽略。",
        "htmlImport.warn.visual.gradient_repeating_approximated" => "重複式的 CSS 漸層已近似處理。",
        "htmlImport.warn.visual.gradient_stops_clamped" => "超出範圍的 CSS 漸層停駐點已近似處理。",
        "htmlImport.warn.visual.blur_radius_unsupported" => "不支援的 CSS 模糊半徑已忽略。",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "不支援的 CSS 濾鏡 drop-shadow() 已忽略。"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "不支援的 CSS 濾鏡函式已忽略。",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "不支援的 CSS backdrop-filter 函式已忽略。"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "不支援的 CSS background-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "個別填色上的 CSS mix-blend-mode 已近似處理。"
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "不支援的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} 已忽略。",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "漸層上的 CSS background-size 已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "不支援的 CSS radial-gradient 位置已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "橢圓形的 CSS radial-gradient 已近似處理。"
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient 的範圍關鍵字已近似處理。"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "不支援的 CSS radial-gradient 尺寸已忽略。"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "不支援的 CSS text-shadow 圖層已忽略。",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "第一層之後的 CSS text-shadow 圖層已忽略。"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "行內元素上的 CSS text-shadow 已忽略。",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image 未匯入。",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` 的懸掛項目符號已近似處理。"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "不支援的 CSS list-style-type `{{value}}` 已近似處理。"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down 已近似處理。",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none 已忽略。",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position 已忽略。",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "圖片上不支援的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "行內的 <svg> 元素已改以預留位置匯入。",
        "htmlImport.warn.media.input_type_fallback" => "不支援的 <input> 類型已近似處理。",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 元素已改以預留位置匯入。",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "來源類型皆無法解碼的 <picture> 已近似處理。"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML 的 rowspan 屬性未匯入。",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "列群組遭 CSS 拆散之表格的欄寬已近似處理。"
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "沒有確定寬度之 CSS 表格的欄寬已近似處理。"
        }
        "htmlImport.warn.resource.invalid_base_href" => "無效的 <base href> {{href}} 已忽略。",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "位於專案來源之外的 <base href> {{href}} 已忽略。"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "外部樣式表 {{url}} 無法使用。",
        "htmlImport.warn.resource.image_outside_origin" => {
            "位於專案來源之外的圖片 {{url}} 已改以預留位置匯入。"
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "無法使用的圖片 {{url}} 已改以預留位置匯入。"
        }
        "htmlImport.warn.resource.css_import_invalid" => "無效的 CSS @import {{prelude}} 已忽略。",
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} 無法使用。"
        }
        "htmlImport.warn.resource.css_import_cycle" => "循環的 CSS @import {{url}} 已忽略。",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "超過深度 {{max_depth}} 的 CSS @import {{url}} 已忽略。"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} 無法使用。",
        "htmlImport.warn.project.multiple_html_entries" => {
            "找到 {{count}} 個 HTML 進入點，已選用 {{entry}}，其餘已近似處理。"
        }
        "htmlImport.warn.snapshot.truncated" => "部分瀏覽器快照已捨棄。",
        "htmlImport.warn.snapshot.node_limit" => "已達節點上限，其餘快照內容已略過。",
        "htmlImport.warn.snapshot.tainted_images" => {
            "有 {{count}} 張受 CORS 汙染的圖片以遠端 URL 保留，無法使用。"
        }
        "htmlImport.warn.snapshot.invalid_rect" => "矩形範圍缺漏或無效的快照節點已捨棄。",
        "htmlImport.warn.snapshot.unknown_kind" => "類型不明的快照節點已捨棄。",
        "htmlImport.warn.snapshot.rejected" => "瀏覽器快照（{{reason}}）已捨棄。",
        "htmlImport.warn.snapshot.unsupported_transform" => "不支援的快照 transform 已忽略。",
        "htmlImport.warn.css.media_empty_query" => "空白的 @media 查詢條件已忽略。",
        "htmlImport.warn.css.media_unsupported_type" => "不支援的 @media 類型 '{{name}}' 已忽略。",
        "htmlImport.warn.css.media_unsupported_condition" => {
            "不支援的 @media 條件 '{{input}}' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "無效的 @media 方向 '{{value}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "不支援的 @media 特性 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "不支援的 @media 範圍 '({{input}})' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_range" => "無效的 @media 範圍 '({{input}})' 已忽略。",
        "htmlImport.warn.css.media_invalid_length" => "無效的 @media 長度 '{{value}}' 已忽略。",
        "htmlImport.diagnostics.title" => "HTML 匯入完成",
        "htmlImport.diagnostics.summary" => "降級項目：{{count}}",
        "htmlImport.diagnostics.dismiss" => "關閉",
        "htmlImport.diagnostics.expand" => "顯示詳細資料",
        "htmlImport.diagnostics.collapse" => "隱藏詳細資料",
        "htmlImport.diagnostics.more" => "另有 {{count}} 項",
        _ => return super::zh_tw_collab::lookup(key),
    })
}
