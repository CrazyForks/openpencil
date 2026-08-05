//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `zh_cn_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜索图片…",
        "imagePanel.searching" => "搜索中…",
        "imagePanel.noResults" => "未找到结果",
        "imagePanel.searchPrompt" => "搜索图片",
        "imagePanel.sourceNotice" => "图片来自 {{source}}。自由许可 — 使用前请核实许可协议。",
        "imagePanel.genNotConfigured" => "图片生成未配置",
        "imagePanel.openSettings" => "打开设置",
        "imagePanel.promptPlaceholder" => "描述要生成的图片…",
        "providerProbe.connectedViaCli" => "已通过 {{name}} CLI 连接",
        "providerProbe.cliExitedWithError" => "{{name}} CLI 退出并报错",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI 未输出版本信息",
        "providerProbe.modelQueryFailed" => "{{name}} 模型查询失败或超时",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 模型查询失败。请先运行 {{command}} 完成认证。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 模型查询需要认证。请先运行 {{command}} 登录。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} 返回了无法识别的模型列表",
        "providerProbe.connectedAs" => "已以 @{{login}}{{method}} 身份连接",
        "providerProbe.connectedViaGithub" => "已通过 GitHub 连接",
        "importProgress.figmaTitle" => "正在解析 Figma 文件…",
        "importProgress.htmlTitle" => "正在解析 HTML 和页面资源…",
        "importProgress.htmlSubtitle" => "正在读取样式和图片，请稍候",
        "importProgress.largeFileSubtitle" => "大型文件需要几秒钟，请稍候",
        "account.signedOutHint" => "登录后即可同步你的设置与偏好",
        "code.noUsableCode" => "AI 未返回可用代码。请重试，或切换 AI 模型后再试。",
        "code.previousResultKept" => "上次生成的代码仍已保留",
        "promptCenter.title" => "提示词中心",
        "promptCenter.searchPlaceholder" => "搜索提示词…",
        "promptCenter.category.all" => "全部",
        "promptCenter.category.starter" => "快速上手",
        "promptCenter.category.mobileApp" => "移动 App",
        "promptCenter.category.webPage" => "网页",
        "promptCenter.category.dashboard" => "仪表盘",
        "promptCenter.category.component" => "组件",
        "promptCenter.category.modify" => "改稿",
        "promptCenter.category.custom" => "我的",
        "promptCenter.empty" => "没有匹配的提示词",
        "promptCenter.saveCurrent" => "保存当前输入",
        "promptCenter.saveTitlePlaceholder" => "提示词标题",
        "promptCenter.save" => "保存",
        "promptCenter.cancel" => "取消",
        "promptCenter.delete" => "删除",
        "promptCenter.screens" => "{{count}} 屏",
        "promptCenter.freeform" => "自由发挥",
        "promptCenter.item.wander.title" => "Wander · 旅行行程规划",
        "promptCenter.item.forage.title" => "Forage · 时令菜谱",
        "promptCenter.item.still.title" => "Still · 冥想与睡前",
        "promptCenter.item.hearth.title" => "Hearth · 智能家居",
        "promptCenter.item.meteo.title" => "Meteo · 沉浸式天气",
        "promptCenter.item.marginalia.title" => "Marginalia · 阅读与批注",
        "promptCenter.item.lingua.title" => "Lingua · 语言学习",
        "promptCenter.item.daybreak.title" => "Daybreak · 咖啡预订",
        "promptCenter.item.verdant.title" => "Verdant · 植物养护",
        "promptCenter.item.companion.title" => "Companion · 宠物生活",
        "promptCenter.item.relic.title" => "Relic · 精品二手市集",
        "promptCenter.item.nocturne.title" => "Nocturne · 观星指南",
        "promptCenter.item.marquee.title" => "Marquee · 观影清单",
        "promptCenter.item.ritual.title" => "Ritual · 习惯养成",
        "promptCenter.item.ember.title" => "Ember · 心情日记",
        "promptCenter.item.volt.title" => "Volt · 电动车伴侣",
        "promptCenter.item.aloft.title" => "Aloft · 航班追踪",
        "promptCenter.item.gallery.title" => "Gallery · 展览与文化活动",
        "promptCenter.item.nightcap.title" => "Nightcap · 家庭调酒",
        "promptCenter.item.bloom.title" => "Bloom · 亲子成长记录",
        "promptCenter.item.extremeWeather.title" => "极限 · 天气 App",
        "promptCenter.item.extremeNowPlaying.title" => "极限 · 正在播放",
        "promptCenter.item.extremeDailyApp.title" => "极限 · 每日必开 App",
        "promptCenter.item.extremeCalendar.title" => "极限 · 日历",
        "promptCenter.item.extremeCalm.title" => "极限 · 宁静",
        "promptCenter.item.webOrbit.title" => "Orbit · AI 工作台官网",
        "promptCenter.item.webAtelier.title" => "Atelier · 家居品牌电商",
        "promptCenter.item.dashboardPulse.title" => "Pulse · 增长分析台",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 物流运维中心",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · 企业数据表",
        "promptCenter.item.componentFormLab.title" => "Form Lab · 表单组件系统",
        "promptCenter.item.modifyPolishCurrent.title" => "精修当前界面",
        "promptCenter.item.modifyCompleteStates.title" => "补齐组件状态",
        "collab.ownerConfirm.title" => "确认你要加入谁的会话",
        "collab.ownerConfirm.hint" => "此会话的任何内容都尚未载入。",
        "collab.ownerConfirm.account" => "已验证账户",
        "collab.ownerConfirm.device" => "已验证设备",
        "collab.ownerConfirm.claimedName" => "该账户自选的名称（未经验证）",
        "collab.action.confirmOwner" => "加入此会话",
        "collab.action.rejectOwner" => "不加入",
        "collab.error.ownerNotConfirmed" => "你未确认主持人，因此未载入任何内容。",
        "sceneTemplate.title" => "场景模板",
        "sceneTemplate.searchPlaceholder" => "搜索场景或模板",
        "sceneTemplate.empty" => "没有匹配的模板",
        "sceneTemplate.frames" => "{{count}} 页",
        "sceneTemplate.generate.placeholder" => "描述主题，AI 直接生成整副演示文稿",
        "sceneTemplate.generate.button" => "生成",
        "sceneTemplate.generate.hint" => "新建一个文档，按主题直接生成整副演示文稿。",
        "sceneTemplate.generate.promptTemplate" => "为以下主题制作一份演示文稿（PPT）：{{topic}}",
        "sceneTemplate.card.addToCanvas" => "加入画布",
        "sceneTemplate.card.generateFrom" => "以此生成",
        "sceneTemplate.generate.basis" => "基于：",
        "sceneTemplate.filter.all" => "全部",
        "sceneTemplate.scene.tutorial" => "教程图",
        "sceneTemplate.scene.comparison" => "对比图",
        "sceneTemplate.scene.carousel" => "轮播",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.scene.card" => "卡片",
        "sceneTemplate.item.screenshotTutorial.title" => "三步截图教程卡",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "封面、三个操作步骤和结尾行动号召，替换截图与说明即可发布。"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "知识观点轮播",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "封面、三个论点和总结页，适合把一个观点拆成可滑动的连续卡片。"
        }
        "sceneTemplate.item.beforeAfter.title" => "改版前后对比",
        "sceneTemplate.item.beforeAfter.summary" => {
            "左右并置的前后对比，配改动说明，适合复盘与作品展示。"
        }
        "sceneTemplate.item.slideDeck.title" => "演示文稿 · 六页",
        "sceneTemplate.item.slideDeck.summary" => {
            "封面、目录、要点、数据、图表和结尾，16:9 投影比例，替换文案即可上台。"
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "知识卡片 · 竖版",
        "sceneTemplate.item.knowledgeCardVertical.summary" => {
            "3:4 单张图文卡，标题、四条要点和署名条，换掉文案就能发小红书。"
        }
        "sceneTemplate.item.knowledgeCardSquare.title" => "知识卡片 · 方版",
        "sceneTemplate.item.knowledgeCardSquare.summary" => {
            "1:1 方形卡，同一套版式的紧凑版，适合公众号头图与朋友圈。"
        }
        "sceneTemplate.item.pitchDeckDark.title" => "路演 deck · 深色",
        "sceneTemplate.item.pitchDeckDark.summary" => {
            "封面、问题、方案、数据、里程碑和联系页，深底大字，适合融资路演与产品发布。"
        }
        "sceneTemplate.item.lectureDeckLight.title" => "课件 deck · 浅色",
        "sceneTemplate.item.lectureDeckLight.summary" => {
            "课程封面、学习目标、概念讲解、例题、对比表和小结作业，纸白底耐看，适合上课投影。"
        }
        "sceneTemplate.item.minimalKeynote.title" => "极简 Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => {
            "纯白留白、超大字号、一页一个意思，八页里没有一张卡片，适合发布会与主题演讲。"
        }
        "sceneTemplate.item.gradientTech.title" => "渐变科技风",
        "sceneTemplate.item.gradientTech.summary" => {
            "深色渐变底加玻璃拟态卡，含架构、性能对比与客户墙，适合开发者产品发布。"
        }
        "fileMenu.newFromTemplate" => "从模板新建",
        "fileMenu.exportSlideshowHtml" => "导出放映 HTML...",
        "fileMenu.exportPptx" => "导出 PowerPoint...",
        "dialog.slideshowHtmlTitle" => "导出放映",
        "dialog.slideshowHtmlSummary" => "已导出 {{count}} 张幻灯片到：",
        "dialog.slideshowHtmlEmpty" => "当前演示文稿没有可导出的幻灯片。",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "可导入的 HTML 内容不可用。",
        "htmlImport.warn.content.empty_body" => "HTML 正文中可导入的内容不可用。",
        "htmlImport.warn.content.dom_depth_truncated" => {
            "嵌套层级超过 {{max_depth}} 层的 HTML 已丢弃。"
        }
        "htmlImport.warn.content.node_limit_truncated" => "已达节点数上限，页面剩余内容已略去。",
        "htmlImport.warn.content.node_limit_mapping" => "已达节点数上限，部分 HTML 树已略去。",
        "htmlImport.warn.content.node_limit_inline_row" => "已达节点数上限，某个内联排版行已略去。",
        "htmlImport.warn.content.node_limit_pseudo" => "已达节点数上限，生成的伪元素已略去。",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "嵌套超过 {{max_depth}} 层 @ 规则的 CSS 规则已忽略。"
        }
        "htmlImport.warn.css.unterminated_rule" => "未闭合的 CSS 规则已忽略。",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 规则未导入。",
        "htmlImport.warn.css.nesting_unsupported" => "嵌套的 CSS 样式规则已忽略。",
        "htmlImport.warn.css.invalid_layer_name" => "无效的 @layer 名称 '{{name}}' 已忽略。",
        "htmlImport.warn.css.unsupported_statement" => "不支持的 @{{name}} 语句已忽略。",
        "htmlImport.warn.css.media_without_viewport" => "没有视口的 @media 规则已忽略。",
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "无效的 @layer 块名称 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.unsupported_container_block" => "@container 块已忽略。",
        "htmlImport.warn.css.unsupported_block" => "不支持的 @{{name}} 块已忽略。",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face 网络字体 '{{family}}' 不可用。"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "绝对定位元素的百分比偏移已近似处理。"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "百分比的 position:relative 偏移已近似处理。"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "没有确定轴向的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "位于不确定包含块内的 CSS aspect-ratio 已忽略。"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky 已忽略。",
        "htmlImport.warn.layout.grid_tracks_approximated" => "不支持的 CSS 网格轨道已近似处理。",
        "htmlImport.warn.layout.float_ignored" => "CSS float 已忽略。",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "节点级的 CSS mix-blend-mode 已近似处理。"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll 已近似处理。"
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "负的 CSS 外边距已忽略。",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "视觉盒上的 CSS 外边距已忽略。",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box 的百分比尺寸已近似处理。"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "显式起始线留下的空 CSS 网格单元已近似处理。"
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "跨度与起始线不匹配的 CSS 网格项已近似处理。"
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "已达节点数上限，CSS 网格行的包装容器已略去。"
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "使用 auto-fit / auto-fill 的 CSS 网格轨道宽度已近似处理。"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas 定位未导入。"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row 定位未导入。",
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` 已近似处理。"
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => "块轴方向的 CSS 自动外边距未导入。",
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "已达节点数上限，CSS 自动外边距对齐已略去。"
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "尺寸不确定的元素上的 CSS 流内偏移已丢弃。"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "已达节点数上限，某个 CSS 流内偏移已略去。"
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS 流内偏移（position:relative 内缩、transform 平移）已近似处理。"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "某个盒无法承载偏移包装容器，其 CSS 流内偏移已丢弃。"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "列向弹性容器上的 flex-wrap 未导入。"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse 已近似处理。",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "宽度不确定的容器上的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "换行弹性容器上的 CSS align-content 未导入。"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "子项主轴尺寸不确定时的 flex-wrap 已忽略。"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "已达节点数上限，flex-wrap 的换行行已略去。"
        }
        "htmlImport.warn.transform.unsupported_syntax" => "不支持的 CSS transform 语法已忽略。",
        "htmlImport.warn.transform.unsupported_function" => {
            "不支持的 CSS transform 函数（3D、matrix3d）已忽略。"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "不确定轴向上的百分比 CSS transform 平移已丢弃。"
        }
        "htmlImport.warn.transform.non_finite_matrix" => "产生非有限矩阵的 CSS transform 已忽略。",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform 斜切已丢弃。",
        "htmlImport.warn.transform.degenerate_scale" => {
            "缩放为零或非有限的 CSS transform 已近似处理。"
        }
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform 镜像已近似处理。",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin 的 Z 轴偏移已忽略。",
        "htmlImport.warn.transform.scale_not_baked" => {
            "无法烘焙进节点尺寸的 CSS transform 缩放已丢弃。"
        }
        "htmlImport.warn.transform.scale_baked" => {
            "烘焙进节点尺寸的 CSS transform 缩放已近似处理。"
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "自动尺寸元素上的 CSS transform 缩放已忽略。"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "带方向或带间隔的 CSS background-repeat 已近似处理。"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "显式指定的 CSS 背景平铺尺寸已忽略。"
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "自动尺寸元素上的 CSS background-size 已近似处理。"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "需要图片固有尺寸的 CSS background-size 已近似处理。"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "不支持的 CSS background-position 已忽略。"
        }
        "htmlImport.warn.visual.background_image_url_empty" => "空的 CSS 背景图片 URL 已忽略。",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS 锥形渐变已忽略。",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "不支持的 CSS background-image 图层已忽略。"
        }
        "htmlImport.warn.visual.background_color_unresolved" => "无法解析的 CSS 背景色已忽略。",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position 已忽略。",
        "htmlImport.warn.visual.border_colors_approximated" => {
            "分边设置的 CSS 边框颜色已近似处理。"
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "各边不一致的 CSS 边框样式已近似处理。"
        }
        "htmlImport.warn.visual.border_style_complex" => "复杂的 CSS 边框样式已近似处理。",
        "htmlImport.warn.visual.border_style_unsupported" => "不支持的 CSS 边框样式已近似处理。",
        "htmlImport.warn.visual.border_radius_elliptical" => "椭圆形的 CSS 边框圆角已近似处理。",
        "htmlImport.warn.visual.border_radius_unsupported" => "不支持的 CSS 边框圆角已忽略。",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "不支持的 CSS box-shadow 图层已忽略。"
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS 渐变的颜色插值方式已忽略。",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "不支持的 CSS linear-gradient 方向已忽略。"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS 渐变的颜色提示点已忽略。",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => "不支持的 CSS 渐变色标已忽略。",
        "htmlImport.warn.visual.gradient_too_few_stops" => "可用色标少于两个的 CSS 渐变已忽略。",
        "htmlImport.warn.visual.gradient_repeating_approximated" => "重复的 CSS 渐变已近似处理。",
        "htmlImport.warn.visual.gradient_stops_clamped" => "超出范围的 CSS 渐变色标已近似处理。",
        "htmlImport.warn.visual.blur_radius_unsupported" => "不支持的 CSS 模糊半径已忽略。",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "不支持的 CSS filter drop-shadow() 已忽略。"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "不支持的 CSS filter 函数已忽略。",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "不支持的 CSS backdrop-filter 函数已忽略。"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "不支持的 CSS background-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "单个填充上的 CSS mix-blend-mode 已近似处理。"
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "不支持的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} 已忽略。",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "渐变上的 CSS background-size 已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "不支持的 CSS radial-gradient 位置已忽略。"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "椭圆形的 CSS radial-gradient 已近似处理。"
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient 的范围关键字已近似处理。"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "不支持的 CSS radial-gradient 尺寸已忽略。"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "不支持的 CSS text-shadow 图层已忽略。",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "第一层之后的 CSS text-shadow 图层已忽略。"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "内联元素上的 CSS text-shadow 已忽略。",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image 未导入。",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` 的悬挂标记已近似处理。"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "不支持的 CSS list-style-type `{{value}}` 已近似处理。"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down 已近似处理。",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none 已忽略。",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position 已忽略。",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "图片上不支持的 CSS mix-blend-mode 已忽略。"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "内联的 <svg> 元素已作为占位符导入。",
        "htmlImport.warn.media.input_type_fallback" => "不支持的 <input> 类型已近似处理。",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 元素已作为占位符导入。",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "仅含无法解码源类型的 <picture> 已近似处理。"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML rowspan 属性未导入。",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "行组未被 CSS 扁平化的表格，其列宽已近似处理。"
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "宽度不确定的 CSS 表格，其列宽已近似处理。"
        }
        "htmlImport.warn.resource.invalid_base_href" => "无效的 <base href> {{href}} 已忽略。",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "项目源之外的 <base href> {{href}} 已忽略。"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "外部样式表 {{url}} 不可用。",
        "htmlImport.warn.resource.image_outside_origin" => {
            "项目源之外的图片 {{url}} 已作为占位符导入。"
        }
        "htmlImport.warn.resource.image_unavailable" => "不可用的图片 {{url}} 已作为占位符导入。",
        "htmlImport.warn.resource.css_import_invalid" => "无效的 CSS @import {{prelude}} 已忽略。",
        "htmlImport.warn.resource.css_import_unresolvable" => "CSS @import {{reference}} 不可用。",
        "htmlImport.warn.resource.css_import_cycle" => "循环引用的 CSS @import {{url}} 已忽略。",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "超出 {{max_depth}} 层深度的 CSS @import {{url}} 已忽略。"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} 不可用。",
        "htmlImport.warn.project.multiple_html_entries" => {
            "发现 {{count}} 个 HTML 入口，已选用 {{entry}}，其余已近似处理。"
        }
        "htmlImport.warn.snapshot.truncated" => "部分浏览器快照已丢弃。",
        "htmlImport.warn.snapshot.node_limit" => "已达节点数上限，快照剩余内容已略去。",
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} 张受 CORS 污染的图片以远程 URL 保留，不可用。"
        }
        "htmlImport.warn.snapshot.invalid_rect" => "矩形缺失或无效的快照节点已丢弃。",
        "htmlImport.warn.snapshot.unknown_kind" => "类型未知的快照节点已丢弃。",
        "htmlImport.warn.snapshot.rejected" => "浏览器快照（{{reason}}）已丢弃。",
        "htmlImport.warn.snapshot.unsupported_transform" => "不支持的快照变换已忽略。",
        "htmlImport.warn.css.media_empty_query" => "空的 @media 查询已忽略。",
        "htmlImport.warn.css.media_unsupported_type" => "不支持的 @media 类型 '{{name}}' 已忽略。",
        "htmlImport.warn.css.media_unsupported_condition" => {
            "不支持的 @media 条件 '{{input}}' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "无效的 @media 方向 '{{value}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "不支持的 @media 特性 '{{name}}' 已忽略。"
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "不支持的 @media 范围 '({{input}})' 已忽略。"
        }
        "htmlImport.warn.css.media_invalid_range" => "无效的 @media 范围 '({{input}})' 已忽略。",
        "htmlImport.warn.css.media_invalid_length" => "无效的 @media 长度 '{{value}}' 已忽略。",
        "htmlImport.diagnostics.title" => "HTML 导入完成",
        "htmlImport.diagnostics.summary" => "降级项：{{count}}",
        "htmlImport.diagnostics.dismiss" => "关闭",
        "htmlImport.diagnostics.expand" => "显示详情",
        "htmlImport.diagnostics.collapse" => "隐藏详情",
        "htmlImport.diagnostics.more" => "+{{count}} 项",
        "dialog.pptxTitle" => "导出 PowerPoint",
        "dialog.pptxSummary" => "已导出 {{count}} 张幻灯片到：",
        "dialog.pptxEmpty" => "当前演示文稿没有可导出的幻灯片。",
        "settings.agents.acpQuickAdd" => "快速添加",
        "settings.agents.acpPresetAdd" => "添加",
        "settings.agents.acpNotInstalled" => "未安装",
        "assetCenter.title" => "资产中心",
        "assetCenter.tab.templates" => "模板",
        "assetCenter.tab.styles" => "风格",
        "assetCenter.style.empty" => "没有匹配的风格",
        "assetCenter.style.pinned" => "已钉住",
        "assetCenter.style.searchPlaceholder" => "搜索风格或标签",
        "assetCenter.style.generateHint" => "新建一个文档，按主题生成；已钉住的风格会被直接采用。",
        "slidesPanel.tabSlides" => "幻灯片",
        "slidesPanel.tabCards" => "卡片",
        "slidesPanel.present" => "放映",
        _ => return super::zh_cn_collab::lookup(key),
    })
}
