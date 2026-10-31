//! The tray icon and its menu, the same on both systems.

use std::cell::Cell;
use std::time::{Duration, Instant};

use langflip_core::Lang;
use rust_i18n::t;

use crate::language::{self, LANGUAGES};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub enum Action {
    Enable(bool),
    SwitchLayout,
    Autostart(bool),
    /// The person clicked a word in the «Исключения» submenu.
    RemoveKeepWord(String),
    /// The person picked a menu language. `None` follows the system.
    SetLanguage(Option<String>),
    OpenSettings,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    enabled: CheckMenuItem,
    autostart: CheckMenuItem,
    keep_words: Submenu,
    language: Submenu,
    /// «System language», then a row per language in [`LANGUAGES`].
    languages: Vec<CheckMenuItem>,
    open: MenuItem,
    quit: MenuItem,
    /// What the icon shows now, so a timer tick with no change skips the redraw.
    shown: Cell<(bool, Option<Lang>)>,
    /// A left click that waits to see whether a second one makes it a double click.
    click: Cell<Option<Instant>>,
}

/// The app icon with the layout in the corner. `mise run tray-icons` renders them from the SVGs beside them.
const EN_PNG: &[u8] = include_bytes!("../assets/tray/en.png");
const RU_PNG: &[u8] = include_bytes!("../assets/tray/ru.png");
const OTHER_PNG: &[u8] = include_bytes!("../assets/tray/other.png");
/// The menu id of a word in the «Исключения» submenu starts with this.
const KEEP_ID: &str = "keep:";
/// The menu id of a row in the Language submenu: this, then the code. The code is empty for the system.
const LANG_ID: &str = "lang:";
/// The Windows default double click time.
const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// The RGBA pixels and the side of a square PNG built into the exe.
fn pixels(png: &[u8]) -> (Vec<u8>, u32) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .expect("the tray PNGs are built in");
    let mut rgba = vec![0; reader.output_buffer_size().expect("a 32 px PNG fits")];
    let frame = reader
        .next_frame(&mut rgba)
        .expect("the tray PNGs are built in");
    rgba.truncate(frame.buffer_size());
    (rgba, frame.width)
}

/// The app icon with `EN` or `RU` in the corner, grey when off. Another layout shows the app icon alone.
fn rgba(on: bool, lang: Option<Lang>) -> (Vec<u8>, u32) {
    let (mut rgba, size) = pixels(match lang {
        Some(Lang::En) => EN_PNG,
        Some(Lang::Ru) => RU_PNG,
        None => OTHER_PNG,
    });
    if !on {
        for [r, g, b, _] in rgba.as_chunks_mut::<4>().0 {
            let grey = (u32::from(*r) * 30 + u32::from(*g) * 59 + u32::from(*b) * 11) / 100;
            [*r, *g, *b] = [grey as u8; 3];
        }
    }
    (rgba, size)
}

fn icon(on: bool, lang: Option<Lang>) -> Icon {
    let (rgba, size) = rgba(on, lang);
    Icon::from_rgba(rgba, size, size).expect("icon size matches its pixels")
}

fn tooltip(on: bool) -> String {
    if on {
        t!("tooltip.on")
    } else {
        t!("tooltip.off")
    }
    .into_owned()
}

impl Tray {
    /// The menu shows its checks and words once [`Tray::set_language`] runs.
    pub fn new(enabled: bool, autostart: bool) -> Tray {
        let enabled_item = CheckMenuItem::new(t!("menu.fix"), true, enabled, None);
        let autostart_item = CheckMenuItem::new(t!("menu.autostart"), true, autostart, None);
        let keep_words = Submenu::new(t!("menu.keepWords", count = 0), true);
        let language_menu = Submenu::new(t!("menu.language"), true);
        let system = ("", t!("menu.languageSystem").into_owned());
        let rows = LANGUAGES.map(|(code, name)| (code, name.to_owned()));
        let languages: Vec<CheckMenuItem> = std::iter::once(system)
            .chain(rows)
            .map(|(code, name)| {
                CheckMenuItem::with_id(format!("{LANG_ID}{code}"), name, true, false, None)
            })
            .collect();
        let _ = language_menu.append(&languages[0]);
        let _ = language_menu.append(&PredefinedMenuItem::separator());
        for item in &languages[1..] {
            let _ = language_menu.append(item);
        }
        let open = MenuItem::new(t!("menu.settings"), true, None);
        let quit = MenuItem::new(t!("menu.quit"), true, None);
        let menu = Menu::new();
        let _ = menu.append(&enabled_item);
        let _ = menu.append(&autostart_item);
        let _ = menu.append(&keep_words);
        let _ = menu.append(&language_menu);
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
            keep_words,
            language: language_menu,
            languages,
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
        } else if let Some(word) = id.as_ref().strip_prefix(KEEP_ID) {
            Some(Action::RemoveKeepWord(word.to_owned()))
        } else if let Some(code) = id.as_ref().strip_prefix(LANG_ID) {
            Some(Action::SetLanguage(
                (!code.is_empty()).then(|| code.to_owned()),
            ))
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

    /// Lists the words Langflip keeps as typed. A click on one removes it.
    pub fn set_keep_words(&self, words: &[String]) {
        while self.keep_words.remove_at(0).is_some() {}
        self.keep_words
            .set_text(t!("menu.keepWords", count = words.len()));
        let hint = if words.is_empty() {
            t!("menu.keepWordsNone")
        } else {
            t!("menu.keepWordsHint")
        };
        let _ = self.keep_words.append(&MenuItem::new(hint, false, None));
        for word in words {
            let item = MenuItem::with_id(format!("{KEEP_ID}{word}"), word, true, None);
            let _ = self.keep_words.append(&item);
        }
    }

    /// Checks the chosen language and relabels the menu in the language now set.
    pub fn set_language(&self, choice: Option<&str>, keep_words: &[String]) {
        let chosen = format!("{LANG_ID}{}", language::chosen(choice).unwrap_or_default());
        for item in &self.languages {
            item.set_checked(item.id().as_ref() == chosen);
        }
        self.enabled.set_text(t!("menu.fix"));
        self.autostart.set_text(t!("menu.autostart"));
        self.language.set_text(t!("menu.language"));
        self.languages[0].set_text(t!("menu.languageSystem"));
        self.open.set_text(t!("menu.settings"));
        self.quit.set_text(t!("menu.quit"));
        let _ = self
            .icon
            .set_tooltip(Some(tooltip(self.enabled.is_checked())));
        self.set_keep_words(keep_words);
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

    const LANGS: [Option<Lang>; 3] = [Some(Lang::En), Some(Lang::Ru), None];

    #[test]
    fn each_layout_has_a_32_px_rgba_icon() {
        for lang in LANGS {
            let (rgba, size) = rgba(true, lang);
            assert_eq!((size, rgba.len()), (32, 32 * 32 * 4));
        }
    }

    #[test]
    fn layouts_look_different() {
        let [en, ru, other] = LANGS.map(|lang| rgba(true, lang).0);
        assert!(en != ru && en != other && ru != other);
    }

    #[test]
    fn off_is_grey() {
        for lang in LANGS {
            let (rgba, _) = rgba(false, lang);
            assert!(
                rgba.as_chunks::<4>()
                    .0
                    .iter()
                    .all(|[r, g, b, _]| r == g && g == b)
            );
        }
    }
}
