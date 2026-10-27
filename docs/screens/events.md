# Events as JSON

**English** · [Русский](../ru/screens/events.md)

Frames leave the program as JSON, and events can come in as JSON too. Inside a program, events stay typed: a Rust handler gets `cx.key`, not a string to parse. The JSON form is for the places where an event crosses a boundary: the C low level (`uison_app.on_event`), messages sent to a screen from outside, and test scenarios. There is one form for every language, and the shared test file [`scenarios/events.jsonl`](../../scenarios/events.jsonl) keeps Rust and C++ in agreement.

## Raw events

What the platform reports: the mouse, a finger, keys, the window. A C program on the low level receives these in `on_event` ([C lesson 9](../c/lessons/09-frame.md), [C++ lesson 10](../cpp/lessons/10-frame.md)).

```js
{"type":"down","x":120,"y":48.5,"button":"left"}   // button: left, right, other, touch (a finger)
{"type":"up","x":120,"y":48.5,"button":"left"}
{"type":"move","x":121,"y":49}
{"type":"leave"}                                   // the pointer left the window
{"type":"scroll","x":300,"y":200,"dx":0,"dy":-40}
{"type":"key","code":"char","text":"ж","shift":false,"ctrl":false,"alt":false,"cmd":true}
{"type":"resize","w":1280,"h":800}
{"type":"safe-area","top":47,"right":0,"bottom":34,"left":0}
{"type":"appearance","dark":true}
{"type":"paste","text":"from the clipboard"}
{"type":"tick"}  {"type":"image"}  {"type":"wake"}
```

- Coordinates are in points from the top left of the window, written to 1/1000 of a point.
- `code`: `char` (the typed text is in `text`), `enter`, `backspace`, `delete`, `tab`, `escape`, `left`, `right`, `up`, `down`, `home`, `end`, `pageup`, `pagedown`, `back` (Android), `menu`, `f10`. For other keys `text` is `null`.
- `cmd` is the primary modifier: ⌘ on Apple, Ctrl elsewhere.
- Readers skip unknown keys, and a missing modifier counts as `false`.

## Messages to a screen

One level up from raw events. A message doesn't say where the mouse was pressed. It says which action fired and what changed in the data:

```js
{"action":"save","event":"click","key":"ok","pos":[12,8.5],"clicks":2}
{"data":{"form.query":"jpg","form.agree":true}}
```

- **`action`** runs the handler as if the element with this `key` had fired the event. The handler sees the same `cx.event`, `cx.key`, `cx.pos` (relative to the node) and `cx.clicks`. Only `action` is required: `event` defaults to `click`, `key` to empty, `clicks` to 1, and without `pos` there is no position.
- **`data`** sets values by path, as a field, switch or slider would. Derived data (`derive`) is recomputed and the screen is redrawn.

How to send one:

| language | call |
|---|---|
| Rust | `ui.apply(r#"{"action":"save","key":"ok"}"#)` |
| C | `uison_ui_apply(ui, UISON_JSON({"data":{"form.query":"jpg"}}))` |
| C++ | `ui.apply(R"({"action":"save","key":"ok"})")` |

The call returns `false` if it is not a message the screen understands (invalid JSON, or neither `action` nor `data`). Inside the program a message travels like any other event (`Event::Message` in Rust), so a scenario can send it too.

## In test scenarios

A scenario (`UISON_SCRIPT`) can send both kinds as they are, next to steps that address elements by id:

```js
[
  ["tap", "draft/1/0"], ["type", "Buy milk"],
  ["event", {"type":"key","code":"enter"}],
  ["action", {"action":"remove","key":"del-1"}],
  ["action", {"data":{"filter":"done"}}],
  ["capture", "01_done"], ["done"]
]
```
