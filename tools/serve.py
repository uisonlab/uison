#!/usr/bin/env python3
"""Server for uison web builds: static files + receiving screenshots and logs for automated checks.

    python3 tools/serve.py target/web/studio [port]

POST /capture?name=…  — canvas PNG → <dir>/capture/<name>.png
POST /log             — a log line from the browser → stdout
GET  /done            — the drive.js scenario has finished (prints DONE)
"""

import http.server
import pathlib
import sys
import urllib.parse

root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
port = int(sys.argv[2]) if len(sys.argv) > 2 else 8000
captures = root / "capture"


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def __init__(self, *a, **kw):
        super().__init__(*a, directory=str(root), **kw)

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def body(self):
        return self.rfile.read(int(self.headers.get("Content-Length", 0)))

    def reply(self, text=b"ok"):
        self.send_response(200)
        self.send_header("Content-Length", str(len(text)))
        self.end_headers()
        self.wfile.write(text)

    def do_POST(self):
        url = urllib.parse.urlparse(self.path)
        if url.path == "/capture":
            name = urllib.parse.parse_qs(url.query).get("name", ["capture"])[0]
            captures.mkdir(exist_ok=True)
            (captures / f"{pathlib.Path(name).name}.png").write_bytes(self.body())
            print(f"capture {name}", flush=True)
        elif url.path == "/log":
            print(self.body().decode("utf-8", "replace"), flush=True)
        self.reply()

    def do_GET(self):
        if self.path == "/done":
            print("DONE", flush=True)
            return self.reply()
        super().do_GET()

    def log_message(self, *a):
        pass


print(f"uison: http://localhost:{port}/  ({root})", flush=True)
http.server.ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
