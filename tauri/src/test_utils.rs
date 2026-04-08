/// Pretty test reporter for Orbit's Rust tests.
///
/// Prints a formatted header on start, labels each assertion with ✓ or ✗,
/// and prints a final PASSED / FAILED summary on drop.
///
/// # Usage
///
/// ```rust
/// use crate::test_utils::TestCase;
///
/// #[test]
/// fn should_create_project_with_correct_name() {
///     let mut t = TestCase::new("should_create_project_with_correct_name");
///
///     t.phase("Seed");
///     let db = make_db();
///
///     t.phase("Act");
///     let project = db.create_project("my-app", "/tmp/my-app").unwrap();
///
///     t.phase("Assert");
///     t.eq("project.name", &project.name, &"my-app".to_string());
///     t.ok("project.id is positive", project.id > 0);
/// }
/// ```
///
/// Output with `cargo test -- --nocapture`:
///
/// ```text
/// ┌──────── should_create_project_with_correct_name ────────┐
///   ▸ Seed
///   ▸ Act
///   ▸ Assert
///   ✓ project.name
///   ✓ project.id is positive
///   └─ PASSED (2 checks)
/// ```

// ANSI escape codes
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";

const BOX_WIDTH: usize = 62;

pub struct TestCase {
    name: String,
    passed: usize,
}

impl TestCase {
    /// Starts a new test case and prints the header.
    ///
    /// Call at the top of every `#[test]` function, before any setup.
    pub fn new(name: &str) -> Self {
        let inner = format!(" {} ", name);
        let dash_count = BOX_WIDTH.saturating_sub(inner.len());
        let left = dash_count / 2;
        let right = dash_count - left;

        println!(
            "\n{CYAN}{BOLD}┌{left}{inner}{right}┐{RESET}",
            left = "─".repeat(left),
            right = "─".repeat(right),
        );

        Self {
            name: name.to_string(),
            passed: 0,
        }
    }

    /// Labels the current phase of the test cycle.
    ///
    /// Use with: `"Seed"`, `"Act"`, `"Assert"`, `"Cleanup"`.
    pub fn phase(&self, label: &str) {
        println!("  {BLUE}{DIM}▸ {label}{RESET}");
    }

    /// Asserts a boolean condition is `true`.
    ///
    /// Prints `✓ desc` on pass, `✗ desc` on fail (then panics).
    pub fn ok(&mut self, desc: &str, condition: bool) {
        if condition {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!("      {YELLOW}condition was false{RESET}");
            panic!("assertion failed: {desc}");
        }
    }

    /// Asserts `left == right`.
    ///
    /// Prints both values on failure for easy debugging.
    pub fn eq<T: PartialEq + std::fmt::Debug>(&mut self, desc: &str, left: T, right: T) {
        if left == right {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!("      {YELLOW}got:     {RESET}{left:?}");
            println!("      {YELLOW}expected:{RESET} {right:?}");
            panic!(
                "assertion failed: {desc}\n  got:      {left:?}\n  expected: {right:?}",
                left = left,
                right = right
            );
        }
    }

    /// Asserts `left != right`.
    pub fn ne<T: PartialEq + std::fmt::Debug>(&mut self, desc: &str, left: T, right: T) {
        if left != right {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!("      {YELLOW}both sides were:{RESET} {left:?}");
            panic!("assertion failed (expected different): {desc}\n  value: {left:?}");
        }
    }

    /// Asserts an `Option` is `Some`.
    pub fn some<T>(&mut self, desc: &str, opt: &Option<T>) {
        if opt.is_some() {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!("      {YELLOW}expected Some(_), got None{RESET}");
            panic!("assertion failed: {desc} — expected Some, got None");
        }
    }

    /// Asserts an `Option` is `None`.
    pub fn none<T: std::fmt::Debug>(&mut self, desc: &str, opt: &Option<T>) {
        if opt.is_none() {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!(
                "      {YELLOW}expected None, got Some({:?}){RESET}",
                opt.as_ref().unwrap()
            );
            panic!("assertion failed: {desc} — expected None");
        }
    }

    /// Asserts a `Result` is `Ok`.
    pub fn is_ok<T, E: std::fmt::Debug>(&mut self, desc: &str, result: &Result<T, E>) {
        if result.is_ok() {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!(
                "      {YELLOW}expected Ok(_), got Err({:?}){RESET}",
                result.as_ref().err().unwrap()
            );
            panic!("assertion failed: {desc} — expected Ok");
        }
    }

    /// Asserts a `Result` is `Err`.
    pub fn is_err<T: std::fmt::Debug, E>(&mut self, desc: &str, result: &Result<T, E>) {
        if result.is_err() {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc}");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!(
                "      {YELLOW}expected Err(_), got Ok({:?}){RESET}",
                result.as_ref().ok().unwrap()
            );
            panic!("assertion failed: {desc} — expected Err");
        }
    }

    /// Asserts a slice/vec has the expected length.
    pub fn len<T>(&mut self, desc: &str, collection: &[T], expected: usize) {
        let actual = collection.len();
        if actual == expected {
            self.passed += 1;
            println!("  {GREEN}✓{RESET} {desc} (len = {expected})");
        } else {
            println!("  {RED}✗{RESET} {RED}{BOLD}{desc}{RESET}");
            println!("      {YELLOW}got len:{RESET}      {actual}");
            println!("      {YELLOW}expected len:{RESET} {expected}");
            panic!("assertion failed: {desc}\n  got len: {actual}\n  expected len: {expected}");
        }
    }

    /// Asserts a slice/vec is empty.
    pub fn empty<T>(&mut self, desc: &str, collection: &[T]) {
        self.len(desc, collection, 0);
    }
}

impl Drop for TestCase {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let inner = format!(" FAILED: {} ", self.name);
            let dash_count = BOX_WIDTH.saturating_sub(inner.len());
            let left = dash_count / 2;
            let right = dash_count - left;
            println!(
                "  {RED}{BOLD}└{left}{inner}{right}┘{RESET}\n",
                left = "─".repeat(left),
                right = "─".repeat(right),
            );
        } else {
            println!(
                "  {GREEN}{BOLD}└─ PASSED{RESET} {DIM}({} checks){RESET}\n",
                self.passed
            );
        }
    }
}

// ─── JSONL Line Builders ──────────────────────────────────────────────────────
//
// Canonical test fixtures for Claude Code stream-json format.
// Use these in every test that needs JSONL input — never inline raw strings.

/// A valid `assistant` line with a text block.
pub fn assistant_text(text: &str) -> String {
    let text_j = serde_json::to_string(text).expect("assistant_text: serialize text");
    format!(
        r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","stop_reason":null,"content":[{{"type":"text","text":{text_j}}}],"usage":{{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
    )
}

/// A valid `assistant` line with `stop_reason: end_turn`.
pub fn assistant_end_turn(text: &str) -> String {
    let text_j = serde_json::to_string(text).expect("assistant_end_turn: serialize text");
    format!(
        r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","stop_reason":"end_turn","content":[{{"type":"text","text":{text_j}}}],"usage":{{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
    )
}

/// A valid `assistant` line with a thinking block.
pub fn assistant_thinking(thinking: &str) -> String {
    let thinking_j =
        serde_json::to_string(thinking).expect("assistant_thinking: serialize thinking");
    format!(
        r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","stop_reason":null,"content":[{{"type":"thinking","thinking":{thinking_j}}}],"usage":{{"input_tokens":5,"output_tokens":2,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
    )
}

/// A valid `assistant` line with a `tool_use` block.
/// `input_json` must be a valid JSON object string, e.g. `r#"{"command":"ls"}"#`.
pub fn assistant_tool_use(tool: &str, input_json: &str) -> String {
    let tool_j = serde_json::to_string(tool).expect("assistant_tool_use: serialize tool");
    format!(
        r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","stop_reason":null,"content":[{{"type":"tool_use","id":"toolu_01","name":{tool_j},"input":{input_json}}}],"usage":{{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":0}}}}}}"#
    )
}

/// A valid `user` line with plain text content.
pub fn user_text(text: &str) -> String {
    let text_j = serde_json::to_string(text).expect("user_text: serialize text");
    format!(
        r#"{{"type":"user","timestamp":"2026-01-01T00:00:01Z","message":{{"content":{text_j}}}}}"#
    )
}

/// A valid `user` line with a `tool_result` block.
pub fn tool_result(content: &str) -> String {
    let content_j = serde_json::to_string(content).expect("tool_result: serialize content");
    format!(
        r#"{{"type":"user","timestamp":"2026-01-01T00:00:01Z","message":{{"content":[{{"type":"tool_result","tool_use_id":"toolu_01","content":{content_j}}}]}}}}"#
    )
}

/// A `system` stop_hook_summary line.
pub fn system_stop_hook() -> String {
    r#"{"type":"system","timestamp":"2026-01-01T00:00:02Z","message":{"subtype":"stop_hook_summary"}}"#.to_string()
}

/// A `progress` line with streaming content.
pub fn progress_line(content: &str) -> String {
    let content_j = serde_json::to_string(content).expect("progress_line: serialize content");
    format!(r#"{{"type":"progress","timestamp":"2026-01-01T00:00:00Z","content":{content_j}}}"#)
}

/// An `assistant` line with token counts set explicitly.
pub fn assistant_with_tokens(
    text: &str,
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
) -> String {
    let text_j = serde_json::to_string(text).expect("assistant_with_tokens: serialize text");
    format!(
        r#"{{"type":"assistant","timestamp":"2026-01-01T00:00:00Z","message":{{"model":"claude-sonnet-4-6","stop_reason":null,"content":[{{"type":"text","text":{text_j}}}],"usage":{{"input_tokens":{input},"output_tokens":{output},"cache_creation_input_tokens":{cache_write},"cache_read_input_tokens":{cache_read}}}}}}}"#
    )
}

// ─── Database Fixtures ────────────────────────────────────────────────────────

/// Creates an in-memory `DatabaseService` wrapped in `Arc`, ready to use.
/// Returns `Arc` because `SessionManager::new()` requires `Arc<DatabaseService>`.
/// Panics with a clear message if the DB cannot be opened.
/// Use in every test that needs a database — never share across tests.
pub fn make_db() -> std::sync::Arc<crate::services::database::DatabaseService> {
    std::sync::Arc::new(
        crate::services::database::DatabaseService::open_in_memory()
            .expect("test setup: failed to open in-memory DB"),
    )
}

/// Inserts a session and returns its ID. Panics with a clear message if it fails.
pub fn seed_session(db: &crate::services::database::DatabaseService) -> crate::models::SessionId {
    db.create_session(None, Some("test-session"), "C:/test/proj", "ignore", None)
        .expect("test setup: seed_session failed")
}

/// Inserts multiple JSONL output lines into a session.
pub fn seed_outputs(
    db: &crate::services::database::DatabaseService,
    session_id: crate::models::SessionId,
    lines: &[&str],
) {
    for line in lines {
        db.insert_output(session_id, line)
            .expect("test setup: seed_outputs failed");
    }
}

// ─── File Fixtures ────────────────────────────────────────────────────────────

/// Writes JSONL lines to a file inside a `TempDir`. Returns the file path.
/// The `TempDir` must be kept alive for the duration of the test.
pub fn write_jsonl(dir: &tempfile::TempDir, filename: &str, lines: &[&str]) -> std::path::PathBuf {
    use std::io::Write;
    let path = dir.path().join(filename);
    let mut f =
        std::fs::File::create(&path).expect("test setup: write_jsonl could not create file");
    for line in lines {
        writeln!(f, "{}", line).expect("test setup: write_jsonl could not write line");
    }
    path
}

/// Appends JSONL lines to an existing file. Use for incremental `parse_journal` tests.
pub fn append_jsonl(path: &std::path::Path, lines: &[&str]) {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("test setup: append_jsonl could not open file");
    for line in lines {
        writeln!(f, "{}", line).expect("test setup: append_jsonl could not write");
    }
}

// ─── Tests do próprio helper ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_count_passed_checks_correctly() {
        let mut t = TestCase::new("should_count_passed_checks_correctly");
        t.phase("Act");
        t.ok("true is true", true);
        t.ok("1 + 1 is 2", 1 + 1 == 2);
        t.phase("Assert");
        assert_eq!(t.passed, 2);
    }

    #[test]
    fn should_pass_eq_for_equal_values() {
        let mut t = TestCase::new("should_pass_eq_for_equal_values");
        t.eq("strings match", "hello".to_string(), "hello".to_string());
        t.eq("numbers match", 42u32, 42u32);
        assert_eq!(t.passed, 2);
    }

    #[test]
    fn should_pass_some_when_option_has_value() {
        let mut t = TestCase::new("should_pass_some_when_option_has_value");
        let val: Option<i32> = Some(1);
        t.some("option is some", &val);
        assert_eq!(t.passed, 1);
    }

    #[test]
    fn should_pass_none_when_option_is_empty() {
        let mut t = TestCase::new("should_pass_none_when_option_is_empty");
        let val: Option<i32> = None;
        t.none("option is none", &val);
        assert_eq!(t.passed, 1);
    }

    #[test]
    fn should_pass_is_ok_for_ok_result() {
        let mut t = TestCase::new("should_pass_is_ok_for_ok_result");
        let result: Result<i32, &str> = Ok(42);
        t.is_ok("result is ok", &result);
        assert_eq!(t.passed, 1);
    }

    #[test]
    fn should_pass_is_err_for_err_result() {
        let mut t = TestCase::new("should_pass_is_err_for_err_result");
        let result: Result<i32, &str> = Err("something failed");
        t.is_err("result is err", &result);
        assert_eq!(t.passed, 1);
    }

    #[test]
    fn should_pass_len_for_correct_collection_size() {
        let mut t = TestCase::new("should_pass_len_for_correct_collection_size");
        let items = vec![1, 2, 3];
        t.len("vec has 3 items", &items, 3);
        assert_eq!(t.passed, 1);
    }

    #[test]
    fn should_pass_empty_for_empty_vec() {
        let mut t = TestCase::new("should_pass_empty_for_empty_vec");
        let items: Vec<i32> = vec![];
        t.empty("vec is empty", &items);
        assert_eq!(t.passed, 1);
    }

    #[test]
    fn should_produce_valid_json_for_all_builders() {
        let lines: Vec<String> = vec![
            assistant_text("hello"),
            assistant_text(r#"he said "hi" and she said "bye""#),
            assistant_end_turn("done"),
            assistant_thinking("let me think about this..."),
            assistant_tool_use("Bash", r#"{"command":"ls -la"}"#),
            user_text("fix the bug"),
            tool_result("file1.rs\nfile2.rs"),
            progress_line("stdout chunk"),
            assistant_with_tokens("response", 10, 5, 2, 3),
            system_stop_hook(),
        ];
        for line in &lines {
            assert!(
                serde_json::from_str::<serde_json::Value>(line).is_ok(),
                "builder produced invalid JSON:\n{line}"
            );
        }
    }
}
