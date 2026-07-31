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
        _ => return super::vi_collab::lookup(key),
    })
}
