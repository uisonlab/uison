// Drives the real mouse and keyboard for debugging a uison window.
// Build: swiftc -O -o target/drive tools/drive.swift
// Needs the Accessibility permission for the app it is launched from.
//
// drive <process> <content height> commands... — coordinates in points inside the window.
// Commands: where | map file | tap id | hover id | move x y | click x y | rclickat x y | down x y | up x y | drag x y
//           dragto x0 y0 x1 y1 steps | scroll x y dy | type text | key code | cmdkey code | wait ms
//           mkey shift+alt code (modifiers: cmd, shift, alt, ctrl) | clicks id n (double, triple click)
//           shifttap id (click with Shift held) | rclick id (right button) | dragid id0 dx0 id1 dx1 steps (drag from id0 to id1, dx from the left edge)
//
// Together with UISON_CAPTURE=dir (window screenshots after each paint) and UISON_STATS=1:
//   UISON_CAPTURE=cap UISON_STATS=1 target/release/demo &
//   target/drive demo 560 move 240 470 wait 500 click 58 352
// By element ids (robust to layout changes):
//   UISON_HITMAP=map.txt target/release/todo &
//   target/drive todo 620 map map.txt tap draft/1/0 type "Bread" key 36 tap del-1
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count >= 3, let contentH = Double(args[2]) else { print("usage"); exit(2) }
let owner = args[1]

func windowOrigin() -> CGPoint {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as! [[String: Any]]
    for w in list where (w[kCGWindowOwnerName as String] as? String) == owner && (w[kCGWindowLayer as String] as? Int) == 0 {
        let b = w[kCGWindowBounds as String] as! [String: Double]
        // The content is the bottom contentH points of the window (the title bar is on top).
        return CGPoint(x: b["X"]!, y: b["Y"]! + b["Height"]! - contentH)
    }
    print("window of \(owner) not found"); exit(1)
}

let o = windowOrigin()
func pt(_ x: String, _ y: String) -> CGPoint { CGPoint(x: o.x + Double(x)!, y: o.y + Double(y)!) }
func mouse(_ t: CGEventType, _ p: CGPoint, flags: CGEventFlags = [], clicks: Int64 = 1) {
    let right = t == .rightMouseDown || t == .rightMouseUp || t == .rightMouseDragged
    let e = CGEvent(mouseEventSource: nil, mouseType: t, mouseCursorPosition: p, mouseButton: right ? .right : .left)!
    e.flags = flags
    e.setIntegerValueField(.mouseEventClickState, value: clicks)
    e.post(tap: .cghidEventTap)
    usleep(30_000)
}

/// Virtual key codes of the modifiers and their flags.
let modKeys: [String: (CGKeyCode, CGEventFlags)] = [
    "cmd": (55, .maskCommand), "shift": (56, .maskShift), "alt": (58, .maskAlternate), "ctrl": (59, .maskControl),
]

/// Hold the modifiers (as real keys, so `NSEvent.modifierFlags` sees them), run, release.
func withMods(_ spec: String, _ body: (CGEventFlags) -> Void) {
    let keys = spec.split(separator: "+").map { modKeys[String($0)]! }
    var flags: CGEventFlags = []
    for (vk, f) in keys {
        flags.insert(f)
        let e = CGEvent(keyboardEventSource: nil, virtualKey: vk, keyDown: true)!
        e.flags = flags
        e.post(tap: .cghidEventTap)
    }
    body(flags)
    for (vk, f) in keys.reversed() {
        flags.remove(f)
        let e = CGEvent(keyboardEventSource: nil, virtualKey: vk, keyDown: false)!
        e.flags = flags
        e.post(tap: .cghidEventTap)
    }
    usleep(30_000)
}

/// Left edge of a hit area + dx, vertically centered.
func areaAt(_ id: String, _ dx: Double) -> CGPoint {
    let c = area(id)
    guard let path = mapPath, let text = try? String(contentsOfFile: path, encoding: .utf8) else { return c }
    for line in text.split(separator: "\n") {
        let f = line.split(separator: "\t").map(String.init)
        if f.count == 5, f[0] == id, let x = Double(f[1]) { return CGPoint(x: o.x + x + dx, y: c.y) }
    }
    return c
}

var i = 3
var mapPath: String? = nil
/// Center of a hit area by id from the UISON_HITMAP map (re-read before every tap).
func area(_ id: String) -> CGPoint {
    guard let path = mapPath, let text = try? String(contentsOfFile: path, encoding: .utf8) else {
        print("no hit map: use 'map <file>' and run the app with UISON_HITMAP=<file>"); exit(1)
    }
    for line in text.split(separator: "\n") {
        let f = line.split(separator: "\t").map(String.init)
        if f.count == 5, f[0] == id, let x = Double(f[1]), let y = Double(f[2]), let w = Double(f[3]), let h = Double(f[4]) {
            return CGPoint(x: o.x + x + w / 2, y: o.y + y + h / 2)
        }
    }
    print("no hit area '\(id)'"); exit(1)
}
while i < args.count {
    let c = args[i]; i += 1
    switch c {
    case "where": print("content origin on screen:", o)
    case "map": mapPath = args[i]; i += 1
    case "tap": // tap <id> — click an element by id from the UISON_HITMAP map
        let p = area(args[i]); i += 1
        mouse(.mouseMoved, p); mouse(.leftMouseDown, p); mouse(.leftMouseUp, p)
    case "rclick": // rclick <id> — right button
        let p = area(args[i]); i += 1
        mouse(.mouseMoved, p); mouse(.rightMouseDown, p); mouse(.rightMouseUp, p)
    case "rclickat": // rclickat <x> <y> — right button at a point (custom drawing without hit areas)
        let p = pt(args[i], args[i+1]); i += 2
        mouse(.mouseMoved, p); mouse(.rightMouseDown, p); mouse(.rightMouseUp, p)
    case "hover": // hover <id>
        mouse(.mouseMoved, area(args[i])); i += 1
    case "move": mouse(.mouseMoved, pt(args[i], args[i+1])); i += 2
    case "down": mouse(.leftMouseDown, pt(args[i], args[i+1])); i += 2
    case "up": mouse(.leftMouseUp, pt(args[i], args[i+1])); i += 2
    case "drag": mouse(.leftMouseDragged, pt(args[i], args[i+1])); i += 2
    case "click":
        let p = pt(args[i], args[i+1]); i += 2
        mouse(.mouseMoved, p); mouse(.leftMouseDown, p); mouse(.leftMouseUp, p)
    case "scroll":
        let p = pt(args[i], args[i+1]); let dy = Int32(args[i+2])!; i += 3
        mouse(.mouseMoved, p)
        CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1, wheel1: dy, wheel2: 0, wheel3: 0)!.post(tap: .cghidEventTap)
        usleep(16_000)
    case "type":
        for ch in args[i].utf16 {
            for down in [true, false] {
                let e = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: down)!
                e.flags = []
                var u = ch; e.keyboardSetUnicodeString(stringLength: 1, unicodeString: &u)
                e.post(tap: .cghidEventTap)
            }
            usleep(40_000)
        }
        i += 1
    case "dragto": // dragto x0 y0 x1 y1 steps — press, drag, release
        let a = pt(args[i], args[i+1]), b = pt(args[i+2], args[i+3]); let n = Int(args[i+4])!; i += 5
        mouse(.mouseMoved, a); mouse(.leftMouseDown, a)
        for k in 1...n {
            let t = Double(k) / Double(n)
            mouse(.leftMouseDragged, CGPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t))
        }
        mouse(.leftMouseUp, b)
    case "key", "cmdkey": // key <virtualKey> (53 = Escape); cmdkey <virtualKey> — with ⌘ (12 = Q, 45 = N)
        let vk = CGKeyCode(args[i])!; i += 1
        let cmd = c == "cmdkey"
        // Command is pressed and released explicitly, otherwise the system considers it held.
        if cmd { CGEvent(keyboardEventSource: nil, virtualKey: 55, keyDown: true)!.post(tap: .cghidEventTap) }
        for down in [true, false] {
            let e = CGEvent(keyboardEventSource: nil, virtualKey: vk, keyDown: down)!
            e.flags = cmd ? .maskCommand : []
            e.post(tap: .cghidEventTap)
        }
        if cmd {
            let up = CGEvent(keyboardEventSource: nil, virtualKey: 55, keyDown: false)!
            up.flags = []
            up.post(tap: .cghidEventTap)
        }
        usleep(30_000)
    case "mkey": // mkey shift+alt 123
        let spec = args[i], vk = CGKeyCode(args[i+1])!; i += 2
        withMods(spec) { flags in
            for down in [true, false] {
                let e = CGEvent(keyboardEventSource: nil, virtualKey: vk, keyDown: down)!
                e.flags = flags
                e.post(tap: .cghidEventTap)
            }
        }
    case "clicks": // clicks <id> <n> — a series of clicks with clickState 1…n
        let p = area(args[i]); let n = Int64(args[i+1])!; i += 2
        mouse(.mouseMoved, p)
        for k in 1...n { mouse(.leftMouseDown, p, clicks: k); mouse(.leftMouseUp, p, clicks: k) }
    case "shifttap": // shifttap <id>
        let p = area(args[i]); i += 1
        withMods("shift") { flags in mouse(.leftMouseDown, p, flags: flags); mouse(.leftMouseUp, p, flags: flags) }
    case "dragid": // dragid <id0> <dx0> <id1> <dx1> <steps>
        let a = areaAt(args[i], Double(args[i+1])!), b = areaAt(args[i+2], Double(args[i+3])!); let n = Int(args[i+4])!; i += 5
        mouse(.mouseMoved, a); mouse(.leftMouseDown, a)
        for k in 1...n {
            let t = Double(k) / Double(n)
            mouse(.leftMouseDragged, CGPoint(x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t))
        }
        mouse(.leftMouseUp, b)
    case "wait": usleep(UInt32(args[i])! * 1000); i += 1
    default: print("unknown command \(c)"); exit(2)
    }
}
