// 업무 캘린더 Windows 프로그램
// - 화면은 www/ (웹 앱과 같은 파일)을 그대로 쓴다.
// - 바탕화면 고정 모드: 테두리 없이 다른 창들 뒤(바탕화면 위)에 늘 떠 있는 창
// - 트레이 아이콘: 열기 / 바탕화면에 고정 / Windows 시작 시 실행 / 종료
// - 창을 닫으면 트레이로 숨고, 완전히 끄려면 트레이 → 종료
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent, Wry,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_window_state::StateFlags;

#[derive(Default)]
struct AppState {
    pinned: Mutex<bool>,
    pin_item: Mutex<Option<CheckMenuItem<Wry>>>,
    auto_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

#[derive(serde::Serialize)]
struct DesktopState {
    pinned: bool,
    autostart: bool,
}

fn pin_file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("pinned"))
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn apply_pin(app: &AppHandle, pinned: bool) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.set_decorations(!pinned);
        let _ = w.set_always_on_bottom(pinned);
        let _ = w.set_skip_taskbar(pinned);
        let _ = w.unminimize();
        let _ = w.show();
        if !pinned {
            let _ = w.set_focus();
        }
    }
    let st = app.state::<AppState>();
    *st.pinned.lock().unwrap() = pinned;
    if let Some(item) = st.pin_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(pinned);
    }
    if let Some(f) = pin_file(app) {
        if let Some(dir) = f.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::write(f, if pinned { "1" } else { "0" });
    }
    let _ = app.emit("desktop-state", ());
}

fn set_auto(app: &AppHandle, on: bool) {
    let al = app.autolaunch();
    let _ = if on { al.enable() } else { al.disable() };
    let now = al.is_enabled().unwrap_or(false);
    if let Some(item) = app.state::<AppState>().auto_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(now);
    }
    let _ = app.emit("desktop-state", ());
}

#[tauri::command]
fn desktop_state(app: AppHandle) -> DesktopState {
    DesktopState {
        pinned: *app.state::<AppState>().pinned.lock().unwrap(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

#[tauri::command]
fn set_pinned(app: AppHandle, pinned: bool) -> DesktopState {
    apply_pin(&app, pinned);
    desktop_state(app)
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> DesktopState {
    set_auto(&app, enabled);
    desktop_state(app)
}

fn main() {
    tauri::Builder::default()
        // 두 번 실행하면 새 창 대신 기존 창을 띄운다 (가장 먼저 등록해야 함)
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![desktop_state, set_pinned, set_autostart])
        .setup(|app| {
            let handle = app.handle().clone();
            let open = MenuItem::with_id(app, "open", "열기", true, None::<&str>)?;
            let pin = CheckMenuItem::with_id(app, "pin", "바탕화면에 고정", true, false, None::<&str>)?;
            let auto_on = app.autolaunch().is_enabled().unwrap_or(false);
            let auto = CheckMenuItem::with_id(app, "autostart", "Windows 시작 시 실행", true, auto_on, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &pin, &auto, &sep, &quit])?;
            {
                let st = app.state::<AppState>();
                *st.pin_item.lock().unwrap() = Some(pin.clone());
                *st.auto_item.lock().unwrap() = Some(auto.clone());
            }

            let mut tray = TrayIconBuilder::with_id("main")
                .tooltip("업무 캘린더")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id().as_ref() {
                    "open" => show_main(app),
                    "pin" => {
                        let now = *app.state::<AppState>().pinned.lock().unwrap();
                        apply_pin(app, !now);
                    }
                    "autostart" => {
                        let now = app.autolaunch().is_enabled().unwrap_or(false);
                        set_auto(app, !now);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = e
                    {
                        show_main(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // 지난번 고정 상태 복원 후 창 표시
            let pinned = pin_file(&handle)
                .and_then(|f| fs::read_to_string(f).ok())
                .map(|s| s.trim() == "1")
                .unwrap_or(false);
            apply_pin(&handle, pinned);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("업무 캘린더 실행 중 오류");
}
