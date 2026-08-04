#!/usr/bin/env python3
"""knowledge-card-vertical.op + knowledge-card-square.op — 中文知识卡片

两套模板共用一份生成器，因为它们是同一套设计语言的两个画幅：竖版
1080×1440 发小红书图文，方版 1080×1080 发公众号头图和朋友圈。分成两个
文件写，改一处配色就得记得改另一处；合成一个，两张卡永远同源。

版式只做「标题 + 要点 + 署名」三段，是刻意的：卡片场景的用户要改的是
文案，不是结构。多一个装饰区就多一处他们得先看懂再删掉的东西。

硬契约（与 skills 的中文排版约定一致）：
  - 内容距边缘 ≥80px（竖版 88/96，方版 84/88）
  - 配色全部走 color_vars，改主色只改 $c-accent 一处
  - 正文与背景对比度 ≥2.0（本配色最低一对是 3.67，见文件末尾注释）
  - CJK 行高：大标题 1.15，小标题 1.3，正文 1.6
  - 顶层 frame 必须显式写 x/y，否则多帧会全部堆在原点
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from oplib import Ids, color_vars, frame, rect, solid, text, write_doc

ids = Ids()

# 暖纸底 + 单一强调色。知识卡片是「读」的东西，底色带一点暖比纯白更耐看，
# 也和已有的 knowledge-carousel（冷调靛蓝）拉开距离。
VARS = color_vars({
    "c-bg":          "#FFF7F0",
    "c-surface":     "#FFFFFF",
    "c-ink":         "#241A12",
    "c-muted":       "#7A6455",
    "c-accent":      "#C8502B",
    "c-accent-soft": "#FAE3D8",
    "c-border":      "#EDDCD0",
})

# 正文用的中文字族。oplib 的默认族按字号/字重挑（<34 且 <600 走 Inter），
# 那条规则是为中英混排写的；这两张卡是纯中文，所以每个文本节点都显式写死
# 中文族，免得小字号正文落到一个没有汉字的字族上再靠 fallback 兜。
CJK = "Noto Sans SC"
# 序号和数字用 Inter：等宽感更强，"01" 这类标号排出来才齐。
NUM = "Inter"


def card(name, *, width, height, pad_y, pad_x, children):
    """一张卡。固定画幅，绝不 fit_content —— 发布尺寸是硬约束。

    `space_between` 把「头部 / 要点 / 署名」三段推到各自该在的位置：
    卡片的留白不该是均匀的 gap，而是三段之间自然撑开的呼吸。
    """
    node = frame(ids, name, width=width, height=height, layout="vertical",
                 padding=[pad_y, pad_x], gap=0,
                 justifyContent="space_between", alignItems="start",
                 fill=solid("$c-bg"), clipContent=True)
    node["children"] = children
    node["x"] = 0
    node["y"] = 0
    return node


def block(name, children, gap=32):
    """透明结构容器。没有 fill 才是结构层；一旦有 fill 它就是一张卡面了。"""
    node = frame(ids, name, width="fill_container", height="fit_content",
                 layout="vertical", gap=gap, alignItems="start", fill=[])
    node["children"] = children
    return node


def eyebrow(label, size=26):
    """小标签。整张卡唯一的胶囊，用来交代这是哪一类内容。"""
    node = frame(ids, "标签", width="fit_content", height="fit_content",
                 layout="horizontal", padding=[12, 24], gap=0,
                 cornerRadius=999, alignItems="center",
                 justifyContent="center", fill=solid("$c-accent-soft"))
    node["children"] = [
        text(ids, "标签文字", label, size, 600, "$c-accent", family=CJK,
             width="fit_content", growth="auto", line_height=1.4),
    ]
    return node


def rule(width=132, height=12):
    return rect(ids, "强调短线", width=width, height=height,
                cornerRadius=height // 2, fill=solid("$c-accent"))


def numbered(no, size):
    """实心序号圆。白字压在主色上，是卡里对比度最高的一处，读者的视线锚点。"""
    node = frame(ids, f"序号 {no}", width=size, height=size,
                 layout="horizontal", alignItems="center",
                 justifyContent="center", cornerRadius=size // 2,
                 fill=solid("$c-accent"))
    node["children"] = [
        text(ids, "序号数字", str(no), round(size * 0.5), 700, "#FFFFFF",
             family=NUM, width="fit_content", growth="auto", line_height=1.0),
    ]
    return node


def point(no, title, desc, *, badge, title_size, desc_size, gap):
    node = frame(ids, f"要点 {no}", width="fill_container",
                 height="fit_content", layout="horizontal", gap=gap,
                 alignItems="start", fill=[])
    node["children"] = [
        numbered(no, badge),
        block("要点文案", [
            text(ids, "要点标题", title, title_size, 600, "$c-ink",
                 family=CJK, line_height=1.3),
            text(ids, "要点说明", desc, desc_size, 400, "$c-muted",
                 family=CJK, line_height=1.6),
        ], gap=10),
    ]
    return node


def signature(*, avatar, name_size, slogan_size, gap):
    """署名条。一条细分隔线 + 头像占位 + 名字 + 一句 slogan。

    头像用 frame 而不是 rectangle：frame 会递归渲染子节点，圆里那个字才
    显示得出来；同时 frame 也是合法的图片拖放目标，用户把自己的头像拖上去
    就直接替换掉这个占位。
    """
    disc = frame(ids, "头像占位", width=avatar, height=avatar,
                 layout="horizontal", alignItems="center",
                 justifyContent="center", cornerRadius=avatar // 2,
                 fill=solid("$c-accent-soft"))
    disc["children"] = [
        text(ids, "头像文字", "笔", round(avatar * 0.42), 700, "$c-accent",
             family=CJK, width="fit_content", growth="auto", line_height=1.0),
    ]

    row = frame(ids, "署名行", width="fill_container", height="fit_content",
                layout="horizontal", gap=gap, alignItems="center", fill=[])
    row["children"] = [
        disc,
        block("署名文案", [
            text(ids, "账号名", "@ 你的账号名", name_size, 600, "$c-ink",
                 family=CJK, line_height=1.3),
            text(ids, "一句话简介", "每周更新一条可落地的学习方法",
                 slogan_size, 400, "$c-muted", family=CJK, line_height=1.5),
        ], gap=6),
    ]

    divider = rect(ids, "分隔线", width="fill_container", height=2,
                   fill=solid("$c-border"))
    return block("署名", [divider, row], gap=gap)


# ----------------------------------------------------------- 竖版 1080×1440
VERTICAL_POINTS = [
    ("先提问，再翻开",
     "带着「我想解决什么」进去，信息才有地方落。"),
    ("合上书，先复述",
     "读完立刻用自己的话讲一遍，卡壳的地方就是没懂的地方。"),
    ("一次只带走三条",
     "贪多必忘。记住三个能用的点，比划满一整本有用。"),
    ("两天之内用一次",
     "写进笔记、讲给朋友、或者直接做一遍，用过才算学过。"),
]


def vertical():
    head = block("卡头", [
        eyebrow("学习方法 · 01"),
        text(ids, "主标题", "为什么你读了很多书\n却什么都记不住", 88, 700,
             "$c-ink", family=CJK, line_height=1.15),
        rule(),
        text(ids, "副标题", "不是记性差，是你少做了这一步。", 30, 400,
             "$c-muted", family=CJK, line_height=1.5),
    ], gap=32)

    points = block("要点列表", [
        point(no, title, desc, badge=56, title_size=36, desc_size=27, gap=24)
        for no, (title, desc) in enumerate(VERTICAL_POINTS, 1)
    ], gap=28)

    return card("知识卡片 · 竖版", width=1080, height=1440, pad_y=96, pad_x=88,
                children=[head, points,
                          signature(avatar=88, name_size=30, slogan_size=24,
                                    gap=24)])


# ----------------------------------------------------------- 方版 1080×1080
SQUARE_POINTS = [
    ("先提问，再翻开",
     "带着问题进去，信息才有地方落。"),
    ("合上书，先复述",
     "讲不出来的地方，就是还没懂的地方。"),
    ("两天之内用一次",
     "用过一次，才算真的学过。"),
]


def square():
    head = block("卡头", [
        eyebrow("学习方法"),
        text(ids, "主标题", "读完就忘？\n先补上这一步", 76, 700, "$c-ink",
             family=CJK, line_height=1.15),
        rule(112, 10),
        text(ids, "副标题", "三个动作，把书里的东西真正带走。", 28, 400,
             "$c-muted", family=CJK, line_height=1.5),
    ], gap=26)

    points = block("要点列表", [
        point(no, title, desc, badge=52, title_size=34, desc_size=26, gap=22)
        for no, (title, desc) in enumerate(SQUARE_POINTS, 1)
    ], gap=24)

    return card("知识卡片 · 方版", width=1080, height=1080, pad_y=88, pad_x=84,
                children=[head, points,
                          signature(avatar=76, name_size=28, slogan_size=23,
                                    gap=22)])


# 对比度（WCAG 相对亮度比，op-design-lint 的门槛是 2.0）：
#   c-ink   on c-bg          16.09      c-muted on c-bg          5.24
#   c-accent on c-bg          4.27      c-accent on c-accent-soft 3.67
#   #FFFFFF on c-accent       4.53
# 最低一对 3.67 仍高出门槛近一倍，换主色时保留这个余量即可。

if __name__ == "__main__":
    write_doc(sys.argv[1], VARS, [vertical()], "知识卡片 · 竖版 3:4")
    write_doc(sys.argv[2], VARS, [square()], "知识卡片 · 方版 1:1")
