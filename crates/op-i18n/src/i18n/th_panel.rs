//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `th_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "ค้นหารูปภาพ…",
        "imagePanel.searching" => "กำลังค้นหา…",
        "imagePanel.noResults" => "ไม่พบผลลัพธ์",
        "imagePanel.searchPrompt" => "ค้นหารูปภาพ",
        "imagePanel.sourceNotice" => {
            "รูปภาพจาก {{source}} ใบอนุญาตแบบเสรี — โปรดตรวจสอบใบอนุญาตก่อนใช้งาน"
        }
        "imagePanel.genNotConfigured" => "ยังไม่ได้ตั้งค่าการสร้างรูปภาพ",
        "imagePanel.openSettings" => "เปิดการตั้งค่า",
        "imagePanel.promptPlaceholder" => "อธิบายรูปภาพ…",
        "providerProbe.connectedViaCli" => "เชื่อมต่อผ่าน {{name}} CLI แล้ว",
        "providerProbe.cliExitedWithError" => "{{name}} CLI ปิดการทำงานพร้อมข้อผิดพลาด",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI ไม่ได้แสดงข้อมูลเวอร์ชัน",
        "providerProbe.modelQueryFailed" => "การขอรายการโมเดลของ {{name}} ล้มเหลวหรือหมดเวลา",
        "providerProbe.modelQueryFailedRunLogin" => {
            "การขอรายการโมเดลของ {{name}} ล้มเหลว เรียกใช้ {{command}} หนึ่งครั้งเพื่อยืนยันตัวตน"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "การขอรายการโมเดลของ {{name}} ต้องยืนยันตัวตน เรียกใช้ {{command}} หนึ่งครั้งเพื่อลงชื่อเข้าใช้"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} ส่งคืนรายการโมเดลที่ไม่รู้จัก",
        "promptCenter.title" => "ศูนย์พรอมป์",
        "promptCenter.searchPlaceholder" => "ค้นหาพรอมป์…",
        "promptCenter.category.all" => "ทั้งหมด",
        "promptCenter.category.starter" => "เริ่มต้นอย่างรวดเร็ว",
        "promptCenter.category.mobileApp" => "แอปมือถือ",
        "promptCenter.category.webPage" => "หน้าเว็บ",
        "promptCenter.category.dashboard" => "แดชบอร์ด",
        "promptCenter.category.component" => "คอมโพเนนต์",
        "promptCenter.category.modify" => "ปรับแก้",
        "promptCenter.category.custom" => "ของฉัน",
        "promptCenter.empty" => "ไม่พบพรอมป์ที่ตรงกัน",
        "promptCenter.saveCurrent" => "บันทึกข้อความปัจจุบันเป็นพรอมป์",
        "promptCenter.saveTitlePlaceholder" => "ใส่ชื่อพรอมป์",
        "promptCenter.save" => "บันทึก",
        "promptCenter.cancel" => "ยกเลิก",
        "promptCenter.delete" => "ลบ",
        "promptCenter.screens" => "{{count}} หน้าจอ",
        "promptCenter.freeform" => "อิสระ",
        "promptCenter.item.wander.title" => "Wander · วางแผนการเดินทาง",
        "promptCenter.item.forage.title" => "Forage · สูตรอาหารตามฤดูกาล",
        "promptCenter.item.still.title" => "Still · สมาธิและการนอน",
        "promptCenter.item.hearth.title" => "Hearth · บ้านอัจฉริยะ",
        "promptCenter.item.meteo.title" => "Meteo · สภาพอากาศแบบเต็มอารมณ์",
        "promptCenter.item.marginalia.title" => "Marginalia · อ่านและจดบันทึก",
        "promptCenter.item.lingua.title" => "Lingua · เรียนภาษา",
        "promptCenter.item.daybreak.title" => "Daybreak · สั่งกาแฟ",
        "promptCenter.item.verdant.title" => "Verdant · ดูแลต้นไม้",
        "promptCenter.item.companion.title" => "Companion · ชีวิตสัตว์เลี้ยง",
        "promptCenter.item.relic.title" => "Relic · ตลาดสินค้ามือสองคัดสรร",
        "promptCenter.item.nocturne.title" => "Nocturne · คู่มือดูดาว",
        "promptCenter.item.marquee.title" => "Marquee · รายการภาพยนตร์",
        "promptCenter.item.ritual.title" => "Ritual · สร้างนิสัย",
        "promptCenter.item.ember.title" => "Ember · บันทึกอารมณ์",
        "promptCenter.item.volt.title" => "Volt · ผู้ช่วยรถยนต์ไฟฟ้า",
        "promptCenter.item.aloft.title" => "Aloft · ติดตามเที่ยวบิน",
        "promptCenter.item.gallery.title" => "Gallery · นิทรรศการและวัฒนธรรม",
        "promptCenter.item.nightcap.title" => "Nightcap · ผสมเครื่องดื่มที่บ้าน",
        "promptCenter.item.bloom.title" => "Bloom · บันทึกการเติบโตของลูก",
        "promptCenter.item.extremeWeather.title" => "แอปสภาพอากาศ · ทำให้ฉันทึ่ง",
        "promptCenter.item.extremeNowPlaying.title" => "กำลังเล่น · สวยพร้อมเผยแพร่",
        "promptCenter.item.extremeDailyApp.title" => "แอปที่อยากเปิดทุกวัน",
        "promptCenter.item.extremeCalendar.title" => "ออกแบบปฏิทินขึ้นใหม่",
        "promptCenter.item.extremeCalm.title" => "ความสงบในหนึ่งหน้าจอ",
        "promptCenter.item.webOrbit.title" => "Orbit · หน้าแลนดิ้งเวิร์กเบนช์ AI",
        "promptCenter.item.webAtelier.title" => "Atelier · อีคอมเมิร์ซเฟอร์นิเจอร์",
        "promptCenter.item.dashboardPulse.title" => "Pulse · แดชบอร์ดวิเคราะห์การเติบโต",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · ปฏิบัติการโลจิสติกส์",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · ตารางข้อมูลองค์กร",
        "promptCenter.item.componentFormLab.title" => "Form Lab · ระบบคอมโพเนนต์ฟอร์ม",
        "promptCenter.item.modifyPolishCurrent.title" => "ปรับแต่งหน้าจอปัจจุบัน",
        "promptCenter.item.modifyCompleteStates.title" => "เติมสถานะคอมโพเนนต์ให้ครบ",
        "sceneTemplate.title" => "เทมเพลตฉาก",
        "sceneTemplate.searchPlaceholder" => "ค้นหาฉากหรือเทมเพลต…",
        "sceneTemplate.empty" => "ไม่พบเทมเพลตที่ตรงกัน",
        "sceneTemplate.frames" => "{{count}} หน้า",
        "sceneTemplate.filter.all" => "ทั้งหมด",
        "sceneTemplate.scene.tutorial" => "ภาพสอนใช้งาน",
        "sceneTemplate.scene.comparison" => "ภาพเปรียบเทียบ",
        "sceneTemplate.scene.carousel" => "การ์ดความรู้",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "การ์ดสอนใช้งานด้วยภาพหน้าจอ 3 ขั้นตอน"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "ประกอบด้วยหน้าปก ขั้นตอนการใช้งาน 3 ขั้น และคำกระตุ้นการตัดสินใจตอนท้าย เพียงเปลี่ยนภาพหน้าจอและคำอธิบายก็พร้อมเผยแพร่"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "คารูเซลความรู้และมุมมอง",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "ประกอบด้วยหน้าปก ประเด็นหลัก 3 ข้อ และหน้าสรุป เหมาะสำหรับแยกหนึ่งแนวคิดเป็นการ์ดต่อเนื่องที่ปัดดูได้"
        }
        "sceneTemplate.item.beforeAfter.title" => "เปรียบเทียบก่อนและหลังปรับดีไซน์",
        "sceneTemplate.item.beforeAfter.summary" => {
            "วางภาพก่อนและหลังเทียบกันซ้ายขวา พร้อมคำอธิบายการเปลี่ยนแปลง เหมาะสำหรับการสรุปบทเรียนและนำเสนอในพอร์ตโฟลิโอ"
        }
        "sceneTemplate.item.slideDeck.title" => "งานนำเสนอ · 6 สไลด์",
        "sceneTemplate.item.slideDeck.summary" => {
            "ประกอบด้วยหน้าปก สารบัญ ประเด็นสำคัญ ข้อมูล แผนภูมิ และหน้าปิด ในอัตราส่วนฉายภาพ 16:9 เพียงเปลี่ยนข้อความก็พร้อมนำเสนอ"
        }
        "fileMenu.newFromTemplate" => "สร้างใหม่จากเทมเพลต",
        "collab.ownerConfirm.title" => "ยืนยันว่าคุณกำลังเข้าร่วมกับใคร",
        "collab.ownerConfirm.hint" => "ยังไม่มีเนื้อหาใดจากเซสชันนี้ถูกโหลด",
        "collab.ownerConfirm.account" => "บัญชีที่ยืนยันแล้ว",
        "collab.ownerConfirm.device" => "อุปกรณ์ที่ยืนยันแล้ว",
        "collab.ownerConfirm.claimedName" => "ชื่อที่บัญชีนี้ตั้งเอง (ยังไม่ยืนยัน)",
        "collab.action.confirmOwner" => "เข้าร่วมเซสชันนี้",
        "collab.action.rejectOwner" => "ไม่เข้าร่วม",
        "collab.error.ownerNotConfirmed" => "คุณไม่ได้ยืนยันผู้จัด จึงไม่มีการโหลดข้อมูลใด",
        "fileMenu.exportSlideshowHtml" => "ส่งออกสไลด์โชว์เป็น HTML...",
        "dialog.slideshowHtmlTitle" => "ส่งออกสไลด์โชว์",
        "dialog.slideshowHtmlSummary" => "ส่งออก {{count}} สไลด์ไปยัง:",
        "dialog.slideshowHtmlEmpty" => "งานนำเสนอนี้ไม่มีสไลด์ที่มองเห็นให้ส่งออก",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "เนื้อหา HTML ที่นำเข้าได้ ไม่พร้อมใช้งาน",
        "htmlImport.warn.content.empty_body" => {
            "เนื้อหาที่นำเข้าได้ใน body ของ HTML ไม่พร้อมใช้งาน"
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML ที่ซ้อนลึกเกิน {{max_depth}} ระดับ ถูกตัดออก"
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "ถึงขีดจำกัดจำนวนโหนด เนื้อหาหน้าที่เหลือถูกข้ามไป"
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "ถึงขีดจำกัดจำนวนโหนด บางส่วนของโครงสร้าง HTML ถูกข้ามไป"
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "ถึงขีดจำกัดจำนวนโหนด แถวจัดรูปแบบแบบ inline ถูกข้ามไป"
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "ถึงขีดจำกัดจำนวนโหนด pseudo-element ที่สร้างขึ้นถูกข้ามไป"
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "กฎ CSS ที่ซ้อนลึกเกิน {{max_depth}} at-rule ถูกละเว้น"
        }
        "htmlImport.warn.css.unterminated_rule" => "กฎ CSS ที่ไม่ปิดท้าย ถูกละเว้น",
        "htmlImport.warn.css.marker_rules_unsupported" => "กฎ CSS ::marker ไม่ได้นำเข้า",
        "htmlImport.warn.css.nesting_unsupported" => "กฎสไตล์ CSS แบบซ้อน ถูกละเว้น",
        "htmlImport.warn.css.invalid_layer_name" => {
            "ชื่อ @layer '{{name}}' ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.warn.css.unsupported_statement" => "คำสั่ง @{{name}} ที่ไม่รองรับ ถูกละเว้น",
        "htmlImport.warn.css.media_without_viewport" => "กฎ @media ที่ไม่มี viewport ถูกละเว้น",
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "ชื่อบล็อก @layer '{{name}}' ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.warn.css.unsupported_container_block" => "บล็อก @container ถูกละเว้น",
        "htmlImport.warn.css.unsupported_block" => "บล็อก @{{name}} ที่ไม่รองรับ ถูกละเว้น",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "เว็บฟอนต์ @font-face '{{family}}' ไม่พร้อมใช้งาน"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "ระยะเยื้องแบบเปอร์เซ็นต์ขององค์ประกอบที่จัดตำแหน่งแบบสัมบูรณ์ ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "ระยะเยื้อง position:relative แบบเปอร์เซ็นต์ ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "CSS aspect-ratio ที่ไม่มีแกนขนาดแน่นอน ถูกละเว้น"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "CSS aspect-ratio ภายในบล็อกครอบที่ขนาดไม่แน่นอน ถูกละเว้น"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky ถูกละเว้น",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "แทร็ก CSS grid ที่ไม่รองรับ ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.float_ignored" => "CSS float ถูกละเว้น",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "CSS mix-blend-mode ระดับโหนด ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "ระยะขอบ CSS ที่เป็นค่าลบ ถูกละเว้น",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "ระยะขอบ CSS บนกล่องแสดงผล ถูกละเว้น"
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "การกำหนดขนาดแบบเปอร์เซ็นต์ของ content-box ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "ช่อง CSS grid ที่ว่างจากการระบุเส้นเริ่มต้น ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "รายการ CSS grid ที่ระยะ span ไม่พอดีกับเส้นเริ่มต้น ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "ถึงขีดจำกัดจำนวนโหนด ตัวครอบแถวของ CSS grid ถูกข้ามไป"
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "ความกว้างแทร็ก CSS grid ที่ใช้ auto-fit / auto-fill ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "การจัดวางด้วย CSS grid-template-areas ไม่ได้นำเข้า"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "การจัดวางด้วย CSS grid-row ไม่ได้นำเข้า"
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "ระยะขอบ auto ตามแกนบล็อกของ CSS ไม่ได้นำเข้า"
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "ถึงขีดจำกัดจำนวนโหนด การจัดแนวด้วยระยะขอบ auto ของ CSS ถูกข้ามไป"
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "ระยะเยื้องในโฟลว์ของ CSS บนองค์ประกอบที่ไม่มีขนาดแน่นอน ถูกตัดออก"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "ถึงขีดจำกัดจำนวนโหนด ระยะเยื้องในโฟลว์ของ CSS ถูกข้ามไป"
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "ระยะเยื้องในโฟลว์ของ CSS (ระยะ position:relative, การเลื่อนด้วย transform) ถูกประมาณค่า"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "ระยะเยื้องในโฟลว์ของ CSS บนกล่องที่รองรับตัวครอบระยะเยื้องไม่ได้ ถูกตัดออก"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap บนคอนเทนเนอร์ flex แนวคอลัมน์ ไม่ได้นำเข้า"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse ถูกประมาณค่า",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap บนคอนเทนเนอร์ที่ไม่มีความกว้างแน่นอน ถูกละเว้น"
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "CSS align-content บนคอนเทนเนอร์ flex ที่ตัดขึ้นบรรทัดใหม่ ไม่ได้นำเข้า"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap ที่ขนาดลูกตามแกนหลักไม่แน่นอน ถูกละเว้น"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "ถึงขีดจำกัดจำนวนโหนด แถวของ flex-wrap ถูกข้ามไป"
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "ไวยากรณ์ CSS transform ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "ฟังก์ชัน CSS transform ที่ไม่รองรับ (3D, matrix3d) ถูกละเว้น"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "การเลื่อนแบบเปอร์เซ็นต์ของ CSS transform บนแกนที่ขนาดไม่แน่นอน ถูกตัดออก"
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "CSS transform ที่ให้เมทริกซ์ค่าไม่จำกัด ถูกละเว้น"
        }
        "htmlImport.warn.transform.skew_dropped" => "การเอียงของ CSS transform ถูกตัดออก",
        "htmlImport.warn.transform.degenerate_scale" => {
            "CSS transform ที่มีสเกลเป็นศูนย์หรือค่าไม่จำกัด ถูกประมาณค่า"
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "การกลับด้านของ CSS transform ถูกประมาณค่า"
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "ระยะเยื้องแกน Z ของ CSS transform-origin ถูกละเว้น"
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "สเกลของ CSS transform ที่รวมเข้ากับขนาดโหนดไม่ได้ ถูกตัดออก"
        }
        "htmlImport.warn.transform.scale_baked" => {
            "สเกลของ CSS transform ที่รวมเข้ากับขนาดโหนดแล้ว ถูกประมาณค่า"
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "สเกลของ CSS transform บนองค์ประกอบที่ขนาดเป็น auto ถูกละเว้น"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "CSS background-repeat แบบระบุทิศทางหรือเว้นระยะ ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "ขนาดไทล์พื้นหลังของ CSS ที่ระบุไว้ ถูกละเว้น"
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "CSS background-size บนองค์ประกอบที่ขนาดเป็น auto ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "CSS background-size ที่ต้องใช้ขนาดจริงของภาพ ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "CSS background-position ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "URL ภาพพื้นหลัง CSS ที่ว่างเปล่า ถูกละเว้น"
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => "เกรเดียนต์แบบ conic ของ CSS ถูกละเว้น",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "เลเยอร์ CSS background-image ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "สีพื้นหลัง CSS ที่แปลงค่าไม่ได้ ถูกละเว้น"
        }
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position ถูกละเว้น",
        "htmlImport.warn.visual.border_colors_approximated" => {
            "สีเส้นขอบ CSS แยกรายด้าน ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "รูปแบบเส้นขอบ CSS ที่ผสมกันรายด้าน ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "รูปแบบเส้นขอบ CSS ที่ซับซ้อน ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "รูปแบบเส้นขอบ CSS ที่ไม่รองรับ ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.border_radius_elliptical" => "รัศมีมุม CSS แบบวงรี ถูกประมาณค่า",
        "htmlImport.warn.visual.border_radius_unsupported" => "รัศมีมุม CSS ที่ไม่รองรับ ถูกละเว้น",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "เลเยอร์ CSS box-shadow ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "วิธีการไล่ค่าสีของเกรเดียนต์ CSS ถูกละเว้น"
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "ทิศทาง CSS linear-gradient ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "ตัวช่วยไล่สีของเกรเดียนต์ CSS ถูกละเว้น"
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "จุดสีของเกรเดียนต์ CSS ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "เกรเดียนต์ CSS ที่มีจุดสีใช้งานได้น้อยกว่าสองจุด ถูกละเว้น"
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "เกรเดียนต์ CSS แบบซ้ำ ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "จุดสีของเกรเดียนต์ CSS ที่อยู่นอกช่วง ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => "รัศมีเบลอ CSS ที่ไม่รองรับ ถูกละเว้น",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "CSS filter drop-shadow() ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "ฟังก์ชัน CSS filter ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "ฟังก์ชัน CSS backdrop-filter ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "CSS background-blend-mode ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "CSS mix-blend-mode บนการเติมสีแต่ละชั้น ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "CSS mix-blend-mode ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} ถูกละเว้น",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "CSS background-size บนเกรเดียนต์ ถูกละเว้น"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "ตำแหน่ง CSS radial-gradient ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "CSS radial-gradient แบบวงรี ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "คีย์เวิร์ดขอบเขตของ CSS radial-gradient ถูกประมาณค่า"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "ขนาด CSS radial-gradient ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "เลเยอร์ CSS text-shadow ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "เลเยอร์ CSS text-shadow ถัดจากชั้นแรก ถูกละเว้น"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "CSS text-shadow บนองค์ประกอบแบบ inline ถูกละเว้น"
        }
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image ไม่ได้นำเข้า",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "จุดนำรายการแบบยื่นออกด้วย `list-style-position: outside` ถูกประมาณค่า"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "CSS list-style-type `{{value}}` ที่ไม่รองรับ ถูกประมาณค่า"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down ถูกประมาณค่า",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none ถูกละเว้น",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position ถูกละเว้น",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "CSS mix-blend-mode ที่ไม่รองรับบนภาพ ถูกละเว้น"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "องค์ประกอบ <svg> แบบ inline ถูกนำเข้าเป็นตัวยึดตำแหน่ง"
        }
        "htmlImport.warn.media.input_type_fallback" => "ชนิดของ <input> ที่ไม่รองรับ ถูกประมาณค่า",
        "htmlImport.warn.media.element_placeholder" => {
            "องค์ประกอบ <{{tag}}> ถูกนำเข้าเป็นตัวยึดตำแหน่ง"
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "<picture> ที่มีแต่ชนิดต้นทางซึ่งถอดรหัสไม่ได้ ถูกประมาณค่า"
        }
        "htmlImport.warn.table.rowspan_ignored" => "แอตทริบิวต์ rowspan ของ HTML ไม่ได้นำเข้า",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "ความกว้างคอลัมน์ของตารางที่กลุ่มแถวไม่ถูกยุบรวมโดย CSS ถูกประมาณค่า"
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "ความกว้างคอลัมน์ของตาราง CSS ที่ไม่มีความกว้างแน่นอน ถูกประมาณค่า"
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "<base href> {{href}} ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "<base href> {{href}} ที่อยู่นอกต้นทางของโปรเจกต์ ถูกละเว้น"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "สไตล์ชีตภายนอก {{url}} ไม่พร้อมใช้งาน"
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "ภาพ {{url}} ที่อยู่นอกต้นทางของโปรเจกต์ ถูกนำเข้าเป็นตัวยึดตำแหน่ง"
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "ภาพ {{url}} ที่ไม่พร้อมใช้งาน ถูกนำเข้าเป็นตัวยึดตำแหน่ง"
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "CSS @import {{prelude}} ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} ไม่พร้อมใช้งาน"
        }
        "htmlImport.warn.resource.css_import_cycle" => "CSS @import {{url}} ที่วนซ้ำ ถูกละเว้น",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "CSS @import {{url}} ที่ลึกเกินระดับ {{max_depth}} ถูกละเว้น"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} ไม่พร้อมใช้งาน",
        "htmlImport.warn.project.multiple_html_entries" => {
            "พบจุดเริ่ม HTML {{count}} รายการ เลือกใช้ {{entry}} ส่วนที่เหลือถูกประมาณค่า"
        }
        "htmlImport.warn.snapshot.truncated" => "สแนปช็อตของเบราว์เซอร์บางส่วน ถูกตัดออก",
        "htmlImport.warn.snapshot.node_limit" => {
            "ถึงขีดจำกัดจำนวนโหนด เนื้อหาสแนปช็อตที่เหลือถูกข้ามไป"
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "ภาพที่ติดข้อจำกัด CORS {{count}} ภาพ ซึ่งเก็บไว้เป็น URL ระยะไกล ไม่พร้อมใช้งาน"
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "โหนดสแนปช็อตที่ไม่มีกรอบสี่เหลี่ยมหรือกรอบไม่ถูกต้อง ถูกตัดออก"
        }
        "htmlImport.warn.snapshot.unknown_kind" => "โหนดสแนปช็อตชนิดที่ไม่รู้จัก ถูกตัดออก",
        "htmlImport.warn.snapshot.rejected" => "สแนปช็อตของเบราว์เซอร์ ({{reason}}) ถูกตัดออก",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "การแปลงรูปทรงในสแนปช็อตที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.css.media_empty_query" => "คิวรี @media ที่ว่างเปล่า ถูกละเว้น",
        "htmlImport.warn.css.media_unsupported_type" => {
            "ชนิด @media '{{name}}' ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "เงื่อนไข @media '{{input}}' ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "การวางแนว @media '{{value}}' ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "คุณสมบัติ @media '{{name}}' ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "ช่วงค่า @media '({{input}})' ที่ไม่รองรับ ถูกละเว้น"
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "ช่วงค่า @media '({{input}})' ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "ความยาว @media '{{value}}' ที่ไม่ถูกต้อง ถูกละเว้น"
        }
        "htmlImport.diagnostics.title" => "นำเข้า HTML เสร็จสิ้น",
        "htmlImport.diagnostics.summary" => "รายการที่ลดทอน: {{count}}",
        "htmlImport.diagnostics.dismiss" => "ปิด",
        "htmlImport.diagnostics.expand" => "แสดงรายละเอียด",
        "htmlImport.diagnostics.collapse" => "ซ่อนรายละเอียด",
        "htmlImport.diagnostics.more" => "+{{count}} รายการ",
        _ => return super::th_collab::lookup(key),
    })
}
