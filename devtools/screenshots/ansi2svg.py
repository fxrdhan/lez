#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 fxrdhan
# SPDX-License-Identifier: EUPL-1.2
"""Draw terminal transcripts, ANSI colors included, as terminal windows.

Each --window is a transcript, a color scheme, a width in columns and a
title. The windows sit side by side on a transparent canvas, as tall as the
longest transcript.

Every word sits at its own column, so the layout holds with any monospace
font whose advance is 0.6 em, JetBrains Mono among them, and with icons
from a wider Nerd Font. The color schemes are Catppuccin Mocha (dark) and
Latte (light).
"""

import argparse
import re
import unicodedata
from xml.sax.saxutils import escape

THEMES = {
    "dark": {
        "background": "#1e1e2e",
        "title_bar": "#181825",
        "separator": "#11111b",
        "foreground": "#cdd6f4",
        "title": "#7f849c",
        "cursor": "#f5e0dc",
        "edge": "#ffffff",
        "ansi": [
            "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af",
            "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
            "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af",
            "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
        ],
    },
    "light": {
        "background": "#eff1f5",
        "title_bar": "#e6e9ef",
        "separator": "#dce0e8",
        "foreground": "#4c4f69",
        "title": "#8c8fa1",
        "cursor": "#dc8a78",
        "edge": "#000000",
        "ansi": [
            "#5c5f77", "#d20f39", "#40a02b", "#df8e1d",
            "#1e66f5", "#ea76cb", "#179299", "#acb0be",
            "#6c6f85", "#d20f39", "#40a02b", "#df8e1d",
            "#1e66f5", "#ea76cb", "#179299", "#bcc0cc",
        ],
    },
}

FONT_FAMILY = "JetBrains Mono"
FONT_SIZE = 13.0
CELL_WIDTH = FONT_SIZE * 0.6
LINE_HEIGHT = 17.0
PADDING_X = 18.0
PADDING_Y = 14.0
TITLE_BAR = 36.0
RADIUS = 12.0
GAP = 32.0

# CSI sequences (SGR among them), and OSC sequences such as hyperlinks.
ESCAPE = re.compile(r"\x1b\[([0-9;:?]*)([@-~])|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)")


def xterm_color(index, theme):
    if index < 16:
        return theme["ansi"][index]
    if index < 232:
        index -= 16
        levels = [0, 95, 135, 175, 215, 255]
        red, green, blue = (levels[index // 36], levels[index // 6 % 6], levels[index % 6])
    else:
        red = green = blue = 8 + (index - 232) * 10
    return f"#{red:02x}{green:02x}{blue:02x}"


class Style:
    def __init__(self):
        self.reset()

    def reset(self):
        self.foreground = None
        self.background = None
        self.bold = False
        self.dim = False
        self.italic = False
        self.underline = False
        self.reverse = False
        self.hidden = False
        self.strike = False

    def key(self):
        return (self.foreground, self.background, self.bold, self.dim, self.italic,
                self.underline, self.reverse, self.hidden, self.strike)

    def apply(self, parameters, theme):
        codes = [int(code) if code else 0 for code in re.split("[;:]", parameters)]
        index = 0
        while index < len(codes):
            code = codes[index]
            if code == 0:
                self.reset()
            elif code == 1:
                self.bold = True
            elif code == 2:
                self.dim = True
            elif code == 3:
                self.italic = True
            elif code == 4:
                self.underline = True
            elif code == 7:
                self.reverse = True
            elif code == 8:
                self.hidden = True
            elif code == 9:
                self.strike = True
            elif code == 22:
                self.bold = self.dim = False
            elif code == 23:
                self.italic = False
            elif code == 24:
                self.underline = False
            elif code == 27:
                self.reverse = False
            elif code == 28:
                self.hidden = False
            elif code == 29:
                self.strike = False
            elif 30 <= code <= 37:
                self.foreground = theme["ansi"][code - 30]
            elif 40 <= code <= 47:
                self.background = theme["ansi"][code - 40]
            elif 90 <= code <= 97:
                self.foreground = theme["ansi"][code - 90 + 8]
            elif 100 <= code <= 107:
                self.background = theme["ansi"][code - 100 + 8]
            elif code == 39:
                self.foreground = None
            elif code == 49:
                self.background = None
            elif code in (38, 48) and index + 1 < len(codes):
                color = None
                if codes[index + 1] == 5 and index + 2 < len(codes):
                    color = xterm_color(codes[index + 2], theme)
                    index += 2
                elif codes[index + 1] == 2 and index + 4 < len(codes):
                    color = "#{:02x}{:02x}{:02x}".format(*codes[index + 2:index + 5])
                    index += 4
                if code == 38:
                    self.foreground = color
                else:
                    self.background = color
            index += 1


def width(char):
    if unicodedata.combining(char):
        return 0
    return 2 if unicodedata.east_asian_width(char) in "WF" else 1


def parse(text, theme):
    """Split a transcript into lines of [column, text, style key, cells] runs."""
    lines = []
    for raw in text.split("\n"):
        style = Style()
        runs = []
        column = 0
        position = 0
        for match in list(ESCAPE.finditer(raw)) + [None]:
            end = match.start() if match else len(raw)
            for char in raw[position:end]:
                pieces = " " * (8 - column % 8) if char == "\t" else char
                for piece in pieces:
                    if runs and runs[-1][2] == style.key():
                        runs[-1][1] += piece
                        runs[-1][3] += width(piece)
                    else:
                        runs.append([column, piece, style.key(), width(piece)])
                    column += width(piece)
            if match is None:
                break
            if match.group(2) == "m":
                style.apply(match.group(1), theme)
            position = match.end()
        lines.append(runs)
    while lines and not lines[-1]:
        lines.pop()
    return lines


def window_size(columns, rows):
    return (2 * PADDING_X + columns * CELL_WIDTH,
            TITLE_BAR + 2 * PADDING_Y + rows * LINE_HEIGHT)


def draw_window(out, lines, theme, columns, rows, title, left, top):
    window_width, window_height = window_size(columns, rows)
    right = left + window_width
    bar_bottom = top + TITLE_BAR
    out.append(f'<rect x="{left:g}" y="{top:g}" width="{window_width:g}" '
               f'height="{window_height:g}" rx="{RADIUS:g}" fill="{theme["background"]}"/>')
    # The title bar: rounded at the top only.
    out.append(f'<path d="M{left:g},{bar_bottom:g} V{top + RADIUS:g} '
               f'A{RADIUS:g},{RADIUS:g} 0 0 1 {left + RADIUS:g},{top:g} '
               f'H{right - RADIUS:g} A{RADIUS:g},{RADIUS:g} 0 0 1 {right:g},{top + RADIUS:g} '
               f'V{bar_bottom:g} Z" fill="{theme["title_bar"]}"/>')
    out.append(f'<rect x="{left:g}" y="{bar_bottom - 1:g}" width="{window_width:g}" '
               f'height="1" fill="{theme["separator"]}"/>')
    for offset, color in zip((20, 40, 60), ("#ff5f57", "#febc2e", "#28c840")):
        out.append(f'<circle cx="{left + offset:g}" cy="{top + TITLE_BAR / 2:g}" r="6" '
                   f'fill="{color}" stroke="#000000" stroke-opacity="0.12" stroke-width="0.5"/>')
    out.append(f'<text x="{left + window_width / 2:g}" y="{top + TITLE_BAR / 2 + 4.5:g}" '
               f'text-anchor="middle" font-size="12" fill="{theme["title"]}">'
               f'{escape(title)}</text>')
    # A hairline edge, which keeps the window apart from the backdrop.
    out.append(f'<rect x="{left + 0.5:g}" y="{top + 0.5:g}" width="{window_width - 1:g}" '
               f'height="{window_height - 1:g}" rx="{RADIUS - 0.5:g}" fill="none" '
               f'stroke="{theme["edge"]}" stroke-opacity="0.1"/>')

    text_left = left + PADDING_X
    text_top = bar_bottom + PADDING_Y
    for row, runs in enumerate(lines):
        y = text_top + row * LINE_HEIGHT
        baseline = y + FONT_SIZE * 0.95
        for column, text, key, cells in runs:
            if column >= columns:
                continue
            (foreground, background, bold, dim, italic,
             underline, reverse, hidden, strike) = key
            if reverse:
                foreground, background = (background or theme["background"],
                                          foreground or theme["cursor"])
            foreground = foreground or theme["foreground"]
            x = text_left + column * CELL_WIDTH
            span = min(cells, columns - column) * CELL_WIDTH
            if background:
                out.append(f'<rect x="{x:g}" y="{y:g}" width="{span:g}" '
                           f'height="{LINE_HEIGHT:g}" fill="{background}"/>')
            if hidden:
                continue
            attributes = f'fill="{foreground}"'
            if bold:
                attributes += ' font-weight="bold"'
            if italic:
                attributes += ' font-style="italic"'
            if dim:
                attributes += ' fill-opacity="0.6"'
            # Each word at its own column, so neither a renderer's handling
            # of spaces nor an icon wider than a cell can shift what follows.
            for word in re.finditer(r"\S+", text):
                offset = sum(width(char) for char in text[:word.start()])
                out.append(f'<text x="{x + offset * CELL_WIDTH:g}" y="{baseline:g}" '
                           f'{attributes}>{escape(word.group())}</text>')
            if underline:
                out.append(f'<rect x="{x:g}" y="{baseline + 2:g}" width="{span:g}" '
                           f'height="1" fill="{foreground}"/>')
            if strike:
                out.append(f'<rect x="{x:g}" y="{baseline - FONT_SIZE * 0.3:g}" '
                           f'width="{span:g}" height="1" fill="{foreground}"/>')


def compose(windows):
    rows = max(len(lines) for lines, _, _, _ in windows)
    widths = [window_size(columns, rows)[0] for _, _, columns, _ in windows]
    canvas_width = sum(widths) + GAP * (len(windows) - 1)
    canvas_height = window_size(0, rows)[1]
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{canvas_width:g}" '
        f'height="{canvas_height:g}" viewBox="0 0 {canvas_width:g} {canvas_height:g}">',
        f'<g font-family="{FONT_FAMILY}" font-size="{FONT_SIZE:g}" xml:space="preserve">',
    ]
    left = 0.0
    for (lines, theme, columns, title), window_width in zip(windows, widths):
        draw_window(out, lines, theme, columns, rows, title, left, 0.0)
        left += window_width + GAP
    out.append("</g></svg>")
    return "\n".join(out) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--window", nargs=4, action="append", required=True,
                        metavar=("TRANSCRIPT", "THEME", "COLUMNS", "TITLE"))
    arguments = parser.parse_args()
    windows = []
    for transcript, theme_name, columns, title in arguments.window:
        theme = THEMES[theme_name]
        with open(transcript, encoding="utf-8") as file:
            lines = parse(file.read(), theme)
        windows.append((lines, theme, int(columns), title))
    print(compose(windows), end="")


if __name__ == "__main__":
    main()
