#!/usr/bin/python3
# Real input in a GNOME session through org.gnome.Mutter.RemoteDesktop (+ ScreenCast for absolute positions and screenshots),
# no root and no permission dialogs. Commands run in order in one session (it ends with the process):
#   tools/gnomedrive.py move X Y | click [left|right|middle] [n] | down B | up B | drag X0 Y0 X1 Y1 [steps]
#             key NAME (xkb keysym name, mods: ctrl+shift+a) | press KEY | release KEY | type TEXT
#             wheel N | smooth DX DY (touchpad) | wait MS | shot FILE (the whole screen, PNG, via PipeWire + GStreamer)
# Keys go to the focused window — check it with a shot first. Mutter types only keysyms of the current layout
# (Cyrillic needs the ru layout: key super+space). From a snap terminal (VS Code) start it with tools/gnome-run.sh.
import sys, time, subprocess, ctypes, gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib

bus = Gio.bus_get_sync(Gio.BusType.SESSION)
def call(dest, path, iface, method, args=None, sig=None):
    return bus.call_sync(dest, path, iface, method, args, sig, Gio.DBusCallFlags.NONE, 10000, None)

RD, SC = 'org.gnome.Mutter.RemoteDesktop', 'org.gnome.Mutter.ScreenCast'
rd = call(RD, '/org/gnome/Mutter/RemoteDesktop', RD, 'CreateSession').unpack()[0]
sid = call(RD, rd, 'org.freedesktop.DBus.Properties', 'Get', GLib.Variant('(ss)', (RD + '.Session', 'SessionId'))).unpack()[0]
sc = call(SC, '/org/gnome/Mutter/ScreenCast', SC, 'CreateSession',
          GLib.Variant('(a{sv})', ({'remote-desktop-session-id': GLib.Variant('s', sid)},))).unpack()[0]
stream = call(SC, sc, SC + '.Session', 'RecordMonitor',
              GLib.Variant('(sa{sv})', ('', {'cursor-mode': GLib.Variant('u', 1)}))).unpack()[0]
node = [None]
bus.signal_subscribe(SC, SC + '.Stream', 'PipeWireStreamAdded', stream, None, 0,
                     lambda *a: node.__setitem__(0, a[5].unpack()[0]))
call(RD, rd, RD + '.Session', 'Start')
ctx = GLib.MainContext.default()
t0 = time.time()
while node[0] is None and time.time() - t0 < 5:
    ctx.iteration(False); time.sleep(0.01)

def s(method, fmt, *args):
    call(RD, rd, RD + '.Session', method, GLib.Variant(fmt, args) if fmt else None)
    ctx.iteration(False)

xkb = ctypes.CDLL('libxkbcommon.so.0')
xkb.xkb_keysym_from_name.restype = ctypes.c_uint32
xkb.xkb_utf32_to_keysym.restype = ctypes.c_uint32
def keysym(name):
    if len(name) == 1:
        c = ord(name)
        return c if 0x20 <= c < 0x7f else xkb.xkb_utf32_to_keysym(c)
    k = xkb.xkb_keysym_from_name(name.encode(), 1)
    if not k: sys.exit('unknown key ' + name)
    return k
MODS = {'ctrl': 'Control_L', 'shift': 'Shift_L', 'alt': 'Alt_L', 'super': 'Super_L'}
BTN = {'left': 0x110, 'right': 0x111, 'middle': 0x112}
pos = [0.0, 0.0]
def move(x, y):
    pos[:] = [x, y]
    s('NotifyPointerMotionAbsolute', '(sdd)', stream, x, y)
def btn(b, down):
    s('NotifyPointerButton', '(ib)', BTN[b], down)
def ks(k, down):
    s('NotifyKeyboardKeysym', '(ub)', k, down)

a = sys.argv[1:]
i = 0
def take(n):
    global i
    v = a[i:i + n]; i += n; return v
while i < len(a):
    c = a[i]; i += 1
    if c == 'move':
        x, y = map(float, take(2)); move(x, y)
    elif c == 'click':
        b = 'left'; n = 1
        if i < len(a) and a[i] in BTN: b = take(1)[0]
        if i < len(a) and a[i].isdigit(): n = int(take(1)[0])
        for _ in range(n):
            btn(b, True); time.sleep(0.03); btn(b, False); time.sleep(0.06)
    elif c in ('down', 'up'):
        btn(take(1)[0], c == 'down')
    elif c == 'drag':
        x0, y0, x1, y1 = map(float, take(4))
        n = int(take(1)[0]) if i < len(a) and a[i].isdigit() else 20
        move(x0, y0); time.sleep(0.05); btn('left', True); time.sleep(0.1)
        for k in range(1, n + 1):
            move(x0 + (x1 - x0) * k / n, y0 + (y1 - y0) * k / n); time.sleep(0.016)
        time.sleep(0.1); btn('left', False)
    elif c == 'key':
        parts = take(1)[0].split('+')
        mods = [keysym(MODS[m]) for m in parts[:-1]]
        for m in mods: ks(m, True)
        k = keysym(parts[-1]); ks(k, True); time.sleep(0.02); ks(k, False)
        for m in reversed(mods): ks(m, False)
    elif c in ('press', 'release'):
        ks(keysym(MODS.get(a[i], a[i])), c == 'press'); i += 1
    elif c == 'type':
        for ch in take(1)[0]:
            k = keysym(ch); ks(k, True); ks(k, False); time.sleep(0.02)
    elif c == 'wheel':
        n = int(take(1)[0])
        s('NotifyPointerAxisDiscrete', '(ui)', 0, n)
    elif c == 'smooth':
        dx, dy = map(float, take(2))
        for _ in range(10):
            s('NotifyPointerAxis', '(ddu)', dx / 10, dy / 10, 4); time.sleep(0.016)
        # Flags: 1 finish, 4 finger. Mutter 46 sends no axis_stop for it, so no inertia this way.
        s('NotifyPointerAxis', '(ddu)', 0.0, 0.0, 1 | 4)
    elif c == 'wait':
        t = time.time() + int(take(1)[0]) / 1000
        while time.time() < t:
            ctx.iteration(False); time.sleep(0.005)
    elif c == 'shot':
        f = take(1)[0]
        subprocess.run(['gst-launch-1.0', '-q', 'pipewiresrc', f'path={node[0]}', 'always-copy=true', 'num-buffers=3', '!',
                        'videoconvert', '!', 'pngenc', 'snapshot=true', '!', 'filesink', f'location={f}'],
                       timeout=15, check=True)
    else:
        sys.exit('unknown command ' + c)
    ctx.iteration(False)
call(RD, rd, RD + '.Session', 'Stop')
