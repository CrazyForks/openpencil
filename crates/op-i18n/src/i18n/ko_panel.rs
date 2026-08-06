//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ko_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "이미지 검색…",
        "imagePanel.searching" => "검색 중…",
        "imagePanel.noResults" => "결과가 없습니다",
        "imagePanel.searchPrompt" => "이미지를 검색하세요",
        "imagePanel.sourceNotice" => {
            "{{source}} 제공 이미지. 자유 라이선스 — 사용 전에 라이선스를 확인하세요."
        }
        "imagePanel.genNotConfigured" => "이미지 생성이 설정되지 않았습니다",
        "imagePanel.openSettings" => "설정 열기",
        "imagePanel.promptPlaceholder" => "이미지를 설명하세요…",
        "providerProbe.connectedViaCli" => "{{name}} CLI를 통해 연결됨",
        "providerProbe.cliExitedWithError" => "{{name}} CLI가 오류와 함께 종료되었습니다",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI가 버전 정보를 출력하지 않았습니다",
        "providerProbe.modelQueryFailed" => "{{name}} 모델 조회에 실패했거나 시간이 초과되었습니다",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 모델 조회에 실패했습니다. {{command}}을(를) 한 번 실행해 인증하세요."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 모델 조회에는 인증이 필요합니다. {{command}}을(를) 한 번 실행해 로그인하세요."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}}이(가) 인식할 수 없는 모델 목록을 반환했습니다"
        }
        "promptCenter.title" => "프롬프트 센터",
        "promptCenter.searchPlaceholder" => "프롬프트 검색…",
        "promptCenter.category.all" => "전체",
        "promptCenter.category.starter" => "빠른 시작",
        "promptCenter.category.mobileApp" => "모바일 앱",
        "promptCenter.category.webPage" => "웹 페이지",
        "promptCenter.category.dashboard" => "대시보드",
        "promptCenter.category.component" => "컴포넌트",
        "promptCenter.category.modify" => "수정",
        "promptCenter.category.custom" => "나의 프롬프트",
        "promptCenter.empty" => "일치하는 프롬프트가 없습니다",
        "promptCenter.saveCurrent" => "현재 입력 저장",
        "promptCenter.saveTitlePlaceholder" => "프롬프트 제목",
        "promptCenter.save" => "저장",
        "promptCenter.cancel" => "취소",
        "promptCenter.delete" => "삭제",
        "promptCenter.screens" => "{{count}}개 화면",
        "promptCenter.freeform" => "자유 형식",
        "promptCenter.item.wander.title" => "Wander · 여행 일정",
        "promptCenter.item.forage.title" => "Forage · 제철 레시피",
        "promptCenter.item.still.title" => "Still · 명상과 수면",
        "promptCenter.item.hearth.title" => "Hearth · 스마트 홈",
        "promptCenter.item.meteo.title" => "Meteo · 몰입형 날씨",
        "promptCenter.item.marginalia.title" => "Marginalia · 독서와 주석",
        "promptCenter.item.lingua.title" => "Lingua · 언어 학습",
        "promptCenter.item.daybreak.title" => "Daybreak · 커피 주문",
        "promptCenter.item.verdant.title" => "Verdant · 식물 관리",
        "promptCenter.item.companion.title" => "Companion · 반려동물 생활",
        "promptCenter.item.relic.title" => "Relic · 엄선 중고 마켓",
        "promptCenter.item.nocturne.title" => "Nocturne · 별 관측 가이드",
        "promptCenter.item.marquee.title" => "Marquee · 영화 감상 목록",
        "promptCenter.item.ritual.title" => "Ritual · 습관 만들기",
        "promptCenter.item.ember.title" => "Ember · 감정 일기",
        "promptCenter.item.volt.title" => "Volt · 전기차 동반자",
        "promptCenter.item.aloft.title" => "Aloft · 항공편 추적",
        "promptCenter.item.gallery.title" => "Gallery · 전시와 문화 행사",
        "promptCenter.item.nightcap.title" => "Nightcap · 홈 칵테일",
        "promptCenter.item.bloom.title" => "Bloom · 아이 성장 기록",
        "promptCenter.item.extremeWeather.title" => "극한 · 날씨 앱",
        "promptCenter.item.extremeNowPlaying.title" => "극한 · 지금 재생",
        "promptCenter.item.extremeDailyApp.title" => "극한 · 매일 쓰고 싶은 앱",
        "promptCenter.item.extremeCalendar.title" => "극한 · 캘린더",
        "promptCenter.item.extremeCalm.title" => "극한 · 평온",
        "promptCenter.item.webOrbit.title" => "Orbit · AI 워크벤치 랜딩 페이지",
        "promptCenter.item.webAtelier.title" => "Atelier · 가구 브랜드 커머스",
        "promptCenter.item.dashboardPulse.title" => "Pulse · 성장 분석 대시보드",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 물류 운영",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · 엔터프라이즈 데이터 테이블",
        "promptCenter.item.componentFormLab.title" => "Form Lab · 폼 컴포넌트 시스템",
        "promptCenter.item.modifyPolishCurrent.title" => "현재 화면 다듬기",
        "promptCenter.item.modifyCompleteStates.title" => "컴포넌트 상태 완성하기",
        "collab.ownerConfirm.title" => "참여할 상대를 확인하세요",
        "collab.ownerConfirm.hint" => "이 세션의 내용은 아직 아무것도 불러오지 않았습니다.",
        "collab.ownerConfirm.account" => "검증된 계정",
        "collab.ownerConfirm.device" => "검증된 기기",
        "collab.ownerConfirm.claimedName" => "이 계정이 정한 이름(검증되지 않음)",
        "collab.action.confirmOwner" => "이 세션에 참여",
        "collab.action.rejectOwner" => "참여 안 함",
        "collab.error.ownerNotConfirmed" => "호스트를 확인하지 않아 아무것도 불러오지 않았습니다.",
        "sceneTemplate.title" => "장면 템플릿",
        "sceneTemplate.searchPlaceholder" => "장면 또는 템플릿 검색…",
        "sceneTemplate.empty" => "일치하는 템플릿이 없습니다",
        "sceneTemplate.frames" => "{{count}}페이지",
        "sceneTemplate.generate.placeholder" => "주제를 입력하면 AI가 슬라이드 전체를 생성합니다",
        "sceneTemplate.generate.button" => "생성",
        "sceneTemplate.generate.hint" => "새 문서를 만들고 주제에 맞춰 슬라이드 전체를 생성합니다.",
        "sceneTemplate.generate.promptTemplate" => "다음 주제로 프레젠테이션(PPT)을 만들어 주세요: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "캔버스에 추가",
        "sceneTemplate.card.generateFrom" => "이걸로 생성",
        "sceneTemplate.generate.basis" => "기준: ",
        "sceneTemplate.filter.all" => "전체",
        "sceneTemplate.scene.tutorial" => "튜토리얼",
        "sceneTemplate.scene.comparison" => "비교",
        "sceneTemplate.scene.carousel" => "캐러셀",
        "sceneTemplate.scene.slides" => "슬라이드",
        "sceneTemplate.scene.card" => "카드",
        "sceneTemplate.item.screenshotTutorial.title" => "3단계 스크린샷 튜토리얼 카드",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "표지, 3단계 작업, 마지막 CTA로 구성되어 있어 스크린샷과 설명만 바꾸면 바로 게시할 수 있습니다."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "지식·인사이트 캐러셀",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "표지, 세 가지 논점, 요약 페이지로 구성되어 있어 하나의 관점을 넘겨 보는 연속 카드로 풀어내기에 적합합니다."
        }
        "sceneTemplate.item.beforeAfter.title" => "개편 전후 비교",
        "sceneTemplate.item.beforeAfter.summary" => {
            "전후 화면을 좌우로 나란히 배치하고 변경 설명을 더해, 회고나 작업물 소개에 적합합니다."
        }
        "sceneTemplate.item.slideDeck.title" => "프레젠테이션 · 6페이지",
        "sceneTemplate.item.slideDeck.summary" => {
            "표지, 목차, 핵심 내용, 데이터, 차트, 마무리로 구성된 16:9 발표 자료로, 문구만 바꾸면 바로 발표할 수 있습니다."
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "지식 카드 · 세로형",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "제목, 네 가지 핵심, 서명란을 담은 3:4 단일 카드입니다. 문구만 바꾸면 바로 게시할 수 있습니다.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "지식 카드 · 정사각형",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "같은 레이아웃의 1:1 카드로, 게시글 헤더나 SNS 공유에 알맞은 밀도입니다.",
        "sceneTemplate.item.pitchDeckDark.title" => "피치덱 · 다크",
        "sceneTemplate.item.pitchDeckDark.summary" => "표지, 문제, 해법, 지표, 로드맵, 연락처까지 6장. 어두운 바탕에 큰 글씨로, 투자 유치와 발표에 맞췄습니다.",
        "sceneTemplate.item.lectureDeckLight.title" => "강의 자료 · 라이트",
        "sceneTemplate.item.lectureDeckLight.summary" => "강의 표지, 학습 목표, 개념 설명, 예제, 비교표, 정리와 과제. 종이 같은 흰 바탕이라 오래 봐도 편합니다.",
        "sceneTemplate.item.minimalKeynote.title" => "미니멀 키노트",
        "sceneTemplate.item.minimalKeynote.summary" => "여백과 초대형 타이포로 한 장에 한 문장씩 가운데 정렬. 아홉 장 내내 카드가 없고 목차는 가는 선과 숫자뿐입니다. 신제품 발표와 기조연설용.",
        "sceneTemplate.item.gradientTech.title" => "그라데이션 테크",
        "sceneTemplate.item.gradientTech.summary" => "어두운 그라데이션 바탕에 글래스 카드. 아키텍처·성능 비교·고객사 월까지 갖춘 개발자 제품 발표용.",
        "fileMenu.newFromTemplate" => "템플릿에서 새로 만들기",
        "fileMenu.exportSlideshowHtml" => "슬라이드쇼 HTML 내보내기...",
        "fileMenu.exportPptx" => "PowerPoint 내보내기...",
        "dialog.slideshowHtmlTitle" => "슬라이드쇼 내보내기",
        "dialog.slideshowHtmlSummary" => "슬라이드 {{count}}장을 다음 위치로 내보냈습니다:",
        "dialog.slideshowHtmlEmpty" => "이 프레젠테이션에는 내보낼 슬라이드가 없습니다.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "가져올 수 있는 HTML 콘텐츠를 사용할 수 없습니다.",
        "htmlImport.warn.content.empty_body" => "HTML 본문에서 가져올 수 있는 콘텐츠를 사용할 수 없습니다.",
        "htmlImport.warn.content.dom_depth_truncated" => "{{max_depth}}단계보다 깊게 중첩된 HTML이 제거되었습니다.",
        "htmlImport.warn.content.node_limit_truncated" => "노드 한도에 도달하여 남은 페이지 콘텐츠가 생략되었습니다.",
        "htmlImport.warn.content.node_limit_mapping" => "노드 한도에 도달하여 HTML 트리의 일부가 생략되었습니다.",
        "htmlImport.warn.content.node_limit_inline_row" => "노드 한도에 도달하여 인라인 서식 행이 생략되었습니다.",
        "htmlImport.warn.content.node_limit_pseudo" => "노드 한도에 도달하여 생성된 의사 요소가 생략되었습니다.",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "at-규칙 {{max_depth}}단계보다 깊게 중첩된 CSS 규칙이 무시되었습니다."
        }
        "htmlImport.warn.css.unterminated_rule" => "닫히지 않은 CSS 규칙이 무시되었습니다.",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 규칙을 가져오지 않았습니다.",
        "htmlImport.warn.css.nesting_unsupported" => "중첩된 CSS 스타일 규칙이 무시되었습니다.",
        "htmlImport.warn.css.invalid_layer_name" => "잘못된 @layer 이름('{{name}}')이 무시되었습니다.",
        "htmlImport.warn.css.unsupported_statement" => "지원되지 않는 @{{name}} 구문이 무시되었습니다.",
        "htmlImport.warn.css.media_without_viewport" => "뷰포트가 없는 @media 규칙이 무시되었습니다.",
        "htmlImport.warn.css.invalid_layer_block_name" => "잘못된 @layer 블록 이름('{{name}}')이 무시되었습니다.",
        "htmlImport.warn.css.unsupported_container_block" => "@container 블록이 무시되었습니다.",
        "htmlImport.warn.css.unsupported_block" => "지원되지 않는 @{{name}} 블록이 무시되었습니다.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face 웹 글꼴('{{family}}')을 사용할 수 없습니다."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "절대 위치 요소의 백분율 오프셋이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "백분율 position:relative 오프셋이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "확정된 축이 없는 CSS aspect-ratio가 무시되었습니다."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "확정되지 않은 포함 블록 안의 CSS aspect-ratio가 무시되었습니다."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky가 무시되었습니다.",
        "htmlImport.warn.layout.grid_tracks_approximated" => "지원되지 않는 CSS grid 트랙이 근사 처리되었습니다.",
        "htmlImport.warn.layout.float_ignored" => "CSS float가 무시되었습니다.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "노드 수준의 CSS mix-blend-mode가 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "음수 CSS 여백이 무시되었습니다.",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "시각적 박스의 CSS 여백이 무시되었습니다.",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box 백분율 크기 지정이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "명시적 시작 선으로 생긴 빈 CSS grid 셀이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => "스팬이 시작 선에 맞지 않는 CSS grid 항목이 근사 처리되었습니다.",
        "htmlImport.warn.layout.grid_rows_node_limit" => "노드 한도에 도달하여 CSS grid 행 래퍼가 생략되었습니다.",
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "auto-fit / auto-fill을 사용하는 CSS grid 트랙 너비가 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas 배치를 가져오지 않았습니다."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row 배치를 가져오지 않았습니다.",
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column 값(`{{value}}`)이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => "CSS 블록 축 auto 여백을 가져오지 않았습니다.",
        "htmlImport.warn.layout.auto_margin_node_limit" => "노드 한도에 도달하여 CSS auto 여백 정렬이 생략되었습니다.",
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "크기가 확정되지 않은 요소의 CSS 흐름 내 오프셋이 제거되었습니다."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => "노드 한도에 도달하여 CSS 흐름 내 오프셋이 생략되었습니다.",
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS 흐름 내 오프셋(position:relative 인셋, transform 이동)이 근사 처리되었습니다."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "오프셋 래퍼를 넣을 수 없는 박스의 CSS 흐름 내 오프셋이 제거되었습니다."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "column 방향 flex 컨테이너의 flex-wrap을 가져오지 않았습니다."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse가 근사 처리되었습니다.",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "너비가 확정되지 않은 컨테이너의 flex-wrap이 무시되었습니다."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "줄바꿈되는 flex 컨테이너의 CSS align-content를 가져오지 않았습니다."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "주축 크기가 확정되지 않은 자식이 있는 flex-wrap이 무시되었습니다."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => "노드 한도에 도달하여 flex-wrap 행이 생략되었습니다.",
        "htmlImport.warn.transform.unsupported_syntax" => "지원되지 않는 CSS transform 구문이 무시되었습니다.",
        "htmlImport.warn.transform.unsupported_function" => {
            "지원되지 않는 CSS transform 함수(3D, matrix3d)가 무시되었습니다."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "확정되지 않은 축의 백분율 CSS transform 이동이 제거되었습니다."
        }
        "htmlImport.warn.transform.non_finite_matrix" => "유한하지 않은 행렬을 만드는 CSS transform이 무시되었습니다.",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform skew가 제거되었습니다.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "배율이 0이거나 유한하지 않은 CSS transform이 근사 처리되었습니다."
        }
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform 대칭 이동이 근사 처리되었습니다.",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin의 Z 오프셋이 무시되었습니다.",
        "htmlImport.warn.transform.scale_not_baked" => "노드 크기에 반영할 수 없는 CSS transform 배율이 제거되었습니다.",
        "htmlImport.warn.transform.scale_baked" => "노드 크기에 반영된 CSS transform 배율이 근사 처리되었습니다.",
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "자동 크기 요소의 CSS transform 배율이 무시되었습니다."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "방향 지정 또는 간격 CSS background-repeat가 근사 처리되었습니다."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => "명시적 CSS 배경 타일 크기가 무시되었습니다.",
        "htmlImport.warn.visual.background_size_auto_box" => {
            "자동 크기 요소의 CSS background-size가 근사 처리되었습니다."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "이미지의 고유 크기가 필요한 CSS background-size가 근사 처리되었습니다."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "지원되지 않는 CSS background-position이 무시되었습니다."
        }
        "htmlImport.warn.visual.background_image_url_empty" => "비어 있는 CSS 배경 이미지 URL이 무시되었습니다.",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS 원뿔형 그라데이션이 무시되었습니다.",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "지원되지 않는 CSS background-image 레이어가 무시되었습니다."
        }
        "htmlImport.warn.visual.background_color_unresolved" => "확인할 수 없는 CSS 배경색이 무시되었습니다.",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position이 무시되었습니다.",
        "htmlImport.warn.visual.border_colors_approximated" => "면별 CSS 테두리 색상이 근사 처리되었습니다.",
        "htmlImport.warn.visual.border_styles_approximated" => "면별로 다른 CSS 테두리 스타일이 근사 처리되었습니다.",
        "htmlImport.warn.visual.border_style_complex" => "복잡한 CSS 테두리 스타일이 근사 처리되었습니다.",
        "htmlImport.warn.visual.border_style_unsupported" => "지원되지 않는 CSS 테두리 스타일이 근사 처리되었습니다.",
        "htmlImport.warn.visual.border_radius_elliptical" => "타원형 CSS 테두리 반경이 근사 처리되었습니다.",
        "htmlImport.warn.visual.border_radius_unsupported" => "지원되지 않는 CSS 테두리 반경이 무시되었습니다.",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "지원되지 않는 CSS box-shadow 레이어가 무시되었습니다."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS 그라데이션 색상 보간 방식이 무시되었습니다.",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "지원되지 않는 CSS linear-gradient 방향이 무시되었습니다."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS 그라데이션 색상 힌트가 무시되었습니다.",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "지원되지 않는 CSS 그라데이션 색상 정지점이 무시되었습니다."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "사용 가능한 정지점이 두 개 미만인 CSS 그라데이션이 무시되었습니다."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => "반복되는 CSS 그라데이션이 근사 처리되었습니다.",
        "htmlImport.warn.visual.gradient_stops_clamped" => "범위를 벗어난 CSS 그라데이션 정지점이 근사 처리되었습니다.",
        "htmlImport.warn.visual.blur_radius_unsupported" => "지원되지 않는 CSS 흐림 반경이 무시되었습니다.",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "지원되지 않는 CSS 필터 drop-shadow()가 무시되었습니다."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "지원되지 않는 CSS 필터 함수가 무시되었습니다.",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "지원되지 않는 CSS backdrop-filter 함수가 무시되었습니다."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "지원되지 않는 CSS background-blend-mode가 무시되었습니다."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "개별 채우기의 CSS mix-blend-mode가 근사 처리되었습니다."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "지원되지 않는 CSS mix-blend-mode가 무시되었습니다."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS 속성({{property}})이 무시되었습니다.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "그라데이션에 적용된 CSS background-size가 무시되었습니다."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "지원되지 않는 CSS radial-gradient 위치가 무시되었습니다."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "타원형 CSS radial-gradient가 근사 처리되었습니다."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient 범위 키워드가 근사 처리되었습니다."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "지원되지 않는 CSS radial-gradient 크기가 무시되었습니다."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "지원되지 않는 CSS text-shadow 레이어가 무시되었습니다.",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "첫 번째 이후의 CSS text-shadow 레이어가 무시되었습니다."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "인라인 요소의 CSS text-shadow가 무시되었습니다.",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image를 가져오지 않았습니다.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` 내어쓰기 마커가 근사 처리되었습니다."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "지원되지 않는 CSS list-style-type 값(`{{value}}`)이 근사 처리되었습니다."
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down이 근사 처리되었습니다.",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none이 무시되었습니다.",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position이 무시되었습니다.",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "이미지에 적용된 지원되지 않는 CSS mix-blend-mode가 무시되었습니다."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "인라인 <svg> 요소를 자리 표시자로 가져왔습니다.",
        "htmlImport.warn.media.input_type_fallback" => "지원되지 않는 <input> 유형이 근사 처리되었습니다.",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 요소를 자리 표시자로 가져왔습니다.",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "디코딩할 수 없는 소스 유형만 있는 <picture>가 근사 처리되었습니다."
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML rowspan 속성을 가져오지 않았습니다.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "CSS가 행 그룹을 평탄화하지 않은 표의 열 너비가 근사 처리되었습니다."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "너비가 확정되지 않은 CSS 표의 열 너비가 근사 처리되었습니다."
        }
        "htmlImport.warn.resource.invalid_base_href" => "잘못된 <base href> 값({{href}})이 무시되었습니다.",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "프로젝트 출처를 벗어난 <base href> 값({{href}})이 무시되었습니다."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "외부 스타일시트({{url}})를 사용할 수 없습니다.",
        "htmlImport.warn.resource.image_outside_origin" => {
            "프로젝트 출처를 벗어난 이미지({{url}})를 자리 표시자로 가져왔습니다."
        }
        "htmlImport.warn.resource.image_unavailable" => "사용할 수 없는 이미지({{url}})를 자리 표시자로 가져왔습니다.",
        "htmlImport.warn.resource.css_import_invalid" => {
            "잘못된 CSS @import 구문({{prelude}})이 무시되었습니다."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import 참조({{reference}})를 사용할 수 없습니다."
        }
        "htmlImport.warn.resource.css_import_cycle" => "순환하는 CSS @import 주소({{url}})가 무시되었습니다.",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "깊이 {{max_depth}}단계를 넘어선 CSS @import 주소({{url}})가 무시되었습니다."
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import 주소({{url}})를 사용할 수 없습니다.",
        "htmlImport.warn.project.multiple_html_entries" => {
            "HTML 진입점 {{count}}개 중 {{entry}} 항목을 선택했고 나머지는 근사 처리되었습니다."
        }
        "htmlImport.warn.snapshot.truncated" => "브라우저 스냅샷의 일부가 제거되었습니다.",
        "htmlImport.warn.snapshot.node_limit" => "노드 한도에 도달하여 남은 스냅샷 콘텐츠가 생략되었습니다.",
        "htmlImport.warn.snapshot.tainted_images" => {
            "원격 URL로 유지된 CORS 오염 이미지 {{count}}개를 사용할 수 없습니다."
        }
        "htmlImport.warn.snapshot.invalid_rect" => "사각형 정보가 없거나 잘못된 스냅샷 노드가 제거되었습니다.",
        "htmlImport.warn.snapshot.unknown_kind" => "알 수 없는 종류의 스냅샷 노드가 제거되었습니다.",
        "htmlImport.warn.snapshot.rejected" => "브라우저 스냅샷({{reason}})이 제거되었습니다.",
        "htmlImport.warn.snapshot.unsupported_transform" => "지원되지 않는 스냅샷 변환이 무시되었습니다.",
        "htmlImport.warn.css.media_empty_query" => "비어 있는 @media 쿼리가 무시되었습니다.",
        "htmlImport.warn.css.media_unsupported_type" => "지원되지 않는 @media 유형('{{name}}')이 무시되었습니다.",
        "htmlImport.warn.css.media_unsupported_condition" => {
            "지원되지 않는 @media 조건('{{input}}')이 무시되었습니다."
        }
        "htmlImport.warn.css.media_invalid_orientation" => "잘못된 @media 방향('{{value}}')이 무시되었습니다.",
        "htmlImport.warn.css.media_unsupported_feature" => {
            "지원되지 않는 @media 기능('{{name}}')이 무시되었습니다."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "지원되지 않는 @media 범위 '({{input}})' 조건이 무시되었습니다."
        }
        "htmlImport.warn.css.media_invalid_range" => "잘못된 @media 범위 '({{input}})' 조건이 무시되었습니다.",
        "htmlImport.warn.css.media_invalid_length" => "잘못된 @media 길이('{{value}}')가 무시되었습니다.",
        "htmlImport.diagnostics.title" => "HTML 가져오기 완료",
        "htmlImport.diagnostics.summary" => "품질 저하 항목: {{count}}",
        "htmlImport.diagnostics.dismiss" => "닫기",
        "htmlImport.diagnostics.expand" => "세부 정보 보기",
        "htmlImport.diagnostics.collapse" => "세부 정보 숨기기",
        "htmlImport.diagnostics.more" => "+{{count}}개 더",
        "dialog.pptxTitle" => "PowerPoint 내보내기",
        "dialog.pptxSummary" => "슬라이드 {{count}}장을 다음 위치로 내보냈습니다:",
        "dialog.pptxEmpty" => "이 프레젠테이션에는 내보낼 슬라이드가 없습니다.",
        "settings.agents.acpQuickAdd" => "빠른 추가",
        "settings.agents.acpPresetAdd" => "추가",
        "settings.agents.acpNotInstalled" => "설치되지 않음",
        "assetCenter.title" => "에셋 센터",
        "assetCenter.tab.templates" => "템플릿",
        "assetCenter.tab.styles" => "스타일",
        "assetCenter.style.empty" => "일치하는 스타일이 없습니다",
        "assetCenter.style.pinned" => "고정됨",
        "assetCenter.style.searchPlaceholder" => "스타일 또는 태그 검색",
        "assetCenter.style.generateHint" => "주제로 새 문서를 생성합니다. 고정한 스타일이 그대로 사용됩니다.",
        "ai.pinnedStyle" => "스타일: {{name}}",
        "assetCenter.style.import" => "스타일 가져오기",
        "assetCenter.style.mine" => "내 스타일",
        "assetCenter.style.builtIn" => "기본 제공 스타일",
        "assetCenter.style.importTitle" => "DESIGN.md 가져오기",
        "assetCenter.style.importHint" => "DESIGN.md 전문을 붙여 넣은 뒤 가져오기를 확인하세요.",
        "assetCenter.style.importSource" => "styles.refero.design 같은 DESIGN.md 라이브러리에서 내용을 복사할 수 있습니다.",
        "assetCenter.style.importConfirm" => "가져오기",
        "assetCenter.style.importCancel" => "취소",
        "assetCenter.style.importPlaceholder" => "여기에 DESIGN.md 붙여 넣기",
        "assetCenter.style.importEmpty" => "이 파일은 비어 있거나 스타일 가이드로 보기에는 너무 짧습니다.",
        "assetCenter.style.importNotText" => "이 파일은 Markdown 텍스트로 읽을 수 없습니다.",
        "assetCenter.style.importTooLarge" => "이 파일은 512 KB를 넘습니다.",
        "slidesPanel.tabSlides" => "슬라이드",
        "slidesPanel.tabCards" => "카드",
        "slidesPanel.present" => "발표",
        "settings.agents.heroTitle" => "AI 제공자 연결",
        "settings.agents.heroSubtitle" => "OpenPencil은 로컬 CLI 에이전트와 API 제공자를 직접 구동합니다. 하나를 연결하면 디자인 생성을 시작할 수 있습니다.",
        "settings.agents.statusConnected" => "연결됨",
        "settings.agents.statusNotConnected" => "연결 안 됨",
        "settings.agents.statusChecking" => "상태 확인 중…",
        "settings.mcp.heroTitle" => "외부에서 MCP로 OpenPencil 연결",
        "settings.mcp.heroSubtitle" => "MCP를 지원하는 CLI나 편집기를 이 워크스페이스로 연결하면 내장 에이전트와 같은 도구로 캔버스를 다룰 수 있습니다.",
        "settings.mcp.terminalFootnote" => "* 시작할 때 선택한 CLI 도구에 MCP가 자동으로 설정됩니다.",
        "settings.mcp.customConfigTitle" => "사용자 지정 MCP 서버 구성",
        "settings.mcp.customConfigDesc" => "표준 MCP server 블록을 읽는 클라이언트에 그대로 붙여 넣으세요.",
        "settings.mcp.copyConfig" => "MCP 구성 복사",
        "settings.system.heroTitle" => "시스템 환경설정",
        "settings.system.heroSubtitle" => "이 설치본의 모양, 업데이트, 캔버스 동작.",
        "settings.system.appearance" => "모양",
        "settings.system.appearanceLight" => "라이트",
        "settings.system.appearanceDark" => "다크",
        "settings.system.pencilCursor" => "펜 커서",
        "settings.images.heroTitle" => "디자인에 넣을 이미지",
        "settings.images.heroSubtitle" => "Openverse에서 사진을 검색하거나 공급자를 연결해 필요할 때 생성하세요.",
        "settings.fonts.heroTitle" => "이 문서의 글꼴",
        "settings.fonts.heroSubtitle" => "문서가 요구하지만 이 컴퓨터에 없는 글꼴을 해결하고, 가져온 글꼴을 관리합니다.",
        "settings.account.heroTitle" => "내 계정",
        "settings.account.heroSubtitle" => "로그인하면 여러 기기에서 워크스페이스와 라이선스를 동기화할 수 있습니다.",
        "tooltip.topbar.file" => "파일",
        "tooltip.topbar.import" => "가져오기",
        "tooltip.topbar.language" => "언어",
        "tooltip.topbar.collaboration" => "협업",
        "tooltip.topbar.preview" => "미리보기",
        "tooltip.topbar.exitPreview" => "미리보기 종료",
        "tooltip.topbar.account" => "계정",
        "settings.agents.providerRollMore" => "외 {{count}}곳",
        _ => return super::ko_collab::lookup(key),
    })
}
