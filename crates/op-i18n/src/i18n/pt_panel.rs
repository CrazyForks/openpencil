//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `pt_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Pesquisar imagens…",
        "imagePanel.searching" => "Pesquisando…",
        "imagePanel.noResults" => "Nenhum resultado",
        "imagePanel.searchPrompt" => "Pesquise imagens",
        "imagePanel.sourceNotice" => {
            "Imagens de {{source}}. Licença livre — verifique a licença antes de usar."
        }
        "imagePanel.genNotConfigured" => "Geração de imagens não configurada",
        "imagePanel.openSettings" => "Abrir configurações",
        "imagePanel.promptPlaceholder" => "Descreva a imagem…",
        "providerProbe.connectedViaCli" => "Conectado via CLI do {{name}}",
        "providerProbe.cliExitedWithError" => "A CLI do {{name}} terminou com erro",
        "providerProbe.cliNoVersionOutput" => "A CLI do {{name}} não produziu informação de versão",
        "providerProbe.modelQueryFailed" => "A consulta de modelos do {{name}} falhou ou expirou",
        "providerProbe.modelQueryFailedRunLogin" => "A consulta de modelos do {{name}} falhou. Execute {{command}} uma vez para autenticar.",
        "providerProbe.modelQueryNeedsAuth" => "A consulta de modelos do {{name}} exige autenticação. Execute {{command}} uma vez para entrar.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} devolveu um catálogo de modelos não reconhecido",
        "promptCenter.title" => "Central de prompts",
        "promptCenter.searchPlaceholder" => "Pesquisar prompts…",
        "promptCenter.category.all" => "Tudo",
        "promptCenter.category.starter" => "Início rápido",
        "promptCenter.category.mobileApp" => "App móvel",
        "promptCenter.category.webPage" => "Página web",
        "promptCenter.category.dashboard" => "Painel",
        "promptCenter.category.component" => "Componente",
        "promptCenter.category.modify" => "Modificar",
        "promptCenter.category.custom" => "Meus prompts",
        "promptCenter.empty" => "Nenhum prompt correspondente",
        "promptCenter.saveCurrent" => "Salvar a entrada atual como prompt",
        "promptCenter.saveTitlePlaceholder" => "Título do prompt",
        "promptCenter.save" => "Salvar",
        "promptCenter.cancel" => "Cancelar",
        "promptCenter.delete" => "Excluir",
        "promptCenter.screens" => "{{count}} telas",
        "promptCenter.freeform" => "Forma livre",
        "promptCenter.item.wander.title" => "Wander · Roteiros de viagem",
        "promptCenter.item.forage.title" => "Forage · Receitas da estação",
        "promptCenter.item.still.title" => "Still · Meditação e sono",
        "promptCenter.item.hearth.title" => "Hearth · Casa inteligente",
        "promptCenter.item.meteo.title" => "Meteo · Clima imersivo",
        "promptCenter.item.marginalia.title" => "Marginalia · Leitura e anotações",
        "promptCenter.item.lingua.title" => "Lingua · Aprendizado de idiomas",
        "promptCenter.item.daybreak.title" => "Daybreak · Pedido de café",
        "promptCenter.item.verdant.title" => "Verdant · Cuidados com plantas",
        "promptCenter.item.companion.title" => "Companion · Vida com pets",
        "promptCenter.item.relic.title" => "Relic · Mercado selecionado de usados",
        "promptCenter.item.nocturne.title" => "Nocturne · Guia de observação das estrelas",
        "promptCenter.item.marquee.title" => "Marquee · Lista de filmes",
        "promptCenter.item.ritual.title" => "Ritual · Criação de hábitos",
        "promptCenter.item.ember.title" => "Ember · Diário de humor",
        "promptCenter.item.volt.title" => "Volt · Companheiro para veículo elétrico",
        "promptCenter.item.aloft.title" => "Aloft · Rastreamento de voos",
        "promptCenter.item.gallery.title" => "Gallery · Exposições e cultura",
        "promptCenter.item.nightcap.title" => "Nightcap · Coquetelaria em casa",
        "promptCenter.item.bloom.title" => "Bloom · Registro do crescimento familiar",
        "promptCenter.item.extremeWeather.title" => "Extremo · App de clima",
        "promptCenter.item.extremeNowPlaying.title" => "Extremo · Em reprodução",
        "promptCenter.item.extremeDailyApp.title" => "Extremo · Abrir todos os dias",
        "promptCenter.item.extremeCalendar.title" => "Extremo · Reinventar o calendário",
        "promptCenter.item.extremeCalm.title" => "Extremo · Uma tela de calma",
        "promptCenter.item.webOrbit.title" => "Orbit · Página do espaço de trabalho com IA",
        "promptCenter.item.webAtelier.title" => "Atelier · Comércio de móveis",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Painel de análise de crescimento",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Operações logísticas",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Tabela de dados empresarial",
        "promptCenter.item.componentFormLab.title" => {
            "Form Lab · Sistema de componentes de formulário"
        }
        "promptCenter.item.modifyPolishCurrent.title" => "Aprimorar a tela atual",
        "promptCenter.item.modifyCompleteStates.title" => "Completar estados dos componentes",
        "collab.ownerConfirm.title" => "Confirme a quem você vai se juntar",
        "collab.ownerConfirm.hint" => "Nada desta sessão foi carregado ainda.",
        "collab.ownerConfirm.account" => "Conta verificada",
        "collab.ownerConfirm.device" => "Dispositivo verificado",
        "collab.ownerConfirm.claimedName" => "Nome escolhido por esta conta (não verificado)",
        "collab.action.confirmOwner" => "Entrar nesta sessão",
        "collab.action.rejectOwner" => "Não entrar",
        "collab.error.ownerNotConfirmed" => "Você não confirmou o anfitrião, então nada foi carregado.",
        "sceneTemplate.title" => "Modelos de cenas",
        "sceneTemplate.searchPlaceholder" => "Pesquisar cenas ou modelos…",
        "sceneTemplate.empty" => "Nenhum modelo correspondente",
        "sceneTemplate.frames" => "Páginas: {{count}}",
        "sceneTemplate.generate.placeholder" => "Descreva um tema e a IA gera a apresentação inteira",
        "sceneTemplate.generate.button" => "Gerar",
        "sceneTemplate.generate.hint" => "Um documento novo, criado a partir do seu tema como apresentação completa.",
        "sceneTemplate.generate.promptTemplate" => "Crie uma apresentação (PPT) sobre o seguinte tema: {{topic}}",
        "sceneTemplate.filter.all" => "Tudo",
        "sceneTemplate.scene.tutorial" => "Tutoriais",
        "sceneTemplate.scene.comparison" => "Comparação",
        "sceneTemplate.scene.carousel" => "Carrossel",
        "sceneTemplate.scene.slides" => "Slides",
        "sceneTemplate.scene.card" => "Cartões",
        "sceneTemplate.item.screenshotTutorial.title" => "Tutorial com capturas · 3 passos",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Capa, três passos e uma chamada para ação no final. Substitua as capturas de tela e os textos para publicar."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Carrossel de conhecimento e ideias",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Capa, três pontos e uma página de resumo, ideal para transformar uma ideia em cards deslizáveis."
        }
        "sceneTemplate.item.beforeAfter.title" => "Comparativo antes e depois",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Comparação lado a lado do antes e depois, com notas das mudanças; ideal para retrospectivas e portfólios."
        }
        "sceneTemplate.item.slideDeck.title" => "Apresentação · 6 slides",
        "sceneTemplate.item.slideDeck.summary" => {
            "Capa, agenda, pontos-chave, dados, gráfico e encerramento, no formato 16:9. Substitua os textos e apresente."
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "Cartão de conhecimento · Retrato",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Um único cartão 3:4 com título, quatro pontos-chave e uma assinatura. Troque os textos e publique.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Cartão de conhecimento · Quadrado",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Um cartão 1:1 com a mesma composição, compacto para uma imagem de capa ou uma publicação social.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch deck · Escuro",
        "sceneTemplate.item.pitchDeckDark.summary" => "Capa, problema, solução, números, roteiro e página de contato. Tipografia grande sobre fundo escuro, feito para captação e lançamentos.",
        "sceneTemplate.item.lectureDeckLight.title" => "Material de aula · Claro",
        "sceneTemplate.item.lectureDeckLight.summary" => "Capa do curso, objetivos, explicação do conceito, exercício resolvido, tabela comparativa e fechamento. Fundo branco papel, confortável durante toda a aula.",
        "sceneTemplate.item.minimalKeynote.title" => "Keynote minimalista",
        "sceneTemplate.item.minimalKeynote.summary" => "Espaço em branco, tipografia enorme e uma ideia por página — oito páginas sem um único cartão. Para lançamentos e palestras.",
        "sceneTemplate.item.gradientTech.title" => "Tech gradiente",
        "sceneTemplate.item.gradientTech.summary" => "Fundo em gradiente escuro com cartões de vidro fosco: arquitetura, desempenho e mural de clientes. Para lançamentos de produto técnico.",
        "fileMenu.newFromTemplate" => "Novo a partir de um modelo",
        "fileMenu.exportSlideshowHtml" => "Exportar apresentação HTML...",
        "fileMenu.exportPptx" => "Exportar para PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Exportar apresentação",
        "dialog.slideshowHtmlSummary" => "{{count}} slides exportados para:",
        "dialog.slideshowHtmlEmpty" => "Esta apresentação não tem slides visíveis para exportar.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "O conteúdo HTML importável está indisponível.",
        "htmlImport.warn.content.empty_body" => {
            "O conteúdo importável no corpo do HTML está indisponível."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "O HTML aninhado além de {{max_depth}} níveis foi descartado."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Limite de nós atingido; o restante do conteúdo da página foi omitido."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Limite de nós atingido; parte da árvore HTML foi omitida."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Limite de nós atingido; uma linha de formatação em linha foi omitida."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Limite de nós atingido; os pseudoelementos gerados foram omitidos."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "As regras CSS aninhadas além de {{max_depth}} regras @ foram ignoradas."
        }
        "htmlImport.warn.css.unterminated_rule" => "Uma regra CSS não terminada foi ignorada.",
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "As regras CSS ::marker não foram importadas."
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "As regras de estilo CSS aninhadas foram ignoradas."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "O nome de @layer inválido '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "A instrução @{{name}} sem suporte foi ignorada."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "As regras @media sem uma área de visualização foram ignoradas."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "O nome de bloco @layer inválido '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.unsupported_container_block" => "O bloco @container foi ignorado.",
        "htmlImport.warn.css.unsupported_block" => "O bloco @{{name}} sem suporte foi ignorado.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "A fonte web @font-face '{{family}}' está indisponível."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Os deslocamentos percentuais de um elemento posicionado de forma absoluta foram aproximados."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Os deslocamentos percentuais de position:relative foram aproximados."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "O aspect-ratio CSS sem um eixo definido foi ignorado."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "O aspect-ratio CSS dentro de um bloco contêiner indefinido foi ignorado."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "O position:sticky CSS foi ignorado.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "As faixas de grid CSS sem suporte foram aproximadas."
        }
        "htmlImport.warn.layout.float_ignored" => "O float CSS foi ignorado.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "O mix-blend-mode CSS no nível do nó foi aproximado."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "O overflow: auto / scroll CSS foi aproximado."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "As margens CSS negativas foram ignoradas."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "As margens CSS em uma caixa visual foram ignoradas."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "O dimensionamento percentual content-box foi aproximado."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "As células vazias do grid CSS deixadas por linhas iniciais explícitas foram aproximadas."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Um item de grid CSS cuja extensão não coube na linha inicial foi aproximado."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Limite de nós atingido; os invólucros de linha do grid CSS foram omitidos."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "As larguras das faixas de grid CSS que usam auto-fit / auto-fill foram aproximadas."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "O posicionamento por grid-template-areas CSS não foi importado."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "O posicionamento por grid-row CSS não foi importado."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "O grid-column CSS `{{value}}` foi aproximado."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "As margens automáticas CSS no eixo de bloco não foram importadas."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Limite de nós atingido; o alinhamento por margem automática CSS foi omitido."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Um deslocamento CSS no fluxo em um elemento sem tamanho definido foi descartado."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Limite de nós atingido; um deslocamento CSS no fluxo foi omitido."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Os deslocamentos CSS no fluxo (insets de position:relative, translação de transform) foram aproximados."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Um deslocamento CSS no fluxo em uma caixa que não pode hospedar um invólucro de deslocamento foi descartado."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "O flex-wrap em um contêiner flex de coluna não foi importado."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "O flex-wrap:wrap-reverse foi aproximado."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "O flex-wrap em um contêiner sem largura definida foi ignorado."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "O align-content CSS em um contêiner flex com quebra não foi importado."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "O flex-wrap com tamanhos indeterminados dos filhos no eixo principal foi ignorado."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Limite de nós atingido; as linhas de flex-wrap foram omitidas."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "A sintaxe de transform CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "As funções de transform CSS sem suporte (3D, matrix3d) foram ignoradas."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Uma translação percentual de transform CSS em um eixo indefinido foi descartada."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Um transform CSS que produziu uma matriz não finita foi ignorado."
        }
        "htmlImport.warn.transform.skew_dropped" => "A inclinação de transform CSS foi descartada.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Um transform CSS com escala zero ou não finita foi aproximado."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "O espelhamento por transform CSS foi aproximado."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "O deslocamento Z de transform-origin CSS foi ignorado."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Uma escala de transform CSS que não pôde ser incorporada ao tamanho do nó foi descartada."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "A escala de transform CSS incorporada ao tamanho do nó foi aproximada."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "A escala de transform CSS em um elemento de tamanho automático foi ignorada."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "O background-repeat CSS direcional ou espaçado foi aproximado."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Um tamanho explícito de ladrilho de fundo CSS foi ignorado."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "O background-size CSS em um elemento de tamanho automático foi aproximado."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "O background-size CSS que precisa do tamanho intrínseco da imagem foi aproximado."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Um background-position CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Uma URL vazia de imagem de fundo CSS foi ignorada."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Os gradientes cônicos CSS foram ignorados."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Uma camada de background-image CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Uma cor de fundo CSS não resolvida foi ignorada."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "O background-position CSS foi ignorado."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "As cores de borda CSS por lado foram aproximadas."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Os estilos de borda CSS mistos por lado foram aproximados."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Um estilo de borda CSS complexo foi aproximado."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Um estilo de borda CSS sem suporte foi aproximado."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Os raios de borda CSS elípticos foram aproximados."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Um raio de borda CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Uma camada de box-shadow CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "O método de interpolação de cores do gradiente CSS foi ignorado."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Uma direção de linear-gradient CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "As dicas de cor do gradiente CSS foram ignoradas."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Uma parada de cor de gradiente CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Um gradiente CSS com menos de duas paradas utilizáveis foi ignorado."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Um gradiente CSS repetido foi aproximado."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "As paradas de gradiente CSS fora do intervalo foram aproximadas."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Um raio de desfoque CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Um drop-shadow() de filtro CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Uma função de filtro CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Uma função backdrop-filter CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Um background-blend-mode CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "O mix-blend-mode CSS em preenchimentos individuais foi aproximado."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Um mix-blend-mode CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.visual.property_not_representable" => {
            "A propriedade CSS {{property}} foi ignorada."
        }
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "O background-size CSS em um gradiente foi ignorado."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Uma posição de radial-gradient CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Um radial-gradient CSS elíptico foi aproximado."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Uma palavra-chave de extensão de radial-gradient CSS foi aproximada."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Um tamanho de radial-gradient CSS sem suporte foi ignorado."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Uma camada de text-shadow CSS sem suporte foi ignorada."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "As camadas de text-shadow CSS após a primeira foram ignoradas."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "O text-shadow CSS em um elemento em linha foi ignorado."
        }
        "htmlImport.warn.list.style_image_ignored" => "O list-style-image CSS não foi importado.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Um marcador suspenso `list-style-position: outside` foi aproximado."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "O list-style-type CSS `{{value}}` sem suporte foi aproximado."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "O object-fit:scale-down CSS foi aproximado."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "O object-fit:none CSS foi ignorado.",
        "htmlImport.warn.media.object_position_ignored" => "O object-position CSS foi ignorado.",
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Um mix-blend-mode CSS sem suporte em uma imagem foi ignorado."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Um elemento <svg> em linha foi importado como espaço reservado."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Um tipo de <input> sem suporte foi aproximado."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "O elemento <{{tag}}> foi importado como espaço reservado."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Um <picture> apenas com tipos de origem não decodificáveis foi aproximado."
        }
        "htmlImport.warn.table.rowspan_ignored" => "O atributo HTML rowspan não foi importado.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "As larguras das colunas de uma tabela com grupos de linhas desachatados pelo CSS foram aproximadas."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "As larguras das colunas de uma tabela CSS sem largura definida foram aproximadas."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "O <base href> inválido {{href}} foi ignorado."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "O <base href> {{href}} fora da origem do projeto foi ignorado."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "A folha de estilos externa {{url}} está indisponível."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "A imagem {{url}} fora da origem do projeto foi importada como espaço reservado."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "A imagem indisponível {{url}} foi importada como espaço reservado."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "O @import CSS inválido {{prelude}} foi ignorado."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "O @import CSS {{reference}} está indisponível."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "O @import CSS cíclico {{url}} foi ignorado."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "O @import CSS {{url}} além da profundidade {{max_depth}} foi ignorado."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "O @import CSS {{url}} está indisponível."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} entradas HTML foram encontradas; {{entry}} foi escolhida e as demais foram aproximadas."
        }
        "htmlImport.warn.snapshot.truncated" => "Parte da captura do navegador foi descartada.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Limite de nós atingido; o restante do conteúdo da captura foi omitido."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} imagens contaminadas por CORS, mantidas como URLs remotas, estão indisponíveis."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Um nó da captura com retângulo ausente ou inválido foi descartado."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Um nó da captura de tipo desconhecido foi descartado."
        }
        "htmlImport.warn.snapshot.rejected" => {
            "A captura do navegador ({{reason}}) foi descartada."
        }
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Um transform de captura sem suporte foi ignorado."
        }
        "htmlImport.warn.css.media_empty_query" => "Uma consulta @media vazia foi ignorada.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "O tipo de @media sem suporte '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "A condição de @media sem suporte '{{input}}' foi ignorada."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "A orientação de @media inválida '{{value}}' foi ignorada."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "O recurso de @media sem suporte '{{name}}' foi ignorado."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "O intervalo de @media sem suporte '({{input}})' foi ignorado."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "O intervalo de @media inválido '({{input}})' foi ignorado."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "O comprimento de @media inválido '{{value}}' foi ignorado."
        }
        "htmlImport.diagnostics.title" => "Importação de HTML concluída",
        "htmlImport.diagnostics.summary" => "Itens degradados: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Dispensar",
        "htmlImport.diagnostics.expand" => "Mostrar detalhes",
        "htmlImport.diagnostics.collapse" => "Ocultar detalhes",
        "htmlImport.diagnostics.more" => "+{{count}} mais",
        "dialog.pptxTitle" => "Exportar para PowerPoint",
        "dialog.pptxSummary" => "{{count}} slides exportados para:",
        "dialog.pptxEmpty" => "Esta apresentação não tem slides visíveis para exportar.",
        "settings.agents.acpQuickAdd" => "Adição rápida",
        "settings.agents.acpPresetAdd" => "Adicionar",
        "settings.agents.acpNotInstalled" => "Não instalado",
        "assetCenter.title" => "Central de recursos",
        "assetCenter.tab.templates" => "Modelos",
        "assetCenter.tab.styles" => "Estilos",
        "assetCenter.style.empty" => "Nenhum estilo correspondente",
        "assetCenter.style.pinned" => "Fixado",
        "assetCenter.style.searchPlaceholder" => "Buscar estilos ou tags",
        "assetCenter.style.generateHint" => "Um novo documento criado a partir do seu tema, no estilo fixado.",
        "slidesPanel.tabSlides" => "Slides",
        "slidesPanel.tabCards" => "Cartões",
        "slidesPanel.present" => "Apresentar",
        _ => return super::pt_collab::lookup(key),
    })
}
