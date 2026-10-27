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

fn get_terminal_width(payload: &JsonValue) -> usize {
    if let Some(w) = payload.get("terminal_width").and_then(|t| t.as_f64()) {
        if w > 0.0 {
            return w as usize;
        }
    }
    if let Ok(cols_str) = env::var("COLUMNS") {
        if let Ok(c) = cols_str.trim().parse::<usize>() {
            if c > 0 {
                return c;
            }
        }
    }
    #[cfg(unix)]
    {
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
    }
    120
}

fn format_model(model_raw: &str) -> String {
    let raw = model_raw.trim();
    if raw.is_empty() {
        return "Gemini".to_string();
    }

    let mut effort = "";
    let lower = raw.to_lowercase();
    if lower.contains("(extra high)") || lower.contains("(xhigh)") {
        effort = " [XH]";
    } else if lower.contains("(max)") {
        effort = " [MAX]";
    } else if lower.contains("(high)") {
        effort = " [H]";
    } else if lower.contains("(medium)") {
        effort = " [M]";
    } else if lower.contains("(low)") {
        effort = " [L]";
    } else if lower.contains("(thinking)") {
        effort = " [T]";
    }

    let clean = if let Some(idx) = raw.find('(') {
        raw[..idx].trim()
    } else {
        raw
    };

    let mut clean_str = clean.to_string();
    for prefix in &["Gemini ", "Claude ", "Google "] {
        if clean_str.starts_with(prefix) {
            clean_str = clean_str[prefix.len()..].trim_start().to_string();
            break;
        }
    }

    format!("{}{}", clean_str, effort)
}

fn format_quota_colored(pct: Option<i64>) -> String {
    match pct {
        Some(p) => {
            let color = if p >= 40 {
                "\x1b[32m"
            } else if p >= 20 {
                "\x1b[33m"
            } else {
                "\x1b[31m"
            };
            format!("{}{}%", color, p)
        }
        None => "\x1b[90m--%".to_string(),
    }
}

fn extract_quota(payload: &JsonValue, model: &str) -> String {
    let lower = model.to_lowercase();
    let is_3p = lower.contains("claude") || lower.contains("gpt");
    let key_5h = if is_3p { "3p-5h" } else { "gemini-5h" };
    let key_wk = if is_3p { "3p-weekly" } else { "gemini-weekly" };
    let target_group = if is_3p { "Claude and GPT models" } else { "Gemini Models" };

    let mut h5_pct = None;
    let mut wk_pct = None;

    // 1. Direct from payload.quota
    if let Some(quota_obj) = payload.get("quota") {
        if let Some(b5) = quota_obj.get(key_5h) {
            if let Some(frac) = b5.get("remaining_fraction").and_then(|f| f.as_f64()) {
                h5_pct = Some((frac * 100.0).round() as i64);
            }
        }
        if let Some(bw) = quota_obj.get(key_wk) {
            if let Some(frac) = bw.get("remaining_fraction").and_then(|f| f.as_f64()) {
                wk_pct = Some((frac * 100.0).round() as i64);
            }
        }
    }

    // 2. Secondary fallback to /tmp/agy_quota_cache.json
    if h5_pct.is_none() || wk_pct.is_none() {
        let mut cache_path = env::var("TMPDIR")
            .map(|d| format!("{}/agy_quota_cache.json", d))
            .unwrap_or_else(|_| "/tmp/agy_quota_cache.json".to_string());
        if !std::path::Path::new(&cache_path).exists() {
            if let Ok(temp) = env::var("TEMP") {
                cache_path = format!("{}/agy_quota_cache.json", temp);
            }
        }
        if let Ok(content) = fs::read_to_string(&cache_path) {
            if let Some(json) = parse_json(&content) {
                if let Some(groups) = json.get("command").and_then(|c| c.get("data")).and_then(|d| d.get("groups")).and_then(|g| g.as_array()) {
                    for g in groups {
                        if g.get("name").and_then(|n| n.as_str()) == Some(target_group) {
                            if let Some(buckets) = g.get("buckets").and_then(|b| b.as_array()) {
                                for b in buckets {
                                    let win = b.get("window").and_then(|w| w.as_str()).unwrap_or("");
                                    let frac = b.get("remaining_fraction").and_then(|f| f.as_f64()).unwrap_or(0.0);
                                    if win == "5h" && h5_pct.is_none() {
                                        h5_pct = Some((frac * 100.0).round() as i64);
                                    } else if win == "weekly" && wk_pct.is_none() {
                                        wk_pct = Some((frac * 100.0).round() as i64);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let h5_str = format_quota_colored(h5_pct);
    let wk_str = format_quota_colored(wk_pct);
    format!(" \x1b[90m│\x1b[0m 5h: {}\x1b[0m \x1b[90m(Wk: {}\x1b[0m\x1b[90m)", h5_str, wk_str)
}

fn build_context_gauge(pct: f64, cols: usize) -> String {
    let pct_int = pct.round() as i64;
    if cols >= 85 {
        let filled = (pct_int / 20).clamp(0, 5) as usize;
        let mut blocks = String::new();
        for i in 0..5 {
            if i < filled {
                blocks.push('█');
            } else {
                blocks.push('░');
            }
        }
        format!(" \x1b[90m│\x1b[0m \x1b[36mCtx: [{}] {:.1}%\x1b[0m", blocks, pct)
    } else if cols >= 70 {
        format!(" \x1b[90m│\x1b[0m \x1b[36mCtx: {}%\x1b[0m", pct_int)
    } else {
        String::new()
    }
}

fn main() {
    let mut raw_stdin = String::new();
    let _ = io::stdin().read_to_string(&mut raw_stdin);
    let payload = parse_json(&raw_stdin).unwrap_or(JsonValue::Null);

    let cols = get_terminal_width(&payload);

    // 1. Identity & Reasoning
    let raw_model = if let Some(m_obj) = payload.get("model") {
        match m_obj {
            JsonValue::Object(_) => m_obj
                .get("display_name")
                .or_else(|| m_obj.get("id"))
                .and_then(|s| s.as_str())
                .unwrap_or("Gemini"),
            JsonValue::String(s) => s.as_str(),
            _ => "Gemini",
        }
    } else {
        "Gemini"
    };
    let model_disp = format_model(raw_model);

    // 2. Execution Mode Badges
    let raw_mode = payload
        .get("cycle_mode")
        .and_then(|m| m.as_str())
        .unwrap_or("default")
        .to_lowercase();

    let mode_badge = match raw_mode.as_str() {
        "auto" => "\x1b[1;30;42m AUTO \x1b[0m",
        "plan" => "\x1b[1;30;46m PLAN \x1b[0m",
        "accept-edits" => "\x1b[1;30;43m EDITS \x1b[0m",
        _ => "\x1b[1;37;44m DEFAULT \x1b[0m",
    };

    // 3. Compact sandbox status
    let sandbox = payload
        .get("sandbox")
        .and_then(|s| s.get("enabled"))
        .and_then(|e| e.as_bool())
        .unwrap_or(false);
    let sb_badge = if sandbox { " \x1b[32m\x1b[0m" } else { "" };

    // 4. Host / SSH Detection
    let has_ssh = env::var("SSH_TTY").is_ok()
        || env::var("SSH_CONNECTION").is_ok()
        || env::var("SSH_CLIENT").is_ok();
    let host_info = if has_ssh {
        "\x1b[38;2;166;173;200m\u{F08B9} SSH\x1b[0m "
    } else {
        ""
    };

    // 5. VCS status strictly from JSON payload
    let mut git_info = String::new();
    if let Some(vcs) = payload.get("vcs") {
        if let Some(branch) = vcs.get("branch").and_then(|b| b.as_str()) {
            if !branch.is_empty() {
                let dirty = if vcs.get("dirty").and_then(|d| d.as_bool()).unwrap_or(false) {
                    "\x1b[33m*\x1b[0m"
                } else {
                    ""
                };
                git_info = format!(" \x1b[90m│\x1b[0m \x1b[35m\u{F062C} {}{}", branch, dirty);
            }
        }
    }

    // 6. Quota
    let quota_info = extract_quota(&payload, raw_model);

    // 7. Context bar tiering
    let ctx_pct = if let Some(cw) = payload.get("context_window") {
        cw.get("used_percentage").and_then(|p| p.as_f64()).unwrap_or(0.0)
    } else if let Some(ctx) = payload.get("context") {
        let used = ctx.get("used_tokens").and_then(|u| u.as_f64()).unwrap_or(0.0);
        let max_t = ctx.get("max_tokens").and_then(|m| m.as_f64()).unwrap_or(1.0);
        (used / max_t) * 100.0
    } else {
        0.0
    };
    let ctx_info = build_context_gauge(ctx_pct, cols);

    let _ = writeln!(
        io::stdout(),
        "{}{}{} \x1b[90m│\x1b[0m \x1b[1m{}\x1b[0m{}{}{}",
        host_info, mode_badge, sb_badge, model_disp, git_info, quota_info, ctx_info
    );
    let _ = io::stdout().flush();
}
