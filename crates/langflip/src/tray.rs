//! The tray icon and its menu, the same on both systems.

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub enum Action {
    Enable(bool),
    #[cfg_attr(not(windows), allow(dead_code))]
    Autostart(bool),
    OpenSettings,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    enabled: CheckMenuItem,
    autostart: Option<CheckMenuItem>,
    open: MenuItem,
    quit: MenuItem,
}

const SIZE: u32 = 32;

/// A square split on the diagonal: blue for English, red for Russian, grey when off.
fn icon(on: bool) -> Icon {
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let edge = x < 2 || y < 2 || x >= SIZE - 2 || y >= SIZE - 2;
            let color = match (on, edge, x + y < SIZE) {
                (_, true, _) => [255, 255, 255, 255],
                (false, _, true) => [120, 120, 120, 255],
                (false, _, false) => [170, 170, 170, 255],
                (true, _, true) => [37, 99, 235, 255],
                (true, _, false) => [220, 38, 38, 255],
            };
            rgba.extend_from_slice(&color);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("icon size matches its pixels")
}

impl Tray {
    /// `autostart` is None where the system has no autostart toggle.
    pub fn new(enabled: bool, autostart: Option<bool>) -> Tray {
        let enabled_item = CheckMenuItem::new("Исправлять раскладку", true, enabled, None);
        let autostart_item =
            autostart.map(|on| CheckMenuItem::new("Запускать при входе", true, on, None));
        let open = MenuItem::new("Открыть настройки", true, None);
        let quit = MenuItem::new("Выход", true, None);
        let menu = Menu::new();
        let _ = menu.append(&enabled_item);
        if let Some(item) = &autostart_item {
            let _ = menu.append(item);
        }
        let _ = menu.append(&open);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit);
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Langflip")
            .with_icon(icon(enabled))
            .build()
            .expect("the tray icon could not be created");
        Tray {
            icon,
            enabled: enabled_item,
            autostart: autostart_item,
            open,
            quit,
        }
    }

    /// The next menu click, if one is waiting.
    pub fn poll(&self) -> Option<Action> {
        let event = MenuEvent::receiver().try_recv().ok()?;
        let id = event.id();
        if id == self.enabled.id() {
            let on = self.enabled.is_checked();
            let _ = self.icon.set_icon(Some(icon(on)));
            Some(Action::Enable(on))
        } else if self.autostart.as_ref().is_some_and(|a| id == a.id()) {
            Some(Action::Autostart(self.autostart.as_ref()?.is_checked()))
        } else if id == self.open.id() {
            Some(Action::OpenSettings)
        } else if id == self.quit.id() {
            Some(Action::Quit)
        } else {
            None
        }
    }

    /// Shows a change made in the settings file.
    pub fn set_enabled(&self, on: bool) {
        self.enabled.set_checked(on);
        let _ = self.icon.set_icon(Some(icon(on)));
    }
}
