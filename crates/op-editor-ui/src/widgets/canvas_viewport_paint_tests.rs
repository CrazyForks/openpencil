//! Sibling test file for `canvas_viewport_paint.rs` (800-line cap
//! convention) — arc tessellation, text-node paint, SVG-path paint,
//! path flattening and `clipContent` child clipping.

mod arc_tests {
    use crate::widgets::canvas_viewport_paint::arc_polygon;
    use crate::Rect;

    #[test]
    fn pie_polygon_starts_at_centre() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        assert_eq!(poly[0].x, 50.0);
        assert_eq!(poly[0].y, 50.0);
        assert!((poly[1].x - 100.0).abs() < 0.01);
        assert!((poly[1].y - 50.0).abs() < 0.01);
    }

    #[test]
    fn donut_polygon_has_outer_and_inner_rings() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 360.0, 0.5);
        assert_eq!(poly.len(), 2 * (90 + 1));
        let last = poly[poly.len() - 1];
        let dist = ((last.x - 50.0).powi(2) + (last.y - 50.0).powi(2)).sqrt();
        assert!((dist - 25.0).abs() < 0.5, "inner radius ~25, got {dist}");
    }

    #[test]
    fn quarter_sweep_end_point_at_90_degrees() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        let last = poly[poly.len() - 1];
        assert!((last.x - 50.0).abs() < 0.01);
        assert!((last.y - 100.0).abs() < 0.01);
    }
}

mod text_tests {
    use crate::layout_scene::{NodeKind, SceneNode, SceneTextAlign, SceneTextVerticalAlign};
    use crate::widgets::canvas_viewport_paint::{paint_node_with_options, paint_svg_path_node};
    use crate::widgets::canvas_viewport_text::paint_text_node;
    use crate::widgets::PaintCx;
    use crate::{Color, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct TextCaptureBackend {
        origins: Vec<Point2D>,
        families: Vec<String>,
        font_sizes: Vec<f32>,
        lines: Vec<String>,
        translates: Vec<Point2D>,
        scales: Vec<(Point2D, Point2D)>,
        fill_rects: Vec<Rect>,
        round_rects: Vec<Rect>,
    }

    impl RenderBackend for TextCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, rect: Rect, _: Color) {
            self.fill_rects.push(rect);
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
            self.origins.push(origin);
            if let Some(run) = layout.runs().first() {
                self.families.push(run.font_family.clone());
                self.font_sizes.push(run.font_size);
                self.lines.push(run.content.clone());
            }
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, offset: Point2D) {
            self.translates.push(offset);
        }
        fn scale(&mut self, scale: Point2D, pivot: Point2D) {
            self.scales.push((scale, pivot));
        }
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
            self.round_rects.push(rect);
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text_weighted(&mut self, text: &str, font_size: f32, _: u16) -> f32 {
            if text.is_ascii() {
                text.chars().count() as f32 * font_size * 0.5
            } else {
                text.chars().count() as f32 * font_size * 0.7 + font_size * font_size * 0.07
            }
        }
        fn text_ascent_family(&mut self, font_size: f32, _: &str, _: u16) -> f32 {
            font_size * 0.75
        }
    }

    fn paint_node(
        cx: &mut PaintCx<'_>,
        node: &SceneNode,
        viewport_origin: Point2D,
        zoom: f32,
        cull: Rect,
    ) {
        let _ = paint_node_with_options(
            cx,
            node,
            viewport_origin,
            zoom,
            None,
            cull,
            None,
            None,
            None,
            None,
        );
    }

    #[test]
    fn rectangle_container_paints_its_children() {
        // Regression: a `rectangle` is a container in the canonical schema,
        // so models nest content (images, labels) inside one. The painter
        // treated NodeKind::Rect as a leaf and never recursed, so a photo
        // inside an image-area rectangle rendered as a blank fill.
        let mut rect = SceneNode::leaf("card", NodeKind::Rect);
        rect.bounds = Rect::xywh(0.0, 0.0, 200.0, 120.0);
        let mut label = SceneNode::leaf("label", NodeKind::Text);
        label.bounds = Rect::xywh(0.0, 0.0, 200.0, 20.0);
        label.text = Some("INSIDE".to_string());
        label.font_size = 14.0;
        label.line_height = 1.0;
        rect.children.push(label);

        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        paint_node(&mut cx, &rect, Point2D::ZERO, 1.0, rect.bounds);

        assert!(
            backend.lines.iter().any(|l| l == "INSIDE"),
            "rectangle must paint its child text; got {:?}",
            backend.lines
        );
    }

    #[test]
    fn text_node_paint_honors_horizontal_alignment_and_backend_ascent() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("Hi".to_string());
        node.font_family = "Georgia".to_string();
        node.font_size = 20.0;
        node.line_height = 1.0;
        node.text_align = SceneTextAlign::Center;
        node.text_vertical_align = SceneTextVerticalAlign::Middle;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_text_node(&mut cx, &node, node.bounds, 1.0, &None);

        assert_eq!(backend.families, vec!["Georgia".to_string()]);
        let origin = backend.origins[0];
        assert!(
            origin.x > 80.0,
            "center-aligned text should move away from the left edge"
        );
        assert_eq!(
            origin.y, 15.0,
            "canvas text places the alphabetic baseline at the backend-reported ascent"
        );
    }

    #[test]
    fn text_wrap_is_stable_across_canvas_zoom() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 100.0, 40.0);
        node.text = Some("可忽略风险".to_string());
        node.font_size = 20.0;
        node.text_wrap = true;

        let mut backend_1x = TextCaptureBackend::default();
        let mut cx_1x = PaintCx {
            backend: &mut backend_1x,
        };
        paint_node(
            &mut cx_1x,
            &node,
            Point2D::ZERO,
            1.0,
            Rect::xywh(0.0, 0.0, 800.0, 600.0),
        );

        let mut backend_2x = TextCaptureBackend::default();
        let mut cx_2x = PaintCx {
            backend: &mut backend_2x,
        };
        paint_node(
            &mut cx_2x,
            &node,
            Point2D::ZERO,
            2.0,
            Rect::xywh(0.0, 0.0, 800.0, 600.0),
        );

        assert_eq!(
            backend_2x.lines, backend_1x.lines,
            "canvas zoom must not change authored text wrapping"
        );
    }

    #[test]
    fn text_node_uses_viewport_transform_instead_of_zoomed_font_size() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(12.0, 24.0, 100.0, 40.0);
        node.text = Some("Zoom".to_string());
        node.font_size = 20.0;

        let viewport_origin = Point2D::new(80.0, 40.0);
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_node(
            &mut cx,
            &node,
            viewport_origin,
            2.0,
            Rect::xywh(0.0, 0.0, 800.0, 600.0),
        );

        assert_eq!(
            backend.font_sizes,
            vec![20.0],
            "canvas zoom should be a transform; text layout keeps the authored font size"
        );
        assert_eq!(backend.translates, vec![viewport_origin]);
        assert_eq!(
            backend.scales,
            vec![(Point2D::new(2.0, 2.0), Point2D::ZERO)]
        );
    }

    fn edit_caret(
        caret: Option<usize>,
        anchor: Option<usize>,
    ) -> crate::widgets::canvas_viewport::EditCaret {
        let mut input = jian_core::text_input::TextInputState::with_text("hello\nworld");
        if let Some(caret) = caret {
            if let Some(anchor) = anchor {
                input.set_caret(anchor, 0);
                input.drag_to(caret, 0);
            } else {
                input.set_caret(caret, 0);
            }
        }
        crate::widgets::canvas_viewport::EditCaret {
            editing: "t".to_string(),
            input,
            now_ms: 0, // blink phase 0 → caret visible
            selection_color: Color::BLUE,
        }
    }

    #[test]
    fn edit_caret_paints_at_caret_offset() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("hello\nworld".to_string());
        node.font_size = 20.0;
        node.line_height = 1.0;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        // Caret at byte 8 — line 1, col 2 (10 px/char capture metric).
        paint_text_node(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            &Some(edit_caret(Some(8), None)),
        );

        assert_eq!(
            backend.fill_rects,
            vec![Rect::xywh(20.0, 22.0, 1.0, 23.0)],
            "caret paints at the second line's col-2 advance, not at the text end"
        );
        assert!(backend.round_rects.is_empty(), "no selection → no wash");
    }

    #[test]
    fn edit_selection_paints_per_line_rects_and_hides_caret() {
        let mut node = SceneNode::leaf("t", NodeKind::Text);
        node.bounds = Rect::xywh(0.0, 0.0, 200.0, 80.0);
        node.text = Some("hello\nworld".to_string());
        node.font_size = 20.0;
        node.line_height = 1.0;
        let mut backend = TextCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        // anchor 3 .. caret 8 spans the line break.
        paint_text_node(
            &mut cx,
            &node,
            node.bounds,
            1.0,
            &Some(edit_caret(Some(8), Some(3))),
        );

        assert_eq!(
            backend.round_rects,
            vec![
                Rect::xywh(30.0, 0.0, 20.0, 24.0),
                Rect::xywh(0.0, 20.0, 20.0, 24.0),
            ],
            "one wash per intersected line"
        );
        assert!(
            backend.fill_rects.is_empty(),
            "caret hides while a selection is active"
        );
    }

    #[derive(Default)]
    struct SvgCaptureBackend {
        fill_rects: Vec<Rect>,
        fill_rules: Vec<bool>,
    }

    impl RenderBackend for SvgCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn fill_svg_path_in_rect(&mut self, _: &str, rect: Rect, _: Color) {
            self.fill_rects.push(rect);
        }
        fn fill_svg_path_in_rect_with_fill_rule(
            &mut self,
            _: &str,
            rect: Rect,
            _: Color,
            even_odd: bool,
        ) {
            self.fill_rects.push(rect);
            self.fill_rules.push(even_odd);
        }
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn svg_path_node_paint_fits_path_to_node_rect() {
        let mut node = SceneNode::leaf("p", NodeKind::Path);
        node.fill = Some(Color::BLACK);
        let rect = Rect::xywh(10.0, 20.0, 28.0, 28.0);
        let mut backend = SvgCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M10 0 L0 -5 L0 5 Z");

        assert_eq!(backend.fill_rects, vec![rect]);
        assert_eq!(backend.fill_rules, vec![false]);
    }

    #[test]
    fn svg_path_node_forwards_even_odd_fill_rule() {
        let mut node = SceneNode::leaf("ring", NodeKind::Path);
        node.fill = Some(Color::BLACK);
        node.even_odd_fill = true;
        let rect = Rect::xywh(10.0, 20.0, 28.0, 28.0);
        let mut backend = SvgCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M0 0H28V28H0Z M7 7H21V21H7Z");

        assert_eq!(backend.fill_rules, vec![true]);
    }

    #[derive(Default)]
    struct GradientPathCaptureBackend {
        solid_fills: Vec<Rect>,
        linear_gradients: Vec<(Rect, f32, usize)>,
        radial_gradients: Vec<(Rect, usize)>,
        inner_shadows: Vec<(Rect, Color)>,
    }

    impl RenderBackend for GradientPathCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn fill_svg_path_in_rect(&mut self, _: &str, rect: Rect, _: Color) {
            self.solid_fills.push(rect);
        }
        fn fill_svg_path_in_rect_linear_gradient(
            &mut self,
            _: &str,
            rect: Rect,
            stops: &[(f32, Color)],
            angle_deg: f32,
            _: f32,
        ) {
            self.linear_gradients.push((rect, angle_deg, stops.len()));
        }
        fn fill_svg_path_in_rect_radial_gradient(
            &mut self,
            _: &str,
            rect: Rect,
            stops: &[(f32, Color)],
            _: f32,
            _: f32,
            _: f32,
            _: f32,
        ) {
            self.radial_gradients.push((rect, stops.len()));
        }
        fn fill_inner_shadow_svg_path(
            &mut self,
            _: &str,
            rect: Rect,
            _: f32,
            _: f32,
            _: f32,
            color: Color,
        ) {
            self.inner_shadows.push((rect, color));
        }
        fn draw_image(&mut self, _: Rect, _: u64, _: &[u8]) {}
        fn draw_image_with_mode(&mut self, _: Rect, _: u64, _: &[u8], _: ImageDrawMode) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn svg_path_node_with_linear_gradient_paints_gradient_not_solid() {
        use crate::layout_scene::{SceneFillType, SceneGradient, SceneGradientStop};
        let mut node = SceneNode::leaf("p", NodeKind::Path);
        node.fill = Some(Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        });
        node.fill_type = SceneFillType::LinearGradient;
        node.gradient = Some(SceneGradient::Linear {
            angle_deg: 90.0,
            opacity: 1.0,
            stops: vec![
                SceneGradientStop {
                    offset: 0.0,
                    color: Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    },
                },
                SceneGradientStop {
                    offset: 1.0,
                    color: Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    },
                },
            ],
        });
        let rect = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        let mut backend = GradientPathCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M0 0 L10 0 L10 10 Z");

        assert_eq!(
            backend.linear_gradients,
            vec![(rect, 90.0, 2)],
            "linear-gradient path must paint via the gradient method"
        );
        assert!(
            backend.solid_fills.is_empty(),
            "gradient path must not fall back to the solid fill"
        );
    }

    #[test]
    fn svg_path_node_with_inner_shadow_paints_inset_shadow() {
        use crate::layout_scene::{DropShadow, Effect};
        let mut node = SceneNode::leaf("p", NodeKind::Path);
        node.fill = Some(Color {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        let shadow_color = Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.5,
        };
        node.effects = vec![Effect::DropShadow(DropShadow {
            offset_x: 0.0,
            offset_y: 0.0,
            blur: 4.0,
            color: shadow_color,
            inner: true,
        })];
        let rect = Rect::xywh(0.0, 0.0, 20.0, 20.0);
        let mut backend = GradientPathCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };

        paint_svg_path_node(&mut cx, &node, rect, 1.0, "M0 0 L20 0 L20 20 L0 20 Z");

        assert_eq!(
            backend.inner_shadows,
            vec![(rect, shadow_color)],
            "inner-shadow path must route to the inset-shadow painter"
        );
    }
}

mod path_tests {
    use crate::layout_scene::{NodeKind, SceneAnchor, SceneNode, ScenePointType};
    use crate::widgets::canvas_viewport_paint::{flatten_path, world_path_points, WorldPathPoints};
    use crate::{Point2D, Rect};
    use jian_scene::path_geometry::{flatten_path_points, PathPoints};

    fn anchor(x: f32, y: f32, hout: Option<Point2D>) -> SceneAnchor {
        SceneAnchor {
            pos: Point2D::new(x, y),
            handle_in: None,
            handle_out: hout,
            point_type: ScenePointType::Corner,
        }
    }

    #[test]
    fn handle_free_path_falls_back_to_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 0.0)];
        n.path_anchors = vec![anchor(0.0, 0.0, None), anchor(10.0, 0.0, None)];
        assert_eq!(flatten_path(&n), n.points);
    }

    #[test]
    fn handle_free_open_path_borrows_points_without_allocating() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 0.0)];
        n.path_anchors = vec![anchor(0.0, 0.0, None), anchor(10.0, 0.0, None)];

        let points = flatten_path_points(&n);

        assert!(matches!(points, PathPoints::Borrowed(_)));
        assert_eq!(points.as_slice(), n.points.as_slice());
    }

    #[test]
    fn small_filled_path_world_points_use_stack_buffer() {
        let points = [
            Point2D::new(0.0, 0.0),
            Point2D::new(10.0, 0.0),
            Point2D::new(10.0, 10.0),
        ];

        let world = world_path_points(&points, Point2D::new(5.0, 7.0), 2.0);

        assert!(matches!(world, WorldPathPoints::Stack { .. }));
        assert_eq!(
            world.as_slice(),
            &[
                Point2D::new(5.0, 7.0),
                Point2D::new(25.0, 7.0),
                Point2D::new(25.0, 27.0),
            ]
        );
    }

    #[test]
    fn curved_segment_tessellates_into_many_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(100.0, 0.0)];
        n.path_anchors = vec![
            anchor(0.0, 0.0, Some(Point2D::new(0.0, 50.0))),
            anchor(100.0, 0.0, None),
        ];
        let poly = flatten_path(&n);
        assert_eq!(poly.len(), 17);
        assert_eq!(poly[0], Point2D::new(0.0, 0.0));
        assert_eq!(poly[poly.len() - 1], Point2D::new(100.0, 0.0));
        assert!(poly[8].y > 1.0, "curve bows toward the handle");
    }

    #[test]
    fn bounds_kept_so_helper_is_pure() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.bounds = Rect::xywh(1.0, 2.0, 3.0, 4.0);
        let _ = flatten_path(&n);
        assert_eq!(n.bounds, Rect::xywh(1.0, 2.0, 3.0, 4.0));
    }
}

mod clip_tests {
    use crate::layout_scene::{MaskType, NodeKind, SceneNode, SceneStroke, SceneStrokeAlign};
    use crate::widgets::canvas_viewport_paint::{
        paint_node_with_options, paint_scene_nodes_with_options_hiding,
    };
    use crate::widgets::PaintCx;
    use crate::{Color, ImageBlendMode, Point2D, Rect, RenderBackend, TextLayout};

    /// Records the paint-op sequence so the test can assert the clip
    /// brackets the children (and only the children).
    #[derive(Default)]
    struct ClipCaptureBackend {
        ops: Vec<String>,
        content_layer_bounds: Vec<Rect>,
        image_decode_checks: Vec<u64>,
    }

    impl RenderBackend for ClipCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, rect: Rect, _: Color) {
            self.ops
                .push(format!("fill({},{})", rect.origin.x, rect.origin.y));
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {
            self.ops.push("stroke".into());
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, rect: Rect) {
            self.ops.push(format!(
                "clip({},{},{},{})",
                rect.origin.x, rect.origin.y, rect.size.x, rect.size.y
            ));
        }
        fn clip_round_rect(&mut self, rect: Rect, radius: f32) {
            self.ops.push(format!(
                "clip_rr({},{},{},{},r={radius})",
                rect.origin.x, rect.origin.y, rect.size.x, rect.size.y
            ));
        }
        fn clip_svg_path_in_rect(&mut self, d: &str, rect: Rect, even_odd: bool) {
            self.ops.push(format!(
                "clip_path({d},{},{},{},{},evenodd={even_odd})",
                rect.origin.x, rect.origin.y, rect.size.x, rect.size.y
            ));
        }
        fn save(&mut self) {
            self.ops.push("save".into());
        }
        fn push_composite_layer(&mut self, bounds: Rect, _: f32, _: ImageBlendMode) {
            self.content_layer_bounds.push(bounds);
            self.ops.push("content_layer".into());
        }
        fn supports_pixel_masks(&self) -> bool {
            true
        }
        fn push_mask_source_layer(&mut self, luminance: bool) {
            self.ops.push(format!("mask_layer(luma={luminance})"));
        }
        fn restore(&mut self) {
            self.ops.push("restore".into());
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, rect: Rect, _: f32, _: Color) {
            self.ops
                .push(format!("fill({},{})", rect.origin.x, rect.origin.y));
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
            self.ops.push("stroke".into());
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn image_decoded(&mut self, id: u64, _: &[u8], _: u32) -> bool {
            self.image_decode_checks.push(id);
            true
        }
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn paint_node(cx: &mut PaintCx<'_>, node: &SceneNode, cull: Rect) {
        let _ = paint_node_with_options(
            cx,
            node,
            Point2D::ZERO,
            1.0,
            None,
            cull,
            None,
            None,
            None,
            None,
        );
    }

    fn frame_with_child(clip: bool, corner_radius: f32) -> SceneNode {
        let mut child = SceneNode::leaf("c", NodeKind::Rect);
        child.bounds = Rect::xywh(10.0, 10.0, 500.0, 20.0);
        child.fill = Some(Color::RED);
        let mut frame = SceneNode::leaf("f", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.fill = Some(Color::WHITE);
        frame.clip_content = clip;
        frame.corner_radius = corner_radius;
        frame.children = vec![child];
        frame
    }

    fn capture(node: &SceneNode, cull: Rect) -> ClipCaptureBackend {
        let mut backend = ClipCaptureBackend::default();
        {
            let mut cx = PaintCx {
                backend: &mut backend,
            };
            paint_node(&mut cx, node, cull);
        }
        backend
    }

    fn paint(node: &SceneNode) -> Vec<String> {
        capture(node, Rect::xywh(0.0, 0.0, 4000.0, 4000.0)).ops
    }

    fn add_center_stroke(node: &mut SceneNode) {
        node.stroke = Some(SceneStroke {
            color: Color::BLACK,
            width: 2.0,
            sides: None,
            align: SceneStrokeAlign::Center,
        });
    }

    #[test]
    fn clipped_round_container_stroke_overlays_children_once() {
        for kind in [NodeKind::Frame, NodeKind::Rect] {
            let mut container = frame_with_child(true, 9999.0);
            container.kind = kind;
            add_center_stroke(&mut container);

            let ops = paint(&container);
            assert_eq!(
                ops,
                vec![
                    "fill(0,0)".to_string(),
                    "save".to_string(),
                    "clip_rr(0,0,100,100,r=50)".to_string(),
                    "fill(10,10)".to_string(),
                    "restore".to_string(),
                    "stroke".to_string(),
                ]
            );
            assert_eq!(
                ops.iter().filter(|op| op.as_str() == "stroke").count(),
                1,
                "container stroke must be painted exactly once"
            );
        }
    }

    #[test]
    fn leaf_frame_keeps_fill_then_single_stroke() {
        let mut frame = SceneNode::leaf("leaf", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.fill = Some(Color::WHITE);
        add_center_stroke(&mut frame);

        assert_eq!(
            paint(&frame),
            vec!["fill(0,0)".to_string(), "stroke".to_string()]
        );
    }

    #[test]
    fn clip_content_frame_brackets_children_with_sharp_clip() {
        let ops = paint(&frame_with_child(true, 0.0));
        // Own fill paints UN-clipped, then save → clip → child → restore.
        assert_eq!(
            ops,
            vec![
                "fill(0,0)".to_string(),
                "save".to_string(),
                "clip(0,0,100,100)".to_string(),
                "fill(10,10)".to_string(),
                "restore".to_string(),
            ]
        );
    }

    #[test]
    fn clip_content_uses_rounded_clip_clamped_to_half_height() {
        // Authored radius 80 clamps to h/2 = 50 (TS flattener rule).
        let ops = paint(&frame_with_child(true, 80.0));
        assert!(
            ops.contains(&"clip_rr(0,0,100,100,r=50)".to_string()),
            "{ops:?}"
        );
    }

    #[test]
    fn frame_without_clip_content_paints_children_unclipped() {
        let ops = paint(&frame_with_child(false, 0.0));
        assert_eq!(
            ops,
            vec!["fill(0,0)".to_string(), "fill(10,10)".to_string()]
        );
    }

    #[test]
    fn offscreen_image_container_skips_its_complete_subtree() {
        let mut child = filled_rect("child", 1_020.0);
        child.bounds.origin.y = 1_020.0;
        let mut frame = SceneNode::leaf("image-frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(1_000.0, 1_000.0, 100.0, 100.0);
        frame.image_src = Some("data:image/png;base64,QUJD".into());
        frame.image_src_id = 42;
        frame.children = vec![child];

        let capture = capture(&frame, Rect::xywh(0.0, 0.0, 100.0, 100.0));

        assert!(
            capture.ops.is_empty(),
            "an offscreen container must not paint itself or visit descendants"
        );
        assert!(
            capture.image_decode_checks.is_empty(),
            "an offscreen image-filled container must not enter the decode path"
        );
    }

    #[test]
    fn open_offscreen_container_keeps_visible_overflow_descendant() {
        let mut frame = SceneNode::leaf("open-frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(1_000.0, 1_000.0, 100.0, 100.0);
        frame.children = vec![filled_rect("visible-overflow", 20.0)];

        let capture = capture(&frame, Rect::xywh(0.0, 0.0, 100.0, 100.0));

        assert!(
            capture.ops.contains(&"fill(20,0)".to_string()),
            "an unclipped descendant that reaches the viewport must still paint"
        );
    }

    #[test]
    fn clip_content_group_clips_children_too() {
        let mut group = frame_with_child(true, 0.0);
        group.kind = NodeKind::Group;
        group.fill = None;
        let ops = paint(&group);
        assert_eq!(
            ops,
            vec![
                "save".to_string(),
                "clip(0,0,100,100)".to_string(),
                "fill(10,10)".to_string(),
                "restore".to_string(),
            ]
        );
    }

    fn filled_rect(id: &str, x: f32) -> SceneNode {
        let mut node = SceneNode::leaf(id, NodeKind::Rect);
        node.bounds = Rect::xywh(x, 0.0, 10.0, 10.0);
        node.fill = Some(Color::RED);
        node
    }

    fn path_mask(id: &str, x: f32, d: &str) -> SceneNode {
        let mut node = SceneNode::leaf(id, NodeKind::Path);
        node.bounds = Rect::xywh(x, 0.0, 10.0, 10.0);
        node.svg_path = Some(d.to_string());
        node.fill = Some(Color::WHITE);
        node.is_mask = true;
        node
    }

    #[test]
    fn opaque_path_mask_clips_only_front_siblings_and_is_not_painted() {
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        // Scene children are topmost-first. The reverse painter must draw the
        // background, install the mask, then draw the foreground inside it.
        frame.children = vec![
            filled_rect("front", 10.0),
            path_mask("mask", 0.0, "M0 0 L10 0 L10 10 Z"),
            filled_rect("back", 20.0),
        ];

        assert_eq!(
            paint(&frame),
            vec![
                "fill(20,0)".to_string(),
                "save".to_string(),
                "clip_path(M0 0 L10 0 L10 10 Z,0,0,10,10,evenodd=false)".to_string(),
                "fill(10,0)".to_string(),
                "restore".to_string(),
            ]
        );
    }

    #[test]
    fn next_path_mask_starts_a_fresh_sibling_clip_run() {
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![
            filled_rect("front", 10.0),
            path_mask("front-mask", 0.0, "M0 0 L10 0 L10 10 Z"),
            filled_rect("middle", 20.0),
            path_mask("back-mask", 40.0, "M0 0 L10 0 L10 10 Z"),
            filled_rect("back", 30.0),
        ];

        assert_eq!(
            paint(&frame),
            vec![
                "fill(30,0)".to_string(),
                "save".to_string(),
                "clip_path(M0 0 L10 0 L10 10 Z,40,0,10,10,evenodd=false)".to_string(),
                "fill(20,0)".to_string(),
                "restore".to_string(),
                "save".to_string(),
                "clip_path(M0 0 L10 0 L10 10 Z,0,0,10,10,evenodd=false)".to_string(),
                "fill(10,0)".to_string(),
                "restore".to_string(),
            ]
        );
    }

    fn pixel_mask(id: &str, kind: NodeKind, mask_type: MaskType, alpha: f32) -> SceneNode {
        let mut node = SceneNode::leaf(id, kind);
        node.bounds = Rect::xywh(0.0, 0.0, 10.0, 10.0);
        node.fill = Some(Color::WHITE.with_alpha(alpha));
        node.mask_type = Some(mask_type);
        node.is_mask = true;
        node
    }

    #[test]
    fn translucent_alpha_mask_uses_two_layers_and_is_deferred() {
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![
            filled_rect("front", 10.0),
            pixel_mask("mask", NodeKind::Rect, MaskType::Alpha, 0.5),
            filled_rect("back", 20.0),
        ];
        assert_eq!(
            paint(&frame),
            vec![
                "fill(20,0)",
                "content_layer",
                "fill(10,0)",
                "mask_layer(luma=false)",
                "fill(0,0)",
                "restore",
                "restore",
            ]
        );
    }

    #[test]
    fn pixel_mask_layer_is_bounded_to_mask_and_its_front_run() {
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![
            filled_rect("front", 10.0),
            pixel_mask("mask", NodeKind::Rect, MaskType::Alpha, 0.5),
            filled_rect("back", 80.0),
        ];
        let cull = Rect::xywh(0.0, 0.0, 4_000.0, 4_000.0);
        let capture = capture(&frame, cull);
        assert_eq!(
            capture.content_layer_bounds,
            vec![Rect::xywh(0.0, 0.0, 21.0, 11.0)]
        );
        assert_ne!(capture.content_layer_bounds[0], cull);
    }

    #[test]
    fn consecutive_masks_bound_each_sibling_run_independently() {
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![
            filled_rect("front", 10.0),
            pixel_mask("front-mask", NodeKind::Rect, MaskType::Alpha, 0.5),
            filled_rect("middle", 20.0),
            {
                let mut mask = pixel_mask("back-mask", NodeKind::Rect, MaskType::Alpha, 0.5);
                mask.bounds.origin.x = 40.0;
                mask
            },
            filled_rect("back", 80.0),
        ];
        let capture = capture(&frame, Rect::xywh(0.0, 0.0, 4_000.0, 4_000.0));
        assert_eq!(
            capture.content_layer_bounds,
            vec![
                Rect::xywh(19.0, 0.0, 32.0, 11.0),
                Rect::xywh(0.0, 0.0, 21.0, 11.0),
            ]
        );
    }

    #[test]
    fn luminance_mask_requests_luma_before_dst_in() {
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![
            filled_rect("front", 10.0),
            pixel_mask("mask", NodeKind::Rect, MaskType::Luminance, 1.0),
        ];
        let ops = paint(&frame);
        assert!(
            ops.contains(&"mask_layer(luma=true)".to_string()),
            "{ops:?}"
        );
        assert!(!ops.iter().any(|op| op.starts_with("clip_path")), "{ops:?}");
    }

    #[test]
    fn frame_mask_renders_its_subtree_as_the_mask_source() {
        let mut mask = pixel_mask("mask", NodeKind::Frame, MaskType::Alpha, 0.0);
        mask.fill = None;
        mask.children = vec![filled_rect("mask-child", 40.0)];
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![filled_rect("front", 10.0), mask];
        let ops = paint(&frame);
        let start = ops
            .iter()
            .position(|op| op == "mask_layer(luma=false)")
            .unwrap();
        assert_eq!(ops[start + 1], "fill(40,0)", "{ops:?}");
    }

    #[test]
    fn page_root_siblings_use_the_mask_aware_walk() {
        let nodes = vec![
            filled_rect("front", 10.0),
            pixel_mask("mask", NodeKind::Rect, MaskType::Alpha, 0.5),
            filled_rect("back", 20.0),
        ];
        let mut backend = ClipCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let _ = paint_scene_nodes_with_options_hiding(
            &mut cx,
            &nodes,
            Point2D::ZERO,
            1.0,
            None,
            Rect::xywh(0.0, 0.0, 100.0, 100.0),
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            None,
            false,
        );
        assert_eq!(backend.ops[0], "fill(20,0)");
        assert!(backend.ops.contains(&"content_layer".to_string()));
        assert!(backend.ops.contains(&"mask_layer(luma=false)".to_string()));
    }

    #[test]
    fn zero_sized_alpha_mask_still_creates_an_empty_dst_in_source() {
        let mut mask = pixel_mask("zero", NodeKind::Rect, MaskType::Alpha, 1.0);
        mask.bounds.size.x = 0.0;
        let mut frame = SceneNode::leaf("frame", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 100.0, 100.0);
        frame.children = vec![filled_rect("front", 10.0), mask];
        let ops = paint(&frame);
        assert!(ops.contains(&"content_layer".to_string()), "{ops:?}");
        assert!(
            ops.contains(&"mask_layer(luma=false)".to_string()),
            "{ops:?}"
        );
    }
}

mod stroke_align_tests {
    use crate::layout_scene::{NodeKind, SceneNode, SceneStroke, SceneStrokeAlign};
    use crate::widgets::canvas_viewport_paint::paint_node_with_options;
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct StrokeCaptureBackend {
        strokes: Vec<(f32, f32, f32, f32, f32)>,
    }

    impl RenderBackend for StrokeCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, rect: Rect, _: Color, w: f32) {
            self.strokes
                .push((rect.origin.x, rect.origin.y, rect.size.x, rect.size.y, w));
        }
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn stroked_rect(align: SceneStrokeAlign) -> SceneNode {
        let mut n = SceneNode::leaf("s", NodeKind::Rect);
        n.bounds = Rect::xywh(0.0, 0.0, 100.0, 50.0);
        n.stroke = Some(SceneStroke {
            color: Color::BLACK,
            width: 4.0,
            sides: None,
            align,
        });
        n
    }

    fn paint(node: &SceneNode) -> Vec<(f32, f32, f32, f32, f32)> {
        let mut backend = StrokeCaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let _ = paint_node_with_options(
            &mut cx,
            node,
            Point2D::ZERO,
            1.0,
            None,
            Rect::xywh(0.0, 0.0, 4000.0, 4000.0),
            None,
            None,
            None,
            None,
        );
        backend.strokes
    }

    /// Figma's default stroke is INSIDE: the painted band must sit
    /// entirely within the node bounds, so the centered stroke_rect
    /// call gets a half-width inset.
    #[test]
    fn inside_stroke_insets_by_half_width() {
        let strokes = paint(&stroked_rect(SceneStrokeAlign::Inside));
        assert_eq!(strokes, vec![(2.0, 2.0, 96.0, 46.0, 4.0)]);
    }

    /// OUTSIDE strokes outset by half a width.
    #[test]
    fn outside_stroke_outsets_by_half_width() {
        let strokes = paint(&stroked_rect(SceneStrokeAlign::Outside));
        assert_eq!(strokes, vec![(-2.0, -2.0, 104.0, 54.0, 4.0)]);
    }

    /// CENTER keeps the authored rect.
    #[test]
    fn center_stroke_keeps_rect() {
        let strokes = paint(&stroked_rect(SceneStrokeAlign::Center));
        assert_eq!(strokes, vec![(0.0, 0.0, 100.0, 50.0, 4.0)]);
    }
}

mod per_corner_radius_tests {
    use crate::layout_scene::{
        NodeKind, SceneGradient, SceneGradientStop, SceneNode, SceneStroke, SceneStrokeAlign,
    };
    use crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke;
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct RadiusCaptureBackend {
        uniform_fills: usize,
        per_corner_fills: Vec<[f32; 4]>,
        uniform_gradient_radii: Vec<f32>,
        per_corner_gradient_radii: Vec<[f32; 4]>,
        uniform_strokes: usize,
        per_corner_strokes: Vec<[f32; 4]>,
    }

    impl RenderBackend for RadiusCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
            self.uniform_fills += 1;
        }
        fn fill_round_rect_per_corner(&mut self, _: Rect, radii: [f32; 4], _: Color) {
            self.per_corner_fills.push(radii);
        }
        fn fill_round_rect_linear_gradient(
            &mut self,
            _: Rect,
            radius: f32,
            _: &[(f32, Color)],
            _: f32,
            _: f32,
        ) {
            self.uniform_gradient_radii.push(radius);
        }
        fn fill_round_rect_linear_gradient_per_corner(
            &mut self,
            _: Rect,
            radii: [f32; 4],
            _: &[(f32, Color)],
            _: f32,
            _: f32,
        ) {
            self.per_corner_gradient_radii.push(radii);
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {
            self.uniform_strokes += 1;
        }
        fn stroke_round_rect_per_corner(&mut self, _: Rect, radii: [f32; 4], _: Color, _: f32) {
            self.per_corner_strokes.push(radii);
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn painted(radii: [f32; 4]) -> RadiusCaptureBackend {
        let mut node = SceneNode::leaf("r", NodeKind::Rect);
        node.corner_radius = radii.iter().copied().fold(0.0, f32::max);
        node.corner_radii = Some(radii);
        node.fill = Some(Color::BLACK);
        node.stroke = Some(SceneStroke {
            color: Color::RED,
            width: 2.0,
            sides: None,
            align: SceneStrokeAlign::Center,
        });
        let mut backend = RadiusCaptureBackend::default();
        paint_fill_then_stroke(
            &mut PaintCx {
                backend: &mut backend,
            },
            &node,
            Rect::xywh(0.0, 0.0, 100.0, 50.0),
            1.0,
            node.fill,
        );
        backend
    }

    #[test]
    fn differing_radii_use_per_corner_backend_calls() {
        let backend = painted([8.0, 0.0, 8.0, 0.0]);
        assert_eq!(backend.per_corner_fills, vec![[8.0, 0.0, 8.0, 0.0]]);
        assert_eq!(backend.per_corner_strokes, vec![[8.0, 0.0, 8.0, 0.0]]);
        assert_eq!((backend.uniform_fills, backend.uniform_strokes), (0, 0));
    }

    #[test]
    fn equal_radii_keep_uniform_backend_calls() {
        let backend = painted([8.0; 4]);
        assert!(backend.per_corner_fills.is_empty());
        assert!(backend.per_corner_strokes.is_empty());
        assert_eq!((backend.uniform_fills, backend.uniform_strokes), (1, 1));
    }

    #[test]
    fn differing_radii_do_not_use_uniform_gradient_fill() {
        let mut node = SceneNode::leaf("gradient", NodeKind::Rect);
        node.corner_radius = 8.0;
        node.corner_radii = Some([8.0, 0.0, 8.0, 0.0]);
        node.gradient = Some(SceneGradient::Linear {
            angle_deg: 90.0,
            opacity: 1.0,
            stops: vec![
                SceneGradientStop {
                    offset: 0.0,
                    color: Color::BLACK,
                },
                SceneGradientStop {
                    offset: 1.0,
                    color: Color::WHITE,
                },
            ],
        });
        let mut backend = RadiusCaptureBackend::default();
        paint_fill_then_stroke(
            &mut PaintCx {
                backend: &mut backend,
            },
            &node,
            Rect::xywh(0.0, 0.0, 100.0, 50.0),
            1.0,
            node.fill,
        );

        assert!(
            backend.uniform_gradient_radii.is_empty(),
            "a per-corner gradient must not go through the scalar-radius fill"
        );
        assert_eq!(
            backend.per_corner_gradient_radii,
            vec![[8.0, 0.0, 8.0, 0.0]]
        );
    }
}

mod background_blur_tests {
    use crate::layout_scene::{Effect, NodeKind, SceneNode};
    use crate::widgets::canvas_viewport_paint::paint_node;
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    #[derive(Default)]
    struct BackdropCaptureBackend {
        ops: Vec<&'static str>,
    }

    impl RenderBackend for BackdropCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {
            self.ops.push("fill");
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {
            self.ops.push("clip");
        }
        fn clip_round_rect(&mut self, _: Rect, _: f32) {
            self.ops.push("clip_round");
        }
        fn save(&mut self) {
            self.ops.push("save");
        }
        fn restore(&mut self) {
            self.ops.push("restore");
        }
        fn push_backdrop_blur_layer(&mut self, _: f32) {
            self.ops.push("backdrop");
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {
            self.ops.push("fill");
        }
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn background_blur_clips_and_filters_before_node_fill() {
        let mut node = SceneNode::leaf("glass", NodeKind::Rect);
        node.bounds = Rect::xywh(0.0, 0.0, 100.0, 60.0);
        node.corner_radius = 8.0;
        node.fill = Some(Color::BLACK);
        node.effects = vec![Effect::BackgroundBlur { radius: 12.0 }];
        let mut backend = BackdropCaptureBackend::default();
        paint_node(
            &mut PaintCx {
                backend: &mut backend,
            },
            &node,
            Point2D::ZERO,
            1.0,
            Rect::xywh(-100.0, -100.0, 1000.0, 1000.0),
        );
        assert_eq!(
            backend.ops,
            vec![
                "save",
                "clip_round",
                "backdrop",
                "fill",
                "restore",
                "restore"
            ]
        );
    }
}

mod effect_lod_tests {
    use crate::layout_scene::{DropShadow, Effect, NodeKind, SceneNode};
    use crate::widgets::canvas_viewport_paint::paint_node;
    use crate::widgets::PaintCx;
    use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

    /// Records the expensive effect ops so tests can assert sub-pixel
    /// effects skip their save-layers at low zoom.
    #[derive(Default)]
    struct EffectCaptureBackend {
        ops: Vec<&'static str>,
    }

    impl RenderBackend for EffectCaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {
            self.ops.push("fill");
        }
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn push_blur_layer(&mut self, _: f32) {
            self.ops.push("blur");
        }
        fn push_backdrop_blur_layer(&mut self, _: f32) {
            self.ops.push("backdrop");
        }
        fn fill_drop_shadow(&mut self, _: Rect, _: f32, _: f32, _: Color) {
            self.ops.push("shadow");
        }
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    fn effect_node(effects: Vec<Effect>) -> SceneNode {
        let mut node = SceneNode::leaf("fx", NodeKind::Rect);
        node.bounds = Rect::xywh(0.0, 0.0, 100.0, 60.0);
        node.fill = Some(Color::BLACK);
        node.effects = effects;
        node
    }

    fn ops_at_zoom(node: &SceneNode, zoom: f32) -> Vec<&'static str> {
        let mut backend = EffectCaptureBackend::default();
        paint_node(
            &mut PaintCx {
                backend: &mut backend,
            },
            node,
            Point2D::ZERO,
            zoom,
            Rect::xywh(-1000.0, -1000.0, 4000.0, 4000.0),
        );
        backend.ops
    }

    fn ops_fast(node: SceneNode) -> Vec<&'static str> {
        let mut backend = EffectCaptureBackend::default();
        let nodes = vec![node];
        let _ = crate::widgets::canvas_viewport_paint::paint_scene_nodes_with_options_hiding(
            &mut PaintCx {
                backend: &mut backend,
            },
            &nodes,
            Point2D::ZERO,
            1.0,
            None,
            Rect::xywh(-1000.0, -1000.0, 4000.0, 4000.0),
            None,
            None,
            None,
            None,
            None,
            0,
            None,
            None,
            None,
            true,
        );
        backend.ops
    }

    #[test]
    fn fast_interaction_skips_every_effect_layer_at_any_zoom() {
        // During an active pan/zoom gesture the frame budget matters
        // more than effect fidelity (Figma-style interactive degrade):
        // shadows, layer blurs, and backdrop blurs all skip even at
        // fully visible sizes. Quality returns on gesture end.
        let node = effect_node(vec![
            Effect::DropShadow(DropShadow {
                offset_x: 4.0,
                offset_y: 4.0,
                blur: 12.0,
                color: Color::BLACK,
                inner: false,
            }),
            Effect::Blur(crate::layout_scene::BlurEffect { radius: 12.0 }),
            Effect::BackgroundBlur { radius: 12.0 },
        ]);
        let ops = ops_fast(node);
        assert!(!ops.contains(&"shadow"));
        assert!(!ops.contains(&"blur"));
        assert!(!ops.contains(&"backdrop"));
    }

    #[test]
    fn fast_interaction_skips_subpixel_leaves_but_keeps_visible_ones() {
        let mut tiny = SceneNode::leaf("tiny", NodeKind::Rect);
        tiny.bounds = Rect::xywh(0.0, 0.0, 0.5, 0.5);
        tiny.fill = Some(Color::BLACK);
        let mut big = SceneNode::leaf("big", NodeKind::Rect);
        big.bounds = Rect::xywh(10.0, 10.0, 100.0, 100.0);
        big.fill = Some(Color::BLACK);
        let mut frame = SceneNode::leaf("f", NodeKind::Frame);
        frame.bounds = Rect::xywh(0.0, 0.0, 200.0, 200.0);
        frame.children = vec![tiny, big];

        // Fast mode: the frame + the visible leaf fill; the sub-pixel
        // leaf paints nothing.
        assert_eq!(
            ops_fast(frame.clone())
                .iter()
                .filter(|op| **op == "fill")
                .count(),
            1
        );
    }

    #[test]
    fn subpixel_blur_skips_the_blur_save_layer() {
        // 4 px radius → sigma 2 at zoom 1 (visible), sigma 0.1 device
        // px at 5% zoom — invisible, but the save-layer still broke the
        // GPU render pass. A zoomed-out effect-dense page (3.8k blurs
        // visible at once) turned every pan frame into thousands of
        // render-pass submits.
        let node = effect_node(vec![Effect::Blur(crate::layout_scene::BlurEffect {
            radius: 4.0,
        })]);
        assert!(ops_at_zoom(&node, 1.0).contains(&"blur"));
        assert!(!ops_at_zoom(&node, 0.05).contains(&"blur"));
    }

    #[test]
    fn subpixel_backdrop_blur_skips_the_backdrop_layer() {
        let node = effect_node(vec![Effect::BackgroundBlur { radius: 4.0 }]);
        assert!(ops_at_zoom(&node, 1.0).contains(&"backdrop"));
        assert!(!ops_at_zoom(&node, 0.05).contains(&"backdrop"));
    }

    #[test]
    fn subpixel_shadow_skips_the_shadow_draw() {
        // Blur AND offset both under a third of a device pixel: the
        // shadow cannot move or soften the silhouette visibly.
        let node = effect_node(vec![Effect::DropShadow(DropShadow {
            offset_x: 2.0,
            offset_y: 2.0,
            blur: 4.0,
            color: Color::BLACK,
            inner: false,
        })]);
        assert!(ops_at_zoom(&node, 1.0).contains(&"shadow"));
        assert!(!ops_at_zoom(&node, 0.05).contains(&"shadow"));
    }

    #[test]
    fn visible_offset_keeps_the_shadow_even_with_tiny_blur() {
        // A hard-edged shadow displaced 40 doc px is still a visible
        // 2 px fringe at 5% zoom — only fully sub-pixel shadows skip.
        let node = effect_node(vec![Effect::DropShadow(DropShadow {
            offset_x: 40.0,
            offset_y: 0.0,
            blur: 0.0,
            color: Color::BLACK,
            inner: false,
        })]);
        assert!(ops_at_zoom(&node, 0.05).contains(&"shadow"));
    }
}
