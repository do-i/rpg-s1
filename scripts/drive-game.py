#!/usr/bin/env python3
"""Developer helper to drive the Rusted Kingdoms window via XTEST, then screenshot.

Tokens:
  <key>            press once (up/down/left/right/enter/esc/space/tab, a-z, 0-9)
  <key>*N          press N times
  hold:<key>@<sec> hold a key
  wait:<sec>       sleep
  shot:<path>      screenshot the game window

Talks to Xlib through ctypes so it needs nothing beyond libX11/libXtst — the
python-xlib package disappeared with the system Python 3.14 upgrade.
"""
import ctypes
import ctypes.util
import subprocess
import sys
import time

KEYS = {
    "up": "Up", "down": "Down", "left": "Left", "right": "Right",
    "enter": "Return", "esc": "Escape", "space": "space", "tab": "Tab",
    "bs": "BackSpace",
}
for c in "abcdefghijklmnopqrstuvwxyz0123456789":
    KEYS.setdefault(c, c)

WINDOW_NAME = b"Rusted Kingdoms"

xlib = ctypes.CDLL(ctypes.util.find_library("X11") or "libX11.so.6")
xtst = ctypes.CDLL(ctypes.util.find_library("Xtst") or "libXtst.so.6")

Window = ctypes.c_ulong
Atom = ctypes.c_ulong

xlib.XOpenDisplay.restype = ctypes.c_void_p
xlib.XDefaultRootWindow.restype = Window
xlib.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
xlib.XStringToKeysym.restype = ctypes.c_ulong
xlib.XStringToKeysym.argtypes = [ctypes.c_char_p]
xlib.XKeysymToKeycode.restype = ctypes.c_ubyte
xlib.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
xtst.XTestFakeKeyEvent.argtypes = [
    ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]


def query_tree(dpy, window):
    root, parent = Window(), Window()
    children = ctypes.POINTER(Window)()
    count = ctypes.c_uint()
    if not xlib.XQueryTree(ctypes.c_void_p(dpy), window,
                           ctypes.byref(root), ctypes.byref(parent),
                           ctypes.byref(children), ctypes.byref(count)):
        return []
    kids = [children[i] for i in range(count.value)]
    xlib.XFree(children)
    return kids


def window_name(dpy, window):
    name = ctypes.c_char_p()
    if xlib.XFetchName(ctypes.c_void_p(dpy), window, ctypes.byref(name)) and name.value:
        value = name.value
        xlib.XFree(name)
        return value
    return None


def find_window(dpy, fragment):
    stack = [xlib.XDefaultRootWindow(ctypes.c_void_p(dpy))]
    while stack:
        window = stack.pop()
        name = window_name(dpy, window)
        if name and fragment in name:
            return window
        stack.extend(query_tree(dpy, window))
    return None


def geometry(dpy, window):
    """Window size, plus its absolute position on the root."""
    root = Window()
    x, y = ctypes.c_int(), ctypes.c_int()
    width, height = ctypes.c_uint(), ctypes.c_uint()
    border, depth = ctypes.c_uint(), ctypes.c_uint()
    xlib.XGetGeometry(ctypes.c_void_p(dpy), window, ctypes.byref(root),
                      ctypes.byref(x), ctypes.byref(y),
                      ctypes.byref(width), ctypes.byref(height),
                      ctypes.byref(border), ctypes.byref(depth))
    abs_x, abs_y = ctypes.c_int(), ctypes.c_int()
    child = Window()
    xlib.XTranslateCoordinates(ctypes.c_void_p(dpy), window, root, 0, 0,
                               ctypes.byref(abs_x), ctypes.byref(abs_y),
                               ctypes.byref(child))
    return width.value, height.value, abs_x.value, abs_y.value


def press(dpy, key, gap=0.22):
    code = xlib.XKeysymToKeycode(
        ctypes.c_void_p(dpy), xlib.XStringToKeysym(KEYS[key].encode()))
    xtst.XTestFakeKeyEvent(ctypes.c_void_p(dpy), code, 1, 0)
    xlib.XFlush(ctypes.c_void_p(dpy))
    time.sleep(0.05)
    xtst.XTestFakeKeyEvent(ctypes.c_void_p(dpy), code, 0, 0)
    xlib.XFlush(ctypes.c_void_p(dpy))
    time.sleep(gap)


def main():
    dpy = xlib.XOpenDisplay(None)
    if not dpy:
        print("cannot open display", file=sys.stderr)
        return 2
    window = find_window(dpy, WINDOW_NAME)
    if window is None:
        print("window not found", file=sys.stderr)
        return 2
    xlib.XSetInputFocus(ctypes.c_void_p(dpy), window, 2, 0)  # RevertToParent
    xlib.XRaiseWindow(ctypes.c_void_p(dpy), window)
    xlib.XSync(ctypes.c_void_p(dpy), False)
    time.sleep(0.4)

    for token in sys.argv[1:]:
        if token.startswith("shot:"):
            xlib.XSync(ctypes.c_void_p(dpy), False)
            time.sleep(0.3)
            width, height, x, y = geometry(dpy, window)
            subprocess.run(
                ["import", "-window", "root", "-crop",
                 f"{width}x{height}+{x}+{y}", "+repage", token[5:]],
                check=True)
            print("shot", token[5:])
            continue
        if token.startswith("wait:"):
            time.sleep(float(token[5:]))
            continue
        if token.startswith("hold:"):
            key, seconds = token[5:].split("@")
            code = xlib.XKeysymToKeycode(
                ctypes.c_void_p(dpy), xlib.XStringToKeysym(KEYS[key].encode()))
            xtst.XTestFakeKeyEvent(ctypes.c_void_p(dpy), code, 1, 0)
            xlib.XFlush(ctypes.c_void_p(dpy))
            time.sleep(float(seconds))
            xtst.XTestFakeKeyEvent(ctypes.c_void_p(dpy), code, 0, 0)
            xlib.XFlush(ctypes.c_void_p(dpy))
            time.sleep(0.25)
            continue
        if "*" in token:
            key, count = token.split("*")
            for _ in range(int(count)):
                press(dpy, key)
            continue
        press(dpy, token)
    return 0


if __name__ == "__main__":
    sys.exit(main())
