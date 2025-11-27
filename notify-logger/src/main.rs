use anyhow::{Context, Result, bail};
use chrono::Local;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use zbus::message::Body;

use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::OwnedValue;

struct NotifyMessage {
    app_name: String,
    app_icon: String,
    summary: String,
    body: String,
    hints: HashMap<String, OwnedValue>,
}

impl NotifyMessage {
    fn from_body(body: &Body) -> Result<Self> {
        let (
            app_name,
            _replaces_id,
            app_icon,
            summary,
            body_text,
            _actions,
            hints,
            _expire_timeout,
        ): (
            String,
            u32,
            String,
            String,
            String,
            Vec<String>,
            HashMap<String, OwnedValue>,
            i32,
        ) = body
            .deserialize()
            .context("Failed to deserialize Notify body")?;

        Ok(Self {
            app_name,
            app_icon,
            summary,
            body: body_text,
            hints,
        })
    }
}

fn main() -> Result<()> {
    let data_dir = get_data_dir()?;
    fs::create_dir_all(&data_dir)
        .with_context(|| format!("Failed to create data dir {}", data_dir.display()))?;

    let log_path = data_dir.join("notifications.log");
    let mut log_file = open_log_file(&log_path)?;

    let conn = Connection::session().context("Failed to connect to session D-Bus")?;

    let filter = "type='method_call',interface='org.freedesktop.Notifications',member='Notify'";

    conn.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus.Monitoring"),
        "BecomeMonitor",
        &(&[filter][..], 0u32),
    )
    .context("BecomeMonitor failed (maybe access denied?)")?;

    let mut iter = MessageIterator::from(&conn);
    loop {
        let msg = match iter.next() {
            Some(Ok(m)) => m,
            Some(Err(e)) => {
                eprintln!("Error receiving D-Bus message: {e}");
                continue;
            }
            None => {
                eprintln!("D-Bus message iterator ended");
                break;
            }
        };

        let message = match NotifyMessage::from_body(&msg.body()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to parse message body: {e}");
                continue;
            }
        };

        if let Err(e) = write_notification(message, &mut log_file) {
            eprintln!("Failed to log notification: {e}");
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

fn write_notification(message: NotifyMessage, log_file: &mut File) -> Result<()> {
    if message.app_name.is_empty() || message.summary.is_empty() {
        bail!("Appname and summary are required.");
    }

    let ts = Local::now().timestamp();

    writeln!(
        log_file,
        "{}`{}`{}`{}`{}",
        ts, message.app_name, message.app_icon, message.summary, message.body
    )
    .context("Failed to write to log file")?;
    log_file.flush().context("Failed to flush log file")?;

    let human = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] New notification from {}", human, message.app_name);

    Ok(())
}
