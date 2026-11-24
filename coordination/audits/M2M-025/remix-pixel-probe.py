import sys

import gi
gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import Gtk, WebKit2, GLib, Gdk  # noqa: E402

URI = sys.argv[1]
PREFIX = sys.argv[2]

win = Gtk.Window()
win.set_default_size(1400, 900)
web = WebKit2.WebView()
web.load_uri(URI)
win.add(web)
win.show_all()


def snap(label):
    window = win.get_window()
    if window is None:
        print('no window for', label)
        return
    width, height = window.get_width(), window.get_height()
    pixbuf = Gdk.pixbuf_get_from_window(window, 0, 0, width, height)
    if pixbuf is not None:
        pixbuf.savev('%s-%s.png' % (PREFIX, label), 'png', [], [])
        print('saved', label, width, height)


def first():
    snap('first')
    return False


def after_scroll():
    web.run_javascript(
        "document.querySelector('#composer-library-items').scrollTop = 400;",
        None,
        lambda *args: GLib.timeout_add(400, lambda: (snap('scrolled'), False)[1]),
        None
    )
    return False


GLib.timeout_add(600, first)
GLib.timeout_add(1000, after_scroll)
GLib.timeout_add(3000, lambda: (Gtk.main_quit(), False)[1])
Gtk.main()
