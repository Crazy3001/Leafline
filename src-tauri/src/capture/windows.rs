//! Windows Graphics Capture of a single window, read back through a Direct3D 11 staging
//! texture.
//!
//! Only window-manager queries are made about the game window (its title, class, position and
//! size). None of them sends the window a message or opens its process.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use image::RgbaImage;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Dxgi::Common::DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIDevice, IDXGIFactory1, IDXGIOutput6};
use windows::Win32::Graphics::Gdi::{ClientToScreen, MONITOR_DEFAULTTONEAREST, MonitorFromWindow};
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetClassNameW, GetClientRect, GetWindowLongPtrW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, Interface, factory};

use super::{GameWindow, Grab, looks_like_game};
use crate::error::{Error, Result};

/// Smaller windows are dialogs and launchers, not the game (its smallest mode is 800×600).
const MIN_WIDTH: u32 = 640;
const MIN_HEIGHT: u32 = 480;

pub fn windows() -> Result<Vec<GameWindow>> {
    let mut handles: Vec<HWND> = Vec::new();
    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `lparam` is the `handles` vector below, alive for the whole enumeration.
        unsafe { (*(lparam.0 as *mut Vec<HWND>)).push(hwnd) };
        true.into()
    }
    // SAFETY: the callback only pushes to the vector passed in `lparam`.
    unsafe { EnumWindows(Some(collect), LPARAM(&mut handles as *mut Vec<HWND> as isize))? };

    let own = std::process::id();
    Ok(handles
        .into_iter()
        .filter(|&hwnd| unsafe { IsWindowVisible(hwnd).as_bool() } && !cloaked(hwnd) && !tool_window(hwnd))
        .filter(|&hwnd| {
            let mut pid = 0;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            pid != own
        })
        .filter_map(|hwnd| {
            let title = window_text(hwnd);
            if title.trim().is_empty() {
                return None;
            }
            let class = class_name(hwnd);
            let (width, height) = client_rect(hwnd).map(|r| (r.width, r.height)).unwrap_or((0, 0));
            if width < MIN_WIDTH || height < MIN_HEIGHT {
                return None;
            }
            let likely_game = looks_like_game(&title, &class);
            Some(GameWindow { id: hwnd.0 as isize, title, class, width, height, likely_game })
        })
        .collect())
}

pub struct Capturer {
    hwnd: HWND,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    winrt_device: IDirect3DDevice,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    size: SizeInt32,
    /// A picture has arrived, so a quiet spell means nothing changed.
    drawn: bool,
    /// Reused while the window keeps its size.
    staging: Option<(ID3D11Texture2D, u32, u32)>,
}

impl Capturer {
    pub fn new(id: isize) -> Result<Self> {
        let hwnd = HWND(id as *mut c_void);
        if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
            return Err(Error::Invalid("The game window has closed.".into()));
        }
        if !GraphicsCaptureSession::IsSupported()? {
            return Err(Error::Invalid("Windows Graphics Capture needs Windows 10 version 1903 or later.".into()));
        }

        let mut device = None;
        let mut context = None;
        // SAFETY: plain device creation; the out-pointers live until the call returns.
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?
        };
        let device: ID3D11Device = device.ok_or_else(|| Error::Invalid("No Direct3D device.".into()))?;
        let context = context.ok_or_else(|| Error::Invalid("No Direct3D context.".into()))?;
        let dxgi: IDXGIDevice = device.cast()?;
        let winrt_device: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;

        let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd)? };
        let size = item.Size()?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        // Neither is essential: older Windows always draws the cursor and a yellow border.
        let _ = session.SetIsCursorCaptureEnabled(false);
        let _ = session.SetIsBorderRequired(false);
        session.StartCapture()?;

        Ok(Self { hwnd, device, context, winrt_device, pool, session, size, drawn: false, staging: None })
    }

    pub fn grab(&mut self, wait: Duration) -> Result<Grab> {
        if !unsafe { IsWindow(Some(self.hwnd)).as_bool() } {
            return Ok(Grab::Closed);
        }
        if unsafe { IsIconic(self.hwnd).as_bool() } {
            return Ok(Grab::Minimized);
        }

        // Frames queue up between grabs; drop the stale ones and wait briefly for a fresh one,
        // falling back to the newest stale one if the screen is not changing.
        let mut latest = None;
        while let Some(frame) = self.next_frame() {
            latest = Some(frame);
        }
        let deadline = Instant::now() + wait;
        let frame = loop {
            if let Some(frame) = self.next_frame() {
                break Some(frame);
            }
            if Instant::now() >= deadline {
                break latest;
            }
            std::thread::sleep(Duration::from_millis(8));
        };
        let Some(frame) = frame else {
            return Ok(if self.drawn { Grab::Unchanged } else { Grab::Nothing });
        };

        let size = frame.ContentSize()?;
        if size != self.size {
            // The window was resized: the pool's buffers must follow, and this picture is torn.
            self.pool.Recreate(&self.winrt_device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?;
            self.size = size;
            return Ok(Grab::Nothing);
        }
        let picture = self.read(&frame)?;
        self.drawn = true;
        Ok(Grab::Frame(picture))
    }

    pub fn hdr(&self) -> bool {
        hdr_on(self.hwnd).unwrap_or(false)
    }

    fn next_frame(&self) -> Option<Direct3D11CaptureFrame> {
        // A null frame (none waiting) comes back as an error.
        self.pool.TryGetNextFrame().ok()
    }

    /// Copies the client area of `frame` into memory as RGBA.
    fn read(&mut self, frame: &Direct3D11CaptureFrame) -> Result<RgbaImage> {
        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
        let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };

        let staging = match &self.staging {
            Some((staging, w, h)) if *w == desc.Width && *h == desc.Height => staging.clone(),
            _ => {
                let staging_desc = D3D11_TEXTURE2D_DESC {
                    Usage: D3D11_USAGE_STAGING,
                    BindFlags: 0,
                    CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                    MiscFlags: 0,
                    ..desc
                };
                let mut staging = None;
                unsafe { self.device.CreateTexture2D(&staging_desc, None, Some(&mut staging))? };
                let staging = staging.ok_or_else(|| Error::Invalid("No staging texture.".into()))?;
                self.staging = Some((staging.clone(), desc.Width, desc.Height));
                staging
            }
        };

        // The picture covers the window's frame bounds; the game draws only in its client area.
        let content = (self.size.Width.max(0) as u32).min(desc.Width);
        let content_h = (self.size.Height.max(0) as u32).min(desc.Height);
        let (x, y, width, height) = match client_offset(self.hwnd) {
            Some((x, y, w, h)) => {
                let x = x.min(content);
                let y = y.min(content_h);
                (x, y, w.min(content - x), h.min(content_h - y))
            }
            None => (0, 0, content, content_h),
        };
        if width == 0 || height == 0 {
            return Err(Error::Invalid("The game window has no visible area.".into()));
        }

        unsafe { self.context.CopyResource(&staging, &texture) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { self.context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))? };
        let pitch = mapped.RowPitch as usize;
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for row in y..y + height {
            // SAFETY: the mapped texture is `desc.Height` rows of `pitch` bytes, and the client
            // rectangle was clamped to the texture above.
            let line = unsafe {
                std::slice::from_raw_parts(
                    (mapped.pData as *const u8).add(row as usize * pitch + x as usize * 4),
                    width as usize * 4,
                )
            };
            for px in line.as_chunks::<4>().0 {
                rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
        }
        unsafe { self.context.Unmap(&staging, 0) };
        RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| Error::Invalid("Captured picture has the wrong size.".into()))
    }
}

impl Drop for Capturer {
    fn drop(&mut self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}

struct ClientRect {
    width: u32,
    height: u32,
}

fn client_rect(hwnd: HWND) -> Option<ClientRect> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect).ok()? };
    Some(ClientRect { width: (rect.right - rect.left).max(0) as u32, height: (rect.bottom - rect.top).max(0) as u32 })
}

/// Where the client area sits inside the captured picture, and its size.
fn client_offset(hwnd: HWND) -> Option<(u32, u32, u32, u32)> {
    let mut bounds = RECT::default();
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut bounds as *mut RECT as *mut c_void,
            size_of::<RECT>() as u32,
        )
        .ok()?
    };
    let client = client_rect(hwnd)?;
    let mut origin = POINT::default();
    if !unsafe { ClientToScreen(hwnd, &mut origin).as_bool() } {
        return None;
    }
    let x = (origin.x - bounds.left).max(0) as u32;
    let y = (origin.y - bounds.top).max(0) as u32;
    Some((x, y, client.width, client.height))
}

fn window_text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..len])
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..len])
}

/// Windows hidden by the shell, such as suspended Store apps and other virtual desktops.
fn cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    let result = unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut c_void, size_of::<u32>() as u32)
    };
    result.is_ok() && cloaked != 0
}

fn tool_window(hwnd: HWND) -> bool {
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    style & WS_EX_TOOLWINDOW.0 != 0
}

/// Whether the display showing `hwnd` is in HDR (the PQ colour space).
fn hdr_on(hwnd: HWND) -> Result<bool> {
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1()? };
    for adapter in (0..).map_while(|i| unsafe { factory.EnumAdapters1(i) }.ok()) {
        for output in (0..).map_while(|i| unsafe { adapter.EnumOutputs(i) }.ok()) {
            let Ok(output) = output.cast::<IDXGIOutput6>() else { continue };
            let desc = unsafe { output.GetDesc1()? };
            if desc.Monitor == monitor {
                return Ok(desc.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020);
            }
        }
    }
    Ok(false)
}
