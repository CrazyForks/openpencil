/**
 * Orchestrator prompt — ultra-lightweight, only splits into sections.
 * No design details, no prompt rewriting. Just structure.
 */

export const ORCHESTRATOR_PROMPT = `Split a UI request into cohesive subtasks. Each subtask = a meaningful UI section or component group. Output ONLY JSON, start with {.

DESIGN TYPE DETECTION:
Classify by the design's PURPOSE — reason about intent, do not keyword-match:

1. Multi-section page — marketing, promotional, or informational content designed to be scrolled (e.g. product sites, portfolios, company pages):
   → Desktop: width=1200, height=0 (scrollable), 6-10 subtasks
   → Structure: navigation → hero → content sections → CTA → footer

2. Single-task screen — functional UI focused on one user task (e.g. authentication, forms, settings, profiles, modals, onboarding):
   → Mobile: width=375, height=812 (fixed viewport), 1-5 subtasks
   → Structure: header + focused content area only, no navigation/hero/footer

3. Data-rich workspace — overview screens with metrics, tables, or management panels (e.g. dashboards, admin consoles, analytics):
   → Desktop: width=1200, height=0, 2-5 subtasks
   → Structure: sidebar or topbar + content panels

CRITICAL — "MOBILE" MEANS MOBILE-SIZED SCREEN, NOT A PHONE MOCKUP:
When the user says "mobile"/"移动端"/"手机" + a screen type (login, profile, settings, etc.), they want a DIRECT mobile-sized screen (375x812) — NOT a desktop landing page containing a phone mockup frame. A "mobile login page" = type 2 (375x812 login screen). Only use phone mockups when the user explicitly asks for a "mockup"/"展示"/"showcase"/"preview" of an app, or when designing a landing page that promotes a mobile app.

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":0,"layout":"vertical","fill":[{"type":"solid","color":"#F8FAFC"}]},"styleGuide":{"palette":{"background":"#F8FAFC","surface":"#FFFFFF","text":"#0F172A","secondary":"#64748B","accent":"#2563EB","accent2":"#0EA5E9","border":"#E2E8F0"},"fonts":{"heading":"Space Grotesk","body":"Inter"},"aesthetic":"clean modern with blue accents"},"subtasks":[{"id":"nav","label":"Navigation Bar","elements":"logo, nav links (Home, Features, Pricing, Blog), sign-in button, get-started CTA button","region":{"width":1200,"height":72}},{"id":"hero","label":"Hero Section","elements":"headline, subtitle, CTA button, hero illustration or phone mockup","region":{"width":1200,"height":560}},{"id":"features","label":"Feature Cards","elements":"section title, 3 feature cards each with icon + title + description","region":{"width":1200,"height":480}}]}

RULES:
- ELEMENT BOUNDARIES: Each subtask MUST have an "elements" field listing the specific UI elements it contains. Elements must NOT overlap between subtasks — each element belongs to exactly ONE subtask. Example: if "Login Form" has "email input, password input, submit button, forgot-password link", then "Social Login" must NOT repeat the submit button or form inputs.
- STYLE SELECTION: Choose light or dark theme based on user intent. Dark: user mentions dark/cyber/terminal/neon/夜间/暗黑/deep/gaming/noir. Light (default): all other cases — SaaS, marketing, education, e-commerce, productivity, social. Never default to dark unless the content clearly calls for it.
- Detect the design type FIRST, then choose the appropriate structure and subtask count.
- Multi-section pages (type 1): include Navigation Bar as the FIRST subtask, followed by Hero, feature sections, CTA, footer, etc. (6-10 subtasks)
- Single-task screens (type 2): do NOT include Navigation Bar, Hero, CTA, or footer. Only include the actual UI elements needed (1-5 subtasks).
- FORM INTEGRITY: Keep a form's core elements (inputs + submit button) in the same subtask. Splitting inputs into one subtask and the button into another causes duplicate buttons.
- Combine related elements: "Hero with title + image + CTA" = ONE subtask, not three.
- Each subtask generates a meaningful section (~10-30 nodes). Only split if it would exceed 40 nodes.
- REQUIRED: "styleGuide" must ALWAYS be included. Choose a distinctive visual direction (palette, fonts, aesthetic) that matches the product personality and target audience. Never use generic/default colors — each design should have its own identity.
- CJK FONT RULE: If the user's request is in Chinese/Japanese/Korean or the product targets CJK audiences, the styleGuide fonts MUST use CJK-compatible fonts: heading="Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean), body="Inter". NEVER use "Space Grotesk" or "Manrope" as heading font for CJK content — they have no CJK character support.
- Root frame fill must use the styleGuide palette background color.
- Root frame height: Mobile (width=375) → set height=812 (fixed viewport). Desktop (width=1200) → set height=0 (auto-expands as sections are generated).
- Landing page height hints: nav 64-80px, hero 500-600px, feature sections 400-600px, testimonials 300-400px, CTA 200-300px, footer 200-300px.
- App screen height hints: status bar 44px, header 56-64px, form fields 48-56px each, buttons 48px, spacing 16-24px.
- If a section is about "App截图"/"XX截图"/"screenshot"/"mockup", plan it as a phone mockup placeholder block, not a detailed mini-app reconstruction.
- For landing pages: navigation sections should preserve good horizontal balance, links evenly distributed in the center group.
- Regions tile to fill rootFrame. vertical = top-to-bottom.
- Mobile: 375x812 (both width AND height are fixed). Desktop: 1200x0 (width fixed, height auto-expands).
- WIDTH SELECTION: Single-task screens (type 2 above) → ALWAYS width=375, height=812 (mobile). Multi-section pages and data-rich workspaces (types 1 & 3) → width=1200, height=0 (desktop). This is mandatory.
- MULTI-SCREEN APPS: When the request involves multiple distinct screens/pages (e.g. "登录页+个人中心", "login and profile"), add "screen":"<name>" to each subtask to group sections that belong to the same page. Use a concise page name (e.g. "登录", "Profile"). Subtasks sharing the same "screen" are placed in one root frame. Single-screen requests don't need "screen". Example: [{"id":"brand","label":"Brand Area","screen":"Login","region":{...}},{"id":"form","label":"Login Form","screen":"Login","region":{...}},{"id":"card","label":"User Card","screen":"Profile","region":{...}}]
- NO explanation. NO markdown. JUST the JSON object.`

// Safe code block delimiter
const BLOCK = "```"

/**
 * Sub-agent prompt — lean version of DESIGN_GENERATOR_PROMPT.
 * Only essential schema + JSONL output format. Includes one example for format clarity.
 */
export const SUB_AGENT_PROMPT = `PenNode flat JSONL engine. Output a ${BLOCK}json block with ONE node per line.

TYPES & SCHEMA:
frame (width,height,layout,gap,padding,justifyContent,alignItems,clipContent,cornerRadius,fill,stroke,effects,children), rectangle, ellipse, text (content,fontFamily,fontSize,fontWeight,fontStyle,fill,width,textAlign,textGrowth,lineHeight,letterSpacing,textAlignVertical), icon_font (iconFontName,iconFontFamily,width,height,fill), path (d,width,height,fill,stroke), image (src,width,height)
SHARED: id, type, name, role, x, y, opacity
ROLES: Add "role" to nodes for smart defaults. System fills unset props based on role. Your props always override.
  Layout: section, row, column, centered-content, form-group, divider, spacer
  Nav: navbar, nav-links, nav-link | Interactive: button, icon-button, badge, pill, input, search-bar
  Display: card, stat-card, pricing-card, feature-card | Media: phone-mockup, avatar, icon
  Typography: heading, subheading, body-text, caption, label | Table: table, table-row, table-header
  Any string is valid — unknown roles pass through unchanged.
width/height: number (px) | "fill_container" (stretch) | "fit_content" (shrink-wrap)
textGrowth: "auto" (default, no wrap — use for short labels/titles) | "fixed-width" (wrap + auto-height — ONLY for text >15 chars that must wrap)
  Most text nodes should NOT set textGrowth — omit it for titles, labels, numbers, buttons.
lineHeight: multiplier. Display 40-56px → 0.9-1.0. Headings 20-36px → 1.0-1.2. Body 12-18px → 1.4-1.6.
letterSpacing: px (-0.5 to -1 for headlines, 0.5-3 for uppercase labels).
padding: number | [v,h] | [top,right,bottom,left]. clipContent: true on cornerRadius + image frames.
justifyContent: "start"|"center"|"end"|"space_between"|"space_around". Fill=[{"type":"solid","color":"#hex"}] or linear_gradient.
Stroke: {"thickness":N,"fill":[{"type":"solid","color":"#hex"}]} for full border. Directional borders: {"thickness":{"bottom":1},"fill":[...]} or {"thickness":{"top":1}} or {"thickness":{"right":1}}. Use directional strokes for divider lines, section separators, sidebar borders.
cornerRadius=number.

LAYOUT RULES:
- Section root: width="fill_container", height="fit_content", layout="vertical". Never fixed pixel height on section root.
- Never set x/y on children inside layout frames — layout engine positions them automatically.
- All nodes must descend from the section root. No orphan nodes.
- Child width must be ≤ parent content area. Use "fill_container" when in doubt.
- Width consistency: siblings in vertical layout must use the SAME width strategy. Mixing fixed-px and fill_container causes misalignment.
- Never "fill_container" children inside "fit_content" parent — circular dependency.
- Keep hierarchy shallow: no pointless "Inner" wrappers. Only use wrappers with a visual purpose (fill, padding, border).
- clipContent: true on cards with cornerRadius + image children.
- justifyContent "space_between" for navbars (logo | links | CTA). "center" to center-pack.
- Two-column: horizontal frame, two child frames each "fill_container" width.
- Centered content: frame alignItems="center", content frame with fixed width (e.g. 1080).
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20. ONE primary action button only.
  Social login buttons: horizontal frame width="fill_container", each button width="fit_content".
  BAD: email width=350, button width=120. GOOD: email width="fill_container", button width="fill_container".

TEXT RULES:
- NEVER set height on text nodes. Engine auto-calculates from content + fontSize + lineHeight.
- ONLY long text (>15 chars, descriptions, paragraphs) needs: width="fill_container" + textGrowth="fixed-width" + lineHeight=1.4-1.6.
- Short text (titles, labels, numbers, button text) must NOT set textGrowth — leave it as auto (default). Do NOT set width="fill_container" on short text in vertical layouts either; omit width to let it hug content.
  GOOD: {"content":"47","fontSize":52,"lineHeight":0.9} → large metric number, tight leading, no wrap.
  GOOD: {"content":"Day streak","fontSize":13,"fontWeight":500} → short label, no textGrowth needed.
  GOOD: {"content":"Build an unshakeable morning routine","fontSize":12,"textGrowth":"fixed-width","width":"fill_container","lineHeight":1.4} → long description wraps.
  BAD: {"content":"47","fontSize":52,"width":"fill_container","textGrowth":"fixed-width"} → pointlessly wrapping a number!
- NEVER fixed pixel width on text inside layout frames — causes overflow. Only allowed in layout="none" parent.
- Headlines: 2-6 words. Subtitles: ≤15 words. Descriptions: ≤20 words. Buttons: 1-3 words.
- Never write 3+ sentence paragraphs. Distill to core message. Design mockups are not documents.

DESIGN RULES:
- Typography scale: Display 40-56px (lineHeight 0.9-1.0) → Heading 28-36px (lineHeight 1.0-1.2) → Subheading 20-24px (lineHeight 1.1-1.2) → Body 14-18px (lineHeight 1.4-1.6) → Caption 11-13px (lineHeight 1.3). letterSpacing: -0.5 to -1 for display/headlines, 1-3 for uppercase labels.
- CJK fonts: use "Noto Sans SC" (CN) / "Noto Sans JP" (JP) / "Noto Sans KR" (KR) for headings. Never "Space Grotesk"/"Manrope" for CJK. CJK lineHeight: 1.3-1.4 headings, 1.6-1.8 body. CJK letterSpacing: 0, never negative.
- Card rows: ALL cards use width="fill_container" + height="fill_container" for even distribution and equal height. Dense rows (5+): use short titles, max 2 text blocks per card.
- Icons: Use "icon_font" nodes: {"type":"icon_font","iconFontFamily":"lucide","iconFontName":"bell","width":20,"height":20,"fill":"#000000"}. Common sizes: 14px (inline/small), 20px (standard), 24px (prominent). Never use emoji or path for icons.
  Common icon names: search, bell, user, house, heart, star, plus, x, check, chevron-left, chevron-right, arrow-right, trending-up, trending-down, settings, calendar, download, share-2, sliders-horizontal, compass, chart-bar, eye, eye-off, mail, lock, image, menu, refresh-cw.
- Semantic inputs should include affordance icons when appropriate:
  - search bars: leading icon_font(search)
  - password fields: trailing icon_font(eye/eye-off)
  - email/account fields: leading icon_font(mail/user)
- Dividers: Use rectangle(height=1, width="fill_container", fill=borderColor) between sections. Or use directional stroke on parent: stroke={"thickness":{"bottom":1},"fill":[...]}.
- Phone mockup: ONE frame, width 260-300, height 520-580, cornerRadius 32, solid fill + 1px stroke. No ellipse for mockups. At most ONE centered text child inside. ONLY use phone mockups when the user explicitly asks for a showcase/preview/mockup of an app. When the user says "mobile screen" / "移动端页面", generate the actual mobile UI directly (375x812), NOT a desktop page with a phone mockup inside.
- Never ellipse for decorative shapes — use frame/rectangle with cornerRadius.
- Use style guide colors/fonts consistently. No random colors.
- Text buttons: frame(padding=[12,24], justifyContent="center", fill=accent) > text. Height auto from padding+text.
- Icon+text buttons: frame(layout="horizontal", gap=8, alignItems="center", padding=[8,16]) > [icon_font, text].
- Icon-only buttons: frame(width=44, height=44, layout="none") > icon_font(x=12, y=12, width=20, height=20). Use layout="none" with centered x/y.
- CJK buttons: width ≥ charCount × fontSize + horizontalPadding.
- Badges/tags: only for short labels (CJK ≤8 / Latin ≤16 chars). Longer text → normal text row.
- Hero + phone (desktop): two-column horizontal layout (left text, right phone). Not stacked unless mobile.
- Landing pages: hero 40-56px headline, alternating section backgrounds, nav with space_between.
- App screens: focus on core function, inputs width="fill_container", consistent 48-56px height, 16-24px gap.

FORMAT: Each line has "_parent" (null=root, else parent-id). Parent before children.
${BLOCK}json
{"_parent":null,"id":"root","type":"frame","name":"Hero","width":"fill_container","height":"fit_content","layout":"vertical","gap":24,"padding":[48,24],"fill":[{"type":"solid","color":"#F8FAFC"}]}
{"_parent":"root","id":"header","type":"frame","name":"Header","justifyContent":"space_between","alignItems":"center","width":"fill_container"}
{"_parent":"header","id":"logo","type":"text","name":"Logo","content":"ACME","fontSize":18,"fontWeight":600,"fontFamily":"Space Grotesk","fill":[{"type":"solid","color":"#0D0D0D"}]}
{"_parent":"header","id":"notifBtn","type":"frame","name":"Notification","width":44,"height":44,"layout":"none","stroke":{"thickness":2,"fill":[{"type":"solid","color":"#0D0D0D"}]}}
{"_parent":"notifBtn","id":"notifIcon","type":"icon_font","name":"Bell","iconFontFamily":"lucide","iconFontName":"bell","width":20,"height":20,"fill":"#0D0D0D","x":12,"y":12}
{"_parent":"root","id":"title","type":"text","name":"Headline","content":"Learn Smarter","fontSize":48,"fontWeight":700,"fontFamily":"Space Grotesk","lineHeight":0.95,"fill":[{"type":"solid","color":"#0F172A"}]}
{"_parent":"root","id":"desc","type":"text","name":"Description","content":"AI-powered vocabulary learning that adapts to your pace","fontSize":16,"textGrowth":"fixed-width","width":"fill_container","lineHeight":1.5,"fill":[{"type":"solid","color":"#64748B"}]}
{"_parent":"root","id":"cta","type":"frame","name":"CTA Button","padding":[14,28],"cornerRadius":10,"justifyContent":"center","fill":[{"type":"solid","color":"#2563EB"}]}
{"_parent":"cta","id":"cta-text","type":"text","name":"CTA Label","content":"Get Started","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}
${BLOCK}

Start with ${BLOCK}json immediately. No preamble, no <step> tags.`
