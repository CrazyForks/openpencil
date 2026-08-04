(function (options) {
  "use strict";

  // `options.root` narrows the capture to one element's subtree instead of
  // the whole page; anything else (including no argument at all, which is how
  // this file is pasted into a console) captures `document.body`. The emitted
  // JSON has the same shape either way: `snapshot.root` is a node object with
  // page-absolute `rect` coordinates, and the importer places whatever it
  // finds there at the document origin with its descendants relative to it.
  var requestedRoot =
    options && options.root && options.root.nodeType === 1 ? options.root : null;

  var MAX_NODES = 20000;
  var MAX_IMAGE_EDGE = 2048;
  var MAX_IMAGE_DATA_BYTES = 24 * 1024 * 1024;
  var GRAY_PLACEHOLDER =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
  // Keys are lowercase: SVG elements report a lowercase `tagName` (they are
  // not HTML elements), so every tag comparison in this file goes through
  // `tagOf` and matches lowercase or inline `<svg>` slips through as a plain
  // container and its `<path>` children paint as empty boxes.
  var SKIP_TAGS = {
    script: true,
    style: true,
    noscript: true,
    template: true,
    meta: true,
    link: true,
    head: true,
  };
  var MEDIA_TAGS = { img: true, svg: true, canvas: true, video: true };
  var ELEMENT_STYLE_KEYS = [
    "background-color",
    "background-image",
    // Placement of a `url()` background layer. Without these three a sprite
    // sheet imports as the whole sheet stretched over the box, which is how a
    // logo turns into the wrong (and squashed) mark.
    "background-size",
    "background-position",
    "background-repeat",
    // Per-corner radii: the `border-radius` shorthand serializes as up to
    // four lengths ("6px 6px 0px 0px"), which the importer cannot parse as a
    // single length, so it dropped every non-uniform radius.
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
    "box-shadow",
    "border",
    "opacity",
    "overflow",
    "transform",
    "object-fit",
    "object-position",
    "mix-blend-mode",
    // Stacking inputs. CSS paints later siblings on top and lifts positioned
    // / z-indexed boxes out of document order; the canonical child array is
    // front-to-back. Without these the importer cannot reconstruct either,
    // and a full-bleed hero background buries everything it should sit under.
    "position",
    "z-index",
    // The parent's `display` is the third stacking input: CSS gives `z-index`
    // effect on a flex / grid *item* even while it stays `position: static`,
    // and that is exactly the negative-margin + z-index overlay idiom. Without
    // it the importer reads such an item as ordinary flow and inverts the
    // overlap. Pruned when it equals the initial value, so the common block
    // element still costs nothing.
    "display",
  ];
  var TEXT_STYLE_KEYS = [
    "color",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "line-height",
    "letter-spacing",
    "text-align",
    "white-space",
  ];
  // Computed values that carry no information: the importer's own defaults
  // are identical, so emitting them only inflates the payload (they are the
  // majority of every element's style block on a real page).
  var STYLE_DEFAULTS = {
    "background-color": "rgba(0, 0, 0, 0)",
    "background-image": "none",
    "background-size": "auto",
    "background-position": "0% 0%",
    "background-repeat": "repeat",
    "border-top-left-radius": "0px",
    "border-top-right-radius": "0px",
    "border-bottom-right-radius": "0px",
    "border-bottom-left-radius": "0px",
    "box-shadow": "none",
    opacity: "1",
    overflow: "visible",
    transform: "none",
    "object-fit": "fill",
    "object-position": "50% 50%",
    "mix-blend-mode": "normal",
    position: "static",
    "z-index": "auto",
    "white-space": "normal",
    display: "block",
  };

  var nodeCount = 0;
  var embeddedImageBytes = 0;
  var remoteImageCount = 0;
  var truncated = false;

  function round(value) {
    return Math.round(value * 100) / 100;
  }

  function pageRect(rect) {
    return {
      x: round(rect.left + window.scrollX),
      y: round(rect.top + window.scrollY),
      w: round(rect.width),
      h: round(rect.height),
    };
  }

  function takeNode() {
    if (nodeCount >= MAX_NODES) {
      truncated = true;
      return false;
    }
    nodeCount += 1;
    return true;
  }

  function tagOf(element) {
    return (element.tagName || "").toLowerCase();
  }

  function copyStyles(computed, keys) {
    var result = {};
    keys.forEach(function (key) {
      var value = computed.getPropertyValue(key);
      if (value === "" || value === undefined) return;
      // `border` serializes as "<width> <style> <color>"; a zero width means
      // no border whatever the (always present) colour is.
      if (key === "border" && value.indexOf("0px") === 0) return;
      if (STYLE_DEFAULTS[key] === value) return;
      result[key] = value;
    });
    return result;
  }

  var BORDER_SIDES = ["top", "right", "bottom", "left"];

  // Element styles plus the per-side border longhands for the sides that
  // actually draw. The `border` shorthand serializes to "" whenever the four
  // sides differ, so a lone `border-bottom` divider (every table row on a
  // real page) reached the importer as "no border at all".
  function elementStyles(computed) {
    var result = copyStyles(computed, ELEMENT_STYLE_KEYS);
    BORDER_SIDES.forEach(function (side) {
      var width = computed.getPropertyValue("border-" + side + "-width");
      if (!width || parseFloat(width) <= 0) return;
      var style = computed.getPropertyValue("border-" + side + "-style");
      if (!style || style === "none" || style === "hidden") return;
      result["border-" + side + "-width"] = width;
      result["border-" + side + "-style"] = style;
      result["border-" + side + "-color"] =
        computed.getPropertyValue("border-" + side + "-color");
    });
    return result;
  }

  function childHasVisibleBox(element) {
    return Array.prototype.some.call(element.children, function (child) {
      var style = window.getComputedStyle(child);
      var rect = child.getBoundingClientRect();
      return (
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        Number(style.opacity) !== 0 &&
        (rect.width >= 0.5 || rect.height >= 0.5)
      );
    });
  }

  function visibleElement(element, computed, rect) {
    if (
      computed.display === "none" ||
      computed.visibility === "hidden" ||
      Number(computed.opacity) === 0
    ) {
      return false;
    }
    if (rect.width < 0.5 || rect.height < 0.5) {
      return childHasVisibleBox(element);
    }
    return true;
  }

  function buildText(textNode, parentComputed) {
    // Collapse runs of whitespace the way CSS does, but keep a single space at
    // either boundary: the gap between two inline runs lives at the end of one
    // text node, and trimming it welded neighbouring runs together
    // ("252 points by gyan" imported as "252 pointsby gyan"). A node that is
    // nothing but whitespace still drops out.
    //
    // A boundary space is only real when there is a sibling on that side. The
    // leading and trailing whitespace of an *only* child is the source file's
    // pretty-printing indentation, which CSS discards and which otherwise
    // imports as a phantom space — " Hello world " in a box measured for
    // "Hello world", i.e. every indented `<p>` starts one space too far right.
    var text = (textNode.textContent || "").replace(/\s+/g, " ");
    if (!text.trim()) return null;
    if (!textNode.previousSibling) text = text.replace(/^ /, "");
    if (!textNode.nextSibling) text = text.replace(/ $/, "");
    var range = document.createRange();
    range.selectNodeContents(textNode);
    var rect = range.getBoundingClientRect();
    // One client rect per line box: the closest reliable signal for whether
    // the browser wrapped this run. The importer needs it to decide between a
    // hugging single-line label (which must be allowed to grow when our font
    // metrics are wider than the page's) and a run that has to keep wrapping
    // at the captured width.
    //
    // Caveat: a line box is not the only thing that splits a range into
    // several client rects — a bidi direction change inside one unwrapped run
    // splits it too, so this can over-report. Over-reporting is the safe
    // direction (the importer then keeps the captured width instead of
    // hugging), which is why it is not corrected for here.
    var lines = range.getClientRects().length;
    if (typeof range.detach === "function") range.detach();
    if (rect.width < 0.5 || rect.height < 0.5 || !takeNode()) return null;
    return {
      kind: "text",
      rect: pageRect(rect),
      text: text,
      lines: lines || 1,
      styles: copyStyles(parentComputed, TEXT_STYLE_KEYS),
    };
  }

  function scaledCanvas(width, height) {
    var longest = Math.max(width, height, 1);
    var scale = Math.min(1, MAX_IMAGE_EDGE / longest);
    var canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(width * scale));
    canvas.height = Math.max(1, Math.round(height * scale));
    return canvas;
  }

  function keepDataUrl(dataUrl, fallbackUrl) {
    if (!dataUrl || embeddedImageBytes + dataUrl.length > MAX_IMAGE_DATA_BYTES) {
      remoteImageCount += 1;
      return { src: fallbackUrl || GRAY_PLACEHOLDER, tainted: true };
    }
    embeddedImageBytes += dataUrl.length;
    return { src: dataUrl, tainted: false };
  }

  function rasterizeImage(image) {
    var fallback = image.currentSrc || image.src || GRAY_PLACEHOLDER;
    if (!image.complete || !image.naturalWidth || !image.naturalHeight) {
      remoteImageCount += 1;
      return { src: fallback, tainted: true };
    }
    try {
      var canvas = scaledCanvas(image.naturalWidth, image.naturalHeight);
      canvas
        .getContext("2d")
        .drawImage(image, 0, 0, canvas.width, canvas.height);
      return keepDataUrl(canvas.toDataURL("image/png"), fallback);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: fallback, tainted: true };
    }
  }

  var SVG_SHAPE_TAGS = {
    path: true,
    rect: true,
    circle: true,
    ellipse: true,
    line: true,
    polygon: true,
    polyline: true,
  };

  var NUMBER_PATTERN = /-?\d*\.?\d+(?:e[-+]?\d+)?/gi;

  // Twice the shoelace area of a closed point ring. Positive means clockwise
  // in SVG's y-down space.
  function signedArea(points) {
    var total = 0;
    for (var index = 0; index < points.length; index += 1) {
      var current = points[index];
      var next = points[(index + 1) % points.length];
      total += current[0] * next[1] - next[0] * current[1];
    }
    return total;
  }

  // `points` list → path data with a normalized (clockwise) winding.
  //
  // WINDING IS LOAD-BEARING. Every subpath merged into the single emitted `d`
  // has to wind the same way: the browser fills each shape element
  // independently — so winding never interacts there — but the merged path
  // under the `nonzero` rule *cancels* a contained subpath that winds the
  // other way, punching a hole where the page paints solid. Every primitive
  // this file generates therefore winds clockwise: `rect` (right / down /
  // left) and the rounded-rect arcs (`sweep 1`) already did, the ellipse arcs
  // are emitted with `sweep 1` for the same reason, and a `points` ring is
  // reversed here when the author wrote it counter-clockwise.
  function pointsToPathData(element, closed) {
    var raw = (element.getAttribute("points") || "").match(NUMBER_PATTERN);
    if (!raw || raw.length < 4) return "";
    var points = [];
    for (var index = 0; index + 1 < raw.length; index += 2) {
      var x = parseFloat(raw[index]);
      var y = parseFloat(raw[index + 1]);
      if (!isFinite(x) || !isFinite(y)) return "";
      points.push([x, y]);
    }
    if (signedArea(points) < 0) points.reverse();
    var data = "";
    for (var at = 0; at < points.length; at += 1) {
      data += (at ? " " : "M") + points[at][0] + " " + points[at][1];
    }
    return data + (closed ? "Z" : "");
  }

  // Shape element → SVG path data in the element's own user units.
  function shapeToPathData(element, tag) {
    var attr = function (name, fallback) {
      var value = parseFloat(element.getAttribute(name));
      return isFinite(value) ? value : fallback;
    };
    if (tag === "path") return element.getAttribute("d") || "";
    if (tag === "rect") {
      var x = attr("x", 0);
      var y = attr("y", 0);
      var w = attr("width", 0);
      var h = attr("height", 0);
      if (w <= 0 || h <= 0) return "";
      var rx = Math.min(attr("rx", attr("ry", 0)), w / 2);
      var ry = Math.min(attr("ry", attr("rx", 0)), h / 2);
      if (rx > 0 && ry > 0) {
        return (
          "M" + (x + rx) + " " + y + "H" + (x + w - rx) +
          "A" + rx + " " + ry + " 0 0 1 " + (x + w) + " " + (y + ry) +
          "V" + (y + h - ry) +
          "A" + rx + " " + ry + " 0 0 1 " + (x + w - rx) + " " + (y + h) +
          "H" + (x + rx) +
          "A" + rx + " " + ry + " 0 0 1 " + x + " " + (y + h - ry) +
          "V" + (y + ry) +
          "A" + rx + " " + ry + " 0 0 1 " + (x + rx) + " " + y + "Z"
        );
      }
      return "M" + x + " " + y + "h" + w + "v" + h + "h" + -w + "Z";
    }
    if (tag === "circle" || tag === "ellipse") {
      var cx = attr("cx", 0);
      var cy = attr("cy", 0);
      var radiusX = tag === "circle" ? attr("r", 0) : attr("rx", 0);
      var radiusY = tag === "circle" ? attr("r", 0) : attr("ry", 0);
      if (radiusX <= 0 || radiusY <= 0) return "";
      // `sweep 1` — clockwise, matching the `rect` branch. See the winding
      // note on `pointsToPathData`: an ellipse drawn the other way around
      // erases a rect it sits inside once the two are merged into one path.
      return (
        "M" + (cx - radiusX) + " " + cy +
        "a" + radiusX + " " + radiusY + " 0 1 1 " + radiusX * 2 + " 0" +
        "a" + radiusX + " " + radiusY + " 0 1 1 " + -radiusX * 2 + " 0Z"
      );
    }
    if (tag === "line") {
      return (
        "M" + attr("x1", 0) + " " + attr("y1", 0) +
        "L" + attr("x2", 0) + " " + attr("y2", 0)
      );
    }
    return pointsToPathData(element, tag === "polygon");
  }

  // A CTM entry below this is rounding noise rather than a real skew.
  var CTM_SKEW_EPSILON = 1e-6;

  // Union of two user-unit boxes; either may be null.
  function unionBox(left, right) {
    if (!left) return right;
    if (!right) return left;
    var x = Math.min(left.x, right.x);
    var y = Math.min(left.y, right.y);
    return {
      x: x,
      y: y,
      width: Math.max(left.x + left.width, right.x + right.width) - x,
      height: Math.max(left.y + left.height, right.y + right.height) - y,
    };
  }

  function shapeBox(element) {
    if (typeof element.getBBox !== "function") return null;
    try {
      var box = element.getBBox();
      if (!box || !isFinite(box.x) || !isFinite(box.y)) return null;
      if (!isFinite(box.width) || !isFinite(box.height)) return null;
      return box;
    } catch (_error) {
      return null;
    }
  }

  // Vectorize an inline `<svg>` into one path.
  //
  // Skia decodes PNG / JPEG / GIF / WebP — not SVG — so an SVG data URI
  // reaches the canvas as an undecodable image and paints as a placeholder.
  // Icon sets are the overwhelming majority of inline SVG on a real page, and
  // they are plain filled paths in a single colour, which the canonical
  // schema paints natively as a `path` node.
  //
  // Everything is emitted in user units and the reported rect is the art's own
  // page-space bounding box, because the renderer fits path data to the node
  // box by its bounds: taking the box from the shapes' own bounds keeps the
  // artwork's aspect and position exact instead of stretching it to the
  // element rect. Anything that needs more than one flat colour, carries a
  // per-element transform, or references external paint (`<use>`, gradients,
  // masks, nested images) is left to the image path.
  //
  // Returns `{ d, fill, fillRule?, rect }` — a fragment `buildImage` merges
  // into the image node, never a node of its own.
  function vectorizeSvg(svg) {
    if (typeof svg.getScreenCTM !== "function") return null;
    var rootMatrix = svg.getScreenCTM();
    if (!rootMatrix) return null;
    // The rect this emits is an axis-aligned box, so a rotated / skewed CTM
    // cannot be expressed by it — mapping a box through one and keeping only
    // the diagonal produces dimensions that belong to no shape on the page.
    // Those SVGs go down the image path, which carries the transform in the
    // element rect the browser already resolved.
    if (
      Math.abs(rootMatrix.b) > CTM_SKEW_EPSILON ||
      Math.abs(rootMatrix.c) > CTM_SKEW_EPSILON ||
      !(rootMatrix.a > 0) ||
      !(rootMatrix.d > 0)
    ) {
      return null;
    }
    var nodes = svg.querySelectorAll("*");
    var data = [];
    var fill = null;
    var fillRule = "";
    // Bounds of the shapes that actually contribute path data — NOT
    // `svg.getBBox()`, which measures every shape in the tree including the
    // ones skipped below. Material's icon set opens each glyph with
    // `<path fill="none" d="M0 0h24v24H0z"/>`: a full-viewBox sizing rect that
    // paints nothing but doubles the root bbox, so sizing from it scaled and
    // shifted every glyph by exactly that factor.
    var box = null;
    for (var index = 0; index < nodes.length; index += 1) {
      var node = nodes[index];
      var tag = tagOf(node);
      if (tag === "title" || tag === "desc" || tag === "metadata" || tag === "g") {
        // A `<g>` may only carry grouping; a transform on it moves its
        // children out of the shared user space this path assumes.
        if (tag === "g" && node.getAttribute("transform")) return null;
        continue;
      }
      if (!SVG_SHAPE_TAGS[tag]) return null;
      if (node.getAttribute("transform")) return null;
      var style = window.getComputedStyle(node);
      if (style.display === "none" || style.visibility === "hidden") continue;
      // Stroked icon sets (lucide et al.) need stroke geometry the schema's
      // single fill cannot express.
      if (style.stroke && style.stroke !== "none") return null;
      // An unstroked `<line>` paints nothing at all (a line encloses no area,
      // so it cannot be filled); a stroked one already bailed above.
      if (tag === "line") continue;
      var shapeFill = style.fill;
      if (!shapeFill || shapeFill === "none") continue;
      if (shapeFill.indexOf("url(") === 0) return null;
      if (fill === null) fill = shapeFill;
      else if (fill !== shapeFill) return null;
      var segment = shapeToPathData(node, tag);
      if (!segment) return null;
      var segmentBox = shapeBox(node);
      if (!segmentBox) return null;
      box = unionBox(box, segmentBox);
      // The fill rule of the shapes that paint, read once. Mixed rules cannot
      // survive the merge into a single path.
      var shapeRule = style.getPropertyValue("fill-rule") || "nonzero";
      if (!fillRule) fillRule = shapeRule;
      else if (fillRule !== shapeRule) return null;
      data.push(segment);
    }
    if (!data.length || !fill || !box) return null;
    if (!(box.width > 0) || !(box.height > 0)) return null;
    // User units → page pixels through the root CTM, so viewBox scaling and
    // `preserveAspectRatio` are already accounted for. The CTM is known
    // axis-aligned by the guard above, so the off-diagonal terms are zero.
    var vector = {
      rect: pageRect({
        left: rootMatrix.a * box.x + rootMatrix.e,
        top: rootMatrix.d * box.y + rootMatrix.f,
        width: rootMatrix.a * box.width,
        height: rootMatrix.d * box.height,
      }),
      d: data.join(" "),
      fill: fill,
    };
    if (!(vector.rect.w > 0) || !(vector.rect.h > 0)) return null;
    if (fillRule === "evenodd") vector.fillRule = "evenodd";
    return vector;
  }

  function encodeSvg(svg, computed, rect) {
    try {
      var clone = svg.cloneNode(true);
      if (!clone.getAttribute("xmlns")) {
        clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
      }
      // Icon sets (GitHub's octicons, lucide, heroicons) paint with
      // `fill="currentColor"`, which resolves against the inherited text
      // colour. A standalone data URI inherits nothing, so bake the computed
      // colour in or every icon rasterizes black — or, on a dark page,
      // invisible.
      var color = computed.getPropertyValue("color");
      if (color) clone.style.color = color;
      // Serialize at the used box size so the rasterizer scales the viewBox
      // exactly the way the page did; CSS-sized SVGs carry no useful
      // width/height attributes.
      if (rect.width >= 1) clone.setAttribute("width", round(rect.width));
      if (rect.height >= 1) clone.setAttribute("height", round(rect.height));
      var xml = new XMLSerializer().serializeToString(clone);
      var encoded = btoa(unescape(encodeURIComponent(xml)));
      return keepDataUrl("data:image/svg+xml;base64," + encoded, GRAY_PLACEHOLDER);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: GRAY_PLACEHOLDER, tainted: true };
    }
  }

  function captureCanvas(canvas) {
    try {
      return keepDataUrl(canvas.toDataURL("image/png"), GRAY_PLACEHOLDER);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: GRAY_PLACEHOLDER, tainted: true };
    }
  }

  function captureVideo(video) {
    // A hero background video is usually cross-origin (tainting the canvas)
    // and often has no decoded frame yet, so the poster is the honest
    // fallback: a remote poster URL still shows the intended artwork, a gray
    // 1×1 placeholder stretched full-bleed shows nothing.
    var poster = video.poster || GRAY_PLACEHOLDER;
    try {
      var width = video.videoWidth || video.clientWidth;
      var height = video.videoHeight || video.clientHeight;
      if (!video.videoWidth || video.readyState < 2) {
        remoteImageCount += 1;
        return { src: poster, tainted: true };
      }
      var canvas = scaledCanvas(width, height);
      canvas
        .getContext("2d")
        .drawImage(video, 0, 0, canvas.width, canvas.height);
      return keepDataUrl(canvas.toDataURL("image/png"), poster);
    } catch (_error) {
      remoteImageCount += 1;
      return { src: poster, tainted: true };
    }
  }

  function buildImage(element, computed, rect) {
    if (!takeNode()) return null;
    var tag = tagOf(element);
    var captured;
    if (tag === "img") captured = rasterizeImage(element);
    else if (tag === "svg") captured = encodeSvg(element, computed, rect);
    else if (tag === "canvas") captured = captureCanvas(element);
    else captured = captureVideo(element);
    var result = {
      kind: "image",
      tag: tag,
      rect: pageRect(rect),
      src: captured.src,
      styles: elementStyles(computed),
    };
    if (captured.tainted) result.tainted = true;
    // Vector geometry rides ALONGSIDE the image serialization rather than
    // replacing it, and the node keeps `kind: "image"`. An importer that
    // predates the vector fields reads exactly the node it always read (it
    // dispatches on `kind`, and an unknown one is dropped with a warning —
    // the icon would vanish rather than degrade); one that understands them
    // prefers `d` and ignores `src`. `vectorRect` is the artwork's own bounds,
    // which is what the path node has to be sized to; `rect` stays the
    // element's used box so the image fallback keeps landing where it did.
    if (tag === "svg") {
      var vector = vectorizeSvg(element);
      if (vector) {
        result.d = vector.d;
        result.fill = vector.fill;
        result.vectorRect = vector.rect;
        if (vector.fillRule) result.fillRule = vector.fillRule;
      }
    }
    return result;
  }

  function buildElement(element) {
    var tag = tagOf(element);
    if (SKIP_TAGS[tag]) return null;
    var computed = window.getComputedStyle(element);
    var rect = element.getBoundingClientRect();
    if (!visibleElement(element, computed, rect)) return null;
    if (MEDIA_TAGS[tag]) {
      return buildImage(element, computed, rect);
    }
    if (!takeNode()) return null;
    var children = [];
    var collect = function (child) {
      if (truncated) return;
      var mapped = null;
      if (child.nodeType === Node.TEXT_NODE) {
        mapped = buildText(child, computed);
      } else if (child.nodeType === Node.ELEMENT_NODE) {
        mapped = buildElement(child);
      }
      if (mapped) children.push(mapped);
    };
    // An open shadow root holds everything a web component actually renders.
    // Walking only `childNodes` captured such an element as an empty box —
    // GitHub's `<relative-time>` timestamps, and every design system built on
    // custom elements, vanished that way.
    if (element.shadowRoot) {
      Array.prototype.forEach.call(element.shadowRoot.childNodes, collect);
    }
    Array.prototype.forEach.call(element.childNodes, collect);
    return {
      kind: "element",
      tag: tag,
      rect: pageRect(rect),
      styles: elementStyles(computed),
      children: children,
    };
  }

  // A picked element can itself be an <img>/<svg>/<canvas>/<video>, which
  // maps to an image node. The importer reads `snapshot.root` as a container,
  // so wrap that one case in a synthetic element of the same size rather than
  // emitting an image at the top level.
  function buildRoot(element) {
    var node = buildElement(element);
    if (!node || node.kind === "element") return node;
    return {
      kind: "element",
      tag: "div",
      rect: node.rect,
      styles: {},
      children: [node],
    };
  }

  var target = requestedRoot || document.body;
  if (!target) {
    console.error("OpenPencil snapshot: document.body is not available");
    return;
  }
  var root = buildRoot(target);
  if (!root) {
    console.error(
      requestedRoot
        ? "OpenPencil snapshot: the selected element is not visible"
        : "OpenPencil snapshot: the page has no visible body",
    );
    return;
  }
  // The page's own backdrop. A subtree capture (element pick) usually starts
  // below the element that carries it, and a dark-theme page rendered onto
  // the importer's white default reads as washed-out text on the wrong
  // canvas.
  function pageBackground() {
    var nodes = [document.body, document.documentElement];
    for (var index = 0; index < nodes.length; index += 1) {
      if (!nodes[index]) continue;
      var color = window.getComputedStyle(nodes[index]).backgroundColor;
      if (color && color !== "transparent" && !/, *0\)$/.test(color)) return color;
    }
    return "rgb(255, 255, 255)";
  }

  var snapshot = {
    version: 1,
    source: window.location.href,
    title: document.title,
    background: pageBackground(),
    viewport: {
      width: round(window.innerWidth),
      height: round(window.innerHeight),
    },
    root: root,
  };
  if (truncated) snapshot.truncated = true;
  var output = JSON.stringify(snapshot, null, 2);

  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(output).catch(function (error) {
      console.warn("OpenPencil snapshot: clipboard copy failed", error);
    });
  }
  var blobUrl = URL.createObjectURL(
    new Blob([output], { type: "application/json" }),
  );
  var download = document.createElement("a");
  download.href = blobUrl;
  download.download = "snapshot.json";
  download.click();
  setTimeout(function () {
    URL.revokeObjectURL(blobUrl);
  }, 0);
  console.log("OpenPencil snapshot ready", {
    nodes: nodeCount,
    bytes: output.length,
    embeddedImageBytes: embeddedImageBytes,
    remoteImages: remoteImageCount,
    truncated: truncated,
  });
  // The argument is optional on purpose. Pasted into a devtools console the
  // global is undefined and this captures the whole page, exactly as before;
  // a host that wants a subtree sets `{ root: <element> }` on the global
  // first (the Chrome extension does, in its own isolated world).
})(typeof globalThis === "undefined" ? null : globalThis.openpencilSnapshotOptions);
