//! Windows: the icon goes into the .exe as a resource (Explorer, shortcuts, the taskbar; the uison backend
//! loads it for the window). A .res file is written by hand from `assets/icon.ico` — no resource compiler,
//! no dependencies — and given to the MSVC linker, which accepts .res input. Other systems: nothing to do
//! at build time (macOS and Linux get the icon from the package, see `bundle.sh`).

use std::path::PathBuf;

/// `RT_ICON`: one image; `RT_GROUP_ICON`: the directory of images (what LoadImage looks up by id).
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
/// The group id the backend loads (`MAKEINTRESOURCE(1)`).
const ICON_ID: u16 = 1;
const LANG_EN_US: u16 = 0x0409;

/// One resource: a 32-byte header with numeric type and name, the data, padding to 4 bytes.
fn resource(out: &mut Vec<u8>, kind: u16, name: u16, flags: u16, data: &[u8]) {
    out.extend((data.len() as u32).to_le_bytes()); // DataSize
    out.extend(32u32.to_le_bytes()); // HeaderSize
    out.extend([0xFF, 0xFF]);
    out.extend(kind.to_le_bytes());
    out.extend([0xFF, 0xFF]);
    out.extend(name.to_le_bytes());
    out.extend(0u32.to_le_bytes()); // DataVersion
    out.extend(flags.to_le_bytes()); // MemoryFlags
    out.extend(LANG_EN_US.to_le_bytes());
    out.extend(0u32.to_le_bytes()); // Version
    out.extend(0u32.to_le_bytes()); // Characteristics
    out.extend(data);
    out.resize(out.len().next_multiple_of(4), 0);
}

/// .ico (directory + images) → .res: each image an `RT_ICON` 1…n, the group `RT_GROUP_ICON` `ICON_ID`.
fn ico_to_res(ico: &[u8]) -> Vec<u8> {
    let u16_at = |i: usize| u16::from_le_bytes([ico[i], ico[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes(ico[i..i + 4].try_into().unwrap());
    assert!(ico.len() >= 6 && u16_at(2) == 1, "assets/icon.ico is not an icon");
    let count = u16_at(4) as usize;
    let mut res = Vec::new();
    resource(&mut res, 0, 0, 0, &[]); // a .res file starts with an empty entry
    let mut group = Vec::new();
    group.extend([0, 0, 1, 0]); // reserved, type 1 (icon)
    group.extend((count as u16).to_le_bytes());
    for i in 0..count {
        let e = 6 + 16 * i;
        let (size, offset) = (u32_at(e + 8) as usize, u32_at(e + 12) as usize);
        let id = i as u16 + 1;
        resource(&mut res, RT_ICON, id, 0x1010, &ico[offset..offset + size]);
        // GRPICONDIRENTRY: the ICONDIRENTRY's first 12 bytes, then the resource id instead of the offset.
        group.extend(&ico[e..e + 12]);
        group.extend(id.to_le_bytes());
    }
    resource(&mut res, RT_GROUP_ICON, ICON_ID, 0x1030, &group);
    res
}

fn main() {
    println!("cargo::rerun-if-changed=assets/icon.ico");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "windows" || env != "msvc" {
        return;
    }
    let ico = std::fs::read("assets/icon.ico").expect("assets/icon.ico");
    let res = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("icon.res");
    std::fs::write(&res, ico_to_res(&ico)).expect("icon.res");
    println!("cargo::rustc-link-arg-bins={}", res.display());
}
