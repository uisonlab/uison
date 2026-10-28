# Элементы

[English](../../screens/elements.md) · **Русский**

Перевод справочника, который `uison-gen --md` генерирует из схемы экранов (`uison_ui::schema`) и библиотеки `lib/*.json`
(английский — `cargo run -p uison-gen -- --md -o docs/screens/elements.md`); при изменении схемы обновлять и эту страницу.

Свойства — как в JSON. В C++ (`uison_widgets.hpp`) и в Rust DSL имена те же в `snake_case`: `maxLines` → `max_lines`.
Любое свойство можно задать и напрямую: `.prop("maxLines", 2)` (C++ и Rust).

## Общие свойства

Есть у любого узла и компонента.

| свойство | C++ / Rust | что |
|---|---|---|
| `key` | `key` | имя узла: события (cx.key), фокус, hit-области, сценарии проверки |
| `visible` | `visible` | показан ли узел: bool или привязка "@path" / "!@path" |
| `disabled` | `disabled` | недоступен: серый, не нажимается; bool или привязка |
| `w` | `w` | ширина: число (pt), "fill" — всё доступное, "hug" — по содержимому, "50%" |
| `h` | `h` | высота: число, "fill", "hug", "50%" |
| `minW` | `min_w` | наименьший размер, pt |
| `maxW` | `max_w` | наибольший размер, pt |
| `minH` | `min_h` | наименьший размер, pt |
| `maxH` | `max_h` | наибольший размер, pt |
| `flex` | `flex` | доля свободного места в Row/Column (1, 2…) |
| `margin` | `margin` | отступ снаружи: число, [верт, гор] или [верх, право, низ, лево] |
| `place` | `place` | место в Stack: center, top, bottomRight… |
| `pos` | `pos` | положение в Stack: [x, y] |
| `overlay` | `overlay` | поверх соседей и не влияет на размер родителя (всплывающие панели) |
| `on` | `on` | события → действия: { click: "add", change: …, submit: …, hover: … } |
| `cursor` | `cursor` | курсор над узлом: arrow, pointer, text, resize-x, resize-y |
| `tooltip` | `tooltip` | подсказка при наведении (долгом нажатии) |
| `contextMenu` | `context_menu` | меню по правой кнопке: [{ label, action }, "-", …] |
| `alt` | `alt` | текстовая альтернатива для тех, кто не видит экран (агенты, экранные дикторы): что делает кнопка-иконка |

## Поведения

У встроенных узлов (`Box`, `Row`, `Column`, `Stack`, `Wrap`…): из них собраны элементы библиотеки.

| свойство | C++ / Rust | что |
|---|---|---|
| `toggle` | `toggle` | нажатие переключает bool по пути: "@form.agree" |
| `select` | `select` | нажатие записывает значение: { path: "@tab", value: "inbox" } |
| `drag` | `drag` | перетаскивание задаёт число: { path: "@volume", min, max, step }; delta: 1 / -1 — значение при нажатии + смещение (разделитель) |
| `interactive` | `interactive` | состояния hover/pressed для селекторов { state: … } |
| `focusable` | `focusable` | получает фокус клавиатуры (Tab) |
| `dismiss` | `dismiss` | нажатие мимо узла записывает false по пути (закрыть всплывающее) |
| `href` | `href` | ссылка: нажатие открывает адрес в браузере |

## Раскладка

### Box

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `fill` | `fill` | заливка: цвет "#RRGGBB" или токен "$theme.card" |
| `stroke` | `stroke` | цвет рамки |
| `width` | `width` | толщина рамки |
| `radius` | `radius` | скругление углов |
| `clip` | `clip` | обрезать детей по своим границам |
| `align` | `align` | положение ребёнка: center, top, left, bottomRight… |
| `child` | `child` | единственный ребёнок |

События: `click`.

### Row

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `gap` | `gap` | расстояние между детьми |
| `main` | `main` | распределение по главной оси: start, center, end, between, around, evenly |
| `cross` | `cross` | выравнивание по поперечной оси: start, center, end, stretch |
| `stackBelow` | `stack_below` | уже этой ширины — дети столбиком (телефон) |
| `children` | `children` | дети |

События: `click`.

### Column

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `gap` | `gap` | расстояние между детьми |
| `main` | `main` | распределение по главной оси: start, center, end, between, around, evenly |
| `cross` | `cross` | выравнивание по поперечной оси: start, center, end, stretch |
| `children` | `children` | дети |

События: `click`.

### Stack

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `children` | `children` | дети |

События: `click`.

### Center

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `child` | `child` | единственный ребёнок |

События: `click`.

### Align

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `align` | `align` | положение ребёнка: center, top, left, bottomRight… |
| `child` | `child` | единственный ребёнок |

События: `click`.

### Spacer

Своих свойств нет — только общие.

События: `click`.

### Wrap

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `gap` | `gap` | расстояние между детьми |
| `runGap` | `run_gap` | расстояние между строками переноса |
| `cross` | `cross` | выравнивание внутри строки: start, center, end |
| `children` | `children` | дети |

События: `click`.

### Flow

| свойство | C++ / Rust | что |
|---|---|---|
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево] |
| `align` | `align` | выравнивание строк: left, center, right |
| `lineHeight` | `line_height` | межстрочный интервал (множитель) |
| `children` | `children` | дети |

События: `click`.

### Scroll

| свойство | C++ / Rust | что |
|---|---|---|
| `child` | `child` | единственный ребёнок |

События: `click`.

### List

| свойство | C++ / Rust | что |
|---|---|---|
| `items` | `items` | массив данных: "@rows" |
| `as` | `as` | имя элемента внутри шаблона: as: "t" → @t.title |
| `index` | `index` | имя индекса элемента: index: "i" → @i |
| `itemKey` | `item_key` | key строки по данным: "{@m.id}" |
| `itemHeight` | `item_height` | высота строки (виртуализация) |
| `item` | `item` | шаблон строки |

События: `click`.

### For

| свойство | C++ / Rust | что |
|---|---|---|
| `each` | `each` | массив данных: "@todos" |
| `as` | `as` | имя элемента внутри шаблона: as: "t" → @t.title |
| `index` | `index` | имя индекса элемента: index: "i" → @i |
| `do` | `do` | шаблон повтора |

## Текст и ввод

### Text

Главное свойство `text` — первым аргументом: C++ `Text("…", {…})`, Rust `text("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `text` | `text` | текст; {@path} — подстановка данных |
| `size` | `size` | размер шрифта, pt |
| `bold` | `bold` | жирный |
| `color` | `color` | цвет текста |
| `font` | `font` | шрифт: имя или monospace, serif, sans-serif |
| `align` | `align` | выравнивание строк: left, center, right |
| `maxLines` | `max_lines` | не больше строк (дальше — …) |
| `lineHeight` | `line_height` | межстрочный интервал (множитель) |
| `wrap` | `wrap` | false — без переноса строк |
| `decoration` | `decoration` | линия: none, underline, lineThrough |
| `fill` | `fill` | заливка под текстом (выделение), цвет |
| `italic` | `italic` | курсив |

События: `click`.

### Input

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение — привязка "@form.email" |
| `placeholder` | `placeholder` | подсказка в пустом поле |
| `size` | `size` | размер шрифта, pt |
| `bold` | `bold` | жирный |
| `color` | `color` | цвет текста |
| `placeholderColor` | `placeholder_color` | цвет подсказки |
| `caretColor` | `caret_color` | цвет каретки |
| `password` | `password` | скрывать символы |
| `keyboard` | `keyboard` | экранная клавиатура: text, email, number, decimal, phone, url, password |
| `font` | `font` | шрифт: имя или monospace, serif, sans-serif |

События: `click`, `change`, `submit`.

## Графика

### Image

Главное свойство `src` — первым аргументом: C++ `Image("…", {…})`, Rust `image("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `src` | `src` | картинка: путь к файлу, "mem:имя" (из байтов), в браузере — и адрес |
| `fit` | `fit` | вписать: cover, contain, fill |
| `radius` | `radius` | скругление углов картинки |

События: `click`.

### Rect

| свойство | C++ / Rust | что |
|---|---|---|
| `fill` | `fill` | заливка: цвет "#RRGGBB" или токен "$theme.card" |
| `stroke` | `stroke` | цвет рамки |
| `width` | `width` | толщина рамки |
| `radius` | `radius` | скругление углов |
| `at` | `at` | положение: [x, y] |

События: `click`.

### Circle

| свойство | C++ / Rust | что |
|---|---|---|
| `r` | `r` | радиус |
| `fill` | `fill` | заливка: цвет "#RRGGBB" или токен "$theme.card" |
| `stroke` | `stroke` | цвет рамки |
| `width` | `width` | толщина рамки |
| `at` | `at` | положение: [x, y] |

События: `click`.

### Line

| свойство | C++ / Rust | что |
|---|---|---|
| `from` | `from` | начало: [x, y] |
| `to` | `to` | конец: [x, y] |
| `width` | `width` | толщина линии |
| `color` | `color` | цвет линии |
| `cap` | `cap` | концы линии: round, butt |

События: `click`.

### Tri

| свойство | C++ / Rust | что |
|---|---|---|
| `points` | `points` | вершины: [x1, y1, x2, y2, x3, y3] |
| `fill` | `fill` | заливка: цвет "#RRGGBB" или токен "$theme.card" |
| `stroke` | `stroke` | цвет рамки |
| `width` | `width` | толщина рамки |

События: `click`.

### Vector

| свойство | C++ / Rust | что |
|---|---|---|
| `view` | `view` | система координат иконки: [w, h] |
| `color` | `color` | цвет линии |
| `weight` | `weight` | толщина линий иконки |
| `children` | `children` | дети |

События: `click`.

### Canvas

| свойство | C++ / Rust | что |
|---|---|---|
| `draw` | `draw` | функция рисования программы: ui.draw("chart", \|d\| …) — примитивы в d.w |

События: `click`, `hover`.

## Библиотека элементов

Компоненты `lib/*.json`: собраны из встроенных узлов, как и свои компоненты программы.

### Button

Главное свойство `text` — первым аргументом: C++ `Button("…", {…})`, Rust `button("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `text` | `text` | текст; {@path} — подстановка данных; по умолчанию "" |
| `variant` | `variant` | primary \| secondary \| outline \| text \| danger; по умолчанию "primary" |
| `size` | `size` | sm \| md \| lg; по умолчанию "md" |
| `icon` | `icon` | иконка: имя из набора Icon (plus, trash, check…) |
| `fg` | `fg` | Цвет текста и иконки — по варианту и состоянию; по умолчанию вычисляется (вариант, состояние) |

События: `click`.

### IconButton

Главное свойство `icon` — первым аргументом: C++ `IconButton("…", {…})`, Rust `icon_button("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `icon` | `icon` | иконка: имя из набора Icon (plus, trash, check…); по умолчанию "plus" |
| `variant` | `variant` | text \| secondary \| primary; по умолчанию "text" |
| `size` | `size` | sm \| md \| lg; по умолчанию "md" |
| `color` | `color` | цвет; по умолчанию вычисляется (вариант, состояние) |

События: `click`.

### Checkbox

Главное свойство `label` — первым аргументом: C++ `Checkbox("…", {…})`, Rust `checkbox("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `checked` | `checked` | отмечен: привязка "@form.agree"; по умолчанию false |
| `label` | `label` | подпись; по умолчанию "" |

События: `click`, `change`.

### Radio

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение этой кнопки (выбрана, если value группы равно ему); по умолчанию "" |
| `group` | `group` | привязка выбранного значения группы: "@plan" |
| `label` | `label` | подпись; по умолчанию "" |

События: `click`, `change`.

### RadioGroup

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение — привязка "@form.email" |
| `options` | `options` | варианты: [{ value, label }] |
| `direction` | `direction` | расположение: column или row; по умолчанию "column" |

События: `click`, `change`.

### Switch

Главное свойство `label` — первым аргументом: C++ `Switch("…", {…})`, Rust `switch("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `checked` | `checked` | отмечен: привязка "@form.agree"; по умолчанию false |
| `label` | `label` | подпись; по умолчанию "" |

События: `click`, `change`.

### Slider

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение: число или привязка "@volume"; по умолчанию 0 |
| `min` | `min` | наименьшее значение; по умолчанию 0 |
| `max` | `max` | наибольшее значение; по умолчанию 100 |
| `step` | `step` | шаг (0 — плавно); по умолчанию 0 |

События: `click`, `change`.

### Select

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение — привязка "@form.email" |
| `options` | `options` | варианты: [{ value, label }] |
| `placeholder` | `placeholder` | подсказка в пустом поле; по умолчанию "Select…" |
| `label` | `label` | подпись |

События: `click`, `change`.

### Tabs

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение — привязка "@form.email" |
| `tabs` | `tabs` | вкладки: [{ value, label }] |

События: `click`, `change`.

### Tab

| свойство | C++ / Rust | что |
|---|---|---|
| `label` | `label` | подпись; по умолчанию "" |
| `value` | `value` | значение вкладки; по умолчанию "" |
| `group` | `group` | привязка выбранного значения группы: "@plan" |

События: `click`.

### TextField

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение — привязка "@form.email" |
| `label` | `label` | подпись |
| `placeholder` | `placeholder` | подсказка в пустом поле; по умолчанию "" |
| `error` | `error` | текст ошибки под полем (null — нет ошибки) |
| `password` | `password` | скрывать символы; по умолчанию false |
| `keyboard` | `keyboard` | экранная клавиатура: text, email, number, decimal, phone, url, password; по умолчанию "text" |

События: `click`, `change`, `submit`.

### ProgressBar

| свойство | C++ / Rust | что |
|---|---|---|
| `value` | `value` | значение: число или привязка "@volume"; по умолчанию 0 |
| `color` | `color` | цвет; по умолчанию "$theme.primary" |

События: `click`.

### Divider

| свойство | C++ / Rust | что |
|---|---|---|
| `vertical` | `vertical` | вертикальный; по умолчанию false |

События: `click`.

### Card

| свойство | C++ / Rust | что |
|---|---|---|
| `title` | `title` | заголовок |
| `padding` | `padding` | отступ внутри: число, [верт, гор] или [верх, право, низ, лево]; по умолчанию 16 |
| `gap` | `gap` | расстояние между детьми; по умолчанию 12 |
| `children` | `children` | дети |

События: `click`.

### ListTile

Главное свойство `title` — первым аргументом: C++ `ListTile("…", {…})`, Rust `list_tile("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `title` | `title` | заголовок; по умолчанию "" |
| `subtitle` | `subtitle` | строка под заголовком |
| `leading` | `leading` | узел слева (иконка, аватар) |
| `trailing` | `trailing` | узел справа |
| `selected` | `selected` | выделен; по умолчанию false |

События: `click`.

### Badge

Главное свойство `text` — первым аргументом: C++ `Badge("…", {…})`, Rust `badge("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `text` | `text` | число или короткий текст; по умолчанию "" |
| `color` | `color` | цвет; по умолчанию "$theme.danger" |
| `textColor` | `text_color` | цвет текста; по умолчанию "#FFFFFF" |

События: `click`.

### Avatar

Главное свойство `name` — первым аргументом: C++ `Avatar("…", {…})`, Rust `avatar("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `name` | `name` | имя: инициалы и цвет кружка; по умолчанию "" |
| `size` | `size` | размер; по умолчанию 32 |
| `color` | `color` | цвет |
| `src` | `src` | фото вместо инициалов |

События: `click`.

### Section

| свойство | C++ / Rust | что |
|---|---|---|
| `title` | `title` | заголовок; по умолчанию "" |
| `gap` | `gap` | расстояние между детьми; по умолчанию 10 |
| `children` | `children` | дети |

События: `click`.

### Link

Главное свойство `text` — первым аргументом: C++ `Link("…", {…})`, Rust `link("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `text` | `text` | текст; {@path} — подстановка данных; по умолчанию "" |

События: `click`.

### Icon

Главное свойство `name` — первым аргументом: C++ `Icon("…", {…})`, Rust `icon("…")`.

| свойство | C++ / Rust | что |
|---|---|---|
| `name` | `name` | иконка из набора (список ниже); по умолчанию "" |
| `size` | `size` | размер; по умолчанию 16 |
| `color` | `color` | цвет; по умолчанию "$theme.text" |
| `weight` | `weight` | толщина линий; по умолчанию 1.75 |

Иконки: `check`, `close`, `plus`, `minus`, `chevronDown`, `chevronUp`, `chevronLeft`, `chevronRight`, `arrowLeft`, `arrowRight`, `arrowUp`, `arrowDown`, `search`, `menu`, `more`, `info`, `warning`, `star`, `trash`, `edit`, `settings`, `home`, `mail`, `calendar`, `user`.

События: `click`.
