"""Sends mouse and keyboard input to an X display through XTEST.

Usage (DISPLAY must be set):
    xinput.py click X Y | dblclick X Y | rightclick X Y | drag X1 Y1 X2 Y2 | down X Y | up X Y | move X Y | wheel X Y up|down [N] | type TEXT | key NAME | cursor X Y

NAME is an X keysym (Return, Escape, Tab, BackSpace, ...), optionally with
modifiers: ctrl+a, shift+Tab.
"""

import sys
import time

from Xlib import X, XK, display
from Xlib.ext import xtest
from Xlib.protocol import rq

MODIFIERS = {"ctrl": "Control_L", "shift": "Shift_L", "alt": "Alt_L"}

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
    # A printable ASCII character's keysym is its code point. The keyboard map
    # says which key makes it and whether Shift is needed (index 1).
    for ch in text:
        options = [(i, code) for code, i in d.keysym_to_keycodes(ord(ch)) if i in (0, 1)]
        if not options:
            sys.exit(f"can't type {ch!r}")
        index, code = min(options)
        press(([keycode("Shift_L")] if index else []) + [code])


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


def drag(x1, y1, x2, y2, steps=10):
    """Presses at one point, moves in steps to the other and releases there."""
    move(x1, y1)
    xtest.fake_input(d, X.ButtonPress, 1)
    flush()
    for step in range(1, steps + 1):
        move(x1 + (x2 - x1) * step // steps, y1 + (y2 - y1) * step // steps)
        time.sleep(0.03)
    xtest.fake_input(d, X.ButtonRelease, 1)
    flush()


def wheel(x, y, direction, notches=1):
    """Moves to a point and turns the wheel there: X buttons 4 (up) and 5 (down)."""
    move(x, y)
    for _ in range(notches):
        click(x, y, button={"up": 4, "down": 5}[direction])


def button(x, y, event):
    """Moves to a point and presses or releases the left button there."""
    move(x, y)
    xtest.fake_input(d, event, 1)
    flush()


class GetCursorImageAndName(rq.ReplyRequest):
    """XFixes 2.0's GetCursorImageAndName, which python-xlib lacks. Only the
    header is read: the name is also there, as an atom."""

    _request = rq.Struct(rq.Card8("opcode"), rq.Opcode(25), rq.RequestLength())
    _reply = rq.Struct(
        rq.ReplyCode(),
        rq.Pad(1),
        rq.Card16("sequence_number"),
        rq.ReplyLength(),
        rq.Int16("x"),
        rq.Int16("y"),
        rq.Card16("width"),
        rq.Card16("height"),
        rq.Card16("xhot"),
        rq.Card16("yhot"),
        rq.Card32("cursor_serial"),
        rq.Card32("cursor_atom"),
        rq.Card16("nbytes"),
        rq.Pad(2),
    )


def cursor(x, y):
    """Moves to a point and prints the name of the cursor shown there, such as
    pointer, default or text (CSS names, as WebKitGTK sets them)."""
    move(x, y)
    time.sleep(0.3)
    # The extension must be told the client's version before other requests.
    d.xfixes_query_version()
    reply = GetCursorImageAndName(display=d.display, opcode=d.display.get_extension_major("XFIXES"))
    print(d.get_atom_name(reply.cursor_atom) if reply.cursor_atom else "(unnamed)")


command, *args = sys.argv[1:] or ["help"]
if command == "click":
    click(int(args[0]), int(args[1]))
elif command == "dblclick":
    click(int(args[0]), int(args[1]), times=2)
elif command == "rightclick":
    click(int(args[0]), int(args[1]), button=3)
elif command == "drag":
    drag(*map(int, args[:4]))
elif command == "down":
    button(int(args[0]), int(args[1]), X.ButtonPress)
elif command == "up":
    button(int(args[0]), int(args[1]), X.ButtonRelease)
elif command == "move":
    move(int(args[0]), int(args[1]))
elif command == "wheel":
    wheel(int(args[0]), int(args[1]), args[2], int(args[3]) if len(args) > 3 else 1)
elif command == "type":
    type_text(args[0])
elif command == "key":
    key(args[0])
elif command == "cursor":
    cursor(int(args[0]), int(args[1]))
else:
    sys.exit(__doc__)
# Give the webview time to react before the next command or screenshot.
time.sleep(0.4)
