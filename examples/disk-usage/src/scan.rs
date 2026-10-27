//! Background folder scanner: a std thread pool instead of rayon.
//!
//! Threads take folders from a shared stack, read them and push subfolders back. Each folder is
//! a list of entries under its own index; the tree is assembled from the lists at the end.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs::{self, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::tree::{Kind, ScanNode, Tree};

/// Counters for the scanning screen.
#[derive(Default)]
pub struct Progress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
    pub current: Mutex<PathBuf>,
    cancel: AtomicBool,
}

impl Progress {
    fn cancelled(&self) -> bool {
        self.cancel.load(Relaxed)
    }
}

pub enum Outcome {
    Done(Tree),
    Cancelled,
    /// The root itself couldn't be read.
    Unreadable(std::io::Error),
    /// The scanner thread is gone without an answer.
    Crashed,
}

pub struct Finished {
    pub outcome: Outcome,
    pub errors: u64,
    pub elapsed: Duration,
}

pub struct Scan {
    pub root: PathBuf,
    pub progress: Arc<Progress>,
    pub started: Instant,
    rx: Receiver<Finished>,
}

impl Scan {
    pub fn start(root: PathBuf) -> Self {
        let progress = Arc::new(Progress::default());
        let (tx, rx) = mpsc::channel();
        let started = Instant::now();
        let (r, p) = (root.clone(), Arc::clone(&progress));
        let spawned = std::thread::Builder::new().name("scanner".into()).spawn(move || {
            let outcome = match scan(&r, &p) {
                _ if p.cancelled() => Outcome::Cancelled,
                Ok(tree) => Outcome::Done(tree),
                Err(e) => Outcome::Unreadable(e),
            };
            let _ = tx.send(Finished { outcome, errors: p.errors.load(Relaxed), elapsed: started.elapsed() });
        });
        if spawned.is_err() {
            progress.cancel.store(true, Relaxed);
        }
        Self { root, progress, started, rx }
    }

    pub fn cancel(&self) {
        self.progress.cancel.store(true, Relaxed);
    }

    /// Result, once the scan has finished.
    pub fn poll(&self) -> Option<Finished> {
        match self.rx.try_recv() {
            Ok(done) => Some(done),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Finished {
                outcome: Outcome::Crashed,
                errors: self.progress.errors.load(Relaxed),
                elapsed: self.started.elapsed(),
            }),
        }
    }
}

/// A folder entry: a file or a subfolder (`dir` is the index of its list).
struct Entry {
    name: Box<OsStr>,
    size: u64,
    dir: Option<u32>,
}

struct Job {
    id: u32,
    path: PathBuf,
}

/// Shared folder stack: `busy` is how many threads are reading right now; an empty stack and `busy == 0` mean the end.
struct Work {
    stack: Vec<Job>,
    busy: usize,
    next_id: u32,
}

struct Ctx<'a> {
    progress: &'a Progress,
    filter: Filter,
    /// (device, inode) of files with several hard links that were already counted.
    hardlinks: Mutex<HashSet<(u64, u64)>>,
    work: Mutex<Work>,
    wake: Condvar,
}

/// Synchronous (tests and measurements); the app goes through [`Scan::start`].
pub fn scan(root: &Path, progress: &Progress) -> io::Result<Tree> {
    let meta = fs::metadata(root)?;
    let name: Box<OsStr> = root.as_os_str().into();
    if !meta.is_dir() {
        progress.files.fetch_add(1, Relaxed);
        let node = ScanNode { name, size: disk_size(&meta), files: 1, kind: Kind::File, children: vec![] };
        return Ok(Tree::from_scan(root.to_path_buf(), node));
    }
    fs::read_dir(root)?; // "access denied" on the root itself is an error
    let ctx = Ctx {
        progress,
        filter: Filter::new(root),
        hardlinks: Mutex::new(HashSet::new()),
        work: Mutex::new(Work { stack: vec![Job { id: 0, path: root.to_path_buf() }], busy: 0, next_id: 1 }),
        wake: Condvar::new(),
    };
    // Scanning is mostly waiting on the file system: more threads than cores.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let threads = (threads * 2).clamp(4, 32);
    let mut lists: Vec<Vec<Entry>> = Vec::new();
    std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads).map(|_| s.spawn(|| worker(&ctx))).collect();
        for w in workers {
            for (id, list) in w.join().unwrap_or_default() {
                let id = id as usize;
                if lists.len() <= id {
                    lists.resize_with(id + 1, Vec::new);
                }
                lists[id] = list;
            }
        }
    });
    let node = build(&mut lists, 0, name, disk_size(&meta));
    Ok(Tree::from_scan(root.to_path_buf(), node))
}

fn worker(ctx: &Ctx) -> Vec<(u32, Vec<Entry>)> {
    let mut out = Vec::new();
    loop {
        let job = {
            let mut w = ctx.work.lock().unwrap();
            loop {
                if ctx.progress.cancelled() {
                    w.stack.clear();
                }
                if let Some(job) = w.stack.pop() {
                    w.busy += 1;
                    break job;
                }
                if w.busy == 0 {
                    ctx.wake.notify_all();
                    return out;
                }
                w = ctx.wake.wait(w).unwrap();
            }
        };
        let (list, subdirs) = read_dir(ctx, &job.path);
        let mut w = ctx.work.lock().unwrap();
        let mut list = list;
        for (i, path) in subdirs {
            let id = w.next_id;
            w.next_id += 1;
            list[i].dir = Some(id);
            w.stack.push(Job { id, path });
        }
        w.busy -= 1;
        if !w.stack.is_empty() || w.busy == 0 {
            ctx.wake.notify_all();
        }
        drop(w);
        out.push((job.id, list));
    }
}

/// A folder's entries and the subfolders to walk (entry index, path).
fn read_dir(ctx: &Ctx, path: &Path) -> (Vec<Entry>, Vec<(usize, PathBuf)>) {
    let progress = ctx.progress;
    let mut list = Vec::new();
    let mut subdirs = Vec::new();
    let Ok(entries) = fs::read_dir(path) else {
        progress.errors.fetch_add(1, Relaxed);
        return (list, subdirs);
    };
    let (mut files, mut bytes, mut errors) = (0u64, 0u64, 0u64);
    for entry in entries {
        let Ok(entry) = entry else {
            errors += 1;
            continue;
        };
        // DirEntry's metadata doesn't follow symbolic links, so they aren't walked.
        let Ok(meta) = entry.metadata() else {
            errors += 1;
            continue;
        };
        let size = disk_size(&meta);
        if meta.is_dir() {
            let child = entry.path();
            if ctx.filter.allows(&child, &meta) {
                bytes += size;
                subdirs.push((list.len(), child));
                list.push(Entry { name: entry.file_name().into_boxed_os_str(), size, dir: None });
            }
        } else if ctx.first_link(&meta) {
            files += 1;
            bytes += size;
            list.push(Entry { name: entry.file_name().into_boxed_os_str(), size, dir: None });
        }
    }
    progress.files.fetch_add(files, Relaxed);
    progress.bytes.fetch_add(bytes, Relaxed);
    if errors > 0 {
        progress.errors.fetch_add(errors, Relaxed);
    }
    if progress.dirs.fetch_add(1, Relaxed).is_multiple_of(64)
        && let Ok(mut current) = progress.current.try_lock()
    {
        current.clear();
        current.push(path);
    }
    (list, subdirs)
}

/// Tree from the folder lists: recursion as deep as the folders.
fn build(lists: &mut [Vec<Entry>], id: u32, name: Box<OsStr>, own: u64) -> ScanNode {
    let entries = lists.get_mut(id as usize).map(std::mem::take).unwrap_or_default();
    let mut node = ScanNode { name, size: own, files: 0, kind: Kind::Dir, children: Vec::with_capacity(entries.len()) };
    for e in entries {
        node.children.push(match e.dir {
            Some(d) => build(lists, d, e.name, e.size),
            None => ScanNode { name: e.name, size: e.size, files: 1, kind: Kind::File, children: Vec::new() },
        });
    }
    node.finish();
    node
}

impl Ctx<'_> {
    /// `false`: a hard link to an inode already counted.
    #[cfg(unix)]
    fn first_link(&self, meta: &Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() <= 1 {
            return true;
        }
        self.hardlinks.lock().map_or(true, |mut seen| seen.insert((meta.dev(), meta.ino())))
    }

    #[cfg(not(unix))]
    fn first_link(&self, _meta: &Metadata) -> bool {
        let _ = &self.hardlinks;
        true
    }
}

/// Space on disk (allocated blocks, like `du`), not the file length.
#[cfg(unix)]
fn disk_size(meta: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn disk_size(meta: &Metadata) -> u64 {
    meta.len()
}

/// Don't leave the root's file system, like `du -x`.
struct Filter {
    #[cfg(unix)]
    devices: Vec<u64>,
    skip: Vec<PathBuf>,
}

impl Filter {
    #[cfg(unix)]
    fn new(root: &Path) -> Self {
        use std::os::unix::fs::MetadataExt;
        let mut devices = Vec::new();
        let mut skip = Vec::new();
        if let Ok(meta) = fs::metadata(root) {
            devices.push(meta.dev());
        }
        if cfg!(target_os = "macos") {
            // Since Catalina "/" is a read-only system volume and the data is on a separate volume
            // woven into /Users, /Applications, /private… We take both volumes but skip
            // their mount points under /System/Volumes so the data isn't counted twice.
            let volumes = Path::new("/System/Volumes");
            if !root.starts_with(volumes) {
                if let Ok(data) = fs::metadata(volumes.join("Data")) {
                    devices.push(data.dev());
                }
                skip.push(volumes.to_path_buf());
            }
        }
        #[cfg(target_os = "linux")]
        if let Ok(info) = fs::read_to_string("/proc/self/mountinfo") {
            // btrfs subvolumes (/home on Fedora) have their own device number but the same disk.
            for mount in btrfs_subvolume_mounts(&info, root) {
                if let Ok(meta) = fs::metadata(&mount) {
                    devices.push(meta.dev());
                }
            }
        }
        Self { devices, skip }
    }

    #[cfg(not(unix))]
    fn new(_root: &Path) -> Self {
        Self { skip: Vec::new() }
    }

    fn allows(&self, path: &Path, meta: &Metadata) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if !self.devices.is_empty() && !self.devices.contains(&meta.dev()) {
                return false;
            }
        }
        #[cfg(not(unix))]
        let _ = meta;
        !self.skip.iter().any(|s| s == path)
    }
}

/// Mount points under `root` of other btrfs subvolumes on the same device
/// (from `/proc/self/mountinfo`); snapshots are skipped so they aren't counted twice.
#[cfg(any(target_os = "linux", test))]
fn btrfs_subvolume_mounts(mountinfo: &str, root: &Path) -> Vec<PathBuf> {
    // Spaces and the like in paths are octal "\040".
    fn unescape(field: &str) -> PathBuf {
        let bytes = field.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            let octal = bytes.get(i + 1..i + 4).and_then(|d| std::str::from_utf8(d).ok());
            match octal.and_then(|d| u8::from_str_radix(d, 8).ok()) {
                Some(b) if bytes[i] == b'\\' => {
                    out.push(b);
                    i += 4;
                }
                _ => {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
        }
        PathBuf::from(String::from_utf8_lossy(&out).into_owned())
    }

    // "36 35 98:0 /subvol /mnt/point rw,noatime shared:1 - btrfs /dev/sda2 rw"
    let mounts: Vec<(PathBuf, &str, &str)> = mountinfo
        .lines()
        .filter_map(|line| {
            let (left, right) = line.split_once(" - ")?;
            let mount_point = unescape(left.split(' ').nth(4)?);
            let mut right = right.split(' ');
            Some((mount_point, right.next()?, right.next()?))
        })
        .collect();
    let Some((root_mount, fs_type, source)) =
        mounts.iter().filter(|(mp, ..)| root.starts_with(mp)).max_by_key(|(mp, ..)| mp.as_os_str().len())
    else {
        return Vec::new();
    };
    if *fs_type != "btrfs" {
        return Vec::new();
    }
    mounts
        .iter()
        .filter(|(mp, t, src)| {
            t == fs_type && src == source && mp != root_mount && mp.starts_with(root) && !mp.components().any(|c| c.as_os_str() == ".snapshots")
        })
        .map(|(mp, ..)| mp.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::NodeId;

    #[test]
    fn finds_btrfs_subvolumes_of_the_same_device() {
        let info = "\
22 1 0:21 /root / rw,relatime shared:1 - btrfs /dev/nvme0n1p3 rw,subvol=/root
23 22 0:5 / /dev rw - devtmpfs devtmpfs rw
45 22 0:21 /home /home rw,relatime shared:2 - btrfs /dev/nvme0n1p3 rw,subvol=/home
46 22 0:21 /snap /.snapshots rw - btrfs /dev/nvme0n1p3 rw,subvol=/.snapshots
47 22 0:40 / /mnt/usb rw - btrfs /dev/sdb1 rw
48 22 0:21 /data /srv/my\\040data rw - btrfs /dev/nvme0n1p3 rw
";
        let mounts = btrfs_subvolume_mounts(info, Path::new("/"));
        assert_eq!(mounts, [PathBuf::from("/home"), PathBuf::from("/srv/my data")]);
        assert!(btrfs_subvolume_mounts(info, Path::new("/home/me")).is_empty());
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("disk-usage-uison-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn child_named(tree: &Tree, parent: NodeId, name: &str) -> NodeId {
        tree.children(parent).find(|&c| tree.node(c).name_lossy() == name).unwrap()
    }

    #[test]
    fn scans_a_directory() {
        let root = temp_dir("scan");
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::write(root.join("big.bin"), vec![1u8; 64 * 1024]).unwrap();
        fs::write(root.join("a/one.txt"), b"hello").unwrap();
        fs::write(root.join("a/b/two.txt"), vec![2u8; 8 * 1024]).unwrap();
        fs::create_dir(root.join("empty")).unwrap();

        let progress = Progress::default();
        let tree = scan(&root, &progress).unwrap();
        let r = Tree::ROOT;
        assert_eq!(tree.node(r).files, 3);
        assert_eq!(progress.files.load(Relaxed), 3);
        assert_eq!(progress.dirs.load(Relaxed), 4);
        assert_eq!(tree.node(tree.children(r).next().unwrap()).name_lossy(), "big.bin");
        let a = child_named(&tree, r, "a");
        let b = child_named(&tree, a, "b");
        let two = child_named(&tree, b, "two.txt");
        assert_eq!(tree.path(two), root.join("a/b/two.txt"));
        assert_eq!(tree.node(a).files, 2);
        fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn counts_hardlinks_once_and_skips_symlinks() {
        let root = temp_dir("links");
        fs::write(root.join("data.bin"), vec![1u8; 32 * 1024]).unwrap();
        fs::hard_link(root.join("data.bin"), root.join("link.bin")).unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub/x.bin"), vec![1u8; 32 * 1024]).unwrap();
        std::os::unix::fs::symlink(root.join("sub"), root.join("sub-link")).unwrap();
        let tree = scan(&root, &Progress::default()).unwrap();
        let sized: Vec<_> = tree.children(Tree::ROOT).filter(|&c| tree.node(c).size >= 32 * 1024).collect();
        assert_eq!(sized.len(), 2);
        assert!(!tree.node(child_named(&tree, Tree::ROOT, "sub-link")).is_dir());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_root_is_an_error() {
        assert!(scan(Path::new("/definitely/not/here/42"), &Progress::default()).is_err());
    }

    /// `SCAN_BENCH=/path cargo test --release -p disk-usage bench_scan -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_scan() {
        let path = PathBuf::from(std::env::var_os("SCAN_BENCH").expect("SCAN_BENCH"));
        let progress = Progress::default();
        let start = Instant::now();
        let tree = scan(&path, &progress).unwrap();
        println!(
            "{}: {} nodes, {} KiB, {} errors in {} ms",
            path.display(),
            tree.len(),
            tree.node(Tree::ROOT).size / 1024,
            progress.errors.load(Relaxed),
            start.elapsed().as_millis()
        );
    }
}
