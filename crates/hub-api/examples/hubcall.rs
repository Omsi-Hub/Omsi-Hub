//! Runs hub-api commands from a script, in one process (the company session and the line
//! editor keep their state between the commands), for testing the app headless:
//!
//!   cargo run -p hub-api --example hubcall -- script.jsonl
//!
//! Each line of the script is a JSON object:
//!   {"cmd": "co_found", "args": {...}}           the command, its answer printed
//!   "show": ["/open", "/maps"]                    only these parts of the answer (JSON pointers)
//!   "until": "/ready", "secs": 60                 repeat until that part is true / non-empty
//!   "save": {"id": "/open"}                       keep a part of the answer as $id for later lines
//! A `"$name"` string anywhere in `args` is replaced by what was saved under that name.
//! Lines starting with `#` are skipped.

use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Number(_)) => true,
    }
}

fn fill(v: &Value, saved: &HashMap<String, Value>) -> Value {
    match v {
        Value::String(s) if s.starts_with('$') => saved.get(&s[1..]).cloned().unwrap_or_else(|| v.clone()),
        Value::Array(a) => Value::Array(a.iter().map(|x| fill(x, saved)).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, x)| (k.clone(), fill(x, saved))).collect()),
        _ => v.clone(),
    }
}

fn short(v: &Value, max: usize) -> String {
    let t = v.to_string();
    if t.len() > max {
        let mut cut = max;
        while !t.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}… ({} bytes)", &t[..cut], t.len())
    } else {
        t
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("a script file");
    let text = std::fs::read_to_string(&path).expect("the script");
    let mut saved: HashMap<String, Value> = HashMap::new();
    for (n, raw) in text.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let step: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                println!("[{}] bad line: {e}", n + 1);
                std::process::exit(2);
            }
        };
        let cmd = step["cmd"].as_str().unwrap_or("").to_string();
        let args = fill(&step.get("args").cloned().unwrap_or(Value::Null), &saved);
        let until = step.get("until").and_then(Value::as_str).map(str::to_string);
        let secs = step.get("secs").and_then(Value::as_u64).unwrap_or(120);
        // ("sleep": seconds waited before the command)
        if let Some(t) = step.get("sleep").and_then(Value::as_f64) {
            std::thread::sleep(Duration::from_secs_f64(t));
        }
        let started = Instant::now();
        // ("until_gone": repeat until that part is false / empty / missing)
        let gone = step.get("until_gone").and_then(Value::as_str).map(str::to_string);
        let answer = loop {
            let a = hub_api::call(&cmd, &args, &hub_api::Quiet);
            let waiting = match &a {
                Ok(v) => until.as_ref().is_some_and(|u| !truthy(v.pointer(u))) || gone.as_ref().is_some_and(|g| truthy(v.pointer(g))),
                Err(_) => false,
            };
            if waiting && started.elapsed() < Duration::from_secs(secs) {
                std::thread::sleep(Duration::from_millis(500));
            } else {
                break a;
            }
        };
        let took = started.elapsed().as_secs_f32();
        match answer {
            Err(e) => {
                println!("[{}] {cmd} ERROR ({took:.1}s): {e}", n + 1);
                if step.get("may_fail").and_then(Value::as_bool) != Some(true) {
                    std::process::exit(1);
                }
            }
            Ok(v) => {
                if let Some(u) = &until {
                    if !truthy(v.pointer(u)) {
                        println!("[{}] {cmd} TIMEOUT waiting for {u} ({took:.1}s): {}", n + 1, short(&v, 1500));
                        std::process::exit(1);
                    }
                }
                if let Some(s) = step.get("save").and_then(Value::as_object) {
                    for (name, ptr) in s {
                        let got = ptr.as_str().and_then(|p| v.pointer(p)).cloned().unwrap_or(Value::Null);
                        saved.insert(name.clone(), got);
                    }
                }
                // ("into": {"proj": {"/layers": "/layers"}} - a part of the answer put into a
                // part of what was saved)
                if let Some(s) = step.get("into").and_then(Value::as_object) {
                    for (name, parts) in s {
                        for (at, from) in parts.as_object().into_iter().flatten() {
                            let got = from.as_str().and_then(|p| v.pointer(p)).cloned().unwrap_or(Value::Null);
                            if let Some(slot) = saved.get_mut(name).and_then(|x| x.pointer_mut(at)) {
                                *slot = got;
                            } else {
                                println!("[{}] into: no {name}{at}", n + 1);
                            }
                        }
                    }
                }
                // ("dump": "file.json" - the whole answer written out)
                if let Some(file) = step.get("dump").and_then(Value::as_str) {
                    let _ = std::fs::write(file, serde_json::to_string_pretty(&v).unwrap_or_default());
                }
                let max = step.get("max").and_then(Value::as_u64).unwrap_or(1500) as usize;
                match step.get("show").and_then(Value::as_array) {
                    Some(parts) => {
                        let shown: Vec<String> = parts.iter().filter_map(Value::as_str).map(|p| format!("{p}={}", short(v.pointer(p).unwrap_or(&Value::Null), max))).collect();
                        println!("[{}] {cmd} ({took:.1}s): {}", n + 1, shown.join("  "));
                    }
                    None => println!("[{}] {cmd} ({took:.1}s): {}", n + 1, short(&v, max)),
                }
            }
        }
    }
}
