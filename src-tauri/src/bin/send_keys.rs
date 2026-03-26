//! Sidecar binary that injects keystrokes into another process's console.
//! Usage: send-keys <PID> <TEXT>

#[cfg(windows)]
fn main() {
    use std::env;
    use windows_sys::Win32::System::Console::*;

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        return;
    }
    let pid: u32 = match args[1].parse() {
        Ok(p) => p,
        Err(_) => return,
    };
    let text = &args[2];

    unsafe {
        FreeConsole();
        if AttachConsole(pid) == 0 {
            return;
        }

        let handle = GetStdHandle(STD_INPUT_HANDLE);
        if handle.is_null() || handle == -1_isize as *mut _ {
            FreeConsole();
            return;
        }

        for c in text.encode_utf16() {
            let down = INPUT_RECORD {
                EventType: KEY_EVENT as u16,
                Event: INPUT_RECORD_0 {
                    KeyEvent: KEY_EVENT_RECORD {
                        bKeyDown: 1,
                        wRepeatCount: 1,
                        wVirtualKeyCode: 0,
                        wVirtualScanCode: 0,
                        uChar: KEY_EVENT_RECORD_0 { UnicodeChar: c },
                        dwControlKeyState: 0,
                    },
                },
            };
            let up = INPUT_RECORD {
                EventType: KEY_EVENT as u16,
                Event: INPUT_RECORD_0 {
                    KeyEvent: KEY_EVENT_RECORD {
                        bKeyDown: 0,
                        wRepeatCount: 1,
                        wVirtualKeyCode: 0,
                        wVirtualScanCode: 0,
                        uChar: KEY_EVENT_RECORD_0 { UnicodeChar: c },
                        dwControlKeyState: 0,
                    },
                },
            };
            let mut written: u32 = 0;
            let records = [down, up];
            WriteConsoleInputW(handle, records.as_ptr(), 2, &mut written);
        }

        // Send Enter (VK_RETURN = 0x0D)
        let enter_down = INPUT_RECORD {
            EventType: KEY_EVENT as u16,
            Event: INPUT_RECORD_0 {
                KeyEvent: KEY_EVENT_RECORD {
                    bKeyDown: 1,
                    wRepeatCount: 1,
                    wVirtualKeyCode: 0x0D,
                    wVirtualScanCode: 0,
                    uChar: KEY_EVENT_RECORD_0 { UnicodeChar: 13 },
                    dwControlKeyState: 0,
                },
            },
        };
        let enter_up = INPUT_RECORD {
            EventType: KEY_EVENT as u16,
            Event: INPUT_RECORD_0 {
                KeyEvent: KEY_EVENT_RECORD {
                    bKeyDown: 0,
                    wRepeatCount: 1,
                    wVirtualKeyCode: 0x0D,
                    wVirtualScanCode: 0,
                    uChar: KEY_EVENT_RECORD_0 { UnicodeChar: 13 },
                    dwControlKeyState: 0,
                },
            },
        };
        let mut written: u32 = 0;
        let records = [enter_down, enter_up];
        WriteConsoleInputW(handle, records.as_ptr(), 2, &mut written);

        FreeConsole();
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("send-keys is only supported on Windows");
}
