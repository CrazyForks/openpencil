//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `ja_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "画像を検索…",
        "imagePanel.searching" => "検索中…",
        "imagePanel.noResults" => "結果が見つかりません",
        "imagePanel.searchPrompt" => "画像を検索",
        "imagePanel.sourceNotice" => {
            "画像の提供元: {{source}}。自由ライセンス — 使用前にライセンスをご確認ください。"
        }
        "imagePanel.genNotConfigured" => "画像生成が未設定です",
        "imagePanel.openSettings" => "設定を開く",
        "imagePanel.promptPlaceholder" => "画像の内容を入力…",
        "providerProbe.connectedViaCli" => "{{name}} CLI 経由で接続しました",
        "providerProbe.cliExitedWithError" => "{{name}} CLI がエラーで終了しました",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI がバージョン情報を出力しませんでした",
        "providerProbe.modelQueryFailed" => "{{name}} のモデル取得に失敗またはタイムアウトしました",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} のモデル取得に失敗しました。{{command}} を一度実行して認証してください。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} のモデル取得には認証が必要です。{{command}} を一度実行してサインインしてください。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} が認識できないモデル一覧を返しました",
        "promptCenter.title" => "プロンプトセンター",
        "promptCenter.searchPlaceholder" => "プロンプトを検索…",
        "promptCenter.category.all" => "すべて",
        "promptCenter.category.starter" => "はじめに",
        "promptCenter.category.mobileApp" => "モバイルアプリ",
        "promptCenter.category.webPage" => "Web ページ",
        "promptCenter.category.dashboard" => "ダッシュボード",
        "promptCenter.category.component" => "コンポーネント",
        "promptCenter.category.modify" => "修正",
        "promptCenter.category.custom" => "マイプロンプト",
        "promptCenter.empty" => "一致するプロンプトがありません",
        "promptCenter.saveCurrent" => "現在の入力を保存",
        "promptCenter.saveTitlePlaceholder" => "プロンプト名",
        "promptCenter.save" => "保存",
        "promptCenter.cancel" => "キャンセル",
        "promptCenter.delete" => "削除",
        "promptCenter.screens" => "{{count}}画面",
        "promptCenter.freeform" => "自由形式",
        "promptCenter.item.wander.title" => "Wander · 旅行プラン",
        "promptCenter.item.forage.title" => "Forage · 旬のレシピ",
        "promptCenter.item.still.title" => "Still · 瞑想と就寝",
        "promptCenter.item.hearth.title" => "Hearth · スマートホーム",
        "promptCenter.item.meteo.title" => "Meteo · 没入型天気",
        "promptCenter.item.marginalia.title" => "Marginalia · 読書と注釈",
        "promptCenter.item.lingua.title" => "Lingua · 言語学習",
        "promptCenter.item.daybreak.title" => "Daybreak · コーヒー注文",
        "promptCenter.item.verdant.title" => "Verdant · 植物ケア",
        "promptCenter.item.companion.title" => "Companion · ペットライフ",
        "promptCenter.item.relic.title" => "Relic · 厳選リユース市場",
        "promptCenter.item.nocturne.title" => "Nocturne · 星空観察ガイド",
        "promptCenter.item.marquee.title" => "Marquee · 映画ウォッチリスト",
        "promptCenter.item.ritual.title" => "Ritual · 習慣づくり",
        "promptCenter.item.ember.title" => "Ember · 気分日記",
        "promptCenter.item.volt.title" => "Volt · EV コンパニオン",
        "promptCenter.item.aloft.title" => "Aloft · フライト追跡",
        "promptCenter.item.gallery.title" => "Gallery · 展覧会と文化イベント",
        "promptCenter.item.nightcap.title" => "Nightcap · ホームカクテル",
        "promptCenter.item.bloom.title" => "Bloom · 子どもの成長記録",
        "promptCenter.item.extremeWeather.title" => "極限 · 天気アプリ",
        "promptCenter.item.extremeNowPlaying.title" => "極限 · 再生中",
        "promptCenter.item.extremeDailyApp.title" => "極限 · 毎日使いたいアプリ",
        "promptCenter.item.extremeCalendar.title" => "極限 · カレンダー",
        "promptCenter.item.extremeCalm.title" => "極限 · 静けさ",
        "promptCenter.item.webOrbit.title" => "Orbit · AI ワークベンチのランディングページ",
        "promptCenter.item.webAtelier.title" => "Atelier · 家具ブランドの EC サイト",
        "promptCenter.item.dashboardPulse.title" => "Pulse · グロース分析ダッシュボード",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · 物流オペレーション",
        "promptCenter.item.componentDataGrid.title" => {
            "Gridworks · エンタープライズデータテーブル"
        }
        "promptCenter.item.componentFormLab.title" => "Form Lab · フォームコンポーネントシステム",
        "promptCenter.item.modifyPolishCurrent.title" => "現在の画面を磨く",
        "promptCenter.item.modifyCompleteStates.title" => "コンポーネント状態を補完",
        "collab.ownerConfirm.title" => "参加先の相手を確認してください",
        "collab.ownerConfirm.hint" => "このセッションの内容はまだ何も読み込まれていません。",
        "collab.ownerConfirm.account" => "検証済みアカウント",
        "collab.ownerConfirm.device" => "検証済みデバイス",
        "collab.ownerConfirm.claimedName" => "このアカウントが設定した名前（未検証）",
        "collab.action.confirmOwner" => "このセッションに参加",
        "collab.action.rejectOwner" => "参加しない",
        "collab.error.ownerNotConfirmed" => "ホストを確認しなかったため、何も読み込まれませんでした。",
        "sceneTemplate.title" => "シーンテンプレート",
        "sceneTemplate.searchPlaceholder" => "シーンやテンプレートを検索…",
        "sceneTemplate.empty" => "一致するテンプレートがありません",
        "sceneTemplate.frames" => "{{count}}ページ",
        "sceneTemplate.generate.placeholder" => "テーマを入力すると、AI がスライド一式を生成します",
        "sceneTemplate.generate.button" => "生成",
        "sceneTemplate.generate.hint" => "新しいドキュメントを作成し、テーマからスライド一式を生成します。",
        "sceneTemplate.generate.promptTemplate" => "次のテーマでプレゼンテーション（PPT）を作成してください：{{topic}}",
        "sceneTemplate.filter.all" => "すべて",
        "sceneTemplate.scene.tutorial" => "チュートリアル",
        "sceneTemplate.scene.comparison" => "比較",
        "sceneTemplate.scene.carousel" => "カルーセル",
        "sceneTemplate.scene.slides" => "スライド",
        "sceneTemplate.scene.card" => "カード",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "3ステップのスクリーンショットチュートリアルカード"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "表紙、3つの操作ステップ、最後のCTAで構成。スクリーンショットと説明を差し替えるだけで公開できます。"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "ナレッジ・インサイトカルーセル",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "表紙、3つの論点、まとめページで構成。1つの主張をスワイプできる連続カードに展開するのに最適です。"
        }
        "sceneTemplate.item.beforeAfter.title" => "リニューアル前後の比較",
        "sceneTemplate.item.beforeAfter.summary" => {
            "左右に並べたビフォー・アフターに変更内容を添え、振り返りや作品紹介に最適です。"
        }
        "sceneTemplate.item.slideDeck.title" => "プレゼンテーション · 6ページ",
        "sceneTemplate.item.slideDeck.summary" => {
            "表紙、目次、要点、データ、グラフ、締めの6ページ構成。16:9の投影比率で、テキストを差し替えるだけで発表できます。"
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "ナレッジカード · 縦型",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "見出し・4つの要点・署名欄をまとめた3:4の1枚カード。文言を差し替えるだけで投稿できます。",
        "sceneTemplate.item.knowledgeCardSquare.title" => "ナレッジカード · 正方形",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "同じレイアウトの1:1カード。記事のヘッダー画像やSNS投稿に収まる密度です。",
        "sceneTemplate.item.pitchDeckDark.title" => "ピッチデッキ · ダーク",
        "sceneTemplate.item.pitchDeckDark.summary" => "表紙、課題、ソリューション、数字、ロードマップ、連絡先の6枚。暗い地に大きな文字で、資金調達や発表会向けです。",
        "sceneTemplate.item.lectureDeckLight.title" => "授業スライド · ライト",
        "sceneTemplate.item.lectureDeckLight.summary" => "講義表紙、学習目標、概念解説、例題、比較表、まとめと課題。紙のような白地で、90分見続けても疲れません。",
        "sceneTemplate.item.minimalKeynote.title" => "ミニマル Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => "余白と特大の文字で、1 枚に 1 つの主張。8 枚を通してカードは一つも使いません。発表会や基調講演向け。",
        "sceneTemplate.item.gradientTech.title" => "グラデーション テック",
        "sceneTemplate.item.gradientTech.summary" => "ダークなグラデーション地にすりガラスのカード。構成図・性能比較・導入企業の枠まで入った開発者向け発表テンプレート。",
        "fileMenu.newFromTemplate" => "テンプレートから新規作成",
        "fileMenu.exportSlideshowHtml" => "スライドショー HTML を書き出し...",
        "fileMenu.exportPptx" => "PowerPoint を書き出し...",
        "dialog.slideshowHtmlTitle" => "スライドショーを書き出し",
        "dialog.slideshowHtmlSummary" => "{{count}} 枚のスライドを次の場所に書き出しました:",
        "dialog.slideshowHtmlEmpty" => "このプレゼンテーションには書き出せるスライドがありません。",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "インポート可能な HTML コンテンツは利用できません。",
        "htmlImport.warn.content.empty_body" => "HTML 本文内にインポート可能なコンテンツは利用できません。",
        "htmlImport.warn.content.dom_depth_truncated" => "{{max_depth}} 階層より深くネストした HTML を破棄しました。",
        "htmlImport.warn.content.node_limit_truncated" => "ノード上限に達したため、残りのページ内容を省略しました。",
        "htmlImport.warn.content.node_limit_mapping" => "ノード上限に達したため、HTML ツリーの一部を省略しました。",
        "htmlImport.warn.content.node_limit_inline_row" => "ノード上限に達したため、インライン整形行を省略しました。",
        "htmlImport.warn.content.node_limit_pseudo" => "ノード上限に達したため、生成された擬似要素を省略しました。",
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "{{max_depth}} 層より深いアット規則にネストした CSS 規則を無視しました。"
        }
        "htmlImport.warn.css.unterminated_rule" => "終端のない CSS 規則を無視しました。",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker 規則をインポートしませんでした。",
        "htmlImport.warn.css.nesting_unsupported" => "ネストした CSS スタイル規則を無視しました。",
        "htmlImport.warn.css.invalid_layer_name" => "無効な @layer 名 '{{name}}' を無視しました。",
        "htmlImport.warn.css.unsupported_statement" => "未対応の @{{name}} 文を無視しました。",
        "htmlImport.warn.css.media_without_viewport" => "ビューポートのない @media 規則を無視しました。",
        "htmlImport.warn.css.invalid_layer_block_name" => "無効な @layer ブロック名 '{{name}}' を無視しました。",
        "htmlImport.warn.css.unsupported_container_block" => "@container ブロックを無視しました。",
        "htmlImport.warn.css.unsupported_block" => "未対応の @{{name}} ブロックを無視しました。",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face のウェブフォント '{{family}}' は利用できません。"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "絶対配置された要素のパーセント指定オフセットを近似しました。"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "パーセント指定の position:relative オフセットを近似しました。"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "確定した軸のない CSS aspect-ratio を無視しました。"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "サイズ不確定の包含ブロック内の CSS aspect-ratio を無視しました。"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky を無視しました。",
        "htmlImport.warn.layout.grid_tracks_approximated" => "未対応の CSS グリッドトラックを近似しました。",
        "htmlImport.warn.layout.float_ignored" => "CSS float を無視しました。",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "ノード単位の CSS mix-blend-mode を近似しました。"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => "CSS overflow: auto / scroll を近似しました。",
        "htmlImport.warn.layout.negative_margins_ignored" => "負の CSS マージンを無視しました。",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => "視覚ボックス上の CSS マージンを無視しました。",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box のパーセント指定サイズを近似しました。"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => "明示的な開始ラインで生じた空の CSS グリッドセルを近似しました。",
        "htmlImport.warn.layout.grid_span_reflowed" => "開始ラインに収まらないスパンを持つ CSS グリッド項目を近似しました。",
        "htmlImport.warn.layout.grid_rows_node_limit" => "ノード上限に達したため、CSS グリッドの行ラッパーを省略しました。",
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "auto-fit / auto-fill を使う CSS グリッドのトラック幅を近似しました。"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas による配置をインポートしませんでした。"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => "CSS grid-row による配置をインポートしませんでした。",
        "htmlImport.warn.layout.grid_column_unsupported" => "CSS grid-column `{{value}}` を近似しました。",
        "htmlImport.warn.layout.block_auto_margins_ignored" => "ブロック軸方向の CSS 自動マージンをインポートしませんでした。",
        "htmlImport.warn.layout.auto_margin_node_limit" => "ノード上限に達したため、CSS 自動マージンによる配置を省略しました。",
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "サイズが確定しない要素のフロー内 CSS オフセットを破棄しました。"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => "ノード上限に達したため、フロー内 CSS オフセットを省略しました。",
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "フロー内 CSS オフセット（position:relative のインセット、transform の平行移動）を近似しました。"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "オフセット用ラッパーを持てないボックスのフロー内 CSS オフセットを破棄しました。"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "列方向の flex コンテナーの flex-wrap をインポートしませんでした。"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse を近似しました。",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => "幅が確定しないコンテナーの flex-wrap を無視しました。",
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "折り返す flex コンテナーの CSS align-content をインポートしませんでした。"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "子要素の主軸サイズが不確定な flex-wrap を無視しました。"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => "ノード上限に達したため、flex-wrap の行を省略しました。",
        "htmlImport.warn.transform.unsupported_syntax" => "未対応の CSS transform 構文を無視しました。",
        "htmlImport.warn.transform.unsupported_function" => {
            "未対応の CSS transform 関数（3D、matrix3d）を無視しました。"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "不確定な軸に対するパーセント指定の CSS transform 平行移動を破棄しました。"
        }
        "htmlImport.warn.transform.non_finite_matrix" => "非有限の行列を生じる CSS transform を無視しました。",
        "htmlImport.warn.transform.skew_dropped" => "CSS transform の skew を破棄しました。",
        "htmlImport.warn.transform.degenerate_scale" => "拡大率がゼロまたは非有限の CSS transform を近似しました。",
        "htmlImport.warn.transform.mirroring_absolute" => "CSS transform の鏡像反転を近似しました。",
        "htmlImport.warn.transform.origin_z_ignored" => "CSS transform-origin の Z オフセットを無視しました。",
        "htmlImport.warn.transform.scale_not_baked" => {
            "ノードサイズに焼き込めなかった CSS transform の拡大縮小を破棄しました。"
        }
        "htmlImport.warn.transform.scale_baked" => "ノードサイズに焼き込んだ CSS transform の拡大縮小を近似しました。",
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "自動サイズの要素に対する CSS transform の拡大縮小を無視しました。"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "方向指定または間隔付きの CSS background-repeat を近似しました。"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => "明示的な CSS 背景タイルサイズを無視しました。",
        "htmlImport.warn.visual.background_size_auto_box" => {
            "自動サイズの要素に対する CSS background-size を近似しました。"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "画像の固有サイズを必要とする CSS background-size を近似しました。"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "未対応の CSS background-position を無視しました。"
        }
        "htmlImport.warn.visual.background_image_url_empty" => "空の CSS 背景画像 URL を無視しました。",
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS の円錐グラデーションを無視しました。",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "未対応の CSS background-image レイヤーを無視しました。"
        }
        "htmlImport.warn.visual.background_color_unresolved" => "解決できない CSS 背景色を無視しました。",
        "htmlImport.warn.visual.background_position_dropped" => "CSS background-position を無視しました。",
        "htmlImport.warn.visual.border_colors_approximated" => "辺ごとの CSS ボーダー色を近似しました。",
        "htmlImport.warn.visual.border_styles_approximated" => "辺ごとに異なる CSS ボーダースタイルを近似しました。",
        "htmlImport.warn.visual.border_style_complex" => "複雑な CSS ボーダースタイルを近似しました。",
        "htmlImport.warn.visual.border_style_unsupported" => "未対応の CSS ボーダースタイルを近似しました。",
        "htmlImport.warn.visual.border_radius_elliptical" => "楕円形の CSS 角丸半径を近似しました。",
        "htmlImport.warn.visual.border_radius_unsupported" => "未対応の CSS 角丸半径を無視しました。",
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => "未対応の CSS box-shadow レイヤーを無視しました。",
        "htmlImport.warn.visual.gradient_interpolation_ignored" => "CSS グラデーションの色補間方式を無視しました。",
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "未対応の CSS linear-gradient の方向を無視しました。"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => "CSS グラデーションの色ヒントを無視しました。",
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => "未対応の CSS グラデーションの色停止点を無視しました。",
        "htmlImport.warn.visual.gradient_too_few_stops" => "使用可能な停止点が 2 つ未満の CSS グラデーションを無視しました。",
        "htmlImport.warn.visual.gradient_repeating_approximated" => "繰り返しの CSS グラデーションを近似しました。",
        "htmlImport.warn.visual.gradient_stops_clamped" => "範囲外の CSS グラデーション停止点を近似しました。",
        "htmlImport.warn.visual.blur_radius_unsupported" => "未対応の CSS ぼかし半径を無視しました。",
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "未対応の CSS filter の drop-shadow() を無視しました。"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => "未対応の CSS filter 関数を無視しました。",
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "未対応の CSS backdrop-filter 関数を無視しました。"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "未対応の CSS background-blend-mode を無視しました。"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => "個々の塗りに対する CSS mix-blend-mode を近似しました。",
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => "未対応の CSS mix-blend-mode を無視しました。",
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} を無視しました。",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "グラデーションに対する CSS background-size を無視しました。"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "未対応の CSS radial-gradient の位置を無視しました。"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => "楕円形の CSS radial-gradient を近似しました。",
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient の範囲キーワードを近似しました。"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "未対応の CSS radial-gradient のサイズを無視しました。"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => "未対応の CSS text-shadow レイヤーを無視しました。",
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "2 つ目以降の CSS text-shadow レイヤーを無視しました。"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => "インライン要素の CSS text-shadow を無視しました。",
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image をインポートしませんでした。",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` のぶら下げマーカーを近似しました。"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "未対応の CSS list-style-type `{{value}}` を近似しました。"
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down を近似しました。",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none を無視しました。",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position を無視しました。",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "画像に対する未対応の CSS mix-blend-mode を無視しました。"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => "インラインの <svg> 要素をプレースホルダーとしてインポートしました。",
        "htmlImport.warn.media.input_type_fallback" => "未対応の <input> の種類を近似しました。",
        "htmlImport.warn.media.element_placeholder" => "<{{tag}}> 要素をプレースホルダーとしてインポートしました。",
        "htmlImport.warn.media.picture_undecodable_types" => {
            "デコードできない種類のソースのみを持つ <picture> を近似しました。"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML の rowspan 属性をインポートしませんでした。",
        "htmlImport.warn.table.row_groups_unflattened" => "CSS が行グループの平坦化を解除した表の列幅を近似しました。",
        "htmlImport.warn.table.indefinite_width_approximated" => "幅が確定しない CSS 表の列幅を近似しました。",
        "htmlImport.warn.resource.invalid_base_href" => "無効な <base href> {{href}} を無視しました。",
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "プロジェクトのオリジン外の <base href> {{href}} を無視しました。"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => "外部スタイルシート {{url}} は利用できません。",
        "htmlImport.warn.resource.image_outside_origin" => {
            "プロジェクトのオリジン外の画像 {{url}} をプレースホルダーとしてインポートしました。"
        }
        "htmlImport.warn.resource.image_unavailable" => "利用できない画像 {{url}} をプレースホルダーとしてインポートしました。",
        "htmlImport.warn.resource.css_import_invalid" => "無効な CSS @import {{prelude}} を無視しました。",
        "htmlImport.warn.resource.css_import_unresolvable" => "CSS @import {{reference}} は利用できません。",
        "htmlImport.warn.resource.css_import_cycle" => "循環する CSS @import {{url}} を無視しました。",
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "深さ {{max_depth}} を超える CSS @import {{url}} を無視しました。"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} は利用できません。",
        "htmlImport.warn.project.multiple_html_entries" => {
            "HTML エントリーが {{count}} 件見つかりました。{{entry}} を選択し、残りを近似しました。"
        }
        "htmlImport.warn.snapshot.truncated" => "ブラウザースナップショットの一部を破棄しました。",
        "htmlImport.warn.snapshot.node_limit" => "ノード上限に達したため、残りのスナップショット内容を省略しました。",
        "htmlImport.warn.snapshot.tainted_images" => {
            "CORS で汚染された画像 {{count}} 件はリモート URL のまま保持され、利用できません。"
        }
        "htmlImport.warn.snapshot.invalid_rect" => "矩形が欠落または無効なスナップショットノードを破棄しました。",
        "htmlImport.warn.snapshot.unknown_kind" => "種類が不明なスナップショットノードを破棄しました。",
        "htmlImport.warn.snapshot.rejected" => "ブラウザースナップショット（{{reason}}）を破棄しました。",
        "htmlImport.warn.snapshot.unsupported_transform" => "未対応のスナップショット変換を無視しました。",
        "htmlImport.warn.css.media_empty_query" => "空の @media クエリを無視しました。",
        "htmlImport.warn.css.media_unsupported_type" => "未対応の @media タイプ '{{name}}' を無視しました。",
        "htmlImport.warn.css.media_unsupported_condition" => "未対応の @media 条件 '{{input}}' を無視しました。",
        "htmlImport.warn.css.media_invalid_orientation" => "無効な @media の向き '{{value}}' を無視しました。",
        "htmlImport.warn.css.media_unsupported_feature" => "未対応の @media 特性 '{{name}}' を無視しました。",
        "htmlImport.warn.css.media_unsupported_range" => "未対応の @media 範囲 '({{input}})' を無視しました。",
        "htmlImport.warn.css.media_invalid_range" => "無効な @media 範囲 '({{input}})' を無視しました。",
        "htmlImport.warn.css.media_invalid_length" => "無効な @media の長さ '{{value}}' を無視しました。",
        "htmlImport.diagnostics.title" => "HTML インポート完了",
        "htmlImport.diagnostics.summary" => "劣化した項目：{{count}}",
        "htmlImport.diagnostics.dismiss" => "閉じる",
        "htmlImport.diagnostics.expand" => "詳細を表示",
        "htmlImport.diagnostics.collapse" => "詳細を隠す",
        "htmlImport.diagnostics.more" => "他 {{count}} 件",
        "dialog.pptxTitle" => "PowerPoint を書き出し",
        "dialog.pptxSummary" => "{{count}} 枚のスライドを次の場所に書き出しました:",
        "dialog.pptxEmpty" => "このプレゼンテーションには書き出せるスライドがありません。",
        "settings.agents.acpQuickAdd" => "クイック追加",
        "settings.agents.acpPresetAdd" => "追加",
        "settings.agents.acpNotInstalled" => "未インストール",
        "assetCenter.title" => "アセットセンター",
        "assetCenter.tab.templates" => "テンプレート",
        "assetCenter.tab.styles" => "スタイル",
        "assetCenter.style.empty" => "一致するスタイルがありません",
        "assetCenter.style.pinned" => "ピン留め中",
        "assetCenter.style.searchPlaceholder" => "スタイルやタグを検索",
        "assetCenter.style.generateHint" => "新しいドキュメントをトピックから生成します。ピン留めしたスタイルが使われます。",
        "slidesPanel.tabSlides" => "スライド",
        "slidesPanel.tabCards" => "カード",
        "slidesPanel.present" => "再生",
        _ => return super::ja_collab::lookup(key),
    })
}
