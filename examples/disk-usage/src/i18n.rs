//! Interface languages: English (the default) and Russian.
//!
//! Fixed texts of the screen go into the data as `t.<key>` (`"@t.open"` in the JSON); phrases with
//! names and numbers are built here, with Russian plural forms (1 файл, 2 файла, 5 файлов).
//! Numbers follow the language too: `8.2 GB` / `8,2 ГБ` (see `format`).

use std::path::Path;

use uison::prelude::Data;

use crate::format;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Lang {
    #[default]
    En,
    Ru,
}

/// Fixed texts of the screen: key, English, Russian.
const TEXTS: &[(&str, &str, &str)] = &[
    ("open", "Open…", "Открыть…"),
    ("openTip", "Choose a folder to scan (Ctrl/⌘+O)", "Выбрать папку для сканирования (Ctrl/⌘+O)"),
    ("rescan", "Rescan", "Обновить"),
    ("rescanTip", "Scan again (Ctrl/⌘+R)", "Просканировать заново (Ctrl/⌘+R)"),
    ("upTip", "Up one level (Backspace)", "На уровень выше (Backspace)"),
    ("about", "About", "О программе"),
    ("hints", "click: select · center: deselect / up · double-click: zoom · right-click: actions",
        "щелчок: выбрать · центр: снять выбор / вверх · двойной щелчок: приблизить · правая кнопка: действия"),
    ("tagline", "See where your disk space goes", "Куда уходит место на диске"),
    ("choose", "Choose folder…", "Выбрать папку…"),
    ("orScan", "or scan:", "или просканировать:"),
    ("cancel", "Cancel", "Отмена"),
    ("zoomIn", "Zoom in", "Приблизить"),
    ("reveal", "Reveal", "Показать"),
    ("trash", "Trash", "В корзину"),
    ("unqueue", "Remove from queue", "Убрать из очереди"),
    ("queueTitle", "Trash queue", "Очередь в корзину"),
    ("trashAll", "Move to Trash", "Переместить в корзину"),
    ("trashAllTip", "Move everything listed here to the Trash", "Переместить в корзину всё, что в списке"),
    ("clearQueue", "Clear all", "Очистить"),
    ("clearQueueTip", "Empty the list; nothing is moved", "Очистить список; ничего не перемещается"),
    ("unqueueTip", "Remove from the queue", "Убрать из очереди"),
    ("aboutLine", "uison example · a port of the egui app", "пример uison · порт приложения на egui"),
    ("aboutText", "Lightweight disk usage visualizer with a sunburst chart", "Лёгкий анализатор места на диске с солнечной диаграммой"),
    ("close", "Close", "Закрыть"),
];

/// Russian plural form by the number: 1 файл, 2 файла, 5 файлов, 11 файлов, 21 файл.
fn ru_plural(n: u64, [one, few, many]: [&'static str; 3]) -> &'static str {
    match (n % 10, n % 100) {
        (1, r) if r != 11 => one,
        (2..=4, r) if !(12..=14).contains(&r) => few,
        _ => many,
    }
}

impl Lang {
    pub fn from_code(code: &str) -> Lang {
        if code == "ru" { Lang::Ru } else { Lang::En }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
        }
    }

    /// Decimal separator: `8.2 GB` / `8,2 ГБ`.
    pub fn decimal(self) -> char {
        match self {
            Lang::En => '.',
            Lang::Ru => ',',
        }
    }

    /// Byte units from B to EB.
    pub fn units(self) -> &'static [&'static str; 7] {
        match self {
            Lang::En => &["B", "KB", "MB", "GB", "TB", "PB", "EB"],
            Lang::Ru => &["Б", "КБ", "МБ", "ГБ", "ТБ", "ПБ", "ЭБ"],
        }
    }

    pub fn seconds_unit(self) -> &'static str {
        self.pick("s", "с")
    }

    fn pick(self, en: &'static str, ru: &'static str) -> &'static str {
        match self {
            Lang::En => en,
            Lang::Ru => ru,
        }
    }

    /// `n` with a noun: `en` is [one, many], `ru` is [one, few, many].
    fn counted(self, n: u64, en: [&'static str; 2], ru: [&'static str; 3]) -> String {
        let word = match self {
            Lang::En if n == 1 => en[0],
            Lang::En => en[1],
            Lang::Ru => ru_plural(n, ru),
        };
        format!("{} {word}", format::count(n))
    }

    /// Fixed texts into the screen data (`t.*`).
    pub fn apply(self, d: &mut Data) {
        for &(key, en, ru) in TEXTS {
            d.set(&format!("t.{key}"), self.pick(en, ru));
        }
        d.set("t.showIn", self.show_in());
    }

    // ------------------------------------------------------------ counts

    pub fn files(self, n: u64) -> String {
        self.counted(n, ["file", "files"], ["файл", "файла", "файлов"])
    }

    pub fn folders(self, n: u64) -> String {
        self.counted(n, ["folder", "folders"], ["папка", "папки", "папок"])
    }

    pub fn items(self, n: u64) -> String {
        self.counted(n, ["item", "items"], ["объект", "объекта", "объектов"])
    }

    pub fn smaller_objects(self, n: u64) -> String {
        self.counted(n, ["smaller object", "smaller objects"], ["мелкий объект", "мелких объекта", "мелких объектов"])
    }

    // ------------------------------------------------------------ scanning

    #[cfg(target_os = "macos")]
    pub fn stall_hint(self) -> &'static str {
        self.pick(
            "Waiting for the file system… macOS may be asking for permission to access a folder. \
             Grant \"Full Disk Access\" in System Settings → Privacy & Security to avoid the prompts.",
            "Ждём файловую систему… Возможно, macOS спрашивает разрешение на доступ к папке. \
             Чтобы вопросов не было, включите «Доступ ко всему диску» в Системных настройках → Конфиденциальность и безопасность.",
        )
    }

    #[cfg(not(target_os = "macos"))]
    pub fn stall_hint(self) -> &'static str {
        self.pick(
            "Waiting for the file system… (a slow network or removable drive?)",
            "Ждём файловую систему… (медленная сеть или съёмный диск?)",
        )
    }

    pub fn scanning(self, path: &Path) -> String {
        match self {
            Lang::En => format!("Scanning {}", path.display()),
            Lang::Ru => format!("Сканирование {}", path.display()),
        }
    }

    pub fn scan_counts(self, files: u64, dirs: u64, bytes: u64, secs: u64) -> String {
        let s = self.seconds_unit();
        format!("{}  ·  {}  ·  {}  ·  {secs} {s}", self.files(files), self.folders(dirs), format::bytes(bytes, self))
    }

    pub fn scan_cancelled(self) -> &'static str {
        self.pick("Scan cancelled", "Сканирование отменено")
    }

    pub fn scanner_crashed(self) -> &'static str {
        self.pick("The scanner stopped unexpectedly", "Сканер неожиданно остановился")
    }

    pub fn cannot_read(self, path: &Path, error: &std::io::Error) -> String {
        match self {
            Lang::En => format!("Cannot read {}: {error}", path.display()),
            Lang::Ru => format!("Не удаётся прочитать {}: {error}", path.display()),
        }
    }

    /// Title of the folder picker.
    pub fn choose_title(self) -> &'static str {
        self.pick("Choose a folder to scan", "Выберите папку для сканирования")
    }

    // ------------------------------------------------------------ status bar and panels

    pub fn unreadable(self, n: u64) -> String {
        match self {
            Lang::En => format!("{} unreadable", format::count(n)),
            Lang::Ru => format!("недоступно: {}", format::count(n)),
        }
    }

    pub fn scanned_in(self, d: std::time::Duration) -> String {
        match self {
            Lang::En => format!("scanned in {}", format::seconds(d, self)),
            Lang::Ru => format!("просканировано за {}", format::seconds(d, self)),
        }
    }

    pub fn disk_free(self, free: u64, total: u64) -> String {
        let (free, total) = (format::bytes(free, self), format::bytes(total, self));
        match self {
            Lang::En => format!("disk: {free} free of {total}"),
            Lang::Ru => format!("диск: свободно {free} из {total}"),
        }
    }

    /// "12% of Documents": the share of the selection in the view.
    pub fn share_of(self, share: &str, name: &str) -> String {
        match self {
            Lang::En => format!("{share} of {name}"),
            Lang::Ru => format!("{share} от {name}"),
        }
    }

    pub fn empty_folder(self) -> &'static str {
        self.pick("Empty folder", "Пустая папка")
    }

    pub fn queue_tip(self, covered: bool) -> &'static str {
        match covered {
            true => self.pick("Its folder is already in the Trash queue", "Его папка уже в очереди в корзину"),
            false => self.pick("Add to the Trash queue (Del)", "Добавить в очередь в корзину (Del)"),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn show_in(self) -> &'static str {
        self.pick("Show in Finder", "Показать в Finder")
    }

    #[cfg(windows)]
    pub fn show_in(self) -> &'static str {
        self.pick("Show in Explorer", "Показать в Проводнике")
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    pub fn show_in(self) -> &'static str {
        self.pick("Show in File Manager", "Показать в файловом менеджере")
    }

    // ------------------------------------------------------------ Trash

    pub fn trash_failed(self, path: &Path, error: &str) -> String {
        match self {
            Lang::En => format!("Could not move {} to Trash: {error}", path.display()),
            Lang::Ru => format!("Не удалось переместить в корзину {}: {error}", path.display()),
        }
    }

    pub fn trashed_one(self, path: &Path, freed: u64) -> String {
        let freed = format::bytes(freed, self);
        match self {
            Lang::En => format!("Moved to Trash: {} ({freed})", path.display()),
            Lang::Ru => format!("Перемещено в корзину: {} ({freed})", path.display()),
        }
    }

    pub fn trashed_many(self, n: u64, freed: u64) -> String {
        let freed = format::bytes(freed, self);
        match self {
            Lang::En => format!("Moved {} to Trash ({freed})", self.items(n)),
            Lang::Ru => format!("Перемещено в корзину: {} ({freed})", self.items(n)),
        }
    }

    /// The first failure and how many more stayed in the queue.
    pub fn and_more(self, first: &str, more: u64) -> String {
        match self {
            Lang::En => format!("{first} (and {more} more, still queued)"),
            Lang::Ru => format!("{first} (и ещё {more}, остаются в очереди)"),
        }
    }

    // ------------------------------------------------------------ context menu

    pub fn menu_zoom(self) -> &'static str {
        self.pick("Zoom in", "Приблизить")
    }

    pub fn menu_open(self) -> &'static str {
        self.pick("Open", "Открыть")
    }

    pub fn menu_copy(self) -> &'static str {
        self.pick("Copy path", "Копировать путь")
    }

    pub fn menu_unqueue(self) -> &'static str {
        self.pick("Remove from Trash queue", "Убрать из очереди в корзину")
    }

    pub fn menu_covered(self) -> &'static str {
        self.pick("In Trash queue with its folder", "В очереди в корзину вместе с папкой")
    }

    pub fn menu_queue(self) -> &'static str {
        self.pick("Add to Trash queue", "Добавить в очередь в корзину")
    }

    // ------------------------------------------------------------ chart

    pub fn deselect(self) -> &'static str {
        self.pick("× deselect", "× снять выбор")
    }

    pub fn up(self) -> &'static str {
        self.pick("↑ up", "↑ вверх")
    }

    pub fn legend(self) -> &'static str {
        self.pick("share of view", "доля в текущей папке")
    }

    /// "1.2 GB  ·  12% of parent" in the chart tooltip.
    pub fn size_of_parent(self, size: u64, share: &str) -> String {
        let size = format::bytes(size, self);
        match self {
            Lang::En => format!("{size}  ·  {share} of parent"),
            Lang::Ru => format!("{size}  ·  {share} родительской папки"),
        }
    }

    /// "1.2 GB  ·  in Documents": where the small objects are.
    pub fn size_in(self, size: u64, folder: &str) -> String {
        let size = format::bytes(size, self);
        match self {
            Lang::En => format!("{size}  ·  in {folder}"),
            Lang::Ru => format!("{size}  ·  в {folder}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_plurals() {
        let forms = ["файл", "файла", "файлов"];
        let cases = [(0, "файлов"), (1, "файл"), (2, "файла"), (4, "файла"), (5, "файлов"), (11, "файлов"), (12, "файлов"),
            (14, "файлов"), (21, "файл"), (22, "файла"), (101, "файл"), (111, "файлов"), (1_000_002, "файла")];
        for (n, want) in cases {
            assert_eq!(ru_plural(n, forms), want, "{n}");
        }
    }

    #[test]
    fn counts_in_both_languages() {
        assert_eq!(Lang::En.files(1), "1 file");
        assert_eq!(Lang::En.files(2), "2 files");
        assert_eq!(Lang::Ru.files(3), "3 файла");
        assert_eq!(Lang::Ru.items(1), "1 объект");
        assert_eq!(Lang::Ru.smaller_objects(25), "25 мелких объектов");
        assert_eq!(Lang::from_code("ru"), Lang::Ru);
        assert_eq!(Lang::from_code("de"), Lang::En);
    }

    #[test]
    fn every_text_has_both_languages() {
        for (key, en, ru) in TEXTS {
            assert!(!en.is_empty() && !ru.is_empty(), "{key}");
        }
    }
}
