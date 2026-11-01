//! Windows: reads the text before the caret through UI Automation, on a thread of its own.
//!
//! A UI Automation call waits on the app in front, at times for long. The hooks run on the main
//! thread, and Windows drops a hook that keeps it waiting, so the call runs here instead.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, TextPatternRangeEndpoint_End,
    TextPatternRangeEndpoint_Start, TextUnit_Character, UIA_TextPatternId,
};
use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;

/// How many characters before the caret to read: enough for one word.
const CHARS: i32 = 40;
/// How long to wait after a click or a key, so the app has moved its caret.
const SETTLE: Duration = Duration::from_millis(40);

pub struct Caret {
    requests: Sender<u64>,
    results: Receiver<(u64, String)>,
}

impl Caret {
    /// Starts the thread. Each result posts `wake` to `main_thread`.
    pub fn start(main_thread: u32, wake: u32) -> Caret {
        let (requests, inbox) = channel::<u64>();
        let (outbox, results) = channel();
        std::thread::spawn(move || unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let Ok(uia) =
                CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            else {
                return;
            };
            while let Ok(mut id) = inbox.recv() {
                std::thread::sleep(SETTLE);
                // Only the latest request counts.
                while let Ok(next) = inbox.try_recv() {
                    id = next;
                }
                let text = text_before_caret(&uia).unwrap_or_default();
                if outbox.send((id, text)).is_err() {
                    return;
                }
                let _ = PostThreadMessageW(main_thread, wake, WPARAM(0), LPARAM(0));
            }
        });
        Caret { requests, results }
    }

    /// Asks for the text before the caret. The answer carries `id`.
    pub fn request(&self, id: u64) {
        let _ = self.requests.send(id);
    }

    /// The latest answer, if one came.
    pub fn take(&self) -> Option<(u64, String)> {
        self.results.try_iter().last()
    }
}

unsafe fn text_before_caret(uia: &IUIAutomation) -> windows::core::Result<String> {
    unsafe {
        let pattern: IUIAutomationTextPattern = uia
            .GetFocusedElement()?
            .GetCurrentPatternAs(UIA_TextPatternId)?;
        let selection = pattern.GetSelection()?;
        if selection.Length()? == 0 {
            return Ok(String::new());
        }
        let range = selection.GetElement(0)?;
        // Typing replaces a selection, so the word goes on from where the selection starts.
        range.MoveEndpointByRange(
            TextPatternRangeEndpoint_End,
            &range,
            TextPatternRangeEndpoint_Start,
        )?;
        range.MoveEndpointByUnit(TextPatternRangeEndpoint_Start, TextUnit_Character, -CHARS)?;
        Ok(range.GetText(CHARS)?.to_string())
    }
}
