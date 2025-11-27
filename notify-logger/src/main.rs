use anyhow::{Context, Result, bail};
use chrono::Local;
use image::{ImageBuffer, RgbaImage};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use zbus::blocking::{Connection, MessageIterator};
use zbus::message::Body;
use zbus::zvariant::{Array, OwnedValue, Structure};

struct NotifyMessage {
    timestamp: i64,
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

        let ts = Local::now().timestamp();

        Ok(Self {
            timestamp: ts,
            app_name,
            app_icon,
            summary,
            body: body_text,
            hints,
        })
    }
}

fn write_image_data(message: &NotifyMessage, image_dir: &Path) -> Result<Option<PathBuf>> {
    let image_data = match message
        .hints
        .get("image-data")
        .or_else(|| message.hints.get("image_data"))
    {
        Some(v) => v,
        None => return Ok(None),
    };

    let s: Structure = image_data
        .downcast_ref::<Structure>()
        .context("image-data is not a struct")?;

    let fields = s.fields();
    if fields.len() != 7 {
        bail!("image-data struct has {} fields, expected 7", fields.len());
    }

    let width = fields[0].clone().downcast_ref::<i32>()?;
    let height = fields[1].clone().downcast_ref::<i32>()?;
    let rowstride = fields[2].clone().downcast_ref::<i32>()?;
    let has_alpha = fields[3].clone().downcast_ref::<bool>()?;
    let bits_per_sample = fields[4].clone().downcast_ref::<i32>()?;
    let channels = fields[5].clone().downcast_ref::<i32>()?;

    if channels != 4 || bits_per_sample != 8 || !has_alpha {
        return Ok(None);
    }

    let array: Array = Array::try_from(&fields[6]).context("image-data field is not an array")?;
    let mut data = Vec::with_capacity(array.len());
    for v in array.iter() {
        let b = u8::try_from(v).context("image-data array contained non-byte value")?;
        data.push(b);
    }

    let width_us = width as usize;
    let height_us = height as usize;
    let rowstride_us = rowstride as usize;

    if data.len() < rowstride_us * height_us {
        bail!("image-data buffer shorter than rowstride * height");
    }

    let mut rgba = Vec::with_capacity(width_us * height_us * channels as usize);

    for y in 0..height_us {
        let start = y * rowstride_us;
        let end = start + width_us * channels as usize;
        let row = &data[start..end];

        rgba.extend_from_slice(row);
    }

    let img: RgbaImage = ImageBuffer::from_vec(width as u32, height as u32, rgba)
        .ok_or_else(|| anyhow::anyhow!("Failed to build RgbaImage"))?;

    if !image_dir.exists() {
        fs::create_dir_all(&image_dir)?;
    }

    let out_path = image_dir.join(format!("{}.png", message.timestamp));
    img.save(&out_path)
        .with_context(|| format!("Failed to save {}", out_path.display()))?;

    Ok(Some(out_path))
}

fn main() -> Result<()> {
    let log_dir = PathBuf::from("/tmp/notification-history");
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create data dir {}", log_dir.display()))?;

    let log_path = log_dir.join("notifications.log");
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

        let mut message = match NotifyMessage::from_body(&msg.body()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Failed to parse message body: {e}");
                continue;
            }
        };

        let image_path = write_image_data(&message, &log_path.parent().unwrap().join("images"))?;
        if let Some(path) = image_path {
            message.app_icon = path.to_string_lossy().to_string();
        }

        if let Err(e) = write_notification(&message, &mut log_file) {
            eprintln!("Failed to log notification: {e}");
        }
    }

    Ok(())
}

fn open_log_file(path: &PathBuf) -> Result<File> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("Failed to open log file {}", path.display()))?;

    Ok(file)
}

fn write_notification(message: &NotifyMessage, log_file: &mut File) -> Result<()> {
    if message.app_name.is_empty() || message.summary.is_empty() {
        bail!("Appname and summary are required.");
    }

    let summary_sanitized = message.summary.replace('\n', "").replace('\r', "");
    let body_sanitized = message.body.replace('\n', "").replace('\r', "");

    writeln!(
        log_file,
        "{}`{}`{}`{}`{}",
        message.timestamp, message.app_name, message.app_icon, summary_sanitized, body_sanitized
    )
    .context("Failed to write to log file")?;
    log_file.flush().context("Failed to flush log file")?;

    let human = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[{}] New notification from {}", human, message.app_name);

    Ok(())
}
