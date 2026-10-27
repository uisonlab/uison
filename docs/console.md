# Console mode and the player

**English** · [Русский](ru/console.md)

A uison program can run without a window. In console mode it writes its frames to stdout and reads events from stdin, one JSON message per line. A **player** (`uison-player`) starts the program this way and shows it: the program's screen in one tab, and every message in both directions in another. This is the groundwork for running a program on one machine and showing it on another; for now everything stays on one machine.

```sh
cargo build --release -p disk-usage -p uison-player
target/release/uison-player target/release/disk-usage --console ~/Documents
```

## Turning it on in a program

The program decides when to run in console mode. Usually that is a command-line flag:

```rust
fn main() {
    let console = std::env::args().any(|a| a == "--console");
    if console {
        uison::run_console(app(), "My app"); // no window: frames out, events in
    }
    uison::run(app(), "My app");
}
```

`run_console` exists on macOS, Linux and Windows. It works with any app: a screen (`Ui`) or your own frame (`App`). The program still measures text with its own fonts. While it runs on the same machine as the player, the picture matches the normal window.

Anything the program prints by mistake doesn't break the stream: on macOS and Linux, stdout is kept for the protocol, and a stray `println!` goes to stderr. The player shows stderr in its log.

## Messages

From the program:

```js
{"uison":0,"title":"Disk Usage"}                    // first
{"frame":{"size":[1280,840],"background":"#24252A","items":[…]}}   // the whole frame after every change
{"host":"copy","text":"/Users/me/file"}             // the clipboard is the player's
{"host":"open","url":"https://…"}                   // a link
{"host":"paste"}                                    // please send the clipboard
```

A frame is the usual [frame of primitives](c/lessons/09-frame.md), as strict JSON on one line.

To the program, events in [their JSON form](screens/events.md), one per line:

```js
{"type":"resize","w":1280,"h":796}                  // the size of the program's area
{"type":"move","x":412.5,"y":230}  {"type":"down","x":412.5,"y":230,"button":"left"}  {"type":"up",…}
{"type":"key","code":"char","text":"a","shift":false,"ctrl":false,"alt":false,"cmd":false}
{"type":"paste","text":"…"}                         // the answer to {"host":"paste"}
{"action":"rescan"}  {"data":{"form.query":"jpg"}}  // messages to a screen
```

Timers, animation and background threads run inside the program. The program exits when stdin closes.

You can drive a program without the player too:

```sh
printf '%s\n' '{"type":"resize","w":900,"h":600}' '{"action":"about"}' | target/release/disk-usage --console | cut -c1-120
```

## The player

`uison-player [<program> [arguments…]]`

- Without a program, the player opens a **start screen**: type a command line (quotes group words, `~` is your home folder) or pick one of the programs found next to the player and this session's commands. **Stop** ends the program and returns here; so does the program exiting by itself.
- **App** shows the program's frames. The mouse over them and the keys go to the program, and a press that starts there keeps the mouse until it is released. The program gets the area's size (`resize`) and the system theme.
- **JSON** lists the messages: direction, time, kind, size and the start of the text. Click one to see it in full on the right. Frames and mouse moves are hidden by default and have their own switches. The field at the bottom sends your own line to the program, for example `{"action":"rescan"}`.
- The status line shows messages and kilobytes in both directions, frames per second, and the latency from the mouse or a key to the next frame.

## Over SSH

`uison-ssh-player` runs the program on a server and shows it here. The server needs `sshd` and nothing else. The start screen has three steps:

1. **Connect:** host, port, user; a key file (with its passphrase, if any) or a password. On a phone there is no `~/.ssh`: **Choose key…** opens the system's file chooser (Files, Drive) and keeps a copy of the chosen key in the app's own folder, readable by the app only — the key never goes through the clipboard; **New key** makes a key for this device and shows the line to add to `~/.ssh/authorized_keys` on the server. A new server's key is remembered and its fingerprint shown; a changed one is refused. The player checks `~/.ssh/known_hosts` and keeps its own list in `~/.config/uison/known_hosts`.
2. **Upload** (optional): a file on this computer goes to a path on the server, which is `~/uison/<name>` by default. The folder is created and the file made executable.
3. **Run:** the path on the server and its arguments; `--console` is added unless you untick it.

```sh
cargo run --release -p uison-ssh-player -- me@server.example -i ~/.ssh/id_ed25519
```

The SSH library lives in this player only: the program itself still knows nothing but stdin and stdout. The connection stays open between runs, and **Stop** ends the program but keeps the connection.

The player is also a repository of its own, [uisonlab/uison-ssh-player](https://github.com/uisonlab/uison-ssh-player), built against `uison-player` from crates.io; the copy here stays as an example of a player with a custom link.

## Costs

Measured on a Mac (M series) with Disk Usage on `/System/Library/Frameworks`:

| | |
|---|---|
| a frame with the chart | ~125 KB of JSON |
| the program per hover event | 1.7 ms: the app 1.0, parsing its own frame 0.4, compacting to one line 0.3 |
| the player per frame | 1.1 ms: parsing 0.4, embedding 0.3, its own frame 0.4 (then drawing) |
| event → frame back, events back to back | 1.85 ms (in-process: 1.4 ms) |
| zoom animation through the pipe | ~55 frames per second, 6–7 MB/s |

Locally this costs almost nothing. Over a network, ~125 KB per hover would be too much. That is why the remote window will send the screen and data changes rather than frames.
