//! System glue: folder picker, Finder/Explorer, Trash, free space, drives.
//! macOS through the Objective-C runtime (no `std::process::Command`, ~29 KB), Windows through the Shell API,
//! Linux — zenity/kdialog, gio, xdg-open.

use std::path::{Path, PathBuf};

pub fn home_dir() -> Option<PathBuf> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    home.filter(|h| !h.is_empty()).map(PathBuf::from)
}

/// Top-level folders for the start screen.
pub fn volumes() -> Vec<PathBuf> {
    #[cfg(windows)]
    return win::drives();
    #[cfg(not(windows))]
    {
        fn subdirs(dir: &Path) -> impl Iterator<Item = PathBuf> {
            std::fs::read_dir(dir)
                .into_iter()
                .flatten()
                .flatten()
                // Without symbolic links like /Volumes/Macintosh HD → /.
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.path())
        }
        let mut out = vec![PathBuf::from("/")];
        #[cfg(target_os = "macos")]
        out.extend(subdirs(Path::new("/Volumes")));
        #[cfg(not(target_os = "macos"))]
        {
            // Removable drives: /media/$USER/<drive> or /run/media/$USER/<drive>.
            let user = std::env::var_os("USER").unwrap_or_default();
            for base in ["/media", "/run/media", "/mnt"] {
                for dir in subdirs(Path::new(base)) {
                    if dir.file_name() == Some(user.as_os_str()) {
                        out.extend(subdirs(&dir));
                    } else {
                        out.push(dir);
                    }
                }
            }
        }
        out
    }
}

/// System folder picker with a `title`; `None` if cancelled or there is no picker.
pub fn pick_folder(title: &str) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    return mac::pick_folder(title);
    #[cfg(windows)]
    return win::pick_folder(title);
    #[cfg(target_os = "linux")]
    return linux::pick_folder(title);
    #[allow(unreachable_code)]
    {
        let _ = title;
        None
    }
}

/// Show `path` selected in Finder/Explorer.
pub fn reveal(path: &Path) {
    #[cfg(target_os = "macos")]
    mac::reveal(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", path.display())).spawn();
    }
    #[cfg(target_os = "linux")]
    linux::reveal(path);
    let _ = path;
}

/// Open with the default program.
pub fn open(path: &Path) {
    #[cfg(target_os = "macos")]
    mac::open(path);
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "linux")]
    linux::spawn("xdg-open", &[path.as_os_str()]);
    let _ = path;
}

/// To the Trash; an error comes back as text for the status line.
pub fn move_to_trash(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return mac::trash(path);
    #[cfg(windows)]
    return win::trash(path);
    #[cfg(target_os = "linux")]
    return linux::trash(path);
    #[allow(unreachable_code)]
    Err(format!("no Trash here: {}", path.display()))
}

/// Free and total bytes on the volume holding `path`.
pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
        // Fields are read by offset: the structs differ, and only three numbers are needed.
        let mut buf = [0u64; 512];
        #[cfg(target_vendor = "apple")]
        {
            unsafe extern "C" {
                // 64-bit struct statfs: f_bsize u32, f_iosize i32, f_blocks, f_bfree, f_bavail u64…
                #[cfg_attr(target_arch = "x86_64", link_name = "statfs$INODE64")]
                fn statfs(path: *const std::ffi::c_char, buf: *mut u64) -> i32;
            }
            if unsafe { statfs(c.as_ptr(), buf.as_mut_ptr()) } != 0 {
                return None;
            }
            let bsize = buf[0] & 0xFFFF_FFFF;
            return Some((buf[3] * bsize, buf[1] * bsize));
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unsafe extern "C" {
                // struct statvfs (64 bits): f_bsize, f_frsize, f_blocks, f_bfree, f_bavail…
                fn statvfs(path: *const std::ffi::c_char, buf: *mut u64) -> i32;
            }
            if unsafe { statvfs(c.as_ptr(), buf.as_mut_ptr()) } != 0 {
                return None;
            }
            return Some((buf[4] * buf[1], buf[2] * buf[1]));
        }
    }
    #[cfg(windows)]
    return win::disk_space(path);
    #[allow(unreachable_code)]
    None
}

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::{CStr, c_char, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    type Id = *mut c_void;

    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Id;
        fn objc_msgSend();
    }

    fn class(name: &CStr) -> Id {
        unsafe { objc_getClass(name.as_ptr()) }
    }

    fn sel(name: &CStr) -> Id {
        unsafe { sel_registerName(name.as_ptr()) }
    }

    /// `objc_msgSend`: arguments and result are register-sized integers (pointers, BOOL, NSInteger).
    fn msg(obj: Id, name: &CStr, args: &[usize]) -> usize {
        type F0 = unsafe extern "C" fn(Id, Id) -> usize;
        type F1 = unsafe extern "C" fn(Id, Id, usize) -> usize;
        type F3 = unsafe extern "C" fn(Id, Id, usize, usize, usize) -> usize;
        let f = objc_msgSend as unsafe extern "C" fn();
        let s = sel(name);
        unsafe {
            match *args {
                [] => std::mem::transmute::<unsafe extern "C" fn(), F0>(f)(obj, s),
                [a] => std::mem::transmute::<unsafe extern "C" fn(), F1>(f)(obj, s, a),
                [a, b, c] => std::mem::transmute::<unsafe extern "C" fn(), F3>(f)(obj, s, a, b, c),
                _ => unreachable!(),
            }
        }
    }

    fn obj(obj: Id, name: &CStr, args: &[usize]) -> Id {
        msg(obj, name, args) as Id
    }

    fn string(bytes: &[u8]) -> Id {
        let mut bytes = bytes.to_vec();
        bytes.push(0);
        obj(class(c"NSString"), c"stringWithUTF8String:", &[bytes.as_ptr() as usize])
    }

    fn url(path: &Path) -> Id {
        let s = string(path.as_os_str().as_bytes());
        obj(class(c"NSURL"), c"fileURLWithPath:", &[s as usize])
    }

    fn workspace() -> Id {
        obj(class(c"NSWorkspace"), c"sharedWorkspace", &[])
    }

    pub fn pick_folder(title: &str) -> Option<PathBuf> {
        let panel = obj(class(c"NSOpenPanel"), c"openPanel", &[]);
        msg(panel, c"setMessage:", &[string(title.as_bytes()) as usize]);
        msg(panel, c"setCanChooseDirectories:", &[1]);
        msg(panel, c"setCanChooseFiles:", &[0]);
        msg(panel, c"setAllowsMultipleSelection:", &[0]);
        // NSModalResponseOK = 1.
        if msg(panel, c"runModal", &[]) as isize != 1 {
            return None;
        }
        let url = obj(panel, c"URL", &[]);
        let p = msg(url, c"fileSystemRepresentation", &[]) as *const c_char;
        (!p.is_null()).then(|| PathBuf::from(std::ffi::OsStr::from_bytes(unsafe { CStr::from_ptr(p) }.to_bytes())))
    }

    pub fn reveal(path: &Path) {
        let urls = obj(class(c"NSArray"), c"arrayWithObject:", &[url(path) as usize]);
        msg(workspace(), c"activateFileViewerSelectingURLs:", &[urls as usize]);
    }

    pub fn open(path: &Path) {
        msg(workspace(), c"openURL:", &[url(path) as usize]);
    }

    /// Through NSFileManager: Finder (the `trash` default) would ask for automation permission.
    pub fn trash(path: &Path) -> Result<(), String> {
        let fm = obj(class(c"NSFileManager"), c"defaultManager", &[]);
        let mut err: Id = std::ptr::null_mut();
        let ok = msg(fm, c"trashItemAtURL:resultingItemURL:error:", &[url(path) as usize, 0, &mut err as *mut Id as usize]) & 0xFF;
        if ok != 0 {
            return Ok(());
        }
        if err.is_null() {
            return Err("unknown error".into());
        }
        let desc = obj(err, c"localizedDescription", &[]);
        let s = msg(desc, c"UTF8String", &[]) as *const c_char;
        Err(if s.is_null() { "unknown error".into() } else { unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned() })
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    pub fn spawn(cmd: &str, args: &[&OsStr]) {
        // Wait in the background so no zombie is left.
        if let Ok(mut child) = Command::new(cmd).args(args).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
            std::thread::spawn(move || child.wait());
        }
    }

    pub fn pick_folder(title: &str) -> Option<PathBuf> {
        let mut tries: [(&str, &[&str]); 2] =
            [("zenity", &["--file-selection", "--directory"]), ("kdialog", &["--getexistingdirectory"])];
        // The desktop's own dialog first: KDE has zenity too (GTK), but kdialog looks like the rest of Plasma.
        if std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.split(':').any(|d| d == "KDE")) {
            tries.reverse();
        }
        for (cmd, args) in tries {
            match Command::new(cmd).args(args).arg("--title").arg(title).stderr(Stdio::null()).output() {
                Ok(out) if out.status.success() => {
                    let p = out.stdout.strip_suffix(b"\n").unwrap_or(&out.stdout);
                    return (!p.is_empty()).then(|| PathBuf::from(OsStr::from_bytes(p)));
                }
                Ok(_) => return None, // cancelled
                Err(_) => continue,   // no such program
            }
        }
        None
    }

    pub fn reveal(path: &Path) {
        let path = path.to_path_buf();
        std::thread::spawn(move || {
            let mut uri = String::from("file://");
            for &b in path.as_os_str().as_bytes() {
                if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
                    uri.push(b as char);
                } else {
                    uri.push_str(&format!("%{b:02X}"));
                }
            }
            // The file manager selects the file over D-Bus; if that fails, open the folder.
            let selected = Command::new("dbus-send")
                .args([
                    "--session",
                    "--print-reply",
                    "--dest=org.freedesktop.FileManager1",
                    "--type=method_call",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                ])
                .arg(format!("array:string:{uri}"))
                .arg("string:")
                .output()
                .is_ok_and(|out| out.status.success());
            if !selected {
                let dir = if path.is_dir() { path.as_path() } else { path.parent().unwrap_or(&path) };
                let _ = Command::new("xdg-open").arg(dir).status();
            }
        });
    }

    pub fn trash(path: &Path) -> Result<(), String> {
        let out = match Command::new("gio").arg("trash").arg(path).output() {
            Ok(out) => out,
            // KDE without GLib's tools: KIO moves to the same freedesktop Trash.
            Err(e) => ["kioclient5", "kioclient"]
                .iter()
                .find_map(|k| Command::new(k).arg("move").arg(path).arg("trash:/").output().ok())
                .ok_or(format!("gio: {e}"))?,
        };
        if out.status.success() {
            return Ok(());
        }
        // "gio: file:///…: Unable to trash file /…: Permission denied" — the caller names the file, keep the reason.
        let err = String::from_utf8_lossy(&out.stderr);
        Err(err.trim().rsplit(": ").next().unwrap_or_default().to_string())
    }
}

#[cfg(windows)]
mod win {
    use std::ffi::c_void;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::{Path, PathBuf};

    #[repr(C)]
    struct Guid(u32, u16, u16, [u8; 8]);

    const CLSID_FILE_OPEN_DIALOG: Guid = Guid(0xDC1C5A9C, 0xE88A, 0x4DDE, [0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7]);
    const IID_IFILE_OPEN_DIALOG: Guid = Guid(0xD57C7288, 0xD4AD, 0x4768, [0xBE, 0x02, 0x9D, 0x96, 0x95, 0x32, 0xD9, 0x60]);

    #[repr(C)]
    struct FileOp {
        hwnd: *mut c_void,
        func: u32,
        from: *const u16,
        to: *const u16,
        flags: u16,
        aborted: i32,
        mappings: *mut c_void,
        title: *const u16,
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHFileOperationW(op: *mut FileOp) -> i32;
    }
    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoTaskMemFree(p: *mut c_void);
        fn CoCreateInstance(clsid: *const Guid, outer: *mut c_void, ctx: u32, iid: *const Guid, out: *mut *mut c_void) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetActiveWindow() -> *mut c_void;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(dir: *const u16, avail: *mut u64, total: *mut u64, free: *mut u64) -> i32;
        fn GetLogicalDrives() -> u32;
        fn GetDriveTypeW(root: *const u16) -> u32;
    }

    fn wide(p: &Path) -> Vec<u16> {
        p.as_os_str().encode_wide().chain([0]).collect()
    }

    /// Method `n` of a COM object's table (the numbers are checked by tools/vtable.cpp).
    unsafe fn method<F: Copy>(obj: *mut c_void, n: usize) -> F {
        unsafe { std::mem::transmute_copy(&*(*(obj as *const *const usize)).add(n)) }
    }

    unsafe fn release(obj: *mut c_void) {
        unsafe { method::<unsafe extern "system" fn(*mut c_void) -> u32>(obj, 2)(obj) };
    }

    /// The Explorer folder dialog (IFileOpenDialog with FOS_PICKFOLDERS): an address bar, a pasted
    /// path, Quick access — what rfd gives the original. The old SHBrowseForFolderW is just a tree.
    pub fn pick_folder(title: &str) -> Option<PathBuf> {
        const CLSCTX_INPROC_SERVER: u32 = 1;
        const FOS_PICKFOLDERS: u32 = 0x20;
        const FOS_FORCEFILESYSTEM: u32 = 0x40;
        const SIGDN_FILESYSPATH: u32 = 0x8005_8000;
        type Ptr = *mut c_void;
        let title: Vec<u16> = title.encode_utf16().chain([0]).collect();
        unsafe {
            // COM is initialized by the backend (single-threaded apartment of the window's thread).
            let mut dlg: Ptr = std::ptr::null_mut();
            if CoCreateInstance(&CLSID_FILE_OPEN_DIALOG, std::ptr::null_mut(), CLSCTX_INPROC_SERVER, &IID_IFILE_OPEN_DIALOG, &mut dlg) < 0 {
                return None;
            }
            let mut opts = 0u32;
            method::<unsafe extern "system" fn(Ptr, *mut u32) -> i32>(dlg, 10)(dlg, &mut opts); // GetOptions
            method::<unsafe extern "system" fn(Ptr, u32) -> i32>(dlg, 9)(dlg, opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM); // SetOptions
            method::<unsafe extern "system" fn(Ptr, *const u16) -> i32>(dlg, 17)(dlg, title.as_ptr()); // SetTitle
            // Show: the handler runs on the window's thread, so its window is the owner and the
            // dialog is modal to it. A cancel is an error result.
            let mut path = None;
            if method::<unsafe extern "system" fn(Ptr, Ptr) -> i32>(dlg, 3)(dlg, GetActiveWindow()) >= 0 {
                let mut item: Ptr = std::ptr::null_mut();
                if method::<unsafe extern "system" fn(Ptr, *mut Ptr) -> i32>(dlg, 20)(dlg, &mut item) >= 0 {
                    let mut name: *mut u16 = std::ptr::null_mut();
                    // IShellItem::GetDisplayName
                    if method::<unsafe extern "system" fn(Ptr, u32, *mut *mut u16) -> i32>(item, 5)(item, SIGDN_FILESYSPATH, &mut name) >= 0 {
                        let len = (0..).take_while(|&i| *name.add(i) != 0).count();
                        path = Some(PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(name, len))));
                        CoTaskMemFree(name.cast());
                    }
                    release(item);
                }
            }
            release(dlg);
            path
        }
    }

    pub fn trash(path: &Path) -> Result<(), String> {
        const FO_DELETE: u32 = 3;
        const FOF_SILENT: u16 = 0x4;
        const FOF_NOCONFIRMATION: u16 = 0x10;
        const FOF_ALLOWUNDO: u16 = 0x40;
        const FOF_NOERRORUI: u16 = 0x400;
        let mut from = wide(path);
        from.push(0); // the path list ends with two zeros
        let mut op = FileOp {
            hwnd: std::ptr::null_mut(),
            func: FO_DELETE,
            from: from.as_ptr(),
            to: std::ptr::null(),
            flags: FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI,
            aborted: 0,
            mappings: std::ptr::null_mut(),
            title: std::ptr::null(),
        };
        match unsafe { SHFileOperationW(&mut op) } {
            0 if op.aborted == 0 => Ok(()),
            0 => Err("cancelled".into()),
            code => Err(trash_error(code)),
        }
    }

    /// The system's text (in the Windows language) for a `SHFileOperationW` result. Besides Win32
    /// codes it returns old shell codes `DE_*` (0x71–0x88): those are mapped to the nearest Win32 one.
    fn trash_error(code: i32) -> String {
        let win32 = match code {
            0x78 => 5,           // DE_ACCESSDENIEDSRC → ERROR_ACCESS_DENIED
            0x79 | 0x81 => 206,  // DE_PATHTOODEEP, DE_FILENAMETOOLONG → ERROR_FILENAME_EXCED_RANGE
            0x7C => 32,          // DE_INVALIDFILES: seen for a folder with a file open in another program
            0x71..=0x88 | 0x402 => return format!("error {code:#x}"),
            c => c,
        };
        let text = std::io::Error::from_raw_os_error(win32).to_string();
        // Display adds " (os error N)" — the status line needs only the sentence.
        text.split(" (os error").next().unwrap_or(&text).trim_end().trim_end_matches('.').to_string()
    }

    pub fn disk_space(path: &Path) -> Option<(u64, u64)> {
        let (mut free, mut total) = (0u64, 0u64);
        let ok = unsafe { GetDiskFreeSpaceExW(wide(path).as_ptr(), &mut free, &mut total, std::ptr::null_mut()) };
        (ok != 0).then_some((free, total))
    }

    /// Drive letters without probing paths: a disconnected network drive would take seconds to answer.
    pub fn drives() -> Vec<PathBuf> {
        const DRIVE_UNKNOWN: u32 = 0;
        const DRIVE_NO_ROOT_DIR: u32 = 1;
        let mask = unsafe { GetLogicalDrives() };
        (0..26u8)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| PathBuf::from(format!("{}:\\", (b'A' + i) as char)))
            .filter(|p| {
                let kind = unsafe { GetDriveTypeW(wide(p).as_ptr()) };
                kind != DRIVE_UNKNOWN && kind != DRIVE_NO_ROOT_DIR
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn disk_space_of_temp_dir() {
        let (free, total) = super::disk_space(&std::env::temp_dir()).expect("disk space");
        assert!(total > 0 && free <= total);
    }
}
