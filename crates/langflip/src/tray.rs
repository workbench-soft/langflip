//! The tray icon and its menu, the same on both systems.

use std::cell::Cell;
use std::time::{Duration, Instant};

use langflip_core::Lang;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub enum Action {
    Enable(bool),
    SwitchLayout,
    Autostart(bool),
    OpenSettings,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    enabled: CheckMenuItem,
    autostart: CheckMenuItem,
    open: MenuItem,
    quit: MenuItem,
    /// What the icon shows now, so a timer tick with no change skips the redraw.
    shown: Cell<(bool, Option<Lang>)>,
    /// A left click that waits to see whether a second one makes it a double click.
    click: Cell<Option<Instant>>,
}

const SIZE: u32 = 32;
/// The Windows default double click time.
const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// Letters 5 pixels wide and 7 high, a row per byte, the top bit of the 5 on the left.
fn glyph(c: u8) -> [u8; 7] {
    match c {
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        _ => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
    }
}

/// Whether the pixel falls on a letter of the 2 letter code, drawn twice the font size.
fn on_letter(code: &[u8; 2], x: u32, y: u32) -> bool {
    const SCALE: u32 = 2;
    const LEFT: u32 = 5;
    const TOP: u32 = 9;
    let (Some(x), Some(y)) = (x.checked_sub(LEFT), y.checked_sub(TOP)) else {
        return false;
    };
    let (gx, gy) = (x / SCALE, y / SCALE);
    // A letter is 5 columns wide, then 1 blank column.
    let (letter, col) = (gx / 6, gx % 6);
    if letter > 1 || col > 4 || gy > 6 {
        return false;
    }
    glyph(code[letter as usize])[gy as usize] & (0x10 >> col) != 0
}

/// `EN` on blue, `RU` on red, grey when off. Another layout gets a square split on the diagonal.
fn icon(on: bool, lang: Option<Lang>) -> Icon {
    let code = lang.map(|l| match l {
        Lang::En => b"EN",
        Lang::Ru => b"RU",
    });
    let back = match (on, lang) {
        (false, _) => [120, 120, 120, 255],
        (true, Some(Lang::Ru)) => [220, 38, 38, 255],
        (true, _) => [37, 99, 235, 255],
    };
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let edge = x < 2 || y < 2 || x >= SIZE - 2 || y >= SIZE - 2;
            let color = match code {
                _ if edge => [255, 255, 255, 255],
                Some(code) if on_letter(code, x, y) => [255, 255, 255, 255],
                Some(_) => back,
                None if x + y < SIZE => back,
                None if on => [220, 38, 38, 255],
                None => [170, 170, 170, 255],
            };
            rgba.extend_from_slice(&color);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("icon size matches its pixels")
}

fn tooltip(on: bool) -> &'static str {
    if on {
        "Langflip включён. Щелчок выключает, двойной щелчок меняет раскладку"
    } else {
        "Langflip выключен. Щелчок включает, двойной щелчок меняет раскладку"
    }
}

impl Tray {
    pub fn new(enabled: bool, autostart: bool) -> Tray {
        let enabled_item = CheckMenuItem::new("Исправлять раскладку", true, enabled, None);
        let autostart_item = CheckMenuItem::new("Запускать при входе", true, autostart, None);
        let open = MenuItem::new("Открыть настройки", true, None);
        let quit = MenuItem::new("Выход", true, None);
        let menu = Menu::new();
        let _ = menu.append(&enabled_item);
        let _ = menu.append(&autostart_item);
        let _ = menu.append(&open);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&quit);
        let icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            // The left click pauses fixing, the double click switches the layout,
            // the right click opens the menu.
            .with_menu_on_left_click(false)
            .with_tooltip(tooltip(enabled))
            .with_icon(icon(enabled, None))
            .build()
            .expect("the tray icon could not be created");
        Tray {
            icon,
            enabled: enabled_item,
            autostart: autostart_item,
            open,
            quit,
            shown: Cell::new((enabled, None)),
            click: Cell::new(None),
        }
    }

    /// The next click on the icon or the menu, if one is waiting.
    pub fn poll(&self) -> Option<Action> {
        if self
            .click
            .get()
            .is_some_and(|t| t.elapsed() >= DOUBLE_CLICK)
        {
            self.click.set(None);
            let on = !self.enabled.is_checked();
            self.set_enabled(on);
            return Some(Action::Enable(on));
        }
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // Both systems report each click, so 2 in a row make the double click.
                if self.click.take().is_some() {
                    return Some(Action::SwitchLayout);
                }
                self.click.set(Some(Instant::now()));
            }
        }
        let event = MenuEvent::receiver().try_recv().ok()?;
        let id = event.id();
        if id == self.enabled.id() {
            let on = self.enabled.is_checked();
            self.set_enabled(on);
            Some(Action::Enable(on))
        } else if id == self.autostart.id() {
            Some(Action::Autostart(self.autostart.is_checked()))
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
        self.show(on, self.shown.get().1);
        let _ = self.icon.set_tooltip(Some(tooltip(on)));
    }

    /// Shows the layout of the window in front. `None` stands for a layout other than these 2.
    pub fn set_lang(&self, lang: Option<Lang>) {
        self.show(self.shown.get().0, lang);
    }

    fn show(&self, on: bool, lang: Option<Lang>) {
        if self.shown.replace((on, lang)) != (on, lang) {
            let _ = self.icon.set_icon(Some(icon(on, lang)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The letters as text, `#` for a lit pixel, one font pixel per character.
    fn picture(code: &[u8; 2]) -> Vec<String> {
        (0..7)
            .map(|gy| {
                (0..11)
                    .map(|gx| {
                        if on_letter(code, 5 + gx * 2, 9 + gy * 2) {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn draws_en() {
        assert_eq!(picture(b"EN")[0], "#####.#...#");
        assert_eq!(picture(b"EN")[3], "####..#..##");
    }

    #[test]
    fn draws_ru() {
        assert_eq!(picture(b"RU")[0], "####..#...#");
        assert_eq!(picture(b"RU")[6], "#...#..###.");
    }

    #[test]
    fn letters_stay_inside_the_border() {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let edge = x < 2 || y < 2 || x >= SIZE - 2 || y >= SIZE - 2;
                assert!(!(edge && on_letter(b"EN", x, y)));
            }
        }
    }
}
