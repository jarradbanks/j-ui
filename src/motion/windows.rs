//! A hidden top-level window receives setting broadcasts without polling or waking the UI.

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

thread_local! {
    static CONTEXT: std::cell::RefCell<Option<egui::Context>> = const { std::cell::RefCell::new(None) };
}

fn refresh(ctx: &egui::Context) {
    let mut on = windows::core::BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL into the live local variable.
    if unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut on as *mut windows::core::BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        )
    }
    .is_ok()
    {
        super::set_enabled(ctx, on.as_bool());
    }
}

pub fn watch(ctx: egui::Context) {
    refresh(&ctx);
    std::thread::spawn(move || {
        CONTEXT.with(|slot| *slot.borrow_mut() = Some(ctx));
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            lpszClassName: w!("j-ui-motion"),
            ..Default::default()
        };
        // SAFETY: the class name and callback live for the process; the window has no parent,
        // no user pointers and no WS_VISIBLE flag. This thread owns its blocking message loop.
        unsafe {
            if RegisterClassW(&class) == 0 {
                return;
            }
            let Ok(window) = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("j-ui-motion"),
                w!(""),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            ) else {
                return;
            };
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                DispatchMessageW(&message);
            }
            let _ = DestroyWindow(window);
        }
    });
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_SETTINGCHANGE {
        CONTEXT.with(|slot| {
            if let Some(ctx) = slot.borrow().as_ref() {
                refresh(ctx);
                ctx.request_repaint();
            }
        });
    }
    // SAFETY: forward the unchanged OS message and valid window handle to the default procedure.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}
