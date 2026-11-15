//! Windows: a dialog on the first start that says Langflip runs, names its keys and points at the icon.

use rust_i18n::t;
use windows::Win32::UI::Controls::TD_INFORMATION_ICON;

/// Shows the dialog on a thread of its own. A modal loop on the main thread would drop the
/// messages the hooks post there.
pub fn show() {
    if !crate::update::is_release() {
        return;
    }
    std::thread::spawn(|| {
        crate::dialog::show(
            &t!("welcome.title"),
            &t!("welcome.body"),
            &[],
            TD_INFORMATION_ICON,
        );
    });
}
