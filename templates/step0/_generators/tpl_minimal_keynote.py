#!/usr/bin/env python3
"""minimal-keynote.op — 极简 Keynote deck（8 页 1920×1080）

这套的难点是「少」。没有卡片、没有描边、没有底色块，一页只留 2-4 个元素，
版面全靠字阶落差和留白撑。元素越少，任何一处对不齐都无处遁形——所以整套
只有一条左基线（EDGE=160），每页的内容块都从这条线起排，纵向居中。

跟已有三套的区别：slide-deck 是暖白商务、pitch-deck-dark 是深底路演、
lecture-deck-light 是纸白课件，它们都靠「卡片」组织信息；这一套刻意不给
任何容器上色，是发布会舞台上那种一句话一屏的节奏。

排版遵循 skills/domains/slides.md 的硬契约：
  - 每帧固定 1920×1080，绝不 fit_content；内容距边 ≥100（这里 160）
  - 正文 ≥24（取 28-32），标题 ≥40，关键数字 80-200（数据页取 200）
  - 行高：展示字 1.05-1.15，正文 1.45+
  - 最多 2 个字体族（Noto Sans SC 排中文，Inter 排数字），靠字重分层
  - 强调色只用于强调

白底上的对比度（WCAG，实测见文件末尾）最低一对是 5.33:1，全部高于 4.5。
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import Ids, color_vars, frame, rect, solid, text, write_doc

ids = Ids()

# 纯白 + 近黑 + 一个克制的朱红。
#
# 强调色没有用常见的 #FF5A36：那个橙在纯白上只有 3.10:1，当文字用不到
# 4.5 的可读线（实测，见文件末尾）。降明度到 #C7340F 后是 5.35:1，色相
# 仍在同一族，投影时反而更沉得住。整套只有这一个彩色，出现三次以内。
VARS = color_vars({
    "c-bg":     "#FFFFFF",
    "c-ink":    "#0A0A0A",
    "c-muted":  "#6B6B6B",
    "c-accent": "#C7340F",
})

W, H = 1920, 1080
# 留白是这套模板唯一的「装饰」，所以边距给到 160 —— 契约下限 100 的 1.6 倍。
EDGE = 160

CJK = "Noto Sans SC"
NUM = "Inter"


def slide(name, *, gap=40, justify="center"):
    """一帧。固定 1920×1080，无底色块、无描边 —— 白就是版面本身。"""
    node = frame(ids, name, width=W, height=H, layout="vertical",
                 padding=[EDGE, EDGE], gap=gap, justifyContent=justify,
                 alignItems="start", fill=solid("$c-bg"), clipContent=True)
    node["children"] = []
    return node


def col(name, children, *, gap=16, width="fill_container",
        height="fit_content", **props):
    """透明结构容器。这套模板里没有任何一个容器该有 fill。"""
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 gap=gap, alignItems="start", fill=[], **props)
    node["children"] = children
    return node


def row(name, children, *, gap=32, align="center", **props):
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="horizontal", gap=gap, alignItems=align, fill=[],
                 **props)
    node["children"] = children
    return node


def accent_rule(width=120, height=8):
    """整套唯一的图形元素。一条短线，替代所有本该是卡片边框的东西。"""
    return rect(ids, "强调短线", width=width, height=height,
                fill=solid("$c-accent"))


# ------------------------------------------------------------------ 01 封面
def cover():
    lede = col("主张", [
        text(ids, "主标题", "把复杂\n留在里面", 130, 700, "$c-ink",
             family=CJK, line_height=1.08),
        accent_rule(),
        text(ids, "副标题", "第七代产品，今天发布。", 32, 400, "$c-muted",
             family=CJK, line_height=1.45),
    ], gap=40)

    meta = row("落款", [
        text(ids, "场合", "2026 秋季发布会", 26, 500, "$c-ink", family=CJK,
             width="fit_content", growth="auto"),
        text(ids, "日期", "2026.10.08", 26, 400, "$c-muted", family=NUM,
             width="fit_content", growth="auto"),
    ], gap=28, justifyContent="space_between")

    s = slide("01 封面", gap=48, justify="space_between")
    s["children"] = [lede, meta]
    return s


# ------------------------------------------------------------ 02 一句话论点
def statement():
    s = slide("02 论点", gap=44)
    s["children"] = [
        rect(ids, "强调点", width=18, height=18, cornerRadius=9,
             fill=solid("$c-accent")),
        text(ids, "论点", "我们删掉了三十七个按钮。", 76, 700, "$c-ink",
             family=CJK, line_height=1.15),
    ]
    return s


# ------------------------------------------------------------- 03-05 词页
# 一页一个字。注解只有一行，写不下就说明这个词选错了。
WORDS = [
    ("少", "从三十七个按钮，到三个。"),
    ("轻", "整机一点零九公斤，比上一代少了两百四十克。"),
    ("久", "一次充电，用满两天。"),
]


def word_page(index, word, note):
    s = slide(f"0{index + 3} 词 · {word}", gap=36)
    s["children"] = [
        text(ids, "大词", word, 200, 700, "$c-ink", family=CJK,
             line_height=1.05),
        text(ids, "注解", note, 30, 400, "$c-muted", family=CJK,
             line_height=1.5),
    ]
    return s


# ------------------------------------------------------------------ 06 引用
def quote():
    s = slide("06 引用", gap=32)
    s["children"] = [
        # 引号本身当图形用，替掉一切边框和背景块。lineHeight 压到 0.5 是
        # 量出来的：引号的墨迹只占 em 盒顶部约三分之一，默认行高会在它下面
        # 留出一大片空行盒，看起来就像 gap 没设对（同 oplib 里 `●` 用
        # DOT_LINE_HEIGHT 收行盒的做法）。
        text(ids, "引号", "“", 160, 700, "$c-accent", family=NUM,
             line_height=0.5),
        text(ids, "引文", "最好的设计，是让人忘了它存在。", 54, 600,
             "$c-ink", family=CJK, line_height=1.3),
        text(ids, "出处", "— 产品负责人 沈亦文", 28, 400, "$c-muted",
             family=CJK, line_height=1.5),
    ]
    return s


# ------------------------------------------------------------- 07 数据一句话
def one_number():
    s = slide("07 数据", gap=32)
    s["children"] = [
        # 契约允许关键数字 80-200，这里取上限：整页只有它和一行注。
        text(ids, "大数字", "0.4 秒", 200, 700, "$c-ink", family=NUM,
             line_height=1.05),
        text(ids, "注解", "从按下到出图的全部时间，上一代是 2.6 秒。",
             32, 400, "$c-muted", family=CJK, line_height=1.5),
    ]
    return s


# ------------------------------------------------------------------ 08 结尾
def closing():
    s = slide("08 结尾", gap=40)
    s["children"] = [
        accent_rule(),
        text(ids, "结语", "今天开始发货。", 96, 700, "$c-ink", family=CJK,
             line_height=1.12),
        text(ids, "补充", "全国门店同步开售，官网可预约体验。", 30, 400,
             "$c-muted", family=CJK, line_height=1.5),
    ]
    return s


# 顶层 frame 必须显式写 x/y：缺省时每帧都落在原点，八页会叠成一页（前一批
# 模板 2026-08-02 的实录）。纵向 gap 比横向多留 240，是给画布上固定在帧顶部
# 的帧名标签让位——标签是屏幕空间的，不随缩放变小。
BOARD_GAP_X = 120
BOARD_GAP_Y = BOARD_GAP_X + 240
BOARDS_PER_ROW = 3


def build():
    boards = [cover(), statement()]
    boards += [word_page(i, w, n) for i, (w, n) in enumerate(WORDS)]
    boards += [quote(), one_number(), closing()]
    for index, board in enumerate(boards):
        board["x"] = (index % BOARDS_PER_ROW) * (W + BOARD_GAP_X)
        board["y"] = (index // BOARDS_PER_ROW) * (H + BOARD_GAP_Y)
    return boards


# 对比度（WCAG 相对亮度比，投影场景自设门槛 4.5，远高于 op-design-lint 的 2.5）：
#   c-ink    on c-bg   19.80
#   c-muted  on c-bg    5.33  ← 最低一对
#   c-accent on c-bg    5.35
# 换强调色时先量它在纯白上的比值：橙红一族很容易掉到 4.5 以下
#   （#FF5A36 = 3.10、#E5471F = 4.00 都不够，#C7340F = 5.35 才过）。

if __name__ == "__main__":
    write_doc(sys.argv[1], VARS, build(), "极简 Keynote · 16:9")
