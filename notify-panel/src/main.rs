use chrono::DateTime;
use gtk4::gdk::BUTTON_SECONDARY;
use gtk4::prelude::WidgetExt;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, CssProvider, Label, ListBox, Orientation,
    ScrolledWindow,
};
use gtk4::{GestureClick, prelude::*};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

struct Notification {
    ts: i64,
    app: String,
    summary: String,
    body: String,
}

fn log_path() -> PathBuf {
    let mut path = dirs_next::data_local_dir().unwrap_or_else(|| {
        PathBuf::from(format!("{}/.local/share", std::env::var("HOME").unwrap()))
    });
    path.push("notify-history/notifications.log");
    path
}

fn read_notifications(limit: usize) -> Vec<Notification> {
    let path = log_path();

    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut entries: Vec<Notification> = reader
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| line.ok().map(|l| (idx, l)))
        .filter_map(|(_, line)| {
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

fn delete_notification(timestamp: i64) -> std::io::Result<()> {
    let path = log_path();

    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    let reader = BufReader::new(file);
    let mut remaining = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let (ts_str, _) = match line.split_once('\t') {
            Some(x) => x,
            None => {
                remaining.push(line);
                continue;
            }
        };

        let ts = match ts_str.parse::<i64>() {
            Ok(v) => v,
            Err(_) => {
                remaining.push(line);
                continue;
            }
        };

        if ts != timestamp {
            remaining.push(line);
        }
    }

    let mut out = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)?;

    for line in remaining {
        writeln!(out, "{line}")?;
    }

    Ok(())
}

fn load_css() {
    let css_provider = CssProvider::new();
    css_provider.load_from_data(include_str!("../assets/style.css"));

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().unwrap(),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_USER,
    );
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notifications")
        .default_width(500)
        .default_height(800)
        .resizable(true)
        .build();
    window.add_css_class("transparent");

    let vbox = GtkBox::new(Orientation::Vertical, 8);
    vbox.add_css_class("background");

    let scroller = ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let list = ListBox::new();
    list.add_css_class("background");

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
        row.set_css_classes(&vec!["background", "list-item"]);

        unsafe { row.set_data("ts", n.ts) };

        let list_clone = list.clone();
        let row_weak = row.downgrade();
        let gesture = GestureClick::new();
        gesture.set_button(BUTTON_SECONDARY);
        gesture.connect_pressed(move |_, _, _, _| {
            if let Some(row) = row_weak.upgrade() {
                if let Some(ts) = unsafe { row.data::<i64>("ts") } {
                    if let Err(e) = delete_notification(unsafe { ts.read() }) {
                        eprintln!("Failed to delete notification: {e:?}");
                    }

                    list_clone.remove(&row);
                }
            }
        });

        row.add_controller(gesture);

        list.append(&row);
    }

    scroller.set_child(Some(&list));
    vbox.append(&scroller);

    load_css();

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
