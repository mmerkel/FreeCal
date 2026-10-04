"""Sends mouse and keyboard input to an X display through XTEST.

Usage (DISPLAY must be set):
    xinput.py click X Y | dblclick X Y | rightclick X Y | move X Y | type TEXT | key NAME

NAME is an X keysym (Return, Escape, Tab, BackSpace, ...), optionally with
modifiers: ctrl+a, shift+Tab.
"""

import sys
import time

from Xlib import X, XK, display
from Xlib.ext import xtest

MODIFIERS = {"ctrl": "Control_L", "shift": "Shift_L", "alt": "Alt_L"}
# Characters whose keysym name differs from the character itself.
CHARACTER_KEYSYMS = {
    " ": "space", "#": "numbersign", "-": "minus", ".": "period", ",": "comma",
    "/": "slash", ":": "colon", "'": "apostrophe", "<": "less", ">": "greater",
}
SHIFTED = set('#:<>!"$%&()*+?@^_{}|~')

d = display.Display()


def flush():
    # Not d.sync(): on Xvfb its GetPointerControl round trip can fail with
    # BadRRModeError. A flush plus a short pause is enough.
    d.flush()
    time.sleep(0.12)


def keycode(keysym_name):
    code = d.keysym_to_keycode(XK.string_to_keysym(keysym_name))
    if code == 0:
        sys.exit(f"unknown key {keysym_name!r}")
    return code


def press(codes):
    for code in codes:
        xtest.fake_input(d, X.KeyPress, code)
    for code in reversed(codes):
        xtest.fake_input(d, X.KeyRelease, code)
    flush()


def key(spec):
    *mods, name = spec.split("+")
    press([keycode(MODIFIERS.get(m, m)) for m in mods] + [keycode(name)])


def type_text(text):
    for ch in text:
        name = CHARACTER_KEYSYMS.get(ch, ch)
        if ch.isupper() or ch in SHIFTED:
            press([keycode("Shift_L"), keycode(name if ch in SHIFTED else ch.lower())])
        else:
            press([keycode(name)])


def move(x, y):
    xtest.fake_input(d, X.MotionNotify, x=x, y=y)
    flush()


def click(x, y, times=1, button=1):
    move(x, y)
    for _ in range(times):
        xtest.fake_input(d, X.ButtonPress, button)
        flush()
        xtest.fake_input(d, X.ButtonRelease, button)
        flush()


command, *args = sys.argv[1:] or ["help"]
if command == "click":
    click(int(args[0]), int(args[1]))
elif command == "dblclick":
    click(int(args[0]), int(args[1]), times=2)
elif command == "rightclick":
    click(int(args[0]), int(args[1]), button=3)
elif command == "move":
    move(int(args[0]), int(args[1]))
elif command == "type":
    type_text(args[0])
elif command == "key":
    key(args[0])
else:
    sys.exit(__doc__)
# Give the webview time to react before the next command or screenshot.
time.sleep(0.4)
