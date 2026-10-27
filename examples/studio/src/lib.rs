//! uison Studio — a demo app using every feature of the element library.
//!
//! The screen is `app.json` + `components.json` (complex JSON, no code).
//! Here: data, derived values (derive), handlers, timers,
//! frame-by-frame animation, keyboard shortcuts, UI translation through data.
//! The same code runs natively and in the browser (see main.rs).

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use uison::time::now;

use uison::ui::{Cx, Data, IntoJson, Json, Ui, obj};

// ================================================================ data

const PEOPLE: [(&str, &str, &str); 14] = [
    ("Anna Peterson", "Designer", "Kyiv"),
    ("Ben Irving", "Backend developer", "Warsaw"),
    ("Clara Smith", "Product manager", "Berlin"),
    ("David Owens", "iOS developer", "Lviv"),
    ("Emma Cole", "QA engineer", "Krakow"),
    ("Frank White", "Analyst", "Prague"),
    ("Grace Fox", "Marketer", "Vilnius"),
    ("Henry Kim", "DevOps", "Riga"),
    ("Iris Shaw", "Frontend developer", "Odesa"),
    ("Jack Newman", "Tech lead", "Tallinn"),
    ("Laura Moss", "UX researcher", "Barcelona"),
    ("Max Wolf", "Android developer", "Madrid"),
    ("Nina Hayes", "Support", "Vienna"),
    ("Oliver Ford", "CTO", "Lisbon"),
];

const SUBJECTS: [&str; 10] = [
    "Homepage mockups",
    "Release call",
    "Weekly report",
    "List rendering bug",
    "New icons",
    "Sprint plans",
    "Renderer architecture review",
    "Question about the API",
    "Demo invitation",
    "Retro takeaways",
];

const BODIES: [&str; 5] = [
    "Hi! Could you take a look at the updated mockups? I fixed the padding in the cards and added a dark theme. If it all looks good, I'll hand it over to development tonight.",
    "Hi all, a reminder: the release call is at 3 pm. We'll go through the task list, check the testing status and decide what moves to the next version.",
    "Found a bug: when you scroll a long list quickly, the bottom row sometimes isn't redrawn. Looks like a mistake in the damage region calculation; I'll fix it now and send a patch.",
    "I collected the stats for the week: UI response time stays under a millisecond and the binary size hasn't grown. Details are in the attached spreadsheet.",
    "Thanks for the review! I suggest we keep the text frame format: parsing takes less than five percent of the time. Let's discuss the rest at the meeting.",
];

fn email(name: &str) -> String {
    const LAT: [&str; 14] = ["anna", "ben", "clara", "david", "emma", "frank", "grace", "henry", "iris", "jack", "laura", "max", "nina", "oliver"];
    let i = PEOPLE.iter().position(|p| p.0 == name).unwrap_or(0);
    format!("{}@studio.dev", LAT[i])
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn f(&mut self) -> f64 {
        (self.next() % 10_000) as f64 / 10_000.0
    }
}

/// The first four have photos (`ui.image` in `app`), the rest show initials.
const PHOTOS: [(&str, &[u8]); 4] = [
    ("anna", include_bytes!("../../gallery/images/anna.jpg")),
    ("boris", include_bytes!("../../gallery/images/boris.jpg")),
    ("vera", include_bytes!("../../gallery/images/vera.jpg")),
    ("gleb", include_bytes!("../../gallery/images/gleb.jpg")),
];

fn photo(name: &str) -> Json {
    match PEOPLE.iter().position(|p| p.0 == name).and_then(|i| PHOTOS.get(i)) {
        Some((key, _)) => format!("mem:{key}").into_json(),
        None => Json::Null,
    }
}

fn messages(n: usize) -> Json {
    let mut r = Rng(0x5EED_1234_ABCD);
    let list = (0..n)
        .map(|i| {
            let from = PEOPLE[r.below(PEOPLE.len() as u64) as usize].0;
            let body = BODIES[r.below(BODIES.len() as u64) as usize];
            let minutes = i * 37;
            let time = if i < 20 { format!("{:02}:{:02}", 18 - (minutes / 60) % 10, 59 - minutes % 60) } else { format!("Oct {}", 24 - (i / 80) % 24) };
            obj([
                ("from", from.into_json()),
                ("photo", photo(from)),
                ("subject", SUBJECTS[r.below(SUBJECTS.len() as u64) as usize].into_json()),
                ("body", body.into_json()),
                ("preview", body.chars().take(90).collect::<String>().into_json()),
                ("time", time.into_json()),
                ("unread", (i < 9 || r.below(9) == 0).into_json()),
                ("important", (r.below(7) == 0).into_json()),
                ("thread", Json::Array(Vec::new())),
            ])
        })
        .collect();
    Json::Array(list)
}

fn task(id: i64, title: &str, prio: &str, status: &str, due: &str, who: &str) -> Json {
    obj([
        ("id", id.into_json()),
        ("title", title.into_json()),
        ("prio", prio.into_json()),
        ("status", status.into_json()),
        ("due", due.into_json()),
        ("assignee", who.into_json()),
    ])
}

fn tasks() -> Json {
    Json::Array(vec![
        task(1, "Build the complex JSON compiler", "high", "done", "Oct 20", "Jack Newman"),
        task(2, "Draw icons from lines", "mid", "done", "Oct 21", "Anna Peterson"),
        task(3, "Dropdown list and overlay", "high", "doing", "Oct 26", "Iris Shaw"),
        task(4, "Text field with a caret", "mid", "doing", "Oct 27", "David Owens"),
        task(5, "Frame-by-frame chart animation", "low", "todo", "Oct 30", "Frank White"),
        task(6, "Dialogs and tooltips", "mid", "todo", "Nov 2", "Clara Smith"),
        task(7, "Windows backend", "low", "todo", "Nov 15", "Henry Kim"),
        task(8, "Partial redraw tests", "high", "done", "Oct 18", "Emma Cole"),
    ])
}

fn contacts() -> Json {
    Json::Array(
        PEOPLE
            .iter()
            .enumerate()
            .map(|(i, (name, role, city))| {
                obj([
                    ("id", i.into_json()),
                    ("name", name.into_json()),
                    ("photo", photo(name)),
                    ("role", role.into_json()),
                    ("city", city.into_json()),
                    ("online", (i % 3 != 1).into_json()),
                ])
            })
            .collect(),
    )
}

fn people_options() -> Json {
    Json::Array(PEOPLE.iter().map(|(n, ..)| obj([("value", n.into_json()), ("label", n.into_json())])).collect())
}

fn accents() -> Json {
    let a = [("blue", "#007AFF"), ("purple", "#AF52DE"), ("pink", "#FF2D55"), ("orange", "#FF9500"), ("green", "#34C759"), ("graphite", "#8E8E93")];
    Json::Array(a.iter().map(|(id, c)| obj([("id", id.into_json()), ("color", c.into_json())])).collect())
}

/// `m` is the platform's main modifier (`@env.mod`: "⌘" or "Ctrl+").
fn shortcuts(lang: &str, m: &str) -> Json {
    let keys = [format!("{m}1…{m}6"), format!("{m}N"), format!("{m}F"), format!("{m}R"), format!("{m}D"), "Tab".into(), "Esc".into()];
    let ru = ["Разделы", "Новая задача", "Поиск по почте", "Обновить статистику", "Светлая / тёмная тема", "Следующее поле", "Закрыть окно или список"];
    let en = ["Sections", "New task", "Search mail", "Refresh analytics", "Light / dark theme", "Next field", "Close dialog or list"];
    let text = if lang == "en" { en } else { ru };
    Json::Array(keys.iter().zip(text).map(|(k, t)| obj([("keys", k.into_json()), ("text", t.into_json())])).collect())
}

/// UI translation is plain data: templates read `@t.*`.
/// Russian stays as the second language: switching it in Settings demonstrates translation through data.
fn dictionary(lang: &str) -> Json {
    let ru = [
        ("tagline", "демо библиотеки uison"),
        ("inbox", "Входящие"), ("tasks", "Задачи"), ("contacts", "Контакты"), ("stats", "Статистика"), ("form", "Форма"), ("settings", "Настройки"),
        ("undo", "Отменить"), ("unreadOf", "непрочитанных из"), ("searchMail", "Поиск по письмам"),
        ("all", "Все"), ("unread", "Новые"), ("important", "Важные"), ("nothingFound", "Ничего не найдено"),
        ("reply", "Ответить…"), ("send", "Отправить"), ("pickMessage", "Выберите письмо слева"),
        ("doneOf", "готово из"), ("newTask", "Новая задача"), ("empty", "Пусто"),
        ("high", "Срочно"), ("mid", "Обычно"), ("low", "Потом"),
        ("title", "Название"), ("whatToDo", "Что нужно сделать?"), ("priority", "Приоритет"), ("assignee", "Исполнитель"), ("column", "КОЛОНКА"),
        ("todo", "К выполнению"), ("doing", "В работе"), ("doneCol", "Готово"), ("cancel", "Отмена"), ("create", "Создать"),
        ("onlineOf", "в сети из"), ("online", "В сети"), ("searchPeople", "Имя, роль или город"), ("write", "Написать"),
        ("statsHint", "Графики — линии и прямоугольники; «Обновить» анимирует их кадрами"),
        ("week", "Неделя"), ("month", "Месяц"), ("year", "Год"), ("refresh", "Обновить"),
        ("revenue", "Выручка"), ("users", "Активные пользователи"), ("byStatus", "Задачи по статусам"), ("events", "Последние события"), ("vsPrev", "к прошлому периоду"),
        ("formHint", "Проверка полей, пароль, зависимые элементы"), ("account", "Аккаунт"), ("name", "Имя"), ("password", "Пароль"), ("strength", "Надёжность"),
        ("preferences", "Предпочтения"), ("country", "Страна"), ("choose", "Выберите…"), ("age", "Возраст"), ("plan", "ТАРИФ"),
        ("planFree", "Бесплатный"), ("planPro", "Про"), ("planTeam", "Команда"), ("newsletter", "Получать новости"), ("terms", "Принимаю условия"),
        ("clear", "Очистить"), ("submit", "Зарегистрироваться"),
        ("settingsHint", "Всё меняется на лету: тема, акцент, размер текста, язык"), ("appearance", "Оформление"), ("system", "Авто"), ("light", "Светлая"), ("dark", "Тёмная"),
        ("accent", "Цвет акцента"), ("fontSize", "Размер текста"), ("mailList", "Список писем"), ("compact", "Компактный режим"), ("preview", "Показывать начало письма"),
        ("language", "Язык"), ("shortcuts", "Горячие клавиши"), ("about", "О приложении"),
        ("aboutText", "Весь интерфейс — сложный JSON, скомпилированный в примитивы: прямоугольники, линии, круги, треугольники и текст. Кадры сравниваются, и перерисовывается только изменившееся."),
        ("showToast", "Показать уведомление"), ("reset", "Сбросить настройки"),
        ("menuReply", "Ответить"), ("menuForward", "Переслать"), ("menuStar", "Важное"), ("menuDelete", "Удалить"),
        ("tipCompose", "Новое письмо"), ("tipStar", "Отметить как важное"), ("tipDelete", "Удалить письмо"), ("tipTheme", "Светлая или тёмная тема"),
    ];
    let en = [
        ("tagline", "uison library demo"),
        ("inbox", "Inbox"), ("tasks", "Tasks"), ("contacts", "Contacts"), ("stats", "Analytics"), ("form", "Form"), ("settings", "Settings"),
        ("undo", "Undo"), ("unreadOf", "unread of"), ("searchMail", "Search mail"),
        ("all", "All"), ("unread", "Unread"), ("important", "Starred"), ("nothingFound", "Nothing found"),
        ("reply", "Reply…"), ("send", "Send"), ("pickMessage", "Pick a message on the left"),
        ("doneOf", "done of"), ("newTask", "New task"), ("empty", "Empty"),
        ("high", "Urgent"), ("mid", "Normal"), ("low", "Later"),
        ("title", "Title"), ("whatToDo", "What needs to be done?"), ("priority", "Priority"), ("assignee", "Assignee"), ("column", "COLUMN"),
        ("todo", "To do"), ("doing", "In progress"), ("doneCol", "Done"), ("cancel", "Cancel"), ("create", "Create"),
        ("onlineOf", "online of"), ("online", "Online"), ("searchPeople", "Name, role or city"), ("write", "Message"),
        ("statsHint", "Charts are lines and rectangles; Refresh animates them frame by frame"),
        ("week", "Week"), ("month", "Month"), ("year", "Year"), ("refresh", "Refresh"),
        ("revenue", "Revenue"), ("users", "Active users"), ("byStatus", "Tasks by status"), ("events", "Recent events"), ("vsPrev", "vs previous period"),
        ("formHint", "Validation, password, dependent controls"), ("account", "Account"), ("name", "Name"), ("password", "Password"), ("strength", "Strength"),
        ("preferences", "Preferences"), ("country", "Country"), ("choose", "Choose…"), ("age", "Age"), ("plan", "PLAN"),
        ("planFree", "Free"), ("planPro", "Pro"), ("planTeam", "Team"), ("newsletter", "Send me news"), ("terms", "I accept the terms"),
        ("clear", "Clear"), ("submit", "Sign up"),
        ("settingsHint", "Everything changes live: theme, accent, text size, language"), ("appearance", "Appearance"), ("system", "Auto"), ("light", "Light"), ("dark", "Dark"),
        ("accent", "Accent color"), ("fontSize", "Text size"), ("mailList", "Mail list"), ("compact", "Compact mode"), ("preview", "Show message preview"),
        ("language", "Language"), ("shortcuts", "Keyboard shortcuts"), ("about", "About"),
        ("aboutText", "The whole UI is complex JSON compiled into primitives: rectangles, lines, circles, triangles and text. Frames are diffed and only what changed is repainted."),
        ("showToast", "Show a notification"), ("reset", "Reset settings"),
        ("menuReply", "Reply"), ("menuForward", "Forward"), ("menuStar", "Starred"), ("menuDelete", "Delete"),
        ("tipCompose", "New message"), ("tipStar", "Mark as starred"), ("tipDelete", "Delete message"), ("tipTheme", "Light or dark theme"),
    ];
    let pairs: &[(&str, &str)] = if lang == "en" { &en } else { &ru };
    Json::Object(pairs.iter().map(|(k, v)| (std::borrow::Cow::Owned(k.to_string()), v.into_json())).collect())
}

fn initial_data(d: &mut Data) {
    d.set("nav", "inbox");
    d.set("me", obj([("name", "Oliver Ford".into_json()), ("email", "oliver@studio.dev".into_json())]));
    d.set(
        "settings",
        obj([
            ("theme", "system".into_json()),
            ("accent", "blue".into_json()),
            ("fontSize", 13.into_json()),
            ("compact", false.into_json()),
            ("preview", true.into_json()),
            ("lang", "en".into_json()),
        ]),
    );
    d.set("accents", accents());
    d.set("people", people_options());
    d.set("inbox.messages", messages(2000));
    d.set("inbox.q", "");
    d.set("inbox.filter", "all");
    d.set("inbox.sel", Json::Null);
    d.set("inbox.reply", "");
    d.set("inbox.rev", 0);
    d.set("tasks", tasks());
    d.set("nextTask", 100);
    d.set("contacts.all", contacts());
    d.set("contacts.q", "");
    d.set("contacts.filter", "all");
    d.set("modal", obj([("open", false.into_json())]));
    d.set(
        "form",
        obj([
            ("name", "".into_json()),
            ("email", "".into_json()),
            ("password", "".into_json()),
            ("country", Json::Null),
            ("age", 30.into_json()),
            ("plan", "pro".into_json()),
            ("newsletter", true.into_json()),
            ("terms", false.into_json()),
            ("tried", false.into_json()),
        ]),
    );
    d.set("toast", obj([("text", "".into_json()), ("undo", false.into_json())]));
    d.set("stats.period", "month");
    let events = [
        ("check", "Emma closed \"Partial redraw tests\"", "5 minutes ago"),
        ("mail", "Anna sent the homepage mockups", "20 minutes ago"),
        ("user", "Laura joined the project", "1 hour ago"),
        ("star", "Release 0.2: element library", "yesterday"),
        ("warning", "Windows build failed", "yesterday"),
    ];
    d.set(
        "stats.events",
        Json::Array(events.iter().map(|(i, t, w)| obj([("icon", i.into_json()), ("text", t.into_json()), ("time", w.into_json())])).collect()),
    );
}

// ================================================================ derived data

fn arr(d: &Data, path: &str) -> Vec<Json> {
    d.get(path).and_then(Json::as_array).map(<[Json]>::to_vec).unwrap_or_default()
}

fn s<'a>(v: &'a Json, k: &str) -> &'a str {
    v.get(k).and_then(Json::as_str).unwrap_or("")
}

fn truthy(v: &Json, k: &str) -> bool {
    v.get(k).is_some_and(Json::truthy)
}

/// Mail filter: 2000 items, recomputed only when the inputs change.
fn derive_inbox(d: &mut Data, last: &mut String) {
    let sel = d.get("inbox.sel").and_then(Json::as_f64).map(|v| v as usize);
    let sig = format!("{}|{}|{}|{}", d.str("inbox.q"), d.str("inbox.filter"), d.num("inbox.rev"), d.len("inbox.messages"));
    if *last != sig {
        *last = sig;
        let q = d.str("inbox.q").to_lowercase();
        let filter = d.str("inbox.filter").to_string();
        // By reference: copying 2000 messages with their texts just to filter is expensive.
        let msgs = d.get("inbox.messages").and_then(Json::as_array).unwrap_or(&[]);
        let view: Vec<Json> = msgs
            .iter()
            .enumerate()
            .filter(|(_, m)| match filter.as_str() {
                "unread" => truthy(m, "unread"),
                "important" => truthy(m, "important"),
                _ => true,
            })
            .filter(|(_, m)| q.is_empty() || [s(m, "from"), s(m, "subject"), s(m, "body")].iter().any(|t| t.to_lowercase().contains(&q)))
            .map(|(i, m)| {
                obj([
                    ("idx", i.into_json()),
                    ("from", s(m, "from").into_json()),
                    ("photo", m.get("photo").cloned().unwrap_or(Json::Null)),
                    ("subject", s(m, "subject").into_json()),
                    ("preview", s(m, "preview").into_json()),
                    ("time", s(m, "time").into_json()),
                    ("unread", truthy(m, "unread").into_json()),
                    ("important", truthy(m, "important").into_json()),
                ])
            })
            .collect();
        let (unread, total) = (msgs.iter().filter(|m| truthy(m, "unread")).count(), msgs.len());
        d.set("inbox.unread", unread);
        d.set("inbox.total", total);
        d.set("inbox.view", view);
    }
    let current = sel.and_then(|i| d.get(&format!("inbox.messages.{i}")).cloned());
    match current {
        Some(mut m) => {
            let e = email(s(&m, "from"));
            m.set("email", e);
            d.set("inbox.current", m);
        }
        None => d.set("inbox.current", Json::Null),
    }
}

fn derive_board(d: &mut Data) {
    let tasks = arr(d, "tasks");
    let t = d.get("t").cloned().unwrap_or(Json::Null);
    let cols = [("todo", "#8E8E93"), ("doing", "#FF9500"), ("done", "#34C759")];
    let total = tasks.len().max(1);
    let columns: Vec<Json> = cols
        .iter()
        .enumerate()
        .map(|(i, (id, color))| {
            let items: Vec<Json> = tasks.iter().filter(|x| s(x, "status") == *id).cloned().collect();
            let title = s(&t, if *id == "done" { "doneCol" } else { id }).to_string();
            obj([
                ("id", id.into_json()),
                ("title", title.into_json()),
                ("color", color.into_json()),
                ("count", items.len().into_json()),
                ("share", (items.len() as f64 / total as f64).into_json()),
                ("first", (i == 0).into_json()),
                ("last", (i == cols.len() - 1).into_json()),
                ("items", Json::Array(items)),
            ])
        })
        .collect();
    let done = tasks.iter().filter(|x| s(x, "status") == "done").count();
    d.set("board.columns", columns);
    d.set("board.total", tasks.len());
    d.set("board.done", done);
    d.set("board.open", tasks.len() - done);
    d.set("board.progress", done as f64 / total as f64);
    d.set("board.percent", (100 * done / total) as i64);
}

fn derive_contacts(d: &mut Data) {
    let all = arr(d, "contacts.all");
    let q = d.str("contacts.q").to_lowercase();
    let online_only = d.str("contacts.filter") == "online";
    let view: Vec<Json> = all
        .iter()
        .filter(|c| !online_only || truthy(c, "online"))
        .filter(|c| q.is_empty() || [s(c, "name"), s(c, "role"), s(c, "city")].iter().any(|t| t.to_lowercase().contains(&q)))
        .cloned()
        .collect();
    d.set("contacts.online", all.iter().filter(|c| truthy(c, "online")).count());
    d.set("contacts.total", all.len());
    d.set("contacts.view", view);
}

fn money(v: f64) -> String {
    let n = v.round() as i64;
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202F}');
        }
        out.push(ch);
    }
    out
}

/// Charts: bar heights, segments and points from the current (animated) values.
fn derive_stats(d: &mut Data) {
    let bars = arr(d, "stats.cur.bars");
    let labels = arr(d, "stats.labels");
    let max = bars.iter().filter_map(Json::as_f64).fold(1.0, f64::max);
    let hi = bars.iter().filter_map(Json::as_f64).enumerate().fold((0, 0.0), |b, (i, v)| if v > b.1 { (i, v) } else { b }).0;
    let bars_json: Vec<Json> = bars
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let v = v.as_f64().unwrap_or(0.0);
            obj([
                ("label", labels.get(i).cloned().unwrap_or(Json::Null)),
                ("h", (4.0 + 150.0 * v / max).into_json()),
                ("text", format!("{:.0}", v).into_json()),
                ("hi", (i == hi).into_json()),
            ])
        })
        .collect();
    d.set("stats.bars", bars_json);

    // Line chart in a 480×190 area: margins 30 left, 10 top, 20 bottom.
    let series: Vec<f64> = arr(d, "stats.cur.series").iter().filter_map(Json::as_f64).collect();
    let (x0, x1, y0, y1) = (34.0, 472.0, 12.0, 176.0);
    let top = (series.iter().copied().fold(0.0, f64::max) / 100.0).ceil() * 100.0 + 100.0;
    let pt = |i: usize, v: f64| {
        let n = (series.len().max(2) - 1) as f64;
        (x0 + (x1 - x0) * i as f64 / n, y1 - (y1 - y0) * v / top)
    };
    let pts: Vec<(f64, f64)> = series.iter().enumerate().map(|(i, v)| pt(i, *v)).collect();
    let segs: Vec<Json> = pts
        .windows(2)
        .map(|w| obj([("x1", w[0].0.into_json()), ("y1", w[0].1.into_json()), ("x2", w[1].0.into_json()), ("y2", w[1].1.into_json())]))
        .collect();
    let points: Vec<Json> = pts.iter().map(|(x, y)| obj([("x", (x - 4.0).into_json()), ("y", (y - 4.0).into_json())])).collect();
    let grid: Vec<Json> = (0..4)
        .map(|k| {
            let v = top * k as f64 / 3.0;
            let y = y1 - (y1 - y0) * v / top;
            obj([("y", y.into_json()), ("ty", (y - 13.0).into_json()), ("label", money(v).into_json())])
        })
        .collect();
    d.set("stats.segs", segs);
    d.set("stats.points", points);
    d.set("stats.grid", grid);

    let k = arr(d, "stats.cur.kpi");
    let kv = |i: usize| k.get(i).and_then(Json::as_f64).unwrap_or(0.0);
    let t = d.get("t").cloned().unwrap_or(Json::Null);
    let kpis = vec![
        obj([("id", "rev".into_json()), ("icon", "star".into_json()), ("label", s(&t, "revenue").into_json()), ("value", format!("${}", money(kv(0))).into_json()), ("delta", "+12%".into_json()), ("up", true.into_json())]),
        obj([("id", "users".into_json()), ("icon", "user".into_json()), ("label", s(&t, "users").into_json()), ("value", money(kv(1)).into_json()), ("delta", "+4%".into_json()), ("up", true.into_json())]),
        obj([("id", "conv".into_json()), ("icon", "arrowUp".into_json()), ("label", "Conversion".into_json()), ("value", format!("{:.1}%", kv(2)).into_json()), ("delta", "−0.3%".into_json()), ("up", false.into_json())]),
        obj([("id", "check".into_json()), ("icon", "calendar".into_json()), ("label", "Average order".into_json()), ("value", format!("${:.0}", kv(3)).into_json()), ("delta", "+2%".into_json()), ("up", true.into_json())]),
    ];
    d.set("stats.kpis", kpis);
}

pub fn derive_form(d: &mut Data) {
    let pass = d.str("form.password").to_string();
    let mut score = 0;
    score += (pass.chars().count() >= 8) as i32;
    score += pass.chars().any(|c| c.is_ascii_digit()) as i32;
    score += pass.chars().any(|c| c.is_uppercase()) as i32;
    score += pass.chars().any(|c| !c.is_alphanumeric()) as i32;
    d.set("form.strength", if pass.is_empty() { 0.0 } else { (score as f64 + 1.0) / 5.0 });
    d.set("form.strengthLevel", if score <= 1 { 0 } else if score <= 2 { 1 } else { 2 });
    if !d.bool("form.tried") {
        return;
    }
    let email = d.str("form.email").to_string();
    let ok_email = email.split_once('@').is_some_and(|(a, b)| !a.is_empty() && b.contains('.'));
    d.set("form.errors.name", if d.str("form.name").trim().is_empty() { "Enter your name" } else { "" });
    d.set("form.errors.email", if ok_email { "" } else { "Check the address" });
    d.set("form.errors.password", if pass.chars().count() < 8 { "At least 8 characters" } else { "" });
}

fn derive_all(d: &mut Data, inbox_sig: &mut String) {
    let lang = d.str("settings.lang").to_string();
    if d.str("t.lang") != lang {
        let mut t = dictionary(&lang);
        t.set("lang", lang.clone());
        d.set("t", t);
        let m = d.str("env.mod").to_string();
        d.set("shortcuts", shortcuts(&lang, &m));
    }
    let compact = d.bool("settings.compact");
    d.set("ui.rowH", if compact { 46 } else if d.bool("settings.preview") { 66 } else { 52 });
    derive_inbox(d, inbox_sig);
    derive_board(d);
    derive_contacts(d);
    derive_stats(d);
    derive_form(d);
}

// ================================================================ accent colors

fn hex(c: &str) -> (f64, f64, f64) {
    let v = u32::from_str_radix(c.trim_start_matches('#'), 16).unwrap_or(0);
    ((v >> 16 & 255) as f64, (v >> 8 & 255) as f64, (v & 255) as f64)
}

fn mix(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (hex(a), hex(b));
    let m = |x: f64, y: f64| (x + (y - x) * t).round() as u8;
    format!("#{:02X}{:02X}{:02X}", m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// The effective theme is dark: dark is chosen, or "system" while the system is dark.
fn is_dark(d: &Data) -> bool {
    match d.str("settings.theme") {
        "system" => d.bool("env.dark"),
        t => t == "dark",
    }
}

/// Theme tokens from the accent color, different for light and dark themes.
fn apply_accent(cx: &mut Cx) {
    let id = cx.data.str("settings.accent").to_string();
    let color = arr(cx.data, "accents").iter().find(|a| s(a, "id") == id).map(|a| s(a, "color").to_string()).unwrap_or("#007AFF".into());
    let dark = is_dark(cx.data);
    let bg = if dark { "#2C2C2E" } else { "#FFFFFF" };
    cx.set_token("primary", color.as_str());
    cx.set_token("primaryHover", mix(&color, "#000000", 0.08));
    cx.set_token("primaryPressed", mix(&color, "#000000", 0.2));
    cx.set_token("primaryDisabled", mix(&color, bg, 0.6));
    cx.set_token("selection", mix(&color, bg, if dark { 0.72 } else { 0.88 }));
}

// ================================================================ app

#[derive(Default)]
struct Shared {
    /// What was deleted, for Undo: array path, index, value.
    undo: Option<(String, usize, Json)>,
    toast_until: Option<Duration>,
    anim: Option<Anim>,
    rng: u64,
}

struct Anim {
    start: Duration,
    from: Json,
    to: Json,
}

const ANIM: Duration = Duration::from_millis(450);

fn toast(cx: &mut Cx, sh: &RefCell<Shared>, text: &str, undo: bool) {
    cx.data.set("toast.text", text);
    cx.data.set("toast.undo", undo);
    sh.borrow_mut().toast_until = Some(now() + Duration::from_millis(2600));
    cx.timer(2650, "hideToast");
}

fn stats_targets(period: &str, rng: &mut Rng) -> (Json, Json) {
    let labels: Vec<&str> = match period {
        "week" => vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
        "year" => vec!["2019", "2020", "2021", "2022", "2023", "2024", "2025"],
        _ => vec!["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
    };
    let scale = match period {
        "week" => 40.0,
        "year" => 900.0,
        _ => 120.0,
    };
    let bars: Vec<f64> = labels.iter().map(|_| scale * (0.35 + rng.f() * 0.65)).collect();
    let mut v = 300.0 + rng.f() * 300.0;
    let series: Vec<f64> = (0..12)
        .map(|_| {
            v = (v + (rng.f() - 0.42) * 160.0).clamp(80.0, 1100.0);
            v
        })
        .collect();
    let kpi = vec![scale * 1000.0 * (0.8 + rng.f()), 2000.0 + rng.f() * 9000.0, 1.5 + rng.f() * 4.0, 30.0 + rng.f() * 90.0];
    let cur = obj([("bars", bars.into_json()), ("series", series.into_json()), ("kpi", kpi.into_json())]);
    (Json::Array(labels.iter().map(|l| l.into_json()).collect()), cur)
}

/// Linear interpolation of numeric arrays (if lengths differ, the target wins).
fn lerp(from: &Json, to: &Json, t: f64) -> Json {
    match (from, to) {
        (Json::Number(a), Json::Number(b)) => Json::Number(a + (b - a) * t),
        (Json::Array(a), Json::Array(b)) => Json::Array(
            b.iter().enumerate().map(|(i, y)| a.get(i).map_or_else(|| y.clone(), |x| lerp(x, y, t))).collect(),
        ),
        (Json::Object(a), Json::Object(_)) => {
            let mut out = to.clone();
            for (k, x) in a {
                if let Some(y) = to.get(k) {
                    out.set(k, lerp(x, y, t));
                }
            }
            out
        }
        _ => to.clone(),
    }
}

fn star_message(cx: &mut Cx, sh: &RefCell<Shared>, i: usize) {
    let p = format!("inbox.messages.{i}.important");
    let v = !cx.data.bool(&p);
    cx.data.set(&p, v);
    cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
    toast(cx, sh, if v { "Marked as starred" } else { "Star removed" }, false);
}

fn delete_message(cx: &mut Cx, sh: &RefCell<Shared>, i: usize) {
    if let Some(m) = cx.data.remove("inbox.messages", i) {
        sh.borrow_mut().undo = Some(("inbox.messages".into(), i, m));
    }
    // The open message was deleted: close it; another one: shift the selection.
    match cx.data.get("inbox.sel").and_then(Json::as_f64).map(|v| v as usize) {
        Some(s) if s == i => cx.data.set("inbox.sel", Json::Null),
        Some(s) if s > i => cx.data.set("inbox.sel", s - 1),
        _ => {}
    }
    cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
    toast(cx, sh, "Message deleted", true);
}

fn key_id(key: &str, prefix: &str) -> Option<i64> {
    key.strip_prefix(prefix)?.parse().ok()
}

fn move_task(cx: &mut Cx, id: i64, dir: i32) {
    let order = ["todo", "doing", "done"];
    let n = cx.data.len("tasks");
    for i in 0..n {
        let path = format!("tasks.{i}");
        if cx.data.get(&format!("{path}.id")).and_then(Json::as_f64) == Some(id as f64) {
            let st = cx.data.str(&format!("{path}.status")).to_string();
            let pos = order.iter().position(|s| *s == st).unwrap_or(0) as i32 + dir;
            if let Some(next) = order.get(pos.clamp(0, 2) as usize) {
                cx.data.set(&format!("{path}.status"), *next);
            }
        }
    }
}

pub fn app() -> Ui {
    // UISON_RELOAD=examples/studio (debug): JSON edits show up without a restart.
    let mut ui = Ui::load(uison::ui::source!("../app.json")).unwrap_or_else(|e| panic!("app.json: {e}"));
    ui.load_components(uison::ui::source!("../components.json")).unwrap_or_else(|e| panic!("components.json: {e}"));
    ui.load_components(uison::ui::source!("../phone.json")).unwrap_or_else(|e| panic!("phone.json: {e}"));
    for (key, bytes) in PHOTOS {
        ui.image(key, bytes);
    }
    initial_data(&mut ui.data);
    let sh = Rc::new(RefCell::new(Shared { rng: 0xC0FFEE, ..Shared::default() }));
    {
        let mut rng = Rng(42);
        let (labels, cur) = stats_targets("month", &mut rng);
        ui.data.set("stats.labels", labels);
        ui.data.set("stats.cur", cur);
    }
    let sig = Rc::new(RefCell::new(String::new()));
    let sig2 = sig.clone();
    ui.derive(move |d| derive_all(d, &mut sig2.borrow_mut()));

    // ------------------------------------------------ navigation and theme
    for (i, nav) in ["inbox", "tasks", "contacts", "stats", "form", "settings"].iter().enumerate() {
        let action = format!("nav{}", i + 1);
        ui.on(&action, move |cx| cx.data.set("nav", *nav));
        ui.shortcut(&format!("cmd+{}", i + 1), &action);
    }
    ui.on("toggleTheme", |cx| {
        let next = if is_dark(cx.data) { "light" } else { "dark" };
        cx.data.set("settings.theme", next);
        cx.set_theme(next);
        apply_accent(cx);
    });
    ui.shortcut("cmd+d", "toggleTheme");
    ui.on("theme", |cx| {
        let t = cx.data.str("settings.theme").to_string();
        cx.set_theme(&t);
        apply_accent(cx);
    });
    ui.on("accent", apply_accent);
    ui.on("appearance", apply_accent);
    ui.on("fontSize", |cx| {
        let v = cx.data.num("settings.fontSize");
        cx.set_token("fontSize", v);
    });

    // ------------------------------------------------ notifications and undo
    let sh1 = sh.clone();
    ui.on("hideToast", move |cx| {
        let due = sh1.borrow().toast_until.is_some_and(|t| now() + Duration::from_millis(30) >= t);
        if due {
            cx.data.set("toast.text", "");
            sh1.borrow_mut().undo = None;
        }
    });
    let sh1 = sh.clone();
    ui.on("undo", move |cx| {
        if let Some((path, i, v)) = sh1.borrow_mut().undo.take() {
            if let Some(a) = cx.data.get_mut(&path).and_then(Json::as_array_mut) {
                a.insert(i.min(a.len()), v);
            }
            cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
        }
        cx.data.set("toast.text", "");
    });
    let sh1 = sh.clone();
    ui.on("demoToast", move |cx| toast(cx, &sh1, "This notification disappears by itself in 2.6 s", false));

    // ------------------------------------------------ mail
    // Phone: the message opens over the list, "back" returns to the list.
    ui.on("closeMessage", |cx| cx.data.set("inbox.sel", Json::Null));
    // System Back (Android): from a message, back to the list; otherwise change nothing and let the system exit.
    ui.on("back", |cx| {
        if cx.data.get("inbox.sel").is_some_and(|v| !v.is_null()) && cx.data.str("nav") == "inbox" {
            cx.data.set("inbox.sel", Json::Null);
        }
    });
    ui.on("openMessage", |cx| {
        if let Some(i) = cx.data.get("inbox.sel").and_then(Json::as_f64) {
            cx.data.set(&format!("inbox.messages.{i}.unread"), false);
            cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
            cx.data.set("inbox.reply", "");
        }
    });
    let sh1 = sh.clone();
    ui.on("star", move |cx| {
        if let Some(i) = cx.data.get("inbox.sel").and_then(Json::as_f64) {
            star_message(cx, &sh1, i as usize);
        }
    });
    let sh1 = sh.clone();
    ui.on("deleteMessage", move |cx| {
        if let Some(i) = cx.data.get("inbox.sel").and_then(Json::as_f64) {
            delete_message(cx, &sh1, i as usize);
        }
    });
    // Message row context menu: cx.key is `inboxList/<message index>`.
    ui.on("rowReply", |cx| {
        let Some(i) = key_id(cx.key, "inboxList/") else { return };
        cx.data.set("inbox.sel", i);
        cx.data.set(&format!("inbox.messages.{i}.unread"), false);
        cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
        cx.focus("reply/1/0");
    });
    let sh1 = sh.clone();
    ui.on("rowForward", move |cx| toast(cx, &sh1, "Forwarding comes in the next demo version", false));
    let sh1 = sh.clone();
    ui.on("rowStar", move |cx| {
        if let Some(i) = key_id(cx.key, "inboxList/") {
            star_message(cx, &sh1, i as usize);
        }
    });
    let sh1 = sh.clone();
    ui.on("rowDelete", move |cx| {
        if let Some(i) = key_id(cx.key, "inboxList/") {
            delete_message(cx, &sh1, i as usize);
        }
    });
    ui.on("reply", |cx| {
        let text = cx.data.str("inbox.reply").trim().to_string();
        let Some(i) = cx.data.get("inbox.sel").and_then(Json::as_f64) else { return };
        if text.is_empty() {
            return;
        }
        cx.data.push(&format!("inbox.messages.{i}.thread"), obj([("text", text.into_json()), ("time", "just now".into_json())]));
        cx.data.set("inbox.reply", "");
        cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
        cx.data.set("pendingReply", i);
        cx.timer(1400, "autoReply");
    });
    ui.on("autoReply", |cx| {
        let Some(i) = cx.data.get("pendingReply").and_then(Json::as_f64) else { return };
        cx.data.push(
            &format!("inbox.messages.{i}.thread"),
            obj([("text", "Thanks, got it! I'll take a look today 👍".into_json()), ("time", "just now".into_json())]),
        );
        cx.data.update("inbox.rev", |n| n.as_f64().unwrap_or(0.0) + 1.0);
    });
    let sh1 = sh.clone();
    ui.on("compose", move |cx| toast(cx, &sh1, "New message comes in the next demo version", false));
    ui.on("focusSearch", |cx| {
        cx.data.set("nav", "inbox");
        cx.focus("search/1/0");
    });
    ui.shortcut("cmd+f", "focusSearch");

    // ------------------------------------------------ tasks
    ui.on("openModal", |cx| {
        let me = cx.data.str("me.name").to_string();
        cx.data.set(
            "modal",
            obj([
                ("open", true.into_json()),
                ("title", "".into_json()),
                ("error", "".into_json()),
                ("prio", "mid".into_json()),
                ("status", "todo".into_json()),
                ("assignee", me.into_json()),
            ]),
        );
        cx.data.set("nav", "tasks");
        cx.focus("modalTitle/1/0");
    });
    ui.shortcut("cmd+n", "openModal");
    ui.on("closeModal", |cx| cx.data.set("modal.open", false));
    let sh1 = sh.clone();
    ui.on("createTask", move |cx| {
        let title = cx.data.str("modal.title").trim().to_string();
        if title.is_empty() {
            cx.data.set("modal.error", "Enter a title");
            cx.focus("modalTitle/1/0");
            return;
        }
        let id = cx.data.num("nextTask") as i64;
        let (prio, status, who) =
            (cx.data.str("modal.prio").to_string(), cx.data.str("modal.status").to_string(), cx.data.str("modal.assignee").to_string());
        cx.data.push("tasks", task(id, &title, &prio, &status, "today", &who));
        cx.data.set("nextTask", id + 1);
        cx.data.set("modal.open", false);
        toast(cx, &sh1, &format!("Task \"{title}\" created"), false);
    });
    ui.on("moveLeft", |cx| {
        if let Some(id) = key_id(cx.key, "left-") {
            move_task(cx, id, -1);
        }
    });
    ui.on("moveRight", |cx| {
        if let Some(id) = key_id(cx.key, "right-") {
            move_task(cx, id, 1);
        }
    });
    let sh1 = sh.clone();
    ui.on("deleteTask", move |cx| {
        let Some(id) = key_id(cx.key, "del-") else { return };
        let i = arr(cx.data, "tasks").iter().position(|t| t.get("id").and_then(Json::as_f64) == Some(id as f64));
        if let Some(i) = i
            && let Some(t) = cx.data.remove("tasks", i)
        {
            sh1.borrow_mut().undo = Some(("tasks".into(), i, t));
            toast(cx, &sh1, "Task deleted", true);
        }
    });

    // ------------------------------------------------ contacts
    let sh1 = sh.clone();
    ui.on("writeTo", move |cx| {
        let Some(id) = key_id(cx.key, "write-") else { return };
        let name = PEOPLE.get(id as usize).map_or("", |p| p.0);
        cx.data.set("nav", "inbox");
        cx.data.set("inbox.q", name);
        cx.data.set("inbox.filter", "all");
        toast(cx, &sh1, &format!("Messages from: {name}"), false);
    });
    let sh1 = sh.clone();
    ui.on("contactMore", move |cx| toast(cx, &sh1, "Context menu comes in set B", false));

    // ------------------------------------------------ analytics: frame-by-frame animation
    let sh1 = sh.clone();
    ui.on("refresh", move |cx| {
        let period = cx.data.str("stats.period").to_string();
        let mut sh = sh1.borrow_mut();
        let mut rng = Rng(sh.rng);
        let (labels, target) = stats_targets(&period, &mut rng);
        sh.rng = rng.next();
        let from = cx.data.get("stats.cur").cloned().unwrap_or(Json::Null);
        // A different number of bars: start from zero height.
        let from = if cx.data.len("stats.labels") != labels.as_array().map_or(0, <[Json]>::len) {
            let zero = |v: &Json| Json::Array(v.as_array().unwrap_or(&[]).iter().map(|_| Json::Number(0.0)).collect());
            let mut f = from.clone();
            f.set("bars", zero(target.get("bars").unwrap()));
            f
        } else {
            from
        };
        cx.data.set("stats.labels", labels);
        cx.data.set("stats.cur", from.clone());
        sh.anim = Some(Anim { start: now(), from, to: target });
        cx.timer(16, "animate");
    });
    ui.shortcut("cmd+r", "refresh");
    let sh1 = sh.clone();
    ui.on("animate", move |cx| {
        let mut sh = sh1.borrow_mut();
        let Some(a) = &sh.anim else { return };
        let t = ((now() - a.start).as_secs_f64() / ANIM.as_secs_f64()).min(1.0);
        let ease = 1.0 - (1.0 - t).powi(3);
        cx.data.set("stats.cur", lerp(&a.from, &a.to, ease));
        if t < 1.0 {
            cx.timer(16, "animate");
        } else {
            sh.anim = None;
        }
    });

    // ------------------------------------------------ form
    let sh1 = sh.clone();
    ui.on("submitForm", move |cx| {
        cx.data.set("form.tried", true);
        derive_form(cx.data);
        for (field, key) in [("name", "fName"), ("email", "fEmail"), ("password", "fPass")] {
            if cx.data.bool(&format!("form.errors.{field}")) {
                cx.focus(&format!("{key}/1/0"));
                return;
            }
        }
        let name = cx.data.str("form.name").to_string();
        toast(cx, &sh1, &format!("Welcome, {name}!"), false);
    });
    ui.on("clearForm", |cx| {
        for f in ["name", "email", "password"] {
            cx.data.set(&format!("form.{f}"), "");
        }
        cx.data.set("form.tried", false);
        cx.data.set("form.errors", Json::object());
    });

    // ------------------------------------------------ settings
    let sh1 = sh.clone();
    ui.on("resetSettings", move |cx| {
        cx.data.set("settings.theme", "system");
        cx.data.set("settings.accent", "blue");
        cx.data.set("settings.fontSize", 13);
        cx.data.set("settings.compact", false);
        cx.data.set("settings.preview", true);
        cx.data.set("settings.lang", "en");
        cx.set_theme("system");
        cx.set_token("fontSize", 13);
        apply_accent(cx);
        toast(cx, &sh1, "Settings reset", false);
    });
    ui
}

