//! Magnetic edges for the overlay and pinned panels: dragged close to an edge of the screen or of
//! another of these windows, a window jumps to it. Holding Shift places it freely. Only Leafline's
//! own windows are involved.

use tauri::WebviewWindow;

use crate::error::Result;

/// How close an edge must come to catch, in logical pixels.
const REACH: f64 = 12.0;
/// The space left between two windows snapped side by side, in logical pixels.
const GAP: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Edges {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Edges {
    fn across(self) -> (i32, i32) {
        (self.left, self.right)
    }

    fn down(self) -> (i32, i32) {
        (self.top, self.bottom)
    }
}

/// Whether two spans overlap or come within `reach` of it.
fn near(a: (i32, i32), b: (i32, i32), reach: i32) -> bool {
    a.0 <= b.1 + reach && b.0 <= a.1 + reach
}

/// The smallest move along one axis that lines `moving` up with something within `reach`: the
/// sides of the screen, or for another window, either side of it (with `gap` to spare) or level
/// with it.
fn stop(moving: (i32, i32), screens: &[(i32, i32)], others: &[(i32, i32)], reach: i32, gap: i32) -> i32 {
    let (start, end) = moving;
    let screen = screens.iter().flat_map(|&(a, b)| [a - start, b - end]);
    let other = others.iter().flat_map(|&(a, b)| [b + gap - start, a - gap - end, a - start, b - end]);
    screen.chain(other).filter(|d| d.abs() <= reach).min_by_key(|d| d.abs()).unwrap_or(0)
}

/// How far to move `moving` so its edges catch. Another window only counts along an axis if it
/// is level with `moving` on the other one, so a window far above doesn't pull sideways.
fn offset(moving: Edges, screens: &[Edges], others: &[Edges], reach: i32, gap: i32) -> (i32, i32) {
    let spans = |side: fn(Edges) -> (i32, i32)| screens.iter().copied().map(side).collect::<Vec<_>>();
    let level = |side: fn(Edges) -> (i32, i32), cross: fn(Edges) -> (i32, i32)| {
        others.iter().copied().filter(|o| near(cross(moving), cross(*o), reach + gap)).map(side).collect::<Vec<_>>()
    };
    (
        stop(moving.across(), &spans(Edges::across), &level(Edges::across, Edges::down), reach, gap),
        stop(moving.down(), &spans(Edges::down), &level(Edges::down, Edges::across), reach, gap),
    )
}

/// Where a window of `moving`'s size belongs with the cursor at `cursor`, held at `grab` from its
/// top left. Worked out from the cursor alone, so a window that caught an edge carries nothing
/// of that into the next step and comes away once the cursor is out of reach.
fn dragged(cursor: (i32, i32), grab: (i32, i32), moving: Edges) -> Edges {
    let (left, top) = (cursor.0 - grab.0, cursor.1 - grab.1);
    Edges { left, top, right: left + moving.right - moving.left, bottom: top + moving.bottom - moving.top }
}

/// Makes `window` snap while dragged. It works for the window's whole life.
pub fn attach(window: &WebviewWindow) -> Result<()> {
    platform::attach(window)
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::sync::Mutex;

    use tauri::WebviewWindow;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromRect};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_SHIFT};
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetWindowRect, IsWindowVisible, WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_MOVING, WM_NCDESTROY,
    };

    use super::{Edges, GAP, REACH, dragged, offset};
    use crate::error::Result;

    const SUBCLASS_ID: usize = 1;

    /// The windows that snap to each other.
    static WINDOWS: Mutex<Vec<isize>> = Mutex::new(Vec::new());
    /// Where the cursor holds the window being dragged, from its top left.
    static GRAB: Mutex<Option<(i32, i32)>> = Mutex::new(None);

    pub fn attach(window: &WebviewWindow) -> Result<()> {
        let hwnd = window.hwnd()?.0 as isize;
        WINDOWS.lock().expect("snap lock").push(hwnd);
        // A window can only be subclassed from the thread that created it.
        window.run_on_main_thread(move || unsafe {
            let _ = SetWindowSubclass(handle(hwnd), Some(subclass), SUBCLASS_ID, 0);
        })?;
        Ok(())
    }

    fn handle(hwnd: isize) -> HWND {
        HWND(hwnd as *mut c_void)
    }

    fn edges(rect: RECT) -> Edges {
        Edges { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom }
    }

    /// The monitor `rect` is mostly on: its whole area, and the part the taskbar leaves.
    fn screens(rect: &RECT) -> Vec<Edges> {
        let monitor = unsafe { MonitorFromRect(rect, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            vec![edges(info.rcMonitor), edges(info.rcWork)]
        } else {
            Vec::new()
        }
    }

    fn others(hwnd: HWND) -> Vec<Edges> {
        let windows = WINDOWS.lock().expect("snap lock");
        windows
            .iter()
            .map(|&other| handle(other))
            .filter(|&other| other != hwnd && unsafe { IsWindowVisible(other) }.as_bool())
            .filter_map(|other| {
                let mut rect = RECT::default();
                unsafe { GetWindowRect(other, &mut rect) }.ok().map(|()| edges(rect))
            })
            .collect()
    }

    fn cursor() -> Option<(i32, i32)> {
        let mut point = POINT::default();
        unsafe { GetCursorPos(&mut point) }.ok().map(|()| (point.x, point.y))
    }

    /// Notes where the cursor took hold of the window as a drag begins.
    fn hold(hwnd: HWND) {
        let mut frame = RECT::default();
        let held = cursor().filter(|_| unsafe { GetWindowRect(hwnd, &mut frame) }.is_ok());
        *GRAB.lock().expect("snap lock") = held.map(|(x, y)| (x - frame.left, y - frame.top));
    }

    /// Replaces the rectangle Windows is about to drag the window to. Adjusting that rectangle
    /// left caught windows stuck, so this one comes from the cursor.
    fn place(hwnd: HWND, rect: &mut RECT, snapping: bool) {
        let Some(grab) = *GRAB.lock().expect("snap lock") else { return };
        let Some(cursor) = cursor() else { return };
        let free = dragged(cursor, grab, edges(*rect));
        let scale = f64::from(unsafe { GetDpiForWindow(hwnd) }) / 96.0;
        let (reach, gap) = ((REACH * scale).round() as i32, (GAP * scale).round() as i32);
        let free_rect = RECT { left: free.left, top: free.top, right: free.right, bottom: free.bottom };
        let (dx, dy) = if snapping { offset(free, &screens(&free_rect), &others(hwnd), reach, gap) } else { (0, 0) };
        *rect = RECT { left: free.left + dx, top: free.top + dy, right: free.right + dx, bottom: free.bottom + dy };
    }

    unsafe extern "system" fn subclass(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        id: usize,
        _data: usize,
    ) -> LRESULT {
        unsafe {
            match message {
                WM_ENTERSIZEMOVE => hold(hwnd),
                WM_EXITSIZEMOVE => *GRAB.lock().expect("snap lock") = None,
                WM_MOVING => {
                    // The high bit says the key is down.
                    place(hwnd, &mut *(lparam.0 as *mut RECT), GetKeyState(i32::from(VK_SHIFT.0)) >= 0);
                    return LRESULT(1);
                }
                WM_NCDESTROY => {
                    WINDOWS.lock().expect("snap lock").retain(|&w| w != hwnd.0 as isize);
                    let _ = RemoveWindowSubclass(hwnd, Some(subclass), id);
                }
                _ => {}
            }
            DefSubclassProc(hwnd, message, wparam, lparam)
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use tauri::WebviewWindow;

    use crate::error::Result;

    pub fn attach(_window: &WebviewWindow) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Edges = Edges { left: 0, top: 0, right: 2560, bottom: 1440 };
    const REACH: i32 = 15;
    const GAP: i32 = 8;

    fn at(left: i32, top: i32, width: i32, height: i32) -> Edges {
        Edges { left, top, right: left + width, bottom: top + height }
    }

    fn snapped(moving: Edges, others: &[Edges]) -> (i32, i32) {
        offset(moving, &[SCREEN], others, REACH, GAP)
    }

    #[test]
    fn catches_every_screen_edge_within_reach() {
        assert_eq!(snapped(at(10, 500, 300, 200), &[]), (-10, 0));
        assert_eq!(snapped(at(2250, 500, 300, 200), &[]), (10, 0));
        assert_eq!(snapped(at(900, 12, 300, 200), &[]), (0, -12));
        assert_eq!(snapped(at(900, 1235, 300, 200), &[]), (0, 5));
    }

    #[test]
    fn stays_put_out_of_reach() {
        assert_eq!(snapped(at(16, 500, 300, 200), &[]), (0, 0));
        assert_eq!(snapped(at(900, 500, 300, 200), &[at(100, 500, 300, 200)]), (0, 0));
    }

    #[test]
    fn a_corner_catches_both_ways() {
        assert_eq!(snapped(at(4, 7, 300, 200), &[]), (-4, -7));
    }

    #[test]
    fn sits_beside_another_window_with_a_gap() {
        let other = at(500, 300, 340, 560);
        // To the right of it: left edge 850, gap puts it at 848.
        assert_eq!(snapped(at(850, 400, 300, 200), &[other]).0, -2);
        // To the left of it: right edge 495, gap puts it at 492.
        assert_eq!(snapped(at(195, 400, 300, 200), &[other]).0, -3);
    }

    #[test]
    fn stacks_with_a_gap_and_lines_up() {
        let other = at(500, 300, 300, 200);
        // Just below, nearly level: both axes catch.
        assert_eq!(snapped(at(504, 510, 300, 200), &[other]), (-4, -2));
    }

    #[test]
    fn tops_line_up_side_by_side() {
        let other = at(500, 300, 340, 560);
        assert_eq!(snapped(at(848, 306, 300, 200), &[other]), (0, -6));
    }

    #[test]
    fn a_window_far_above_does_not_pull_sideways() {
        let other = at(500, 100, 300, 200);
        assert_eq!(snapped(at(808, 900, 300, 200), &[other]), (0, 0));
    }

    /// Where a window ends up after each step of a drag along `cursors`, snapping as it goes.
    fn drag(start: Edges, grab: (i32, i32), cursors: impl Iterator<Item = (i32, i32)>, others: &[Edges]) -> Vec<Edges> {
        cursors
            .map(|cursor| {
                let free = dragged(cursor, grab, start);
                let (dx, dy) = snapped(free, others);
                Edges { left: free.left + dx, top: free.top + dy, right: free.right + dx, bottom: free.bottom + dy }
            })
            .collect()
    }

    #[test]
    fn a_slow_drag_comes_away_from_an_edge() {
        let start = at(0, 500, 300, 200);
        // One pixel at a time, as a slow hand moves it, from the left edge out to 40.
        let lefts: Vec<i32> =
            drag(start, (100, 10), (100..=140).map(|x| (x, 510)), &[]).iter().map(|e| e.left).collect();
        assert_eq!(&lefts[..=REACH as usize], vec![0; REACH as usize + 1]);
        assert_eq!(lefts[REACH as usize + 1], REACH + 1);
        assert_eq!(lefts[40], 40);
    }

    #[test]
    fn a_slow_drag_comes_away_from_another_window() {
        let other = at(500, 300, 340, 560);
        let start = at(848, 400, 300, 200);
        let lefts: Vec<i32> =
            drag(start, (10, 10), (858..=900).map(|x| (x, 410)), &[other]).iter().map(|e| e.left).collect();
        assert_eq!(lefts[0], 848);
        assert_eq!(*lefts.last().unwrap(), 890);
    }

    #[test]
    fn moves_freely_away_from_anything() {
        let start = at(900, 500, 300, 200);
        let ends = drag(start, (20, 20), [(1000, 700), (1234, 777)].into_iter(), &[]);
        assert_eq!(ends, [at(980, 680, 300, 200), at(1214, 757, 300, 200)]);
    }

    #[test]
    fn the_nearest_edge_wins() {
        // The screen edge is 3 away, the spot beside the other window 9.
        let other = at(339, 400, 300, 200);
        assert_eq!(snapped(at(3, 400, 319, 200), &[other]), (-3, 0));
    }
}
