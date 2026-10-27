#!/usr/bin/python3
# tools/gnome-scale.py [factor] — print the monitor scale, or set it temporarily (method 1: reverts on its own if not confirmed).
import sys, gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib
bus = Gio.bus_get_sync(Gio.BusType.SESSION)
D = 'org.gnome.Mutter.DisplayConfig'
def call(m, args=None):
    return bus.call_sync(D, '/org/gnome/Mutter/DisplayConfig', D, m, args, None, 0, 10000, None).unpack()
serial, monitors, logical, props = call('GetCurrentState')
print('scale', [l[2] for l in logical])
if len(sys.argv) > 1:
    s = float(sys.argv[1])
    cur = {m[0][0]: next(md[0] for md in m[1] if md[6].get('is-current')) for m in monitors}
    lms = [(l[0], l[1], s, l[3], l[4], [(mon[0], cur[mon[0]], {}) for mon in l[5]]) for l in logical]
    call('ApplyMonitorsConfig', GLib.Variant('(uua(iiduba(ssa{sv}))a{sv})', (serial, 1, lms, {})))
    print('set', s)
