mod apps;
mod icons;
mod system;
mod ui;

use gtk4::prelude::*;
use gtk4::{Application, glib};

use ui::{build_ui, check_existing_instance, remove_lock_file, setup_cleanup, write_lock_file};

const APP_ID: &str = "com.bitpop.quickaccess";

fn main() -> glib::ExitCode {
    if let Some(pid) = check_existing_instance() {
        let _ = std::process::Command::new("kill")
            .args(&["-TERM", &pid.to_string()])
            .output();

        remove_lock_file();
        return glib::ExitCode::SUCCESS;
    }

    write_lock_file();

    setup_cleanup();

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(build_ui);

    let exit_code = app.run();

    remove_lock_file();

    exit_code
}
