use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, ListenerOptions, Stream};

pub struct McpTransport {
    stop: Arc<AtomicBool>,
}

const DEFAULT_SOCKET_NAME: &str = "orbit-mcp";

pub fn socket_name_for(socket_name: &str) -> interprocess::local_socket::Name<'static> {
    if GenericNamespaced::is_supported() {
        let leaked: &'static str = Box::leak(socket_name.to_string().into_boxed_str());
        leaked.to_ns_name::<GenericNamespaced>().unwrap()
    } else {
        let path = format!("/tmp/{socket_name}.sock");
        let leaked: &'static str = Box::leak(path.into_boxed_str());
        leaked.to_fs_name::<GenericFilePath>().unwrap()
    }
}

impl McpTransport {
    pub fn start(handler: Arc<dyn Fn(&str) -> String + Send + Sync + 'static>) -> Self {
        Self::start_named(DEFAULT_SOCKET_NAME, handler)
    }

    pub fn start_named(
        socket_name: &str,
        handler: Arc<dyn Fn(&str) -> String + Send + Sync + 'static>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        let socket_name_owned = socket_name.to_string();

        #[cfg(not(windows))]
        if !GenericNamespaced::is_supported() {
            let sock_path = format!("/tmp/{socket_name}.sock");
            let _ = std::fs::remove_file(&sock_path);
        }

        let name = socket_name_for(socket_name);
        let listener = match ListenerOptions::new().name(name).create_sync() {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[orbit:mcp] failed to create local socket listener: {e}");
                return Self { stop };
            }
        };

        std::thread::spawn(move || {
            eprintln!("[orbit:mcp] transport listening on {socket_name_owned}");
            for conn in listener.incoming() {
                if stop_clone.load(Ordering::Relaxed) {
                    break;
                }
                match conn {
                    Ok(stream) => {
                        let h = Arc::clone(&handler);
                        let s = Arc::clone(&stop_clone);
                        std::thread::spawn(move || {
                            handle_connection(stream, h, s);
                        });
                    }
                    Err(e) => {
                        eprintln!("[orbit:mcp] connection accept error: {e}");
                    }
                }
            }
        });

        Self { stop }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_connection(
    stream: Stream,
    handler: Arc<dyn Fn(&str) -> String + Send + Sync>,
    stop: Arc<AtomicBool>,
) {
    let mut buf_read = std::io::BufReader::new(&stream);
    let mut line = String::new();

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        line.clear();
        match buf_read.read_line(&mut line) {
            Ok(0) => break,
            Err(e) => {
                eprintln!("[orbit:mcp] read error: {e}");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let response = handler(trimmed);
                if response.is_empty() {
                    continue;
                }
                let mut writer = &stream;
                if let Err(e) = writer.write_all(response.as_bytes()) {
                    eprintln!("[orbit:mcp] write error: {e}");
                    break;
                }
                if let Err(e) = writer.write_all(b"\n") {
                    eprintln!("[orbit:mcp] write newline error: {e}");
                    break;
                }
                if let Err(e) = writer.flush() {
                    eprintln!("[orbit:mcp] flush error: {e}");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_connects_and_responds() {
        let handler = Arc::new(|req: &str| {
            if req.contains("hello") {
                r#"{"jsonrpc":"2.0","id":1,"result":{"echo":"hello"}}"#.to_string()
            } else {
                r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32600}}"#.to_string()
            }
        });

        let socket_name = format!(
            "orbit-mcp-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        );

        let transport = McpTransport::start_named(&socket_name, handler);

        std::thread::sleep(std::time::Duration::from_millis(100));

        let stream =
            Stream::connect(socket_name_for(&socket_name)).expect("connect to local socket");

        let mut writer = &stream;
        let req = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{"msg":"hello"}}"#;
        writer.write_all(req.as_bytes()).unwrap();
        writer.write_all(b"\n").unwrap();
        writer.flush().unwrap();

        let mut reader = std::io::BufReader::new(&stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(
            line.contains("\"echo\":\"hello\""),
            "expected echo response, got: {line}"
        );

        transport.stop();
    }
}
