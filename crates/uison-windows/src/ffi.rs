//! Hand-written FFI to Win32: only what the backend needs. Strings are UTF-16.

#![allow(non_snake_case, non_camel_case_types, clippy::upper_case_acronyms)]

use std::ffi::c_void;

use crate::com::{GUID, HRESULT};

pub type HWND = *mut c_void;
pub type HINSTANCE = *mut c_void;
pub type HDC = *mut c_void;
pub type HRGN = *mut c_void;
pub type HBITMAP = *mut c_void;
pub type HCURSOR = *mut c_void;
pub type HMONITOR = *mut c_void;
pub type HKEY = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type BOOL = i32;
pub type WNDPROC = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

#[repr(C)]
pub struct WNDCLASSEXW {
    pub cbSize: u32,
    pub style: u32,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: *mut c_void,
    pub hCursor: HCURSOR,
    pub hbrBackground: *mut c_void,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
    pub hIconSm: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: u32,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: u32,
    pub pt: POINT,
    pub lPrivate: u32,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

#[repr(C)]
pub struct TRACKMOUSEEVENT {
    pub cbSize: u32,
    pub dwFlags: u32,
    pub hwndTrack: HWND,
    pub dwHoverTime: u32,
}

#[repr(C)]
pub struct MONITORINFO {
    pub cbSize: u32,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: u32,
}

/// `RGNDATA` header; followed by `nCount` `RECT` rectangles.
#[repr(C)]
pub struct RGNDATAHEADER {
    pub dwSize: u32,
    pub iType: u32,
    pub nCount: u32,
    pub nRgnSize: u32,
    pub rcBound: RECT,
}

#[repr(C)]
pub struct BITMAPINFOHEADER {
    pub biSize: u32,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: u32,
    pub biSizeImage: u32,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: u32,
    pub biClrImportant: u32,
}

// Window messages.
pub const WM_DESTROY: u32 = 0x0002;
pub const WM_SIZE: u32 = 0x0005;
pub const WM_PAINT: u32 = 0x000F;
pub const WM_ERASEBKGND: u32 = 0x0014;
pub const WM_SETTINGCHANGE: u32 = 0x001A;
pub const WM_SETCURSOR: u32 = 0x0020;
pub const WM_KEYDOWN: u32 = 0x0100;
pub const WM_CHAR: u32 = 0x0102;
pub const WM_SYSKEYDOWN: u32 = 0x0104;
pub const WM_TIMER: u32 = 0x0113;
pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_MOUSEWHEEL: u32 = 0x020A;
pub const WM_MOUSEHWHEEL: u32 = 0x020E;
pub const WM_CAPTURECHANGED: u32 = 0x0215;
pub const WM_ENTERSIZEMOVE: u32 = 0x0231;
pub const WM_EXITSIZEMOVE: u32 = 0x0232;
pub const WM_POINTERUPDATE: u32 = 0x0245;
pub const WM_POINTERDOWN: u32 = 0x0246;
pub const WM_POINTERUP: u32 = 0x0247;
pub const WM_POINTERCAPTURECHANGED: u32 = 0x024C;
pub const WM_MOUSELEAVE: u32 = 0x02A3;
pub const WM_DPICHANGED: u32 = 0x02E0;
pub const WM_GETDPISCALEDSIZE: u32 = 0x02E4;
pub const WM_APP: u32 = 0x8000;

pub const CS_OWNDC: u32 = 0x0020;
pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
pub const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
pub const SW_SHOW: i32 = 5;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const HTCLIENT: u16 = 1;
pub const TME_LEAVE: u32 = 0x0002;
pub const MONITOR_DEFAULTTONEAREST: u32 = 2;
pub const PT_TOUCH: u32 = 2;
/// The pointer is touching the screen (flags in the high word of wParam for `WM_POINTER*`).
pub const POINTER_FLAG_INCONTACT: u32 = 0x0004;
pub const SPI_GETWHEELSCROLLLINES: u32 = 0x0068;
pub const PW_RENDERFULLCONTENT: u32 = 2;
pub const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;

pub const IDC_ARROW: usize = 32512;
pub const IMAGE_ICON: u32 = 1;
pub const LR_SHARED: u32 = 0x8000;
pub const SM_CXICON: i32 = 11;
pub const SM_CXSMICON: i32 = 49;
pub const WM_SETICON: u32 = 0x0080;
pub const ICON_SMALL: usize = 0;
pub const ICON_BIG: usize = 1;
pub const IDC_IBEAM: usize = 32513;
pub const IDC_HAND: usize = 32649;

pub const VK_BACK: u32 = 0x08;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_PRIOR: u32 = 0x21;
pub const VK_NEXT: u32 = 0x22;
pub const VK_END: u32 = 0x23;
pub const VK_HOME: u32 = 0x24;
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;
pub const VK_DELETE: u32 = 0x2E;
pub const VK_APPS: u32 = 0x5D;
pub const VK_F10: u32 = 0x79;
pub const MAPVK_VK_TO_CHAR: u32 = 2;

pub const HKEY_CURRENT_USER: HKEY = 0x8000_0001_u32 as i32 as isize;
pub const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
pub const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
pub const COINIT_APARTMENTTHREADED: u32 = 0x2;
pub const CLSCTX_INPROC_SERVER: u32 = 0x1;
pub const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
pub const STD_ERROR_HANDLE: u32 = -12i32 as u32;
pub const ATTACH_PARENT_PROCESS: u32 = -1i32 as u32;

#[link(name = "user32")]
unsafe extern "system" {
    pub fn RegisterClassExW(wc: *const WNDCLASSEXW) -> u16;
    pub fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: HWND,
        menu: *mut c_void,
        inst: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    pub fn DefWindowProcW(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT;
    pub fn GetMessageW(msg: *mut MSG, h: HWND, min: u32, max: u32) -> BOOL;
    pub fn TranslateMessage(msg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    pub fn PostMessageW(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> BOOL;
    pub fn PostQuitMessage(code: i32);
    pub fn ShowWindow(h: HWND, cmd: i32) -> BOOL;
    pub fn SetWindowPos(h: HWND, after: HWND, x: i32, y: i32, w: i32, cy: i32, flags: u32) -> BOOL;
    pub fn GetClientRect(h: HWND, r: *mut RECT) -> BOOL;
    pub fn InvalidateRect(h: HWND, r: *const RECT, erase: BOOL) -> BOOL;
    pub fn ValidateRect(h: HWND, r: *const RECT) -> BOOL;
    pub fn UpdateWindow(h: HWND) -> BOOL;
    pub fn GetUpdateRgn(h: HWND, rgn: HRGN, erase: BOOL) -> i32;
    pub fn BeginPaint(h: HWND, ps: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(h: HWND, ps: *const PAINTSTRUCT) -> BOOL;
    pub fn GetDC(h: HWND) -> HDC;
    pub fn ReleaseDC(h: HWND, dc: HDC) -> i32;
    pub fn PrintWindow(h: HWND, dc: HDC, flags: u32) -> BOOL;
    pub fn SetTimer(h: HWND, id: usize, ms: u32, f: *const c_void) -> usize;
    pub fn KillTimer(h: HWND, id: usize) -> BOOL;
    pub fn LoadCursorW(inst: HINSTANCE, name: *const u16) -> HCURSOR;
    pub fn LoadImageW(inst: HINSTANCE, name: *const u16, kind: u32, cx: i32, cy: i32, flags: u32) -> *mut c_void;
    pub fn CreateIconIndirect(info: *const ICONINFO) -> *mut c_void;
    pub fn GetSystemMetrics(index: i32) -> i32;
    pub fn GetSystemMetricsForDpi(index: i32, dpi: u32) -> i32;
    pub fn SendMessageW(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT;
    pub fn DestroyIcon(icon: *mut c_void) -> BOOL;
    pub fn SetCursor(c: HCURSOR) -> HCURSOR;
    pub fn SetCapture(h: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn GetCapture() -> HWND;
    pub fn TrackMouseEvent(t: *mut TRACKMOUSEEVENT) -> BOOL;
    pub fn GetKeyState(vk: i32) -> i16;
    pub fn MapVirtualKeyW(code: u32, kind: u32) -> u32;
    pub fn SetProcessDpiAwarenessContext(ctx: isize) -> BOOL;
    pub fn GetDpiForWindow(h: HWND) -> u32;
    pub fn AdjustWindowRectExForDpi(r: *mut RECT, style: u32, menu: BOOL, ex: u32, dpi: u32) -> BOOL;
    pub fn ScreenToClient(h: HWND, p: *mut POINT) -> BOOL;
    pub fn SystemParametersInfoW(action: u32, param: u32, pv: *mut c_void, ini: u32) -> BOOL;
    pub fn GetPointerType(id: u32, kind: *mut u32) -> BOOL;
    pub fn MonitorFromWindow(h: HWND, flags: u32) -> HMONITOR;
    pub fn GetMonitorInfoW(m: HMONITOR, info: *mut MONITORINFO) -> BOOL;
    pub fn OpenClipboard(h: HWND) -> BOOL;
    pub fn CloseClipboard() -> BOOL;
    pub fn EmptyClipboard() -> BOOL;
    pub fn SetClipboardData(format: u32, mem: *mut c_void) -> *mut c_void;
    pub fn GetClipboardData(format: u32) -> *mut c_void;
}

pub const CF_UNICODETEXT: u32 = 13;
pub const GMEM_MOVEABLE: u32 = 0x2;
pub const SW_SHOWNORMAL: i32 = 1;

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn ShellExecuteW(h: HWND, verb: *const u16, file: *const u16, params: *const u16, dir: *const u16, show: i32) -> HINSTANCE;
}

#[repr(C)]
pub struct ICONINFO {
    pub fIcon: BOOL,
    pub xHotspot: u32,
    pub yHotspot: u32,
    pub hbmMask: HBITMAP,
    pub hbmColor: HBITMAP,
}

#[link(name = "gdi32")]
unsafe extern "system" {
    pub fn CreateRectRgn(l: i32, t: i32, r: i32, b: i32) -> HRGN;
    pub fn GetRegionData(rgn: HRGN, size: u32, data: *mut c_void) -> u32;
    pub fn DeleteObject(o: *mut c_void) -> BOOL;
    pub fn CreateBitmap(w: i32, h: i32, planes: u32, bits_per_pixel: u32, bits: *const c_void) -> HBITMAP;
    pub fn CreateCompatibleDC(dc: HDC) -> HDC;
    pub fn DeleteDC(dc: HDC) -> BOOL;
    pub fn SelectObject(dc: HDC, o: *mut c_void) -> *mut c_void;
    pub fn CreateDIBSection(
        dc: HDC,
        info: *const BITMAPINFOHEADER,
        usage: u32,
        bits: *mut *mut c_void,
        section: *mut c_void,
        offset: u32,
    ) -> HBITMAP;
}

#[link(name = "winmm")]
unsafe extern "system" {
    pub fn timeBeginPeriod(ms: u32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    pub fn GetStdHandle(id: u32) -> *mut c_void;
    pub fn AttachConsole(pid: u32) -> BOOL;
    pub fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    pub fn GlobalLock(mem: *mut c_void) -> *mut c_void;
    pub fn GlobalUnlock(mem: *mut c_void) -> BOOL;
    pub fn GlobalFree(mem: *mut c_void) -> *mut c_void;
    pub fn GlobalSize(mem: *mut c_void) -> usize;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    pub fn RegGetValueW(
        key: HKEY,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut c_void,
        size: *mut u32,
    ) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    pub fn DwmSetWindowAttribute(h: HWND, attr: u32, value: *const c_void, size: u32) -> HRESULT;
    pub fn DwmFlush() -> HRESULT;
}

#[link(name = "ole32")]
unsafe extern "system" {
    pub fn CoInitializeEx(reserved: *mut c_void, flags: u32) -> HRESULT;
    pub fn CoCreateInstance(
        clsid: *const GUID,
        outer: *mut c_void,
        ctx: u32,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> HRESULT;
}

/// A NUL-terminated UTF-16 string.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn loword(v: usize) -> u16 {
    v as u16
}

pub fn hiword(v: usize) -> u16 {
    (v >> 16) as u16
}

/// Mouse coordinates from lParam are signed (negative while captured).
pub fn lparam_point(l: LPARAM) -> (i32, i32) {
    (l as u16 as i16 as i32, (l >> 16) as u16 as i16 as i32)
}
