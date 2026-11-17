//! The on-board computer's window (docs/superpowers/specs/2026-11-17-boordcomputer-design.md,
//! "The window"): a second window over the game with the Companion's `/device` screen, served by
//! Omsi-Hub's own companion server on this computer (`hub_api::companion::device_url`).
//!
//! * Opened when a drive's game starts (`game-started`), closed when it ends (`game-ended`). On
//!   Linux under Wayland it is not opened: a window cannot stay over the game there.
//! * No frame, transparent around the device, always on top (on Windows only), not in the task
//!   bar and **not activating**: a click in it leaves the keyboard with the game (on Windows
//!   `WS_EX_NOACTIVATE`, and `WS_EX_TOOLWINDOW` to keep it out of alt-tab).
//! * A thread follows the game every [`TICK`]: the device is shown only while the game is the
//!   window in front and not minimised ([`visible`]; elsewhere than on Windows, as best it can
//!   be: while the game runs, a window that is not on top), on the game's monitor. The device in
//!   front with no game window found is hidden.
//! * When Omsi-Hub's companion server starts again on another port (Phone & tablet switched),
//!   the window is pointed at the new address ([`hub_api::companion::device_url`], looked at
//!   every [`URL_EVERY`] ticks; Omsi-Hub's language comes with it).
//! * It sits in a corner of the monitor's work area ([`place`]); where it is let go it snaps to
//!   the nearest corner ([`snap`]), and its corner, offset and width are kept in the settings
//!   (`boordcomputer_corner`, `boordcomputer_offset`, `boordcomputer_width`). The page moves the
//!   window with `startDragging` (the move is the system's: the page never sees it end), so a
//!   move is settled [`SETTLE`] after the window's last `Moved` event, once the mouse button is
//!   up; the size grip's release (`boordcomputer_drop`) settles at once, and S / M / L keep the
//!   device in its corner. Where Omsi-Hub puts the window itself it is moved before it is sized
//!   (a monitor of another scale rescales it on arrival), its own `Moved` events are not taken
//!   for the player's, and a window that did not land where it was put is put right on the next
//!   look ([`reapply`]).

use serde_json::{json, Map, Value};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

/// The window's label (`capabilities/boordcomputer.json` names it).
pub const LABEL: &str = "boordcomputer";
/// How often the thread that follows the game looks.
const TICK: Duration = Duration::from_millis(250);
/// A move is settled (snapped and kept) this long after the window last moved.
const SETTLE: Duration = Duration::from_millis(300);
/// The device's height to its width (the page lays itself out at 960 × 653).
const ASPECT: f64 = 0.68;
/// The smallest and largest width, in logical pixels.
const MIN_WIDTH: i32 = 360;
const MAX_WIDTH: i32 = 1200;
/// Let go this close to an edge (physical pixels), the device goes flush with it.
const SNAP: i32 = 48;
/// The window's `Moved` events this long after Omsi-Hub put it somewhere are its own.
const QUIET: Duration = Duration::from_millis(500);
/// How often the device is put where it belongs again when it did not land there.
const RETRIES: u32 = 3;
/// How often (in [`TICK`]s) the thread following the game asks where the device page is now.
const URL_EVERY: u32 = 8;

const CORNER_KEY: &str = "boordcomputer_corner";
const OFFSET_KEY: &str = "boordcomputer_offset";
const WIDTH_KEY: &str = "boordcomputer_width";

/// A corner of the monitor the device sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    const ALL: [Corner; 4] = [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft, Corner::BottomRight];

    /// Its name in the settings.
    fn name(self) -> &'static str {
        match self {
            Corner::TopLeft => "top-left",
            Corner::TopRight => "top-right",
            Corner::BottomLeft => "bottom-left",
            Corner::BottomRight => "bottom-right",
        }
    }

    fn right(self) -> bool {
        matches!(self, Corner::TopRight | Corner::BottomRight)
    }

    fn bottom(self) -> bool {
        matches!(self, Corner::BottomLeft | Corner::BottomRight)
    }
}

/// A rectangle in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Where the device goes on `monitor` (its work area, physical pixels): in `corner`, `offset`
/// in from it and `width` wide (both logical pixels, `scale` the monitor's scale factor); its
/// width held between 360 and 1200, its height `width` × 0.68, and all of it on the monitor.
pub fn place(monitor: Rect, corner: Corner, offset: (i32, i32), width: i32, scale: f64) -> Rect {
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let logical = f64::from(width.clamp(MIN_WIDTH, MAX_WIDTH));
    let (mut w, mut h) = ((logical * scale).round(), (logical * ASPECT * scale).round());
    // (larger than the monitor: made to fit, the same shape)
    if w > f64::from(monitor.w) {
        w = f64::from(monitor.w);
        h = (w * ASPECT).round();
    }
    if h > f64::from(monitor.h) {
        h = f64::from(monitor.h);
        w = (h / ASPECT).round();
    }
    let (w, h) = (w as i32, h as i32);
    let (ox, oy) = ((f64::from(offset.0) * scale).round() as i32, (f64::from(offset.1) * scale).round() as i32);
    let x = if corner.right() { monitor.x + monitor.w - ox - w } else { monitor.x + ox };
    let y = if corner.bottom() { monitor.y + monitor.h - oy - h } else { monitor.y + oy };
    // (`min` then `max`, not `clamp`: a monitor of no size leaves nothing to clamp into)
    Rect { x: x.min(monitor.x + monitor.w - w).max(monitor.x), y: y.min(monitor.y + monitor.h - h).max(monitor.y), w, h }
}

/// Where the device was let go (`at`, on `monitor`, physical pixels): the corner of the
/// monitor's quarter its middle is in, and its offset in from that corner - 0 on an axis where
/// it is within 48 pixels of the edge.
pub fn snap(monitor: Rect, at: Rect) -> (Corner, (i32, i32)) {
    let right = 2 * at.x + at.w > 2 * monitor.x + monitor.w;
    let bottom = 2 * at.y + at.h > 2 * monitor.y + monitor.h;
    let corner = match (right, bottom) {
        (false, false) => Corner::TopLeft,
        (true, false) => Corner::TopRight,
        (false, true) => Corner::BottomLeft,
        (true, true) => Corner::BottomRight,
    };
    let dx = if right { monitor.x + monitor.w - (at.x + at.w) } else { at.x - monitor.x };
    let dy = if bottom { monitor.y + monitor.h - (at.y + at.h) } else { at.y - monitor.y };
    let flush = |d: i32| if d <= SNAP { 0 } else { d };
    (corner, (flush(dx), flush(dy)))
}

/// Whether the device is shown: only while the game is the program in front and not minimised.
pub fn visible(foreground_pid: Option<u32>, game_pid: Option<u32>, game_minimised: bool) -> bool {
    game_pid.is_some() && foreground_pid == game_pid && !game_minimised
}

/// Whether the device is put at `wanted` (now `tries` times again since it was first put there):
/// Some(the new count) when it is somewhere new, or - a few times, not forever - when the window
/// (`actual`) did not land there (a monitor of another scale rescales the window as it moves).
pub fn reapply(wanted: Rect, placed: Option<Rect>, actual: Option<Rect>, tries: u32) -> Option<u32> {
    if placed != Some(wanted) {
        return Some(0);
    }
    match actual {
        Some(a) if a != wanted && tries < RETRIES => Some(tries + 1),
        _ => None,
    }
}

/// Where the player keeps the device (the settings), in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Spot {
    corner: Corner,
    offset: (i32, i32),
    width: i32,
}

impl Spot {
    const DEFAULT: Spot = Spot { corner: Corner::BottomRight, offset: (24, 24), width: 560 };

    /// From the settings' other keys; what is missing or wrong is the default.
    fn of(other: &Map<String, Value>) -> Spot {
        let d = Spot::DEFAULT;
        let corner = other.get(CORNER_KEY).and_then(Value::as_str).and_then(|n| Corner::ALL.into_iter().find(|c| c.name() == n)).unwrap_or(d.corner);
        let offset = other
            .get(OFFSET_KEY)
            .and_then(Value::as_array)
            .filter(|a| a.len() == 2)
            .and_then(|a| Some((a[0].as_f64()?.round() as i32, a[1].as_f64()?.round() as i32)))
            .filter(|(x, y)| *x >= 0 && *y >= 0)
            .unwrap_or(d.offset);
        let width = other.get(WIDTH_KEY).and_then(Value::as_f64).filter(|w| w.is_finite()).map_or(d.width, |w| (w.round() as i32).clamp(MIN_WIDTH, MAX_WIDTH));
        Spot { corner, offset, width }
    }

    fn load() -> Spot {
        Spot::of(&hub_core::settings::Settings::load().other)
    }

    fn save(self) {
        let mut s = hub_core::settings::Settings::load();
        s.other.insert(CORNER_KEY.into(), json!(self.corner.name()));
        s.other.insert(OFFSET_KEY.into(), json!([self.offset.0, self.offset.1]));
        s.other.insert(WIDTH_KEY.into(), json!(self.width));
        if let Err(e) = s.save() {
            hub_core::log_line(&format!("boordcomputer: its place was not saved: {e}"));
        }
    }
}

/// What the window and the thread following the game share. Never held while the window is
/// asked anything: the window's events take it on the main thread.
struct Shared {
    /// The current drive's number: a thread of an earlier one stops when it changed.
    run: u64,
    spot: Spot,
    /// Where the device was last put.
    placed: Option<Rect>,
    /// When it was last put there: the `Moved` events for [`QUIET`] after are Omsi-Hub's own
    /// (the moves are asynchronous, and a monitor of another scale makes the system move and
    /// size it once more).
    applied_at: Option<Instant>,
    /// How often it was put at `placed` again because it was not there ([`reapply`]).
    tries: u32,
    /// When the player last moved or sized it, while that is not settled.
    moved_at: Option<Instant>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared { run: 0, spot: Spot::DEFAULT, placed: None, applied_at: None, tries: 0, moved_at: None });

fn shared() -> MutexGuard<'static, Shared> {
    SHARED.lock().unwrap_or_else(|e| e.into_inner())
}

/// A monitor's work area and scale factor.
#[derive(Clone, Copy, Debug)]
struct Screen {
    rect: Rect,
    scale: f64,
}

impl Screen {
    fn of(m: &tauri::Monitor) -> Screen {
        let a = m.work_area();
        Screen { rect: Rect { x: a.position.x, y: a.position.y, w: a.size.width as i32, h: a.size.height as i32 }, scale: m.scale_factor() }
    }
}

/// The monitor the device is on (else the primary one).
fn screen_of(win: &WebviewWindow) -> Option<Screen> {
    win.current_monitor().ok().flatten().or_else(|| win.primary_monitor().ok().flatten()).map(|m| Screen::of(&m))
}

/// The device opened for the game `pid` (`game-started`).
pub fn open(app: &AppHandle, pid: u32) {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty()) {
        hub_core::log_line("boordcomputer: not opened, Wayland keeps no window over the game");
        return;
    }
    let Some(url) = hub_api::companion::device_url() else {
        hub_core::log_line("boordcomputer: not opened, the companion's server does not run");
        return;
    };
    // (one of an earlier drive that was not closed goes first, and its label with it)
    if let Some(old) = app.get_webview_window(LABEL) {
        let _ = old.destroy();
        let end = Instant::now() + Duration::from_secs(2);
        while app.get_webview_window(LABEL).is_some() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    let run = {
        let mut s = shared();
        s.run += 1;
        s.spot = Spot::load();
        s.placed = None;
        s.moved_at = None;
        s.run
    };
    if let Err(e) = build(app, &url) {
        hub_core::log_line(&format!("boordcomputer: the window could not open: {e}"));
        return;
    }
    hub_core::log_line(&format!("boordcomputer: opened for the game (pid {pid})"));
    let app = app.clone();
    std::thread::spawn(move || follow(app, run, pid, url));
}

/// The device closed (`game-ended`), a move not settled yet kept first.
pub fn close(app: &AppHandle) {
    shared().run += 1;
    let Some(win) = app.get_webview_window(LABEL) else { return };
    let _ = app.run_on_main_thread(move || {
        if shared().moved_at.is_some() {
            settle(&win);
        }
        let _ = win.destroy();
        let mut s = shared();
        s.placed = None;
        s.moved_at = None;
    });
}

/// The window, hidden until the thread following the game places and shows it.
fn build(app: &AppHandle, url: &str) -> Result<WebviewWindow, String> {
    let url = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let d = Spot::DEFAULT;
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(url))
        .title("Omsi-Hub")
        .inner_size(f64::from(d.width), (f64::from(d.width) * ASPECT).round())
        .decorations(false)
        .transparent(true)
        // (on top only where it hides as soon as the game is not in front: on Windows. Elsewhere
        // it is not known what is in front, and a window on top of every program after alt-tab
        // is what the device must never be)
        .always_on_top(cfg!(windows))
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .shadow(false)
        .resizable(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    #[cfg(windows)]
    if let Ok(hwnd) = win.hwnd() {
        win32::not_activating(hwnd);
    }
    win.on_window_event(|e| {
        if let WindowEvent::Moved(_) = e {
            let mut s = shared();
            // (not before it was placed, and not just after Omsi-Hub put it: those moves are
            // its own, wherever they land - a correction follows on the next look)
            if s.placed.is_some() && s.applied_at.is_none_or(|t| t.elapsed() >= QUIET) {
                s.moved_at = Some(Instant::now());
            }
        }
    });
    Ok(win)
}

// Where the device is put, and what is kept, changes on the main thread only (the page's
// commands, the window's events, and what the thread following the game asks for there), so
// one never undoes another half-way.

/// The device put at `r` (main thread): moved first, so it is sized on the monitor it goes to
/// (moved onto a monitor of another scale, the system rescales it), then put there once more.
fn apply(win: &WebviewWindow, r: Rect) {
    {
        let mut s = shared();
        if s.placed != Some(r) {
            s.tries = 0;
        }
        s.placed = Some(r);
        s.applied_at = Some(Instant::now());
    }
    let _ = win.set_position(PhysicalPosition::new(r.x, r.y));
    let _ = win.set_size(PhysicalSize::new(r.w.max(1) as u32, r.h.max(1) as u32));
    let _ = win.set_position(PhysicalPosition::new(r.x, r.y));
}

/// Where the window is now (physical pixels).
fn actual(win: &WebviewWindow) -> Option<Rect> {
    let (pos, size) = (win.outer_position().ok()?, win.outer_size().ok()?);
    Some(Rect { x: pos.x, y: pos.y, w: size.width as i32, h: size.height as i32 })
}

/// Where the player let the device go: snapped to the nearest corner, kept, and put there
/// (main thread).
fn settle(win: &WebviewWindow) {
    let (Some(at), Some(screen)) = (actual(win), screen_of(win)) else {
        shared().moved_at = None;
        return;
    };
    let (corner, offset) = snap(screen.rect, at);
    let logical = |v: i32| (f64::from(v) / screen.scale).round() as i32;
    let spot = Spot { corner, offset: (logical(offset.0), logical(offset.1)), width: logical(at.w).clamp(MIN_WIDTH, MAX_WIDTH) };
    {
        let mut s = shared();
        s.spot = spot;
        s.moved_at = None;
    }
    spot.save();
    apply(win, place(screen.rect, spot.corner, spot.offset, spot.width, screen.scale));
}

/// What the game looks like now: whether the device is to be shown (None: leave it as it is)
/// and the monitor it is on.
struct Look {
    show: Option<bool>,
    screen: Option<Screen>,
}

#[cfg(windows)]
fn look(win: &WebviewWindow, pid: u32) -> Look {
    let own = win.hwnd().ok();
    let front = win32::foreground();
    let game = win32::window_of(pid, own);
    let minimised = game.is_some_and(win32::minimised);
    let show = match front {
        // (the device itself in front: a click does not activate it, but the system's move of it
        // (`startDragging`) does - the keyboard handed back to the game once the mouse button is
        // up, the device left as it is)
        Some((h, _)) if Some(h) == own => match game {
            Some(game) => {
                if !button_down() {
                    win32::give_back(game);
                }
                None
            }
            // (no game window to hand the keyboard back to - it closed or hides: the device
            // does not stay in front on its own)
            None => Some(false),
        },
        _ => Some(visible(front.map(|f| f.1), Some(pid), minimised)),
    };
    // (the game's monitor: the one its window's middle is on)
    let screen = game
        .filter(|_| !minimised)
        .and_then(win32::rect)
        .and_then(|r| win.monitor_from_point(f64::from(r.x) + f64::from(r.w) / 2.0, f64::from(r.y) + f64::from(r.h) / 2.0).ok().flatten())
        .map(|m| Screen::of(&m))
        .or_else(|| screen_of(win));
    Look { show, screen }
}

/// Elsewhere the program in front is not asked: shown while the game runs (best effort: a
/// normal window there, not always on top - [`build`]), on the monitor the device is on.
#[cfg(not(windows))]
fn look(win: &WebviewWindow, pid: u32) -> Look {
    Look { show: Some(visible(Some(pid), Some(pid), false)), screen: screen_of(win) }
}

fn set_shown(win: &WebviewWindow, show: bool) {
    #[cfg(windows)]
    if let Ok(hwnd) = win.hwnd() {
        // (shown without activating it; Tauri's show would activate it)
        win32::show(hwnd, show);
        return;
    }
    let _ = if show { win.show() } else { win.hide() };
}

/// The thread that follows the game `pid` for the drive `run`, the device opened at `url`.
fn follow(app: AppHandle, run: u64, pid: u32, mut url: String) {
    let mut shown = false;
    let mut ticks = 0u32;
    loop {
        std::thread::sleep(TICK);
        ticks = ticks.wrapping_add(1);
        let (current, moved_at) = {
            let s = shared();
            (s.run == run, s.moved_at)
        };
        if !current {
            return;
        }
        let Some(win) = app.get_webview_window(LABEL) else { return };
        // (a game that ended without `game-ended` reaching here)
        if hub_api::drive::state()["running"] != true {
            close(&app);
            return;
        }
        // (the companion's server started again on another port, or Omsi-Hub's language changed:
        // the device page there)
        if ticks.is_multiple_of(URL_EVERY) {
            if let Some(now) = hub_api::companion::device_url().filter(|u| *u != url) {
                match now.parse::<tauri::Url>() {
                    Ok(to) => {
                        hub_core::log_line("boordcomputer: the device page moved, the window follows");
                        let w = win.clone();
                        let _ = app.run_on_main_thread(move || {
                            let _ = w.navigate(to);
                        });
                    }
                    Err(e) => hub_core::log_line(&format!("boordcomputer: the device page's address is wrong: {e}")),
                }
                url = now;
            }
        }
        let look = look(&win, pid);
        // (the mouse button up: a move of the window by the system, or the page's size grip, is
        // over - and none is starting either)
        let held = button_down();
        let let_go = moved_at.is_some_and(|t| t.elapsed() >= SETTLE) && !held;
        let show = look.show.filter(|s| *s != shown);
        shown = show.unwrap_or(shown);
        let _ = app.run_on_main_thread(move || tend(&win, look.screen, let_go, held, show));
    }
}

/// On the main thread, for the thread following the game: a move let go settled, else (the
/// mouse button up) the device kept in its place on `screen`, put right when it is not there;
/// then shown or hidden.
fn tend(win: &WebviewWindow, screen: Option<Screen>, let_go: bool, held: bool, show: Option<bool>) {
    let (moved_at, spot, placed, tries) = {
        let s = shared();
        (s.moved_at, s.spot, s.placed, s.tries)
    };
    match (moved_at, screen) {
        // (not when it moved again since it was looked at)
        (Some(t), _) if let_go && t.elapsed() >= SETTLE => settle(win),
        (None, Some(screen)) if !held => {
            let r = place(screen.rect, spot.corner, spot.offset, spot.width, screen.scale);
            let now = actual(win);
            match reapply(r, placed, now, tries) {
                Some(tries) => {
                    apply(win, r);
                    shared().tries = tries;
                }
                // (there: a later change of scale may be put right again)
                None if now == Some(r) && tries > 0 => shared().tries = 0,
                None => {}
            }
        }
        _ => {}
    }
    if let Some(show) = show {
        set_shown(win, show);
    }
}

#[cfg(windows)]
use win32::button_down;

/// Whether the main mouse button is held (a move or a size still going on): not known here.
#[cfg(not(windows))]
fn button_down() -> bool {
    false
}

/// The device `width` logical pixels wide. While the mouse button is held it is the page's size
/// grip (bottom right): its top left stays where it is, and the size is kept once let go.
/// Otherwise it is S / M / L: the device stays in its corner, and the width is kept now.
#[tauri::command]
pub fn boordcomputer_size(app: AppHandle, width: f64) {
    let Some(win) = app.get_webview_window(LABEL) else { return };
    let Some(screen) = screen_of(&win) else { return };
    let width = if width.is_finite() { (width.round() as i32).clamp(MIN_WIDTH, MAX_WIDTH) } else { Spot::DEFAULT.width };
    if button_down() {
        let Ok(pos) = win.outer_position() else { return };
        let logical = |v: i32| (f64::from(v) / screen.scale).round() as i32;
        let offset = (logical(pos.x - screen.rect.x), logical(pos.y - screen.rect.y));
        shared().moved_at = Some(Instant::now());
        apply(&win, place(screen.rect, Corner::TopLeft, offset, width, screen.scale));
        return;
    }
    let spot = {
        let mut s = shared();
        s.spot.width = width;
        s.moved_at = None;
        s.spot
    };
    spot.save();
    apply(&win, place(screen.rect, spot.corner, spot.offset, spot.width, screen.scale));
}

/// The size grip let go: snapped and kept now.
#[tauri::command]
pub fn boordcomputer_drop(app: AppHandle) {
    if let Some(win) = app.get_webview_window(LABEL) {
        settle(&win);
    }
}

/// Windows: the device's styles, showing it without activating it, and the game's window.
#[cfg(windows)]
mod win32 {
    use super::Rect;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetSystemMetrics, GetWindow, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SetForegroundWindow, SetWindowLongPtrW,
        SetWindowPos, ShowWindow, GWL_EXSTYLE, GW_OWNER, HWND_TOPMOST, SM_SWAPBUTTON, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW,
    };

    /// A click never activates it, and it is not in alt-tab (`WS_EX_APPWINDOW`, which Tauri sets,
    /// would put it back there).
    pub fn not_activating(hwnd: HWND) {
        unsafe {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let want = (ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize) & !(WS_EX_APPWINDOW.0 as isize);
            if want != ex {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);
            }
        }
    }

    /// Shown (on top, not activated) or hidden.
    pub fn show(hwnd: HWND, show: bool) {
        unsafe {
            if show {
                // (Tauri sets the window's styles again when its own flags change)
                not_activating(hwnd);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            } else {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        }
    }

    /// The window in front and its process.
    pub fn foreground() -> Option<(HWND, u32)> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            (pid != 0).then_some((hwnd, pid))
        }
    }

    /// The main window of the process `pid`: a visible top-level window without an owner (not
    /// `except`, the device).
    pub fn window_of(pid: u32, except: Option<HWND>) -> Option<HWND> {
        struct Find {
            pid: u32,
            except: Option<HWND>,
            found: Option<HWND>,
        }
        unsafe extern "system" fn each(hwnd: HWND, l: LPARAM) -> BOOL {
            let find = unsafe { &mut *(l.0 as *mut Find) };
            let mut pid = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            if pid == find.pid && Some(hwnd) != find.except && unsafe { IsWindowVisible(hwnd) }.as_bool() && unsafe { GetWindow(hwnd, GW_OWNER) }.is_err() {
                find.found = Some(hwnd);
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut find = Find { pid, except, found: None };
        // (an Err when `each` stopped it at the window: found)
        let _ = unsafe { EnumWindows(Some(each), LPARAM(&mut find as *mut Find as isize)) };
        find.found
    }

    /// The game in front again (allowed: this process has the foreground, the device has it).
    pub fn give_back(game: HWND) {
        let _ = unsafe { SetForegroundWindow(game) };
    }

    pub fn minimised(hwnd: HWND) -> bool {
        unsafe { IsIconic(hwnd) }.as_bool()
    }

    /// A window's rectangle on the desktop (physical pixels).
    pub fn rect(hwnd: HWND) -> Option<Rect> {
        let mut r = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
        Some(Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top })
    }

    /// The main mouse button (the right one for a left-handed mouse) held.
    pub fn button_down() -> bool {
        let vk = if unsafe { GetSystemMetrics(SM_SWAPBUTTON) } != 0 { VK_RBUTTON } else { VK_LBUTTON };
        (unsafe { GetAsyncKeyState(i32::from(vk.0)) } as u16 & 0x8000) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_HD: Rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };

    fn inside(r: Rect, m: Rect) -> bool {
        r.x >= m.x && r.y >= m.y && r.x + r.w <= m.x + m.w && r.y + r.h <= m.y + m.h
    }

    #[test]
    fn the_default_is_bottom_right_24_in() {
        assert_eq!(Spot::of(&Map::new()), Spot { corner: Corner::BottomRight, offset: (24, 24), width: 560 });
        let r = place(FULL_HD, Corner::BottomRight, (24, 24), 560, 1.0);
        assert_eq!(r, Rect { x: 1920 - 24 - 560, y: 1080 - 24 - 381, w: 560, h: 381 });
        // (on a monitor at 150 %: the same device in physical pixels, on a monitor left of the
        // primary one)
        let m = Rect { x: -2560, y: 0, w: 2560, h: 1440 };
        let r = place(m, Corner::BottomRight, (24, 24), 560, 1.5);
        assert_eq!(r, Rect { x: -36 - 840, y: 1440 - 36 - 571, w: 840, h: 571 });
    }

    #[test]
    fn a_place_off_the_monitor_is_clamped_into_it() {
        for corner in [Corner::TopLeft, Corner::TopRight, Corner::BottomLeft, Corner::BottomRight] {
            for offset in [(5000, 5000), (-5000, -5000), (5000, -20)] {
                let r = place(FULL_HD, corner, offset, 560, 1.0);
                assert!(inside(r, FULL_HD), "{corner:?} {offset:?}: {r:?}");
                assert_eq!((r.w, r.h), (560, 381));
            }
        }
        // (a device larger than a small monitor: made to fit)
        let small = Rect { x: 0, y: 0, w: 1024, h: 600 };
        let r = place(small, Corner::TopLeft, (0, 0), 1200, 1.0);
        assert!(inside(r, small), "{r:?}");
    }

    #[test]
    fn widths_are_held_between_360_and_1200() {
        assert_eq!(place(FULL_HD, Corner::TopLeft, (0, 0), 100, 1.0).w, 360);
        assert_eq!(place(FULL_HD, Corner::TopLeft, (0, 0), 5000, 1.0).w, 1200);
        assert_eq!(place(FULL_HD, Corner::TopLeft, (0, 0), 760, 1.0).w, 760);
        // (in logical pixels: at 200 % the smallest is 720 physical ones)
        let uhd = Rect { x: 0, y: 0, w: 3840, h: 2160 };
        assert_eq!(place(uhd, Corner::TopLeft, (0, 0), 100, 2.0).w, 720);
        // (and so in the settings)
        let other = json!({ WIDTH_KEY: 9000 }).as_object().cloned().unwrap();
        assert_eq!(Spot::of(&other).width, 1200);
    }

    #[test]
    fn a_drop_near_a_corner_snaps_to_it() {
        let at = Rect { x: 30, y: 30, w: 560, h: 381 };
        assert_eq!(snap(FULL_HD, at), (Corner::TopLeft, (0, 0)));
        let at = Rect { x: 1920 - 560 - 10, y: 1080 - 381 - 47, w: 560, h: 381 };
        assert_eq!(snap(FULL_HD, at), (Corner::BottomRight, (0, 0)));
        // (on a second monitor, right of the first)
        let m = Rect { x: 1920, y: 0, w: 1920, h: 1080 };
        let at = Rect { x: 1920 + 1920 - 560 - 5, y: 12, w: 560, h: 381 };
        assert_eq!(snap(m, at), (Corner::TopRight, (0, 0)));
    }

    #[test]
    fn a_drop_in_the_middle_keeps_its_offset_from_the_nearest_corner() {
        let at = Rect { x: 1200, y: 500, w: 560, h: 381 };
        let (corner, offset) = snap(FULL_HD, at);
        assert_eq!(corner, Corner::BottomRight);
        assert_eq!(offset, (1920 - 1200 - 560, 1080 - 500 - 381));
        // (placed again where it was dropped)
        assert_eq!(place(FULL_HD, corner, offset, 560, 1.0), at);
        let at = Rect { x: 300, y: 200, w: 560, h: 381 };
        assert_eq!(snap(FULL_HD, at), (Corner::TopLeft, (300, 200)));
        // (near one edge only: flush with that edge, the other offset kept)
        let at = Rect { x: 20, y: 600, w: 560, h: 381 };
        assert_eq!(snap(FULL_HD, at), (Corner::BottomLeft, (0, 1080 - 600 - 381)));
    }

    #[test]
    fn hidden_unless_the_game_is_in_front() {
        assert!(visible(Some(7), Some(7), false));
        assert!(!visible(Some(8), Some(7), false), "another program in front");
        assert!(!visible(Some(7), Some(7), true), "the game minimised");
        assert!(!visible(None, Some(7), false), "nothing in front");
        assert!(!visible(Some(7), None, false), "no game");
    }

    /// Where it was kept is read back from the settings; a wrong value is the default.
    #[test]
    fn the_boordcomputer_place_is_read_from_the_settings() {
        let other = json!({ CORNER_KEY: "top-left", OFFSET_KEY: [10, 40], WIDTH_KEY: 760 }).as_object().cloned().unwrap();
        assert_eq!(Spot::of(&other), Spot { corner: Corner::TopLeft, offset: (10, 40), width: 760 });
        let wrong = json!({ CORNER_KEY: "middle", OFFSET_KEY: [10], WIDTH_KEY: "big" }).as_object().cloned().unwrap();
        assert_eq!(Spot::of(&wrong), Spot::DEFAULT);
        for c in Corner::ALL {
            let other = json!({ CORNER_KEY: c.name() }).as_object().cloned().unwrap();
            assert_eq!(Spot::of(&other).corner, c);
        }
    }

    /// Put on a monitor of another scale, the window is rescaled by the system as it lands: the
    /// next look puts it right, a few times at most; somewhere new it is always put.
    #[test]
    fn a_boordcomputer_that_did_not_land_where_it_was_put_is_put_again() {
        let wanted = Rect { x: 1700, y: 900, w: 840, h: 571 };
        let inflated = Rect { x: 1700, y: 900, w: 1260, h: 857 };
        assert_eq!(reapply(wanted, None, None, 0), Some(0), "never put yet");
        assert_eq!(reapply(wanted, Some(Rect { x: 0, ..wanted }), Some(inflated), 3), Some(0), "somewhere new");
        assert_eq!(reapply(wanted, Some(wanted), Some(wanted), 0), None, "there");
        assert_eq!(reapply(wanted, Some(wanted), Some(inflated), 0), Some(1), "rescaled");
        assert_eq!(reapply(wanted, Some(wanted), Some(Rect { x: 1690, ..wanted }), 2), Some(3), "nudged");
        assert_eq!(reapply(wanted, Some(wanted), Some(inflated), RETRIES), None, "not forever");
        assert_eq!(reapply(wanted, Some(wanted), None, 0), None, "not known where it is");
    }

    /// The remote page may call the device's commands and drag its window from Omsi-Hub's own
    /// server on this computer, on any port - and from nowhere else.
    #[test]
    fn the_boordcomputer_capability_lets_only_the_local_page_in() {
        let cap: tauri::utils::acl::capability::Capability = serde_json::from_str(include_str!("../capabilities/boordcomputer.json")).unwrap();
        assert_eq!(cap.windows, [LABEL]);
        let remote = cap.remote.expect("a remote capability");
        let ok = |u: &str| remote.urls.iter().any(|p| p.parse::<tauri::utils::acl::RemoteUrlPattern>().unwrap().test(&u.parse().unwrap()));
        assert!(ok("http://127.0.0.1:47811/device#key=abc"));
        assert!(ok("http://127.0.0.1:50000/device"));
        assert!(!ok("http://192.168.1.20:47811/device"));
        assert!(!ok("https://example.com/device"));
        assert!(!ok("http://127.0.0.1.example.com:47811/device"));
    }
}
