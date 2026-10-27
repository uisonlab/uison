# Layout

**English** · [Русский](../ru/screens/layout.md)

Layout works as in Flutter: **constraints go down, sizes go up, the parent sets the position**. The parent tells the child how much space there is (the minimum and maximum size). The child picks its size within those limits, and the parent puts it in place. The unit is the point (pt). On 2× density screens that is two pixels; thin lines are aligned to physical pixels.

## Sizes: `w`, `h`

| value | what |
|---|---|
| not set, `"hug"` | fits the content (the default; `Input` defaults to `"fill"`) |
| `120` | exactly 120 pt |
| `"fill"` | all available space |
| `"50%"` | a fraction of the parent's available space |

`minW`, `maxW`, `minH`, `maxH` constrain any of these. `fill` never exceeds its own `maxW`: that is how you make "full width, but no wider than 640" columns.

## Padding and margin

- `padding` — inside the node (on containers: `Box`, `Row`, `Column`, `Stack`, `Wrap`, `Center`, `Align`, `Flow`).
- `margin` — outside, on any node.

Both take a number, `[vertical, horizontal]` or `[top, right, bottom, left]`.

## `Row` and `Column`

```js
{ Row: { gap: 8, main: "between", cross: "center", children: [ … ] } }
```

- `gap` — the space between children.
- `main` — distribution along the main axis (horizontal for `Row`): `start`, `center`, `end`, `between`, `around`, `evenly`.
- `cross` — along the cross axis: `start`, `center`, `end`, `stretch`.
- `flex: 1` on a child — its share of the free space along the main axis (`flex: 2` gets twice as much). `w: "fill"` on a child of a `Row` is the same as `flex: 1`.
- `Spacer` — empty space with `flex: 1`: `[Text, Spacer, Button]` pushes the button to the right.
- `cross: "stretch"` stretches children along the cross axis. A child's explicit size along that axis then has no effect (as in Flutter). To give a block a specific width inside `Column { cross: stretch }`, wrap it in `Align`.
- `stackBelow: 600` on a `Row` — with less than 600 pt available, the children stack vertically (a phone layout).

## `Box`, `Center`, `Align`

```js
{ Box: { padding: 16, fill: "$theme.card", radius: 12, stroke: "$theme.border", width: 1, child: { … } } }
{ Center: { child: { Text: { text: "Centered" } } } }
{ Align: { align: "bottomRight", child: { Button: { text: "Next" } } } }
```

`Box` has one child, a fill, a border, rounded corners, `clip` (clip to its own bounds) and `align` (where the child sits inside). Positions: `topLeft`, `top`, `topRight`, `left`, `center`, `right`, `bottomLeft`, `bottom`, `bottomRight`.

## `Stack`

```js
{ Stack: { children: [
  { Image: { src: "photo.jpg", w: 320, h: 200 } },
  { Badge: { text: "New", place: "topRight", margin: 8 } },
  { Box: { pos: [12, 160], child: { Text: { text: "Caption" } } } },
] } }
```

Children are layered on top of each other, and the `Stack` is as large as its largest child. On a child, `place` sets the alignment and `pos: [x, y]` an exact position. `overlay: true` puts a child above everything without affecting the `Stack` size (the `Select` drop-down, pop-up panels).

## `Wrap`

```js
{ Wrap: { gap: 8, runGap: 8, children: [ { Badge: { text: "rust" } }, { Badge: { text: "c++" } } ] } }
```

Children are laid out in a row and wrap to the next one when space runs out. `gap` is the space between children, `runGap` the space between rows, and `cross` the alignment within a row.

## Text and `Flow`

`Text` wraps words to the available width. `maxLines` truncates with an ellipsis, `wrap: false` disables wrapping, `align` aligns the lines, and `lineHeight` sets the line spacing.

`Flow` is a single paragraph made of several pieces: `Text` children with different styles wrap across their boundaries, like text in a browser. This lets you put a bold word or a link in the middle of a line:

```js
{ Flow: { children: [
  { Text: { text: "Click " } },
  { Link: { text: "here", href: "https://example.com" } },
  { Text: { text: " to open the help.", italic: true } },
] } }
```

## Scrolling: `Scroll`

```js
{ Scroll: { child: { Column: { children: [ … ] } } } }
```

Vertical scrolling with the mouse wheel, trackpad and finger (with momentum and bounce), plus a scroll bar. The child is at least as large as the visible area. Nodes outside the visible part don't make it into the frame, so a long page draws as fast as a short one.

## Lists: `For` and `List`

- `For` repeats a template for each array item and expands all items. Good for dozens of rows.
- `List` is a virtualized scroll view: it expands only the visible rows. Good for thousands of rows; all rows have the same `itemHeight`.

Details are in [bindings.md](bindings.md#lists-for-and-list).

## Safe area and keyboard (phone)

By default the screen root sits inside the safe area: nothing hides under the camera cutout or above the home indicator. A screen with `safeArea: false` handles the area itself through `@env.safe.top`… — for example, to fill the background under the system bars. When the on-screen keyboard is open, its height is added to the bottom inset, and the layout shrinks to fit above it.
