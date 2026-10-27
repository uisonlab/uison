#!/usr/bin/python3
# Where a uison window's content is on the screen: tools/gnome-locate.py screen.png capture.png [ox oy] → "X Y" of the
# content origin. capture.png — the window's first UISON_CAPTURE shot; ox oy — the content in it (20 56: own frame with
# shadow and title, 0 0: no own frame — X11, maximized).
# A row piece of the capture (the app's own buffer) is found byte for byte in the screenshot: first from the top-left
# corner of the content (stays the same when the window is resized), then from the right half (the left under the dock).
import sys
from PIL import Image
scr = Image.open(sys.argv[1]).convert('RGB'); cap = Image.open(sys.argv[2]).convert('RGB')
ox, oy = (int(sys.argv[3]), int(sys.argv[4])) if len(sys.argv) > 4 else (20, 56)
sw = scr.width; sb = scr.tobytes(); cb = cap.tobytes()
tries = [(ox + 4, ox + 180, cy) for cy in range(oy + 6, oy + 80, 3)]
tries += [(cap.width // 2, cap.width - ox - 10, cy) for cy in range(oy + 40, cap.height - 40, 37)]
for x0, x1, cy in tries:
    seg = cb[(cy * cap.width + x0) * 3:(cy * cap.width + x1) * 3]
    if len(set(seg)) < 12:
        continue  # a plain row matches anywhere
    hits = []
    for y in range(scr.height):
        row = sb[y * sw * 3:(y + 1) * sw * 3]
        x = row.find(seg)
        while x >= 0 and x % 3:
            x = row.find(seg, x + 1)
        if x >= 0:
            hits.append((x // 3 - (x0 - ox), y - (cy - oy)))
    if len(hits) == 1:
        print(*hits[0]); sys.exit(0)
sys.exit('not found')
