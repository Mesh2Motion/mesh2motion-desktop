"""WebKitGTK computed-style probe for NATIVE-005 (diagnosis only, NOT native acceptance).

Usage: GDK_BACKEND=x11 python3 remix-theme-probe.py <url> <output.json>
Captures computed background of #composer-library-items and
.virtualized-catalog-content at first paint, after scroll and in light theme.
"""
import json
import sys

import gi
gi.require_version('Gtk', '3.0')
gi.require_version('WebKit2', '4.1')
from gi.repository import Gtk, WebKit2, GLib  # noqa: E402

URI = sys.argv[1]
OUT = sys.argv[2]

PROBE = """
(function () {
  var el = document.querySelector('#composer-library-items');
  var content = document.querySelector('.virtualized-catalog-content');
  var root = document.documentElement;
  function info(node) {
    if (!node) return null;
    var s = getComputedStyle(node);
    return {
      bg: s.backgroundColor, display: s.display, position: s.position,
      w: node.clientWidth, h: node.clientHeight, scrollTop: node.scrollTop
    };
  }
  return JSON.stringify({
    readyState: document.readyState,
    theme: root.getAttribute('data-theme') || 'dark',
    bgAlternate: getComputedStyle(root).getPropertyValue('--bg-alternate').trim(),
    viewport: info(el),
    content: info(content)
  });
})()
"""

results = []


def probe(web, label):
    def cb(obj, res, data):
        try:
            text = web.run_javascript_finish(res).get_js_value().to_string()
        except Exception as exc:
            text = 'ERROR: %s' % exc
        results.append({'label': label, 'data': text})
        if os_finished():
            with open(OUT, 'w') as handle:
                json.dump(results, handle, indent=2)
            Gtk.main_quit()
    web.run_javascript(PROBE, None, cb, None)


def os_finished():
    return any(entry['label'] == 'light_after_scroll' for entry in results)


def on_load(web, event):
    if event == WebKit2.LoadEvent.FINISHED:
        probe(web, 'dark_first_paint')


win = Gtk.Window()
win.set_default_size(1400, 900)
web = WebKit2.WebView()
web.connect('load-changed', on_load)
win.add(web)
win.show_all()
web.load_uri(URI)


def step(label, script, next_label=None):
    def cb(obj, res, data):
        try:
            web.run_javascript_finish(res)
        except Exception:
            pass
        if next_label is not None:
            probe(web, next_label)
        else:
            probe(web, label)
    web.run_javascript(script, None, cb, None)
    return False


GLib.timeout_add(700, lambda: probe(web, 'dark_after_paint'))
GLib.timeout_add(1200, lambda: step(
    'dark_after_scroll',
    "document.querySelector('#composer-library-items').scrollTop = 400;",
    'dark_after_scroll'))
GLib.timeout_add(1700, lambda: step(
    'light', "document.documentElement.setAttribute('data-theme', 'light');", 'light'))
GLib.timeout_add(2200, lambda: step(
    'light_after_scroll',
    "document.querySelector('#composer-library-items').scrollTop = 800;",
    'light_after_scroll'))
GLib.timeout_add(5000, lambda: (Gtk.main_quit(), False)[1])
Gtk.main()
with open(OUT, 'w') as handle:
    json.dump(results, handle, indent=2)
