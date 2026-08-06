//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `vi_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Tìm hình ảnh…",
        "imagePanel.searching" => "Đang tìm…",
        "imagePanel.noResults" => "Không có kết quả",
        "imagePanel.searchPrompt" => "Tìm kiếm hình ảnh",
        "imagePanel.sourceNotice" => {
            "Hình ảnh từ {{source}}. Giấy phép tự do — hãy kiểm tra giấy phép trước khi dùng."
        }
        "imagePanel.genNotConfigured" => "Chưa cấu hình tạo hình ảnh",
        "imagePanel.openSettings" => "Mở cài đặt",
        "imagePanel.promptPlaceholder" => "Mô tả hình ảnh…",
        "providerProbe.connectedViaCli" => "Đã kết nối qua CLI {{name}}",
        "providerProbe.cliExitedWithError" => "CLI {{name}} đã thoát với lỗi",
        "providerProbe.cliNoVersionOutput" => "CLI {{name}} không xuất thông tin phiên bản",
        "providerProbe.modelQueryFailed" => {
            "Truy vấn mô hình {{name}} thất bại hoặc hết thời gian chờ"
        }
        "providerProbe.modelQueryFailedRunLogin" => {
            "Truy vấn mô hình {{name}} thất bại. Hãy chạy {{command}} một lần để xác thực."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "Truy vấn mô hình {{name}} cần xác thực. Hãy chạy {{command}} một lần để đăng nhập."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}} đã trả về danh mục mô hình không nhận dạng được"
        }
        "promptCenter.title" => "Trung tâm câu lệnh",
        "promptCenter.searchPlaceholder" => "Tìm câu lệnh…",
        "promptCenter.category.all" => "Tất cả",
        "promptCenter.category.starter" => "Bắt đầu nhanh",
        "promptCenter.category.mobileApp" => "Ứng dụng di động",
        "promptCenter.category.webPage" => "Trang web",
        "promptCenter.category.dashboard" => "Bảng điều khiển",
        "promptCenter.category.component" => "Thành phần",
        "promptCenter.category.modify" => "Chỉnh sửa",
        "promptCenter.category.custom" => "Của tôi",
        "promptCenter.empty" => "Không tìm thấy câu lệnh phù hợp",
        "promptCenter.saveCurrent" => "Lưu nội dung hiện tại thành câu lệnh",
        "promptCenter.saveTitlePlaceholder" => "Nhập tiêu đề câu lệnh",
        "promptCenter.save" => "Lưu",
        "promptCenter.cancel" => "Hủy",
        "promptCenter.delete" => "Xóa",
        "promptCenter.screens" => "{{count}} màn hình",
        "promptCenter.freeform" => "Tự do sáng tạo",
        "promptCenter.item.wander.title" => "Wander · Lập kế hoạch hành trình",
        "promptCenter.item.forage.title" => "Forage · Công thức theo mùa",
        "promptCenter.item.still.title" => "Still · Thiền và giấc ngủ",
        "promptCenter.item.hearth.title" => "Hearth · Nhà thông minh",
        "promptCenter.item.meteo.title" => "Meteo · Thời tiết sống động",
        "promptCenter.item.marginalia.title" => "Marginalia · Đọc và ghi chú",
        "promptCenter.item.lingua.title" => "Lingua · Học ngôn ngữ",
        "promptCenter.item.daybreak.title" => "Daybreak · Đặt cà phê",
        "promptCenter.item.verdant.title" => "Verdant · Chăm sóc cây",
        "promptCenter.item.companion.title" => "Companion · Cuộc sống thú cưng",
        "promptCenter.item.relic.title" => "Relic · Chợ đồ cũ tuyển chọn",
        "promptCenter.item.nocturne.title" => "Nocturne · Hướng dẫn ngắm sao",
        "promptCenter.item.marquee.title" => "Marquee · Danh sách phim",
        "promptCenter.item.ritual.title" => "Ritual · Xây dựng thói quen",
        "promptCenter.item.ember.title" => "Ember · Nhật ký tâm trạng",
        "promptCenter.item.volt.title" => "Volt · Trợ lý xe điện",
        "promptCenter.item.aloft.title" => "Aloft · Theo dõi chuyến bay",
        "promptCenter.item.gallery.title" => "Gallery · Triển lãm và văn hóa",
        "promptCenter.item.nightcap.title" => "Nightcap · Pha chế tại nhà",
        "promptCenter.item.bloom.title" => "Bloom · Nhật ký lớn lên của bé",
        "promptCenter.item.extremeWeather.title" => "Ứng dụng thời tiết · Làm tôi kinh ngạc",
        "promptCenter.item.extremeNowPlaying.title" => "Đang phát · Đẹp đến mức có thể ra mắt",
        "promptCenter.item.extremeDailyApp.title" => "Ứng dụng bạn muốn mở mỗi ngày",
        "promptCenter.item.extremeCalendar.title" => "Tái định nghĩa ứng dụng lịch",
        "promptCenter.item.extremeCalm.title" => "Sự tĩnh lặng trong một màn hình",
        "promptCenter.item.webOrbit.title" => "Orbit · Trang đích bàn làm việc AI",
        "promptCenter.item.webAtelier.title" => "Atelier · Thương mại nội thất",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Bảng phân tích tăng trưởng",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Vận hành logistics",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Bảng dữ liệu doanh nghiệp",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Hệ thống thành phần biểu mẫu",
        "promptCenter.item.modifyPolishCurrent.title" => "Tinh chỉnh màn hình hiện tại",
        "promptCenter.item.modifyCompleteStates.title" => "Hoàn thiện các trạng thái thành phần",
        "sceneTemplate.title" => "Mẫu cảnh",
        "sceneTemplate.searchPlaceholder" => "Tìm cảnh hoặc mẫu…",
        "sceneTemplate.empty" => "Không tìm thấy mẫu phù hợp",
        "sceneTemplate.frames" => "{{count}} trang",
        "sceneTemplate.generate.placeholder" => "Mô tả chủ đề, AI tạo trọn bộ bài trình bày",
        "sceneTemplate.generate.button" => "Tạo",
        "sceneTemplate.generate.hint" => "Một tài liệu mới, dựng từ chủ đề của bạn thành trọn bộ bài trình bày.",
        "sceneTemplate.generate.promptTemplate" => "Hãy tạo một bài trình bày (PPT) về chủ đề sau: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Thêm vào canvas",
        "sceneTemplate.card.generateFrom" => "Tạo theo mẫu này",
        "sceneTemplate.generate.basis" => "Dựa trên: ",
        "sceneTemplate.filter.all" => "Tất cả",
        "sceneTemplate.scene.tutorial" => "Hướng dẫn",
        "sceneTemplate.scene.comparison" => "So sánh",
        "sceneTemplate.scene.carousel" => "Carousel",
        "sceneTemplate.scene.slides" => "Slide",
        "sceneTemplate.scene.card" => "Thẻ",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "Thẻ hướng dẫn 3 bước bằng ảnh chụp màn hình"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Gồm trang bìa, ba bước thao tác và lời kêu gọi hành động ở cuối; chỉ cần thay ảnh chụp màn hình và phần mô tả là có thể đăng."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Chuỗi thẻ kiến thức và góc nhìn",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Gồm trang bìa, ba luận điểm và trang tổng kết, phù hợp để tách một góc nhìn thành chuỗi thẻ liên tiếp có thể vuốt xem."
        }
        "sceneTemplate.item.beforeAfter.title" => "So sánh trước và sau khi thiết kế lại",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Đặt phiên bản trước và sau cạnh nhau, kèm chú thích các thay đổi; phù hợp để nhìn lại dự án và trình bày trong hồ sơ năng lực."
        }
        "sceneTemplate.item.slideDeck.title" => "Bài thuyết trình · 6 trang",
        "sceneTemplate.item.slideDeck.summary" => {
            "Gồm trang bìa, mục lục, ý chính, dữ liệu, biểu đồ và trang kết, theo tỷ lệ trình chiếu 16:9; chỉ cần thay nội dung là sẵn sàng thuyết trình."
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "Thẻ kiến thức · Dọc",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Một thẻ 3:4 duy nhất với tiêu đề, bốn ý chính và dòng ký tên. Thay chữ là đăng được ngay.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Thẻ kiến thức · Vuông",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Thẻ 1:1 theo cùng bố cục, đủ gọn cho ảnh bìa bài viết hoặc bài đăng mạng xã hội.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch deck · Nền tối",
        "sceneTemplate.item.pitchDeckDark.summary" => "Bìa, vấn đề, giải pháp, số liệu, lộ trình và trang liên hệ. Chữ lớn trên nền tối, dành cho gọi vốn và ra mắt sản phẩm.",
        "sceneTemplate.item.lectureDeckLight.title" => "Slide bài giảng · Nền sáng",
        "sceneTemplate.item.lectureDeckLight.summary" => "Bìa bài học, mục tiêu, giảng khái niệm, bài mẫu, bảng so sánh và tổng kết kèm bài tập. Nền trắng ngà, nhìn cả buổi vẫn dễ chịu.",
        "sceneTemplate.item.minimalKeynote.title" => "Keynote tối giản",
        "sceneTemplate.item.minimalKeynote.summary" => "Nhiều khoảng trắng, chữ cực lớn, mỗi trang một câu căn giữa — chín trang không dùng một thẻ nào, mục lục chỉ có đường kẻ mảnh và con số. Dành cho ra mắt và diễn thuyết.",
        "sceneTemplate.item.gradientTech.title" => "Tech chuyển sắc",
        "sceneTemplate.item.gradientTech.summary" => "Nền chuyển sắc tối cùng thẻ kính mờ: kiến trúc, so sánh hiệu năng và tường khách hàng. Dành cho ra mắt sản phẩm cho lập trình viên.",
        "fileMenu.newFromTemplate" => "Tạo mới từ mẫu",
        "collab.ownerConfirm.title" => "Xác nhận bạn đang tham gia phiên của ai",
        "collab.ownerConfirm.hint" => "Chưa có nội dung nào của phiên này được tải.",
        "collab.ownerConfirm.account" => "Tài khoản đã xác minh",
        "collab.ownerConfirm.device" => "Thiết bị đã xác minh",
        "collab.ownerConfirm.claimedName" => "Tên do tài khoản này tự đặt (chưa xác minh)",
        "collab.action.confirmOwner" => "Tham gia phiên này",
        "collab.action.rejectOwner" => "Không tham gia",
        "collab.error.ownerNotConfirmed" => "Bạn chưa xác nhận chủ phiên nên không tải gì cả.",
        "fileMenu.exportSlideshowHtml" => "Xuất trình chiếu HTML...",
        "fileMenu.exportPptx" => "Xuất PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Xuất trình chiếu",
        "dialog.slideshowHtmlSummary" => "Đã xuất {{count}} trang chiếu tới:",
        "dialog.slideshowHtmlEmpty" => "Bản trình bày này không có trang chiếu nào để xuất.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Nội dung HTML có thể nhập không khả dụng.",
        "htmlImport.warn.content.empty_body" => {
            "Nội dung có thể nhập trong phần thân HTML không khả dụng."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML lồng sâu hơn {{max_depth}} cấp đã bị loại bỏ."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Đã đạt giới hạn nút; phần nội dung trang còn lại đã bị lược bỏ."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Đã đạt giới hạn nút; một phần cây HTML đã bị lược bỏ."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Đã đạt giới hạn nút; một hàng định dạng nội dòng đã bị lược bỏ."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Đã đạt giới hạn nút; các phần tử giả được sinh ra đã bị lược bỏ."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "Các quy tắc CSS lồng sâu hơn {{max_depth}} at-rule đã bị bỏ qua."
        }
        "htmlImport.warn.css.unterminated_rule" => "Một quy tắc CSS chưa kết thúc đã bị bỏ qua.",
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "Các quy tắc CSS ::marker không được nhập."
        }
        "htmlImport.warn.css.nesting_unsupported" => "Các quy tắc kiểu CSS lồng nhau đã bị bỏ qua.",
        "htmlImport.warn.css.invalid_layer_name" => {
            "Tên @layer không hợp lệ '{{name}}' đã bị bỏ qua."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "Câu lệnh @{{name}} không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "Các quy tắc @media không có khung nhìn đã bị bỏ qua."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Tên khối @layer không hợp lệ '{{name}}' đã bị bỏ qua."
        }
        "htmlImport.warn.css.unsupported_container_block" => "Khối @container đã bị bỏ qua.",
        "htmlImport.warn.css.unsupported_block" => "Khối @{{name}} không được hỗ trợ đã bị bỏ qua.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "Phông chữ web @font-face '{{family}}' không khả dụng."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Các độ lệch theo phần trăm của một phần tử định vị tuyệt đối đã được xấp xỉ."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Các độ lệch position:relative theo phần trăm đã được xấp xỉ."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "CSS aspect-ratio không có trục xác định đã bị bỏ qua."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "CSS aspect-ratio bên trong khối chứa không xác định đã bị bỏ qua."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky đã bị bỏ qua.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Các dải lưới CSS không được hỗ trợ đã được xấp xỉ."
        }
        "htmlImport.warn.layout.float_ignored" => "CSS float đã bị bỏ qua.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "CSS mix-blend-mode ở cấp nút đã được xấp xỉ."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll đã được xấp xỉ."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "Các lề CSS âm đã bị bỏ qua.",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "Các lề CSS trên một hộp hiển thị đã bị bỏ qua."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Kích thước theo phần trăm của content-box đã được xấp xỉ."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Các ô lưới CSS trống do đường bắt đầu tường minh để lại đã được xấp xỉ."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Một mục lưới CSS có span không vừa với đường bắt đầu đã được xấp xỉ."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Đã đạt giới hạn nút; các lớp bọc hàng lưới CSS đã bị lược bỏ."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "Chiều rộng dải lưới CSS dùng auto-fit / auto-fill đã được xấp xỉ."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "Cách sắp đặt theo CSS grid-template-areas không được nhập."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "Cách sắp đặt theo CSS grid-row không được nhập."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` đã được xấp xỉ."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Các lề auto theo trục khối của CSS không được nhập."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Đã đạt giới hạn nút; căn chỉnh bằng lề auto của CSS đã bị lược bỏ."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Một độ lệch trong luồng CSS trên phần tử không có kích thước xác định đã bị loại bỏ."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Đã đạt giới hạn nút; một độ lệch trong luồng CSS đã bị lược bỏ."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Các độ lệch trong luồng CSS (inset của position:relative, tịnh tiến transform) đã được xấp xỉ."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Một độ lệch trong luồng CSS trên hộp không thể chứa lớp bọc độ lệch đã bị loại bỏ."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap trên vùng chứa flex dạng cột không được nhập."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse đã được xấp xỉ."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap trên vùng chứa không có chiều rộng xác định đã bị bỏ qua."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "CSS align-content trên vùng chứa flex có xuống dòng không được nhập."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap với kích thước trục chính của phần tử con không xác định đã bị bỏ qua."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Đã đạt giới hạn nút; các hàng flex-wrap đã bị lược bỏ."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Cú pháp CSS transform không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Các hàm CSS transform không được hỗ trợ (3D, matrix3d) đã bị bỏ qua."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Một phép tịnh tiến CSS transform theo phần trăm trên trục không xác định đã bị loại bỏ."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Một CSS transform tạo ra ma trận không hữu hạn đã bị bỏ qua."
        }
        "htmlImport.warn.transform.skew_dropped" => "Phép nghiêng của CSS transform đã bị loại bỏ.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Một CSS transform có tỉ lệ bằng không hoặc không hữu hạn đã được xấp xỉ."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "Phép lật gương của CSS transform đã được xấp xỉ."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Độ lệch Z của CSS transform-origin đã bị bỏ qua."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Một tỉ lệ CSS transform không thể gộp vào kích thước nút đã bị loại bỏ."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "Tỉ lệ CSS transform được gộp vào kích thước nút đã được xấp xỉ."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "Tỉ lệ CSS transform trên phần tử có kích thước tự động đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "CSS background-repeat theo hướng hoặc có giãn cách đã được xấp xỉ."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Kích thước ô lát nền CSS khai báo tường minh đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "CSS background-size trên phần tử có kích thước tự động đã được xấp xỉ."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "CSS background-size cần kích thước nội tại của ảnh đã được xấp xỉ."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Một CSS background-position không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Một URL ảnh nền CSS rỗng đã bị bỏ qua."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Các chuyển sắc hình nón của CSS đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Một lớp CSS background-image không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Một màu nền CSS không phân giải được đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS background-position đã bị bỏ qua."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Màu viền CSS theo từng cạnh đã được xấp xỉ."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Các kiểu viền CSS hỗn hợp theo từng cạnh đã được xấp xỉ."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Một kiểu viền CSS phức tạp đã được xấp xỉ."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Một kiểu viền CSS không được hỗ trợ đã được xấp xỉ."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Các bán kính viền CSS dạng elip đã được xấp xỉ."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Một bán kính viền CSS không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Một lớp CSS box-shadow không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "Phương pháp nội suy màu của chuyển sắc CSS đã bị bỏ qua."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Một hướng CSS linear-gradient không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Các điểm gợi ý màu của chuyển sắc CSS đã bị bỏ qua."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Một điểm dừng màu của chuyển sắc CSS không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Một chuyển sắc CSS có ít hơn hai điểm dừng dùng được đã bị bỏ qua."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Một chuyển sắc CSS lặp lại đã được xấp xỉ."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Các điểm dừng chuyển sắc CSS nằm ngoài phạm vi đã được xấp xỉ."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Một bán kính làm mờ CSS không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Một CSS filter drop-shadow() không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Một hàm CSS filter không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Một hàm CSS backdrop-filter không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Một CSS background-blend-mode không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "CSS mix-blend-mode trên từng lớp tô đã được xấp xỉ."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Một CSS mix-blend-mode không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} đã bị bỏ qua.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "CSS background-size trên một chuyển sắc đã bị bỏ qua."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Một vị trí CSS radial-gradient không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Một CSS radial-gradient dạng elip đã được xấp xỉ."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Một từ khóa phạm vi của CSS radial-gradient đã được xấp xỉ."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Một kích thước CSS radial-gradient không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Một lớp CSS text-shadow không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "Các lớp CSS text-shadow sau lớp đầu tiên đã bị bỏ qua."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "CSS text-shadow trên một phần tử nội dòng đã bị bỏ qua."
        }
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image không được nhập.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Một dấu đầu dòng treo `list-style-position: outside` đã được xấp xỉ."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "CSS list-style-type `{{value}}` không được hỗ trợ đã được xấp xỉ."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "CSS object-fit:scale-down đã được xấp xỉ."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none đã bị bỏ qua.",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position đã bị bỏ qua.",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Một CSS mix-blend-mode không được hỗ trợ trên ảnh đã bị bỏ qua."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Một phần tử <svg> nội dòng đã được nhập dưới dạng phần giữ chỗ."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Một kiểu <input> không được hỗ trợ đã được xấp xỉ."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "Phần tử <{{tag}}> đã được nhập dưới dạng phần giữ chỗ."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Một <picture> chỉ có các kiểu nguồn không giải mã được đã được xấp xỉ."
        }
        "htmlImport.warn.table.rowspan_ignored" => "Thuộc tính HTML rowspan không được nhập.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Chiều rộng cột của bảng có nhóm hàng không được CSS làm phẳng đã được xấp xỉ."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Chiều rộng cột của bảng CSS không có chiều rộng xác định đã được xấp xỉ."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "<base href> không hợp lệ {{href}} đã bị bỏ qua."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "<base href> {{href}} nằm ngoài nguồn gốc của dự án đã bị bỏ qua."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "Bảng kiểu bên ngoài {{url}} không khả dụng."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "Ảnh {{url}} nằm ngoài nguồn gốc của dự án đã được nhập dưới dạng phần giữ chỗ."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "Ảnh không khả dụng {{url}} đã được nhập dưới dạng phần giữ chỗ."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "CSS @import {{prelude}} không hợp lệ đã bị bỏ qua."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} không khả dụng."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "CSS @import {{url}} bị lặp vòng đã bị bỏ qua."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "CSS @import {{url}} vượt quá độ sâu {{max_depth}} đã bị bỏ qua."
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} không khả dụng.",
        "htmlImport.warn.project.multiple_html_entries" => {
            "Đã tìm thấy {{count}} mục HTML đầu vào; {{entry}} được chọn và phần còn lại đã được xấp xỉ."
        }
        "htmlImport.warn.snapshot.truncated" => "Một phần ảnh chụp trình duyệt đã bị loại bỏ.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Đã đạt giới hạn nút; phần nội dung ảnh chụp còn lại đã bị lược bỏ."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} ảnh bị nhiễm CORS, giữ dưới dạng URL từ xa, không khả dụng."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Một nút ảnh chụp có khung chữ nhật bị thiếu hoặc không hợp lệ đã bị loại bỏ."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Một nút ảnh chụp thuộc loại không xác định đã bị loại bỏ."
        }
        "htmlImport.warn.snapshot.rejected" => "Ảnh chụp trình duyệt ({{reason}}) đã bị loại bỏ.",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Một phép biến đổi trong ảnh chụp không được hỗ trợ đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_empty_query" => "Một truy vấn @media rỗng đã bị bỏ qua.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Kiểu @media không được hỗ trợ '{{name}}' đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Điều kiện @media không được hỗ trợ '{{input}}' đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Hướng @media không hợp lệ '{{value}}' đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Đặc tính @media không được hỗ trợ '{{name}}' đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Khoảng @media không được hỗ trợ '({{input}})' đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Khoảng @media không hợp lệ '({{input}})' đã bị bỏ qua."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Độ dài @media không hợp lệ '{{value}}' đã bị bỏ qua."
        }
        "htmlImport.diagnostics.title" => "Đã nhập xong HTML",
        "htmlImport.diagnostics.summary" => "Mục bị suy giảm: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Đóng",
        "htmlImport.diagnostics.expand" => "Xem chi tiết",
        "htmlImport.diagnostics.collapse" => "Ẩn chi tiết",
        "htmlImport.diagnostics.more" => "+{{count}} mục nữa",
        "dialog.pptxTitle" => "Xuất PowerPoint",
        "dialog.pptxSummary" => "Đã xuất {{count}} trang chiếu tới:",
        "dialog.pptxEmpty" => "Bản trình bày này không có trang chiếu nào để xuất.",
        "settings.agents.acpQuickAdd" => "Thêm nhanh",
        "settings.agents.acpPresetAdd" => "Thêm",
        "settings.agents.acpNotInstalled" => "Chưa cài đặt",
        "assetCenter.title" => "Trung tâm tài nguyên",
        "assetCenter.tab.templates" => "Mẫu",
        "assetCenter.tab.styles" => "Phong cách",
        "assetCenter.style.empty" => "Không có phong cách phù hợp",
        "assetCenter.style.pinned" => "Đã ghim",
        "assetCenter.style.searchPlaceholder" => "Tìm phong cách hoặc thẻ",
        "assetCenter.style.generateHint" => "Tài liệu mới dựng từ chủ đề của bạn, theo phong cách đã ghim.",
        "ai.pinnedStyle" => "Phong cách: {{name}}",
        "assetCenter.style.import" => "Nhập phong cách",
        "assetCenter.style.mine" => "Phong cách của tôi",
        "assetCenter.style.builtIn" => "Phong cách có sẵn",
        "assetCenter.style.importTitle" => "Nhập DESIGN.md",
        "assetCenter.style.importHint" => "Dán toàn bộ DESIGN.md, rồi xác nhận nhập.",
        "assetCenter.style.importSource" => "Bạn có thể sao chép phong cách từ thư viện DESIGN.md như styles.refero.design.",
        "assetCenter.style.importConfirm" => "Nhập",
        "assetCenter.style.importCancel" => "Hủy",
        "assetCenter.style.importPlaceholder" => "Dán DESIGN.md của bạn vào đây",
        "assetCenter.style.importEmpty" => "Tệp này rỗng, hoặc quá ngắn để là một hướng dẫn phong cách.",
        "assetCenter.style.importNotText" => "Tệp này không đọc được dưới dạng văn bản Markdown.",
        "assetCenter.style.importTooLarge" => "Tệp này lớn hơn 512 KB.",
        "slidesPanel.tabSlides" => "Trang chiếu",
        "slidesPanel.tabCards" => "Thẻ",
        "slidesPanel.present" => "Trình chiếu",
        "settings.agents.heroTitle" => "Kết nối nhà cung cấp AI của bạn",
        "settings.agents.heroSubtitle" => "OpenPencil vận hành các agent CLI cục bộ và nhà cung cấp API của bạn — kết nối một cái để bắt đầu tạo thiết kế.",
        "settings.agents.statusConnected" => "Đã kết nối",
        "settings.agents.statusNotConnected" => "Chưa kết nối",
        "settings.agents.statusChecking" => "Đang kiểm tra…",
        "settings.mcp.heroTitle" => "Kết nối OpenPencil từ bên ngoài qua MCP",
        "settings.mcp.heroSubtitle" => "Trỏ bất kỳ CLI hay trình soạn thảo nào hỗ trợ MCP vào workspace này, rồi điều khiển canvas bằng đúng bộ công cụ của agent tích hợp.",
        "settings.mcp.terminalFootnote" => "* Khi khởi động, MCP được thiết lập tự động cho các công cụ CLI đã chọn.",
        "settings.mcp.customConfigTitle" => "Cấu hình máy chủ MCP tuỳ chỉnh",
        "settings.mcp.customConfigDesc" => "Dán vào bất kỳ client nào đọc khối MCP server chuẩn.",
        "settings.mcp.copyConfig" => "Sao chép cấu hình MCP",
        "settings.system.heroTitle" => "Tuỳ chọn hệ thống",
        "settings.system.heroSubtitle" => "Giao diện, cập nhật và hành vi canvas của bản cài này.",
        "settings.system.appearance" => "Giao diện",
        "settings.system.appearanceLight" => "Sáng",
        "settings.system.appearanceDark" => "Tối",
        "settings.system.pencilCursor" => "Con trỏ bút",
        "settings.images.heroTitle" => "Hình ảnh cho thiết kế",
        "settings.images.heroSubtitle" => "Tìm ảnh trên Openverse, hoặc kết nối nhà cung cấp để tạo ảnh khi cần.",
        "settings.fonts.heroTitle" => "Phông chữ trong tài liệu này",
        "settings.fonts.heroSubtitle" => "Xử lý phông tài liệu cần nhưng máy này thiếu, và quản lý phông bạn đã nhập.",
        "settings.account.heroTitle" => "Tài khoản của bạn",
        "settings.account.heroSubtitle" => "Đăng nhập để đồng bộ workspace và giấy phép giữa các thiết bị.",
        "tooltip.topbar.file" => "Tệp",
        "tooltip.topbar.import" => "Nhập",
        "tooltip.topbar.language" => "Ngôn ngữ",
        "tooltip.topbar.collaboration" => "Cộng tác",
        "tooltip.topbar.preview" => "Xem trước",
        "tooltip.topbar.exitPreview" => "Thoát xem trước",
        "tooltip.topbar.account" => "Tài khoản",
        "settings.agents.providerRollMore" => "và {{count}} nhà cung cấp khác",
        _ => return super::vi_collab::lookup(key),
    })
}
