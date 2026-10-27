//! uison-ssh-player: connects to a server over SSH, uploads a program if asked, runs it in console mode and
//! shows it — the same player as `uison-player`, with an SSH channel for the pipe.
//!
//! `uison-ssh-player [[user@]host[:port]] [-i key]` fills the start screen. The session lives in its own
//! thread (tokio on one thread): connect once, then upload and run as often as needed. The server needs
//! nothing but `sshd` and a POSIX shell: uploading is `cat > path && chmod +x path`.
//!
//! The host key is checked against `~/.ssh/known_hosts` and the player's own list
//! (`~/.config/uison/known_hosts`, `UISON_KNOWN_HOSTS`): a new host is remembered, a changed key refused.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use russh::client::{self, Handle};
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::{ChannelMsg, Disconnect};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use uison_player::uison::ui::Data;
use uison_player::{Link, Poster, What};

/// What the UI asks of the session.
enum Cmd {
    Upload { local: PathBuf, remote: String },
    Run { line: String, name: String, post: Poster },
    /// A line to the running program's stdin.
    Line(String),
    Stop,
    Disconnect,
}

struct Params {
    host: String,
    port: u16,
    user: String,
    /// `None` — a key file.
    password: Option<String>,
    key: PathBuf,
    passphrase: Option<String>,
}

/// The running program, as the player sees it: lines and stop go to the session.
struct Remote(UnboundedSender<Cmd>);

impl Link for Remote {
    fn send(&mut self, line: &str) -> bool {
        self.0.send(Cmd::Line(line.to_string())).is_ok()
    }

    fn stop(&mut self) {
        let _ = self.0.send(Cmd::Stop);
    }
}

fn home() -> String {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    if home.is_empty() { data_dir().to_string_lossy().into_owned() } else { home }
}

/// Where the player keeps its key and its list of hosts: the app's own folder on a phone.
fn data_dir() -> PathBuf {
    if cfg!(target_os = "android") {
        // `UISON_FILES` is `files/assets`, refilled from the APK on every update: keep ours one level up.
        let assets = std::env::var_os("UISON_FILES").map_or_else(|| PathBuf::from("."), PathBuf::from);
        assets.parent().map_or(assets.clone(), PathBuf::from)
    } else if cfg!(target_os = "ios") {
        PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Library")
    } else {
        PathBuf::from(std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default()).join(".config/uison")
    }
}

/// A private key from a file, possibly with Windows line ends or flattened (spaces for line breaks): the
/// `-----BEGIN …-----` line, the base64 body in lines of 70, the `-----END …-----` line.
fn normalize_key(text: &str) -> Option<String> {
    let text = text.trim();
    let begin = text.find("-----BEGIN ")?;
    let head_end = begin + text[begin + 11..].find("-----")? + 11 + 5;
    let end = text.rfind("-----END ")?;
    let body: String = text[head_end..end].chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = String::from(&text[begin..head_end]);
    out.push('\n');
    for chunk in body.as_bytes().chunks(70) {
        out.push_str(std::str::from_utf8(chunk).ok()?);
        out.push('\n');
    }
    out.push_str(text[end..].trim());
    out.push('\n');
    Some(out)
}

/// Keep a chosen key in the app's folder under its own name (only this app can read it). Returns its file
/// and public line.
fn import_key(text: &str, name: &str, passphrase: Option<&str>) -> Result<(PathBuf, String), String> {
    let pem = normalize_key(text).ok_or("Not a private key file (-----BEGIN … PRIVATE KEY-----).")?;
    let key = russh::keys::decode_secret_key(&pem, passphrase).map_err(|e| format!("Not a usable key: {e}"))?;
    let public = key.public_key().to_openssh().map_err(|e| e.to_string())?;
    let path = data_dir().join(name);
    std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
    let mut file = std::fs::OpenOptions::new();
    file.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut file, 0o600);
    std::io::Write::write_all(&mut file.open(&path).map_err(|e| format!("{}: {e}", path.display()))?, pem.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok((path, public))
}

/// The device's own key (created on first use): its file and the public line for `authorized_keys`.
fn device_key() -> Result<(PathBuf, String), String> {
    use std::io::Read;
    let path = data_dir().join("id_ed25519");
    let key = if path.exists() {
        russh::keys::load_secret_key(&path, None).map_err(|e| format!("{}: {e}", path.display()))?
    } else {
        let mut seed = [0u8; 32];
        std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut seed)).map_err(|e| format!("no random source: {e}"))?;
        let mut key = russh::keys::PrivateKey::from(russh::keys::ssh_key::private::Ed25519Keypair::from_seed(&seed));
        key.set_comment("uison-ssh-player");
        let text = key.to_openssh(russh::keys::ssh_key::LineEnding::LF).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(data_dir()).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new();
        file.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut file, 0o600);
        std::io::Write::write_all(&mut file.open(&path).map_err(|e| format!("{}: {e}", path.display()))?, text.as_bytes())
            .map_err(|e| e.to_string())?;
        key
    };
    let public = key.public_key().to_openssh().map_err(|e| e.to_string())?;
    // Also to the log (logcat on Android): a way to take the line off a phone.
    eprintln!("uison-ssh-player: public key: {public}");
    Ok((path, public))
}

/// `~/x` on this computer; a bare name — a file in the player's folder (keys kept there).
fn local_path(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        return PathBuf::from(home()).join(rest);
    }
    let own = data_dir().join(p);
    if !p.is_empty() && !p.contains(['/', '\\']) && own.exists() { own } else { PathBuf::from(p) }
}

/// A key file for the key field: its name if it is in the player's folder.
fn key_field(path: &std::path::Path) -> String {
    match path.file_name() {
        Some(name) if path.parent() == Some(data_dir().as_path()) => name.to_string_lossy().into_owned(),
        _ => path.to_string_lossy().into_owned(),
    }
}

/// A path for the server's shell: quoted, `~/` left to the shell.
fn shell_path(p: &str) -> String {
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    match p.strip_prefix("~/") {
        Some(rest) => format!("~/{}", quote(rest)),
        None => quote(p),
    }
}

fn known_hosts() -> PathBuf {
    std::env::var_os("UISON_KNOWN_HOSTS").map_or_else(|| data_dir().join("known_hosts"), PathBuf::from)
}

struct Client {
    host: String,
    port: u16,
    info: Poster,
    /// What the host key check said; its message stays on the screen.
    seen: Arc<Seen>,
}

#[derive(Default)]
struct Seen {
    /// A new host was remembered.
    new_host: AtomicBool,
    /// The key was refused (changed, a certificate): the connection error adds nothing.
    refused: AtomicBool,
}

impl client::Handler for Client {
    type Error = russh::Error;

    async fn check_server_key(&mut self, key: &PublicKeyOrCertificate) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = key else {
            self.info.post(What::Info("The server offers a certificate; only plain host keys are supported.".into()));
            self.seen.refused.store(true, Ordering::Relaxed);
            return Ok(false);
        };
        let fingerprint = key.fingerprint(Default::default());
        // The user's own list first (what `ssh` knows), then ours.
        let system = russh::keys::check_known_hosts(&self.host, self.port, key);
        let ours = russh::keys::check_known_hosts_path(&self.host, self.port, key, known_hosts());
        match (system, ours) {
            (Ok(true), _) | (_, Ok(true)) => Ok(true),
            (Err(russh::keys::Error::KeyChanged { .. }), _) | (_, Err(russh::keys::Error::KeyChanged { .. })) => {
                self.info.post(What::Info(format!(
                    "The host key of {} has CHANGED ({fingerprint}). Refused: if that is expected, remove the old key from known_hosts.",
                    self.host
                )));
                self.seen.refused.store(true, Ordering::Relaxed);
                Ok(false)
            }
            _ => {
                // A new host: remember it, as `ssh -o StrictHostKeyChecking=accept-new` does.
                let _ = russh::keys::known_hosts::learn_known_hosts_path(&self.host, self.port, key, known_hosts());
                self.info.post(What::Info(format!(
                    "First connection to {}: the server's identity ({fingerprint}) is saved and checked next time.",
                    self.host
                )));
                self.seen.new_host.store(true, Ordering::Relaxed);
                Ok(true)
            }
        }
    }
}

async fn connect(p: &Params, info: &Poster, seen: Arc<Seen>) -> Result<Handle<Client>, String> {
    let config = Arc::new(client::Config { inactivity_timeout: None, ..Default::default() });
    let handler = Client { host: p.host.clone(), port: p.port, info: info.clone(), seen };
    let addr = (p.host.as_str(), p.port);
    let mut s = tokio::time::timeout(Duration::from_secs(15), client::connect(config, addr, handler))
        .await
        .map_err(|_| format!("{}:{}: no answer in 15 s", p.host, p.port))?
        .map_err(|e| format!("{}:{}: {e}", p.host, p.port))?;
    let auth = match &p.password {
        Some(pw) => s.authenticate_password(&p.user, pw).await,
        None => {
            let key = russh::keys::load_secret_key(&p.key, p.passphrase.as_deref()).map_err(|e| format!("{}: {e}", p.key.display()))?;
            let hash = s.best_supported_rsa_hash().await.map_err(|e| e.to_string())?.flatten();
            s.authenticate_publickey(&p.user, PrivateKeyWithHashAlg::new(Arc::new(key), hash)).await
        }
    }
    .map_err(|e| e.to_string())?;
    if !auth.success() {
        return Err(format!("{}@{}: the server did not accept the {}.", p.user, p.host, if p.password.is_some() { "password" } else { "key" }));
    }
    Ok(s)
}

/// Run a short command and collect its stdout (with the exit status).
async fn capture(s: &Handle<Client>, cmd: &str, stdin: Option<&[u8]>) -> Result<(u32, String, String), String> {
    let ch = s.channel_open_session().await.map_err(|e| e.to_string())?;
    let (mut read, write) = ch.split();
    write.exec(true, cmd).await.map_err(|e| e.to_string())?;
    if let Some(data) = stdin {
        write.data(data).await.map_err(|e| e.to_string())?;
        write.eof().await.map_err(|e| e.to_string())?;
    }
    let (mut out, mut err, mut status) = (Vec::new(), Vec::new(), 0);
    while let Some(msg) = read.wait().await {
        match msg {
            ChannelMsg::Data { data } => out.extend_from_slice(&data),
            ChannelMsg::ExtendedData { data, .. } => err.extend_from_slice(&data),
            ChannelMsg::ExitStatus { exit_status } => status = exit_status,
            _ => {}
        }
    }
    Ok((status, String::from_utf8_lossy(&out).into_owned(), String::from_utf8_lossy(&err).into_owned()))
}

/// Lines of a byte stream: complete ones go to `post`, the rest waits for more.
fn lines(buf: &mut Vec<u8>, data: &[u8], post: &Poster, wrap: fn(String) -> What) {
    buf.extend_from_slice(data);
    while let Some(i) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=i).collect();
        post.post(wrap(String::from_utf8_lossy(&line[..line.len() - 1]).into_owned()));
    }
}

/// The program's run: its stdout and stderr to the player, the player's lines to its stdin.
/// `false` — the session must end (disconnect).
async fn run(s: &Handle<Client>, line: &str, name: String, post: Poster, rx: &mut UnboundedReceiver<Cmd>, info: &Poster) -> bool {
    let ch = match s.channel_open_session().await {
        Ok(ch) => ch,
        Err(e) => {
            post.post(What::Exit(e.to_string()));
            return true;
        }
    };
    let (mut read, write) = ch.split();
    if let Err(e) = write.exec(true, line).await {
        post.post(What::Exit(e.to_string()));
        return true;
    }
    post.post(What::Started(name));
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut status = None;
    let mut stopping = false;
    loop {
        tokio::select! {
            msg = read.wait() => match msg {
                Some(ChannelMsg::Data { data }) => lines(&mut out, &data, &post, What::Out),
                Some(ChannelMsg::ExtendedData { data, .. }) => lines(&mut err, &data, &post, What::Err),
                Some(ChannelMsg::ExitStatus { exit_status }) => status = Some(exit_status),
                Some(ChannelMsg::ExitSignal { signal_name, .. }) => status = status.or(Some(128)).inspect(|_| {
                    post.post(What::Err(format!("killed by signal {signal_name:?}")));
                }),
                Some(_) => {}
                None => {
                    post.post(What::Exit(status.map_or("the channel closed".into(), |s| format!("exit status: {s}"))));
                    return true;
                }
            },
            // Stopping: the program has a second to exit after its stdin closes.
            _ = tokio::time::sleep(Duration::from_secs(1)), if stopping => {
                let _ = write.close().await;
                stopping = false;
            }
            cmd = rx.recv() => match cmd {
                Some(Cmd::Line(l)) => {
                    let mut b = l.into_bytes();
                    b.push(b'\n');
                    if write.data(&b[..]).await.is_err() {
                        post.post(What::Exit("the channel closed".into()));
                        return true;
                    }
                }
                Some(Cmd::Stop) => {
                    let _ = write.eof().await;
                    stopping = true;
                }
                Some(Cmd::Upload { .. } | Cmd::Run { .. }) => info.post(What::Info("Stop the program first.".into())),
                None | Some(Cmd::Disconnect) => {
                    let _ = write.close().await;
                    return false;
                }
            },
        }
    }
}

async fn session(p: Params, info: Poster, mut rx: UnboundedReceiver<Cmd>) {
    info.post(What::Info(format!("Connecting to {}…", p.host)));
    let seen = Arc::new(Seen::default());
    let s = match connect(&p, &info, seen.clone()).await {
        Ok(s) => s,
        Err(e) => {
            if !seen.refused.load(Ordering::Relaxed) {
                info.post(What::Info(e));
            }
            return;
        }
    };
    let system = capture(&s, "uname -sm", None).await.map(|(_, out, _)| out.trim().to_string()).unwrap_or_default();
    let by = match &p.password {
        Some(_) => "password".to_string(),
        None => format!("key {}", p.key.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())),
    };
    info.post(What::Set("server".into(), format!("{}@{} · {system} · {by}", p.user, p.host)));
    info.post(What::Set("connected".into(), "yes".into()));
    if !seen.new_host.load(Ordering::Relaxed) {
        info.post(What::Info(String::new()));
    }
    while let Some(cmd) = rx.recv().await {
        match cmd {
            Cmd::Upload { local, remote } => {
                let Ok(bytes) = std::fs::read(&local) else {
                    info.post(What::Info(format!("{}: can't read the file.", local.display())));
                    continue;
                };
                info.post(What::Info(format!("Uploading {} KB…", bytes.len() / 1024)));
                let q = shell_path(&remote);
                let cmd = format!("mkdir -p \"$(dirname {q})\" && cat > {q} && chmod +x {q}");
                match capture(&s, &cmd, Some(&bytes)).await {
                    Ok((0, _, _)) => {
                        info.post(What::Info(format!("Uploaded to {remote} ({} KB), executable.", bytes.len() / 1024)));
                        info.post(What::Set("path".into(), remote));
                    }
                    Ok((code, _, err)) => info.post(What::Info(format!("Upload failed ({code}): {}", err.trim()))),
                    Err(e) => info.post(What::Info(format!("Upload failed: {e}"))),
                }
            }
            Cmd::Run { line, name, post } => {
                if !run(&s, &line, name, post, &mut rx, &info).await {
                    break;
                }
            }
            Cmd::Line(_) | Cmd::Stop => {}
            Cmd::Disconnect => break,
        }
    }
    let _ = s.disconnect(Disconnect::ByApplication, "", "").await;
    info.post(What::Set("connected".into(), String::new()));
    info.post(What::Info("Disconnected.".into()));
}

/// The session in its own thread; the UI talks to it through the sender.
fn start_session(p: Params, info: Poster) -> UnboundedSender<Cmd> {
    let (tx, rx) = unbounded_channel();
    std::thread::spawn(move || match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt.block_on(session(p, info, rx)),
        Err(e) => info.post(What::Info(e.to_string())),
    });
    tx
}

fn params(d: &Data) -> Result<Params, String> {
    let host = d.str("host").trim().to_string();
    if host.is_empty() {
        return Err("Type the server's address.".into());
    }
    let port = match d.str("port").trim() {
        "" => 22,
        p => p.parse().map_err(|_| format!("Port {p}: not a number."))?,
    };
    let opt = |k: &str| Some(d.str(k).to_string()).filter(|s| !s.is_empty());
    Ok(Params {
        host,
        port,
        user: d.str("user").trim().to_string(),
        password: (d.str("auth") == "password").then(|| d.str("password").to_string()),
        key: local_path(d.str("keyPath").trim()),
        passphrase: opt("passphrase"),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let session: Rc<RefCell<Option<UnboundedSender<Cmd>>>> = Rc::default();
    let app = uison_player::build(Some(include_str!("../launcher.json")), |ui, st| {
        let d = &mut ui.data;
        d.set("host", "");
        d.set("port", "22");
        d.set("user", std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default());
        d.set("auth", "key");
        d.set("password", "");
        d.set("keyPath", "~/.ssh/id_ed25519");
        d.set("passphrase", "");
        d.set("connected", "");
        d.set("server", "");
        d.set("local", "");
        d.set("remote", "");
        d.set("path", "");
        d.set("args", "");
        d.set("console", true);
        d.set("pubkey", "");
        // A phone: nothing to upload, a key of its own, desktop programs drawn smaller.
        let phone = cfg!(any(target_os = "android", target_os = "ios"));
        d.set("phone", phone);
        if phone {
            st.borrow_mut().set_min_width(960.0);
            // A key in the app's folder: made here (`id_ed25519`) or put there (`*.pem`, `adb shell run-as`).
            let dir = data_dir();
            let pem = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "pem"));
            let key = Some(dir.join("id_ed25519")).filter(|p| p.exists()).or(pem);
            d.set("keyPath", key.map(|p| key_field(&p)).unwrap_or_default());
        }
        // `[user@]host[:port] [-i key]`.
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if a == "-i" {
                if let Some(k) = it.next() {
                    d.set("keyPath", k.as_str());
                }
                continue;
            }
            let (user, rest) = a.split_once('@').map_or((None, a.as_str()), |(u, r)| (Some(u), r));
            let (host, port) = rest.rsplit_once(':').map_or((rest, None), |(h, p)| (h, Some(p)));
            if let Some(u) = user {
                d.set("user", u);
            }
            d.set("host", host);
            if let Some(p) = port {
                d.set("port", p);
            }
        }

        let (s, sess) = (st.clone(), session.clone());
        ui.on("connect", move |cx| {
            if let Some(old) = sess.borrow_mut().take() {
                let _ = old.send(Cmd::Disconnect);
            }
            match params(cx.data) {
                Ok(p) => *sess.borrow_mut() = Some(start_session(p, s.borrow().poster())),
                Err(e) => cx.data.set("status", e),
            }
        });
        let (s, sess) = (st.clone(), session.clone());
        ui.on("disconnect", move |_| {
            s.borrow_mut().stop();
            if let Some(tx) = sess.borrow_mut().take() {
                let _ = tx.send(Cmd::Disconnect);
            }
        });
        ui.on("newKey", |cx| match device_key() {
            Ok((path, public)) => {
                cx.data.set("keyPath", key_field(&path));
                cx.data.set("pubkey", public);
                cx.data.set("status", "");
            }
            Err(e) => cx.data.set("status", e),
        });
        ui.on("chooseKey", |cx| cx.pick_file("keyChosen"));
        ui.on("keyChosen", |cx| {
            if cx.event == "unsupported" {
                cx.data.set("status", "Type the key file's path.");
                return;
            }
            let chosen = PathBuf::from(cx.key);
            if cx.key.is_empty() {
                return;
            }
            let name = chosen.file_name().map_or_else(|| "key".to_string(), |n| n.to_string_lossy().into_owned());
            let passphrase = Some(cx.data.str("passphrase").to_string()).filter(|p| !p.is_empty());
            let result = std::fs::read_to_string(&chosen).map_err(|e| e.to_string()).and_then(|text| import_key(&text, &name, passphrase.as_deref()));
            // The chooser's copy goes either way: a usable key now lives in the app's folder.
            let _ = std::fs::remove_file(&chosen);
            match result {
                Ok((path, public)) => {
                    cx.data.set("keyPath", key_field(&path));
                    cx.data.set("pubkey", public);
                    cx.data.set("status", "Key kept in the app's own folder.");
                }
                Err(e) => cx.data.set("status", e),
            }
        });
        ui.on("copyKey", |cx| {
            let key = cx.data.str("pubkey").to_string();
            cx.copy(&key);
            cx.data.set("status", "The public key is in the clipboard.");
        });
        let sess = session.clone();
        ui.on("upload", move |cx| {
            let local = cx.data.str("local").trim().to_string();
            if local.is_empty() {
                cx.data.set("status", "Choose a file to upload.");
                return;
            }
            let local = local_path(&local);
            let mut remote = cx.data.str("remote").trim().to_string();
            if remote.is_empty() {
                let name = local.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                remote = format!("~/uison/{name}");
                cx.data.set("remote", remote.as_str());
            }
            if let Some(tx) = sess.borrow().as_ref() {
                let _ = tx.send(Cmd::Upload { local, remote });
            }
        });
        let (s, sess) = (st.clone(), session.clone());
        ui.on("run", move |cx| {
            let path = cx.data.str("path").trim().to_string();
            if path.is_empty() {
                cx.data.set("status", "Type the program's path on the server.");
                return;
            }
            let Some(tx) = sess.borrow().as_ref().cloned() else {
                cx.data.set("status", "Connect first.");
                return;
            };
            let mut line = shell_path(&path);
            let args = cx.data.str("args").trim();
            if !args.is_empty() {
                line.push(' ');
                line.push_str(args);
            }
            if cx.data.bool("console") {
                line.push_str(" --console");
            }
            let name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let post = s.borrow_mut().begin(&line);
            s.borrow_mut().attach(Box::new(Remote(tx.clone())));
            let _ = tx.send(Cmd::Run { line, name, post });
            cx.data.set("status", "Starting…");
        });
    });
    uison_player::uison::run(app, "uison SSH player");
}

#[cfg(test)]
mod tests {
    use super::normalize_key;

    #[test]
    fn a_flattened_key_is_restored() {
        let key = russh::keys::PrivateKey::from(russh::keys::ssh_key::private::Ed25519Keypair::from_seed(&[7; 32]));
        let text = key.to_openssh(russh::keys::ssh_key::LineEnding::LF).unwrap();
        // As a text field, a chat or Windows would give it back: other line breaks, some noise around.
        let flat = format!("  my key:\n{} \n", text.replace('\n', " "));
        let restored = normalize_key(&flat).unwrap();
        let back = russh::keys::decode_secret_key(&restored, None).unwrap();
        assert_eq!(back.public_key(), key.public_key());
        // Untouched text survives too; no key — no result.
        assert!(russh::keys::decode_secret_key(&normalize_key(&text).unwrap(), None).is_ok());
        assert!(normalize_key("ssh-ed25519 AAAA… a public key").is_none());
    }
}
