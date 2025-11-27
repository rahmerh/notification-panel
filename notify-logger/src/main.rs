use anyhow::{Context, Result, bail};
use chrono::Local;
use regex::Regex;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct State {
    capturing: bool,
    string_count: u8,
    app_name: Option<String>,
    summary: Option<String>,
    body: Option<String>,
    icon: Option<String>,
}

impl State {
    fn new() -> Self {
        Self {
            capturing: false,
            string_count: 0,
            app_name: None,
            summary: None,
            body: None,
            icon: None,
        }
    }

    fn reset(&mut self) {
        self.capturing = false;
        self.string_count = 0;
        self.app_name = None;
        self.summary = None;
        self.body = None;
        self.icon = None;
    }
}

fn main() -> Result<()> {
    let data_dir = get_data_dir()?;
    fs::create_dir_all(&data_dir)
        .with_context(|| format!("Failed to create data dir {}", data_dir.display()))?;

    let log_path = data_dir.join("notifications.log");
    let mut log_file = open_log_file(&log_path)?;

    let mut child = Command::new("dbus-monitor")
        .arg("interface='org.freedesktop.Notifications',member='Notify'")
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to start dbus-monitor")?;

    let stdout = child
        .stdout
        .take()
        .context("Failed to capture dbus-monitor stdout")?;
    let reader = BufReader::new(stdout);

    let string_re = Regex::new(r#"string\s+"(.*)""#).unwrap();

    let mut state = State::new();

    for line in reader.lines() {
        let line = line.context("Failed to read line from dbus-monitor")?;

        if line.contains("member=Notify") {
            state.capturing = true;
            state.string_count = 0;
            state.app_name = None;
            state.summary = None;
            state.body = None;
            state.icon = None;
            continue;
        }

        if state.capturing {
            if let Some(caps) = string_re.captures(&line) {
                if let Some(m) = caps.get(1) {
                    let s = m.as_str().to_string();

                    if state.string_count < 4 {
                        state.string_count += 1;
                        match state.string_count {
                            1 => state.app_name = Some(s),
                            2 => state.icon = Some(s),
                            3 => state.summary = Some(s),
                            4 => {
                                state.body = Some(s);
                                flush_entry(&mut log_file, &mut state)?;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_data_dir() -> Result<PathBuf> {
    let base = dirs_next::data_dir().context("$XDG_DATA_HOME or platform data dir not found")?;
    Ok(base.join("notify-history"))
}

fn open_log_file(path: &PathBuf) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open log file {}", path.display()))?;

    Ok(file)
}

fn flush_entry(log_file: &mut File, state: &mut State) -> Result<()> {
    let app_name = match &state.app_name {
        Some(a) => a,
        None => {
            state.reset();
            bail!("Missing app_name while flushing notification");
        }
    };

    let summary = match &state.summary {
        Some(s) => s,
        None => {
            state.reset();
            bail!("Missing summary while flushing notification");
        }
    };

    let body = state.body.as_deref().unwrap_or("");
    let icon = state.icon.as_deref().unwrap_or("");

    let ts = chrono::Local::now().timestamp();

    writeln!(
        log_file,
        "{}`{}`{}`{}`{}",
        ts, app_name, icon, summary, body
    )
    .context("Failed to write to log file")?;
    log_file.flush().context("Failed to flush log file")?;

    let human = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] New notification", human);

    state.reset();
    Ok(())
}
