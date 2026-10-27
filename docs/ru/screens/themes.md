# Темы

[English](../../screens/themes.md) · **Русский**

Тема — набор **токенов**: цвета, радиусы, размеры шрифта. Элементы библиотеки берут цвета только из токенов (`"$theme.primary"`), поэтому смена темы перекрашивает весь экран. Свои компоненты делают так же.

## Выбор темы

| | как |
|---|---|
| в файле экрана | `theme: "dark"` — `"light"`, `"dark"` или `"system"` (по умолчанию: как в системе) |
| из программы до запуска | C `uison_ui_set_theme(ui, "dark")`, C++ `ui.theme("dark")`, Rust `ui.set_theme("dark")` |
| в обработчике | C `uison_cx_set_theme(cx, "light")`, C++ `cx.theme("light")`, Rust `cx.set_theme("light")` |

С `"system"` экран сам переключается, когда меняется тема системы (macOS, Windows, GNOME, iOS, Android, браузер). Тёмная ли сейчас система — `@env.dark`.

## Свои значения токенов

Объект вместо имени — токены поверх выбранной темы:

```js
theme: { primary: "#FF2D55", radius: 4 }
```

На лету, из обработчика: C `uison_cx_set_token(cx, "primary", "\"#34C759\"")` (значение — JSON), C++ `cx.token("primary", "#34C759")`, Rust `cx.set_token("primary", "#34C759")`.

## Токены

| токен | для чего |
|---|---|
| `primary`, `primaryHover`, `primaryPressed`, `primaryDisabled` | главный цвет: кнопки, флажки, ползунки, ссылки |
| `onPrimary` | текст и иконки на `primary` |
| `danger`, `dangerHover`, `dangerPressed` | опасные действия, ошибки |
| `success`, `warning` | успех, предупреждение |
| `background` | фон окна |
| `card` | карточки, панели |
| `surface`, `surfaceHover`, `surfacePressed` | второстепенные кнопки, поля |
| `selection` | выделенная строка, вариант |
| `textSelection` | выделенный текст в поле |
| `tooltip`, `onTooltip` | подсказка и её текст |
| `text` | основной текст |
| `label` | подписи элементов |
| `muted` | второстепенный текст |
| `border`, `borderHover` | рамки полей |
| `divider` | разделители |
| `track`, `trackHover`, `thumb` | дорожка и ручка `Slider`, `Switch` |
| `tabActive` | активная вкладка `Tabs` |
| `shadow`, `scrollbar` | тень, полоса прокрутки |
| `avatars` | цвета кружков `Avatar` (выбор по имени) |
| `radius`, `radiusSm`, `radiusLg` | скругления |
| `fontSize`, `fontSizeSm`, `fontSizeLg`, `fontSizeTitle` | размеры шрифта |

Значения светлой и тёмной темы — в `crates/uison-ui/lib/theme.json`.
