use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};

#[derive(Debug, Clone)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(arr) => Some(arr),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> Option<JsonValue> {
        self.skip_whitespace();
        match self.chars.peek()? {
            '"' => self.parse_string().map(JsonValue::String),
            '{' => self.parse_object().map(JsonValue::Object),
            '[' => self.parse_array().map(JsonValue::Array),
            't' | 'f' => self.parse_bool().map(JsonValue::Bool),
            'n' => self.parse_null(),
            '-' | '0'..='9' => self.parse_number().map(JsonValue::Number),
            _ => None,
        }
    }

    fn parse_null(&mut self) -> Option<JsonValue> {
        let expected = ['n', 'u', 'l', 'l'];
        for &exp in &expected {
            if self.chars.next()? != exp {
                return None;
            }
        }
        Some(JsonValue::Null)
    }

    fn parse_bool(&mut self) -> Option<bool> {
        match self.chars.peek()? {
            't' => {
                let expected = ['t', 'r', 'u', 'e'];
                for &exp in &expected {
                    if self.chars.next()? != exp {
                        return None;
                    }
                }
                Some(true)
            }
            'f' => {
                let expected = ['f', 'a', 'l', 's', 'e'];
                for &exp in &expected {
                    if self.chars.next()? != exp {
                        return None;
                    }
                }
                Some(false)
            }
            _ => None,
        }
    }

    fn parse_number(&mut self) -> Option<f64> {
        let mut num_str = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
                num_str.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        num_str.parse::<f64>().ok()
    }

    fn parse_string(&mut self) -> Option<String> {
        if self.chars.next()? != '"' {
            return None;
        }
        let mut s = String::new();
        while let Some(c) = self.chars.next() {
            match c {
                '"' => return Some(s),
                '\\' => {
                    if let Some(esc) = self.chars.next() {
                        match esc {
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            '/' => s.push('/'),
                            'b' => s.push('\x08'),
                            'f' => s.push('\x0C'),
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            'u' => {
                                let mut hex = String::new();
                                for _ in 0..4 {
                                    if let Some(h) = self.chars.next() {
                                        hex.push(h);
                                    }
                                }
                                if let Ok(u) = u32::from_str_radix(&hex, 16) {
                                    if let Some(ch) = char::from_u32(u) {
                                        s.push(ch);
                                    }
                                }
                            }
                            other => s.push(other),
                        }
                    }
                }
                other => s.push(other),
            }
        }
        None
    }

    fn parse_array(&mut self) -> Option<Vec<JsonValue>> {
        if self.chars.next()? != '[' {
            return None;
        }
        let mut arr = Vec::new();
        self.skip_whitespace();
        if let Some(&']') = self.chars.peek() {
            self.chars.next();
            return Some(arr);
        }

        loop {
            let val = self.parse_value()?;
            arr.push(val);
            self.skip_whitespace();
            match self.chars.next()? {
                ',' => continue,
                ']' => break,
                _ => return None,
            }
        }
        Some(arr)
    }

    fn parse_object(&mut self) -> Option<HashMap<String, JsonValue>> {
        if self.chars.next()? != '{' {
            return None;
        }
        let mut map = HashMap::new();
        self.skip_whitespace();
        if let Some(&'}') = self.chars.peek() {
            self.chars.next();
            return Some(map);
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.chars.next()? != ':' {
                return None;
            }
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_whitespace();
            match self.chars.next()? {
                ',' => continue,
                '}' => break,
                _ => return None,
            }
        }
        Some(map)
    }
}

fn parse_json(input: &str) -> Option<JsonValue> {
    JsonParser::new(input).parse_value()
}

#[cfg(unix)]
fn get_terminal_width() -> usize {
    if let Ok(cols_str) = env::var("COLUMNS") {
        if let Ok(c) = cols_str.trim().parse::<usize>() {
            if c > 0 {
                return c;
            }
        }
    }
    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }
    extern "C" {
        fn ioctl(fd: i32, request: u64, ...) -> i32;
    }
    const TIOCGWINSZ: u64 = 0x5413;
    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    for fd in [1, 0, 2] {
        unsafe {
            if ioctl(fd, TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
                return ws.ws_col as usize;
            }
        }
    }
    120
}

#[cfg(not(unix))]
fn get_terminal_width() -> usize {
    if let Ok(cols_str) = env::var("COLUMNS") {
        if let Ok(c) = cols_str.trim().parse::<usize>() {
            if c > 0 {
                return c;
            }
        }
    }
    120
}

fn format_model(model_raw: &str) -> String {
    let mut clean = model_raw.trim().to_string();
    if clean.is_empty() {
        return "Default".to_string();
    }
    for prefix in &["Gemini ", "Claude ", "Google "] {
        if clean.starts_with(prefix) {
            clean = clean[prefix.len()..].trim_start().to_string();
            break;
        }
    }
    let replacements = [
        ("(Extra High)", "[XH]"),
        ("(XHigh)", "[XH]"),
        ("(Max)", "[MAX]"),
        ("(High)", "[H]"),
        ("(Medium)", "[M]"),
        ("(Low)", "[L]"),
        ("(Thinking)", "[T]"),
    ];
    for (from, to) in &replacements {
        clean = clean.replace(from, to);
    }
    clean.trim().to_string()
}

fn build_context_gauge(used_tokens: f64, max_tokens: f64) -> String {
    if max_tokens <= 0.0 {
        return "[░░░░░] 0.0%".to_string();
    }
    let pct = (used_tokens / max_tokens) * 100.0;
    let filled = (pct / 20.0).floor() as usize;
    let filled = filled.min(5);
    let mut blocks = String::new();
    for i in 0..5 {
        if i < filled {
            blocks.push('█');
        } else {
            blocks.push('░');
        }
    }
    format!("[{}] {:.1}%", blocks, pct)
}

fn read_quota_cache() -> Option<String> {
    let mut cache_path = env::var("TMPDIR")
        .map(|d| format!("{}/agy_quota_cache.json", d))
        .unwrap_or_else(|_| "/tmp/agy_quota_cache.json".to_string());

    if !std::path::Path::new(&cache_path).exists() {
        if let Ok(temp) = env::var("TEMP") {
            cache_path = format!("{}/agy_quota_cache.json", temp);
        }
    }

    let content = fs::read_to_string(&cache_path).ok()?;
    let json = parse_json(&content)?;

    // 1. Flat format
    if let (Some(h5), Some(wk)) = (json.get("5h_pct"), json.get("week_pct")) {
        let h5_str = match h5 {
            JsonValue::Number(n) => format!("{:.0}", n),
            JsonValue::String(s) => s.clone(),
            _ => "--".to_string(),
        };
        let wk_str = match wk {
            JsonValue::Number(n) => format!("{:.0}", n),
            JsonValue::String(s) => s.clone(),
            _ => "--".to_string(),
        };
        return Some(format!("Q:{}%/{}%", h5_str, wk_str));
    }

    // 2. Command groups format
    if let Some(cmd) = json.get("command") {
        if let Some(data) = cmd.get("data") {
            if let Some(groups) = data.get("groups").and_then(|g| g.as_array()) {
                for g in groups {
                    if let Some(buckets) = g.get("buckets").and_then(|b| b.as_array()) {
                        let mut h5_val = None;
                        let mut wk_val = None;
                        for b in buckets {
                            let window = b.get("window").and_then(|w| w.as_str()).unwrap_or("");
                            let frac = b.get("remaining_fraction").and_then(|f| f.as_f64()).unwrap_or(0.0);
                            if window == "5h" {
                                h5_val = Some((frac * 100.0).round() as i64);
                            } else if window == "weekly" {
                                wk_val = Some((frac * 100.0).round() as i64);
                            }
                        }
                        if h5_val.is_some() || wk_val.is_some() {
                            let h5_str = h5_val.map(|v| format!("{}%", v)).unwrap_or_else(|| "--%".to_string());
                            let wk_str = wk_val.map(|v| format!("{}%", v)).unwrap_or_else(|| "--%".to_string());
                            return Some(format!("Q:{}/{}", h5_str, wk_str));
                        }
                    }
                }
            }
        }
    }

    None
}

fn main() {
    let cols = get_terminal_width();

    let mut raw_stdin = String::new();
    let _ = io::stdin().read_to_string(&mut raw_stdin);
    let payload = parse_json(&raw_stdin).unwrap_or(JsonValue::Null);

    // 1. Identity & Reasoning
    let raw_model = payload
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("Default");
    let model_str = format_model(raw_model);

    // 2. Execution Mode Badges
    let raw_mode = payload
        .get("cycle_mode")
        .and_then(|m| m.as_str())
        .unwrap_or("default")
        .to_lowercase();

    let mode_badge = match raw_mode.as_str() {
        "auto" => "\x1b[42;30m AUTO \x1b[0m",
        "plan" => "\x1b[46;30m PLAN \x1b[0m",
        "accept-edits" => "\x1b[43;30m EDITS \x1b[0m",
        _ => "\x1b[44;30m DEFAULT \x1b[0m",
    };

    // 3. Host / SSH Detection
    let has_ssh = env::var("SSH_CLIENT").is_ok()
        || env::var("SSH_TTY").is_ok()
        || env::var("SSH_CONNECTION").is_ok();
    let ssh_badge = if has_ssh {
        "\x1b[38;5;244m󰢹 SSH\x1b[0m "
    } else {
        ""
    };

    // 4. Git Metadata
    let mut git_segment = String::new();
    if let Some(vcs) = payload.get("vcs") {
        if let Some(branch) = vcs.get("branch").and_then(|b| b.as_str()) {
            if !branch.is_empty() {
                let dirty = if vcs.get("dirty").and_then(|d| d.as_bool()).unwrap_or(false) {
                    "*"
                } else {
                    ""
                };
                git_segment = format!("\x1b[34m󰘬 {}{}\x1b[0m", branch, dirty);
            }
        }
    }

    // 5. Context Meter
    let ctx_segment = if let Some(ctx) = payload.get("context") {
        let used = ctx.get("used_tokens").and_then(|u| u.as_f64()).unwrap_or(0.0);
        let max = ctx.get("max_tokens").and_then(|m| m.as_f64()).unwrap_or(1.0);
        let gauge = build_context_gauge(used, max);
        format!("\x1b[35m{}\x1b[0m", gauge)
    } else {
        let gauge = build_context_gauge(0.0, 1.0);
        format!("\x1b[35m{}\x1b[0m", gauge)
    };

    // Assemble segments
    let mut segments = Vec::new();
    segments.push(format!(
        "{}\x1b[32m\x1b[0m {} \x1b[1m{}\x1b[0m",
        ssh_badge, mode_badge, model_str
    ));

    if !git_segment.is_empty() {
        segments.push(git_segment);
    }

    if cols >= 85 {
        if let Some(quota_str) = read_quota_cache() {
            segments.push(format!("\x1b[36m{}\x1b[0m", quota_str));
        }
        segments.push(ctx_segment);
    }

    let divider = " \x1b[38;5;239m│\x1b[0m ";
    let output = segments.join(divider);
    let _ = writeln!(io::stdout(), " {} ", output);
    let _ = io::stdout().flush();
}
