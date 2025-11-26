use chrono::DateTime;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Label, ListBox, Orientation, ScrolledWindow,
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

struct Notification {
    ts: i64,
    app: String,
    summary: String,
    body: String,
}

fn read_notifications(limit: usize) -> Vec<Notification> {
    let mut path = dirs_next::data_local_dir().unwrap_or_else(|| {
        PathBuf::from(format!("{}/.local/share", std::env::var("HOME").unwrap()))
    });
    path.push("notify-history/notifications.log");

    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut entries: Vec<Notification> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 4 {
                return None;
            }
            let ts = parts[0].parse().ok()?;
            Some(Notification {
                ts,
                app: parts[1].to_string(),
                summary: parts[2].to_string(),
                body: parts[3].to_string(),
            })
        })
        .collect();

    entries.sort_by_key(|n| n.ts);
    entries.reverse();
    entries.truncate(limit);
    entries
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notifications")
        .default_width(500)
        .default_height(800)
        .resizable(true)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 8);

    let scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let list = ListBox::new();

    for n in read_notifications(200) {
        let row_box = GtkBox::new(Orientation::Vertical, 2);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);
        row_box.set_margin_start(8);
        row_box.set_margin_end(8);

        let time_str = DateTime::from_timestamp(n.ts, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| n.ts.to_string());

        let title = Label::new(Some(&format!("{}  •  {}", time_str, n.app)));
        title.set_xalign(0.0);
        title.add_css_class("heading");

        let summary = Label::new(Some(&n.summary));
        summary.set_xalign(0.0);
        summary.set_wrap(true);

        let body = if !n.body.trim().is_empty() {
            Some(Label::new(Some(&n.body)))
        } else {
            None
        };

        row_box.append(&title);
        row_box.append(&summary);
        if let Some(b) = body {
            b.set_xalign(0.0);
            b.set_wrap(true);
            row_box.append(&b);
        }

        let row = gtk4::ListBoxRow::new();
        row.set_child(Some(&row_box));
        list.append(&row);
    }

    scroller.set_child(Some(&list));
    vbox.append(&scroller);

    window.set_child(Some(&vbox));
    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("notify.panel")
        .build();

    app.connect_activate(build_ui);

    app.run();
}
