# Agent mode

**English** · [Русский](ru/agents.md)

An LLM agent can use a uison program without screenshots. The program describes its screen by meaning, not pixels: each element has a role, a name, a state and a `key`. The agent acts on that description: press `compose`, type into "Email", choose "Germany". Each action goes through the same path as a person's mouse and keyboard, so the agent can't press a disabled button or a button hidden under a dialog.

```text
$ printf '{"id":1,"query":"text"}\n' | todo --agent
{"uison":0,"agent":1,"title":"uison todo"}
{"seq":1,"text":"scroll #root offset=0 content=800
  heading \"Todos\"
  textfield #draft/1/0 value=\"\" placeholder=\"What needs to be done?\" on=[submit]
  button \"Add\" #root/0/0/0/1/1 disabled
  option \"All\" #root/0/0/0/2/0/all selected
  checkbox \"Buy bread\" #1/0 unchecked
  button \"trash\" #del-1
  …"}
```

There are two ways in, both on stdin and stdout:

- `--agent`: JSON lines, for scripts and your own agents.
- `--mcp`: an [MCP](https://modelcontextprotocol.io) server, so the program becomes a set of tools for Claude Code and other MCP clients.

```sh
cargo build --release -p todo --features agent
claude mcp add todo -- "$PWD/target/release/todo" --mcp
```

## Turning it on in a program

Agent mode is in a separate crate, `uison-agent`. A program without it is unchanged: same frames, same size. The examples `todo`, `form2`, `gallery`, `studio` and `disk-usage` turn it on with the `agent` feature:

```toml
[dependencies]
uison-agent = { version = "0.1", optional = true }

[features]
agent = ["dep:uison-agent"]
```

```rust
fn main() {
    #[cfg(feature = "agent")]
    if let Some(run) = uison_agent::mode(&mut std::env::args_os().collect()) {
        run(app(), "My app"); // --agent or --mcp: no window
    }
    uison::run(app(), "My app");
}
```

Agent mode works with screens (`Ui`). A program that writes its own frame (`App`) has no description to give.

## What the agent sees

`Ui::semantics()` turns the screen into nodes, and the agent receives them as text, one node per line, with nested nodes indented:

| role | comes from | state |
|---|---|---|
| `button` | a click handler, an interactive component | `disabled`, `menu` (has a context menu) |
| `checkbox` | `toggle` on data | `checked` / `unchecked` |
| `option` | `select` (tabs, radio buttons, rows to pick) | `selected` |
| `button` … `collapsed` | `toggle` on local state (a dropdown) | `expanded`, `value` (what it shows) |
| `slider` | `drag` | `value`, `min`, `max`, `step` |
| `textfield` | `Input` | `value`, `placeholder`, `focused`, `password` (dots) |
| `link` | `href` | `url` |
| `list` | `List` (only visible rows are listed) | `count`, `first`, `offset` |
| `scroll` | `Scroll` | `offset`, `content` |
| `dialog`, `menuitem` | a modal (`dismiss`), the open context menu | |
| `text`, `heading`, `image` | text outside controls, `Image` with `alt` | |

A control is named by the text inside it. If it has no text, its name comes from its `tooltip`, or from the `alt` of an icon inside it: `Icon` passes its name, so an icon button reads as `"trash"`. A text right next to a checkbox in a row, or right above a field in a column, becomes that control's label. To name something yourself, set `alt` on any node:

```json
{ IconButton: { icon: "trash", alt: "Delete task", on: { click: "remove" } } }
```

A row of plain text is joined into one line: `text "Anna · Report · 10:42"`. Anything outside a scroll view's visible part is left out, except for interactive nodes, which stay in the list marked `offscreen`.

## Actions

| JSON (`--agent`) | MCP tool | what happens |
|---|---|---|
| `{"do":"click","key":"add"}`, `"name":"Add"`, `"clicks":2`, `"button":"right"` | `click` | press and release on the node; scrolls it into view first if needed |
| `{"do":"type","key":…,"text":"Milk","submit":true}` | `type` | focus, select all, type (`"append":true` adds), Enter |
| `{"do":"key","code":"cmd+z"}` | `press_key` | a key to the focused node (`cmd` is Ctrl on Windows and Linux) |
| `{"do":"hover","key":"star"}` | `hover` | the pointer rests on the node: tooltip, hover handlers |
| `{"do":"scroll","key":"inbox","to":120}`, `"by"`, `"edge"` | `scroll` | to a list item, to a key, by points, to an edge |
| `{"do":"set","key":"volume","value":50}` | `set_value` | a slider is dragged, a checkbox clicked, a field typed into |
| `{"do":"pick","key":"country","option":"Germany"}` | `choose` | opens a dropdown, picks an option; tabs and radio buttons too |
| `{"do":"menu","key":"row-17","item":"Delete"}` | `context_menu` | right click, then the item by name |
| `{"do":"wait","ms":3000,"until":{"name":"Done"}}` | `wait` | real time for background work, until a node or `{"path","value"}` appears |
| `{"query":"data","path":"todos"}` | `read_data` | the data model; large parts are shortened |
| `{"action":"remove","key":"del-1"}` | `call_action` | the handler directly, without the screen's checks |

A node is addressed by `key` (with or without the `#` from the snapshot) or by `name` (with an optional `role`). The best match wins: the whole name, then its start, then a whole word, then a part of a word. If several nodes match equally well, the error lists their keys.

Errors say why: `'sendBtn' is disabled`, `'inc' is covered by 'scrim'`, `no node 'tabs'; under it: tabs/all, tabs/done`, `nothing named 'Sing up'`.

The answer to an action lists only what changed since the agent last looked (`+` added, `-` removed, `~` changed). When most of the screen is new, or the agent hasn't looked yet, the answer is the whole screen. Requests the program makes to the system come back as notices: `{"host":"open","url":…}`, `{"host":"copy","text":…}`, and `{"host":"pick_file","action":…}`, which the agent answers with `{"action":…,"key":"/path"}`.

## Time

Time in agent mode is virtual. After an action, the program settles at once: animations finish, timers fire and a tooltip's delay passes, up to one second of virtual time. Between the agent's steps the program stands still. Work in other threads (a disk scan, a download) still arrives, and `wait` gives it real time. Clicks are spaced apart in virtual time, so two clicks in a row are two clicks, not a double click.

## Measured

On an Apple M-series Mac, text is measured without fonts:

| | todo | Studio | Disk Usage |
|---|---|---|---|
| a frame (JSON) | 4.2 KB | 14.5 KB | 24.2 KB |
| the screen as text for the agent | 0.6 KB | 2.6 KB | 0.9 KB |
| a snapshot | 0.04 ms | 0.06 ms | |
| an action and its answer (median) | 0.7 ms | 4 ms (a frame build is 1–2 ms) | |

Claude Sonnet, using only the program's MCP tools and seeing no screen at all, completed each of these tasks:

| task | tool calls | time | cost |
|---|---|---|---|
| todo: add three items, tick one, delete one, filter, check the data | 11 | 21 s | $0.10 |
| Studio: find a message among 2000, star it, reply; fill and send the sign-up form (dropdown, slider, plan, terms) | 17 | 38 s | $0.11 |
| Disk Usage: wait for the scan, find the biggest file in nested folders, queue it for the Trash, switch to Russian | 14 | 61 s | $0.17 |

## Limits

- A `Canvas` draws its own pictures, and the agent sees only the node (`canvas #chart`), not the shapes. In Disk Usage the chart repeats the list, so this doesn't hide anything.
- Roles are inferred from behavior. A component the library doesn't know may need an `alt`.
- There's one agent per program, and the program has no window while the agent drives it.
