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
      bg: s.backgroundColor,
      bgImage: s.backgroundImage,
      display: s.display,
      position: s.position,
      w: node.clientWidth,
      h: node.clientHeight,
      scrollH: node.scrollHeight,
      scrollTop: node.scrollTop
    };
  }
  return JSON.stringify({
    readyState: document.readyState,
    theme: root.getAttribute('data-theme'),
    bgAlternate: getComputedStyle(root).getPropertyValue('--bg-alternate').trim(),
    bgPrimary: getComputedStyle(root).getPropertyValue('--bg-primary').trim(),
    body: info(document.body),
    viewport: info(el),
    content: info(content)
  });
})()
"""

results = []


def evaluate(web, label):
    def cb(obj, res, data):
        try:
            value = web.run_javascript_finish(res)
            text = value.get_js_value().to_string()
        except Exception as exc:
            text = 'ERROR: %s' % exc
        results.append({'label': label, 'data': text})
        if len(results) == 3:
            with open(OUT, 'w') as handle:
                json.dump(results, handle, indent=2)
            Gtk.main_quit()
    web.run_javascript(PROBE, None, cb, None)


def on_load(web, event):
    if event == WebKit2.LoadEvent.FINISHED:
        evaluate(web, 'first_paint')


win = Gtk.Window()
web = WebKit2.WebView()
web.connect('load-changed', on_load)
win.add(web)
win.show_all()
web.load_uri(URI)


def later():
    evaluate(web, 'immediate_after_paint')
    return False


def much_later():
    web.run_javascript(
        "document.querySelector('#composer-library-items').scrollTop = 400;",
        None,
        lambda *args: evaluate(web, 'after_scroll'),
        None
    )
    return False


GLib.timeout_add(300, later)
GLib.timeout_add(900, much_later)
GLib.timeout_add(4000, lambda: (Gtk.main_quit(), False)[1])
Gtk.main()
with open(OUT, 'w') as handle:
    json.dump(results, handle, indent=2)
