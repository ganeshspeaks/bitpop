use chrono::Local;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, Entry, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, glib,
};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

use crate::apps::{AppEntry, launch_app, load_desktop_apps};
use crate::icons::load_app_icon;
use crate::system::{
    airplane_mode, toggle_bluetooth, toggle_wifi, update_battery, update_bluetooth_status,
    update_wifi_status,
};

const LOCK_FILE: &str = "/tmp/bitpop.lock";

pub fn check_existing_instance() -> Option<u32> {
    if PathBuf::from(LOCK_FILE).exists() {
        if let Ok(mut file) = File::open(LOCK_FILE) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if PathBuf::from(format!("/proc/{}", pid)).exists() {
                        return Some(pid);
                    }
                }
            }
        }
    }
    None
}

pub fn write_lock_file() {
    if let Ok(mut file) = File::create(LOCK_FILE) {
        let pid = std::process::id();
        let _ = writeln!(file, "{}", pid);
    }
}

pub fn remove_lock_file() {
    let _ = fs::remove_file(LOCK_FILE);
}

pub fn setup_cleanup() {
    unsafe {
        libc::signal(libc::SIGTERM, handle_sigterm as libc::sighandler_t);
    }
}

extern "C" fn handle_sigterm(_: libc::c_int) {
    remove_lock_file();
    std::process::exit(0);
}

pub fn build_ui(app: &Application) {
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(include_str!("style.css"));

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not connect to display"),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let window = ApplicationWindow::builder()
        .application(app)
        .title("BitPop")
        .default_width(480)
        .default_height(700)
        .resizable(false)
        .decorated(false)
        .build();

    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.set_margin_top(24);
    main_box.set_margin_bottom(24);
    main_box.set_margin_start(24);
    main_box.set_margin_end(24);
    main_box.set_spacing(16);
    main_box.add_css_class("main-container");

    build_clock_section(&main_box);
    build_battery_section(&main_box);
    let (search_entry, _app_list) = build_app_search_section(&main_box, &window);
    let wifi_btn = build_quick_controls_section(&main_box);
    build_power_section(&main_box, &window);
    build_hint_section(&main_box);

    window.set_child(Some(&main_box));

    search_entry.grab_focus();

    let search_entry_tab_controller = gtk4::EventControllerKey::new();
    let wifi_btn_weak = wifi_btn.downgrade();
    search_entry_tab_controller.connect_key_pressed(move |_, key, _, _modifiers| {
        if key == gtk4::gdk::Key::Tab {
            if let Some(wifi_btn) = wifi_btn_weak.upgrade() {
                wifi_btn.grab_focus();
                return glib::Propagation::Stop;
            }
        }
        glib::Propagation::Proceed
    });
    search_entry.add_controller(search_entry_tab_controller);

    setup_key_handlers(&window, &search_entry);

    window.present();

    if let Some(surface) = window.surface() {
        if let Some(toplevel) = surface.downcast_ref::<gtk4::gdk::Toplevel>() {
            toplevel.set_decorated(false);
        }
    }
}

fn build_clock_section(main_box: &GtkBox) {
    let clock_label = Label::new(None);
    clock_label.add_css_class("clock");
    clock_label.set_halign(gtk4::Align::Center);

    let date_label = Label::new(None);
    date_label.add_css_class("date");
    date_label.set_halign(gtk4::Align::Center);

    update_clock(&clock_label, &date_label);

    let clock_label_weak = clock_label.downgrade();
    let date_label_weak = date_label.downgrade();
    glib::timeout_add_seconds_local(60, move || {
        let Some(clock_label) = clock_label_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let Some(date_label) = date_label_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        update_clock(&clock_label, &date_label);
        glib::ControlFlow::Continue
    });

    main_box.append(&clock_label);
    main_box.append(&date_label);
}

fn update_clock(clock_label: &Label, date_label: &Label) {
    let now = Local::now();
    clock_label.set_text(&now.format("%I:%M %p").to_string());
    date_label.set_text(&now.format("%A, %B %d, %Y").to_string());
}

fn build_battery_section(main_box: &GtkBox) {
    let battery_card = GtkBox::new(Orientation::Vertical, 0);
    battery_card.add_css_class("card");
    battery_card.set_halign(gtk4::Align::Fill);

    let battery_label = Label::new(None);
    battery_label.add_css_class("battery-text");
    battery_label.set_halign(gtk4::Align::Center);
    battery_label.set_margin_top(12);
    battery_label.set_margin_bottom(12);

    update_battery(&battery_label);

    let battery_label_weak = battery_label.downgrade();
    glib::timeout_add_seconds_local(30, move || {
        let Some(battery_label) = battery_label_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        update_battery(&battery_label);
        glib::ControlFlow::Continue
    });

    battery_card.append(&battery_label);
    main_box.append(&battery_card);
}

fn build_app_search_section(main_box: &GtkBox, window: &ApplicationWindow) -> (Entry, ListBox) {
    let search_label = Label::new(Some("LAUNCH APP"));
    search_label.add_css_class("section-label");
    search_label.set_halign(gtk4::Align::Start);
    main_box.append(&search_label);

    let search_entry = Entry::new();
    search_entry.add_css_class("search-entry");
    search_entry.set_placeholder_text(Some("Type to search..."));
    search_entry.set_halign(gtk4::Align::Fill);
    main_box.append(&search_entry);

    let app_list = ListBox::new();
    app_list.add_css_class("app-list");
    app_list.set_selection_mode(gtk4::SelectionMode::Single);
    app_list.set_show_separators(false);

    let scrolled_window = ScrolledWindow::new();
    scrolled_window.add_css_class("app-scrolled-window");
    scrolled_window.set_child(Some(&app_list));
    scrolled_window.set_vexpand(true);
    scrolled_window.set_max_content_height(150);
    scrolled_window.set_propagate_natural_height(true);
    scrolled_window.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    main_box.append(&scrolled_window);

    let all_apps = Rc::new(RefCell::new(load_desktop_apps()));
    update_app_list(&app_list, &all_apps.borrow(), "", window);

    let app_list_weak = app_list.downgrade();
    let all_apps_clone = all_apps.clone();
    let window_weak = window.downgrade();
    search_entry.connect_changed(move |entry| {
        let query = entry.text().to_string().to_lowercase();
        if let Some(app_list) = app_list_weak.upgrade() {
            if let Some(window) = window_weak.upgrade() {
                update_app_list(&app_list, &all_apps_clone.borrow(), &query, &window);
            }
        }
    });

    let all_apps_for_enter = all_apps.clone();
    let window_for_enter = window.downgrade();
    search_entry.connect_activate(move |entry| {
        let query = entry.text().to_string().to_lowercase();
        let apps = all_apps_for_enter.borrow();
        let filtered: Vec<&AppEntry> = apps
            .iter()
            .filter(|app| app.name.to_lowercase().contains(&query))
            .take(1)
            .collect();

        if let Some(app) = filtered.first() {
            let app_clone = (*app).clone();
            drop(apps);
            launch_app(&app_clone);
            if let Some(window) = window_for_enter.upgrade() {
                window.close();
            }
        }
    });

    (search_entry, app_list)
}

fn update_app_list(list_box: &ListBox, apps: &[AppEntry], query: &str, window: &ApplicationWindow) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let filtered: Vec<&AppEntry> = apps
        .iter()
        .filter(|app| app.name.to_lowercase().contains(query))
        .take(10)
        .collect();

    for app in filtered {
        let row = create_app_row(app);
        let app_clone = app.clone();
        let window_weak = window.downgrade();

        row.connect_activate(move |_| {
            launch_app(&app_clone);
            if let Some(window) = window_weak.upgrade() {
                window.close();
            }
        });

        list_box.append(&row);
    }
}

fn create_app_row(app: &AppEntry) -> ListBoxRow {
    let row = ListBoxRow::new();
    row.add_css_class("app-row");
    row.set_selectable(true);
    row.set_activatable(true);

    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_top(8);
    hbox.set_margin_bottom(8);
    hbox.set_margin_start(12);
    hbox.set_margin_end(12);

    // App icon
    let icon = load_app_icon(&app.icon, 24);
    hbox.append(&icon);

    // App name
    let name_label = Label::new(Some(&app.name));
    name_label.add_css_class("app-name");
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_hexpand(true);
    hbox.append(&name_label);

    row.set_child(Some(&hbox));
    row
}

fn build_quick_controls_section(main_box: &GtkBox) -> Button {
    let controls_label = Label::new(Some("QUICK CONTROLS"));
    controls_label.add_css_class("section-label");
    controls_label.set_halign(gtk4::Align::Start);
    main_box.append(&controls_label);

    // WiFi button
    let (wifi_btn, wifi_status) = create_control_button("network-wireless", "WiFi", "Checking...");
    let wifi_status_clone = wifi_status.clone();
    wifi_btn.connect_clicked(move |_| {
        toggle_wifi();
        let wifi_status_weak = wifi_status_clone.downgrade();
        glib::timeout_add_seconds_local(1, move || {
            let Some(wifi_status) = wifi_status_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            update_wifi_status(&wifi_status);
            glib::ControlFlow::Break
        });
    });
    update_wifi_status(&wifi_status);
    main_box.append(&wifi_btn);

    // Bluetooth button
    let (bt_btn, bt_status) = create_control_button("bluetooth", "Bluetooth", "Checking...");
    let bt_status_clone = bt_status.clone();
    bt_btn.connect_clicked(move |_| {
        toggle_bluetooth();
        let bt_status_weak = bt_status_clone.downgrade();
        glib::timeout_add_seconds_local(1, move || {
            let Some(bt_status) = bt_status_weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            update_bluetooth_status(&bt_status);
            glib::ControlFlow::Break
        });
    });
    update_bluetooth_status(&bt_status);
    main_box.append(&bt_btn);

    // Airplane mode button
    let (airplane_btn, _) = create_control_button("airplane-mode", "Airplane Mode", "Disable all");
    let airplane_btn_weak = wifi_btn.downgrade();
    airplane_btn.connect_clicked(move |_| {
        airplane_mode();
        if let Some(_) = airplane_btn_weak.upgrade() {
            // Window will close
        }
    });
    main_box.append(&airplane_btn);

    wifi_btn
}

fn create_control_button(icon_name: &str, title: &str, subtitle: &str) -> (Button, Label) {
    let button = Button::new();
    button.add_css_class("control-btn");

    let hbox = GtkBox::new(Orientation::Horizontal, 12);
    hbox.set_margin_top(12);
    hbox.set_margin_bottom(12);
    hbox.set_margin_start(16);
    hbox.set_margin_end(16);

    // Icon
    let icon = load_app_icon(icon_name, 24);
    hbox.append(&icon);

    // Text box
    let text_box = GtkBox::new(Orientation::Vertical, 2);

    let title_label = Label::new(Some(title));
    title_label.add_css_class("btn-title");
    title_label.set_halign(gtk4::Align::Start);

    let status_label = Label::new(Some(subtitle));
    status_label.add_css_class("btn-subtitle");
    status_label.set_halign(gtk4::Align::Start);

    text_box.append(&title_label);
    text_box.append(&status_label);

    hbox.append(&text_box);

    button.set_child(Some(&hbox));

    (button, status_label)
}

fn build_power_section(main_box: &GtkBox, window: &ApplicationWindow) {
    let power_label = Label::new(Some("POWER"));
    power_label.add_css_class("section-label");
    power_label.set_halign(gtk4::Align::Start);
    power_label.set_margin_top(8);
    main_box.append(&power_label);

    let power_box = GtkBox::new(Orientation::Horizontal, 8);
    power_box.set_halign(gtk4::Align::Fill);
    power_box.set_spacing(8);

    // Log Out button
    let logout_btn = create_power_button("system-log-out", "Log Out");
    let window_weak = window.downgrade();
    logout_btn.connect_clicked(move |_| {
        let _ = Command::new("loginctl")
            .arg("terminate-user")
            .arg("")
            .spawn();
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    power_box.append(&logout_btn);

    // Suspend button
    let suspend_btn = create_power_button("system-suspend", "Suspend");
    let window_weak = window.downgrade();
    suspend_btn.connect_clicked(move |_| {
        let _ = Command::new("systemctl").arg("suspend").spawn();
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    power_box.append(&suspend_btn);

    // Restart button
    let restart_btn = create_power_button("system-reboot", "Restart");
    let window_weak = window.downgrade();
    restart_btn.connect_clicked(move |_| {
        let _ = Command::new("systemctl").arg("reboot").spawn();
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    power_box.append(&restart_btn);

    // Shut Down button
    let shutdown_btn = create_power_button("system-shutdown", "Shut Down");
    let window_weak = window.downgrade();
    shutdown_btn.connect_clicked(move |_| {
        let _ = Command::new("systemctl").arg("poweroff").spawn();
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    power_box.append(&shutdown_btn);

    main_box.append(&power_box);
}

fn create_power_button(icon_name: &str, label: &str) -> Button {
    let button = Button::new();
    button.add_css_class("power-btn");
    button.set_vexpand(true);
    button.set_hexpand(true);

    let vbox = GtkBox::new(Orientation::Vertical, 6);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(8);
    vbox.set_margin_end(8);
    vbox.set_halign(gtk4::Align::Center);

    // Icon
    let icon = load_app_icon(icon_name, 24);
    vbox.append(&icon);

    // Label
    let text_label = Label::new(Some(label));
    text_label.add_css_class("power-label");
    vbox.append(&text_label);

    button.set_child(Some(&vbox));
    button
}

fn build_hint_section(main_box: &GtkBox) {
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_size_request(-1, 8);
    main_box.append(&spacer);

    let hint_label = Label::new(Some(
        "ESC to close • Enter to launch • Click outside to dismiss",
    ));
    hint_label.add_css_class("hint");
    hint_label.set_halign(gtk4::Align::Center);
    main_box.append(&hint_label);
}

fn setup_key_handlers(window: &ApplicationWindow, search_entry: &Entry) {
    let event_controller = gtk4::EventControllerKey::new();
    let window_weak = window.downgrade();
    let search_entry_weak = search_entry.downgrade();
    event_controller.connect_key_pressed(move |_, key, _, _modifiers| {
        let Some(window) = window_weak.upgrade() else {
            return glib::Propagation::Proceed;
        };
        let Some(search_entry) = search_entry_weak.upgrade() else {
            return glib::Propagation::Proceed;
        };

        if key == gtk4::gdk::Key::Escape {
            if !search_entry.text().is_empty() {
                search_entry.set_text("");
                return glib::Propagation::Stop;
            }
            window.close();
            return glib::Propagation::Stop;
        }

        if key == gtk4::gdk::Key::Meta_L
            || key == gtk4::gdk::Key::Meta_R
            || key == gtk4::gdk::Key::Super_L
            || key == gtk4::gdk::Key::Super_R
        {
            window.close();
            return glib::Propagation::Stop;
        }

        glib::Propagation::Proceed
    });
    window.add_controller(event_controller);

    // Click outside to close
    let focus_controller = gtk4::EventControllerFocus::new();
    let window_weak = window.downgrade();
    focus_controller.connect_leave(move |_| {
        if let Some(window) = window_weak.upgrade() {
            window.close();
        }
    });
    window.add_controller(focus_controller);
}
