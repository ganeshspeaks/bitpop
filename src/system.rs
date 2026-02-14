use gtk4::Label;
use std::process::Command;

pub fn update_wifi_status(label: &Label) {
    if let Ok(output) = Command::new("nmcli").args(&["radio", "wifi"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("enabled") {
            if let Ok(active_output) = Command::new("nmcli")
                .args(&["connection", "show", "--active"])
                .output()
            {
                let active_stdout = String::from_utf8_lossy(&active_output.stdout);

                let lines: Vec<&str> = active_stdout.lines().collect();
                if lines.len() > 1 {
                    let first_line = lines[1];
                    let parts: Vec<&str> = first_line.split_whitespace().collect();
                    if !parts.is_empty() {
                        let network_name = parts[0];
                        label.set_text(&format!("On • {}", network_name));
                        return;
                    }
                }
            }
            label.set_text("On");
        } else {
            label.set_text("Off");
        }
    } else {
        label.set_text("Status unknown");
    }
}

pub fn toggle_wifi() {
    if let Ok(output) = Command::new("nmcli").args(&["radio", "wifi"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("enabled") {
            let _ = Command::new("nmcli")
                .args(&["radio", "wifi", "off"])
                .output();
        } else {
            let _ = Command::new("nmcli")
                .args(&["radio", "wifi", "on"])
                .output();
        }
    }
}

pub fn update_bluetooth_status(label: &Label) {
    if let Ok(output) = Command::new("bluetoothctl").arg("show").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Powered: yes") {
            if let Ok(devices_output) = Command::new("bluetoothctl")
                .args(&["devices", "Connected"])
                .output()
            {
                let devices_stdout = String::from_utf8_lossy(&devices_output.stdout);
                let lines: Vec<&str> = devices_stdout.lines().collect();
                if !lines.is_empty() && !lines[0].is_empty() {
                    let first_line = lines[0];
                    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        let device_name = parts[2];
                        label.set_text(&format!("On • {}", device_name));
                        return;
                    }
                }
            }
            label.set_text("On");
        } else {
            label.set_text("Off");
        }
    } else {
        label.set_text("Status unknown");
    }
}

pub fn toggle_bluetooth() {
    if let Ok(output) = Command::new("bluetoothctl").arg("show").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Powered: yes") {
            let _ = Command::new("bluetoothctl")
                .args(&["power", "off"])
                .output();
        } else {
            let _ = Command::new("bluetoothctl").args(&["power", "on"]).output();
        }
    }
}

pub fn airplane_mode() {
    let _ = Command::new("nmcli")
        .args(&["radio", "all", "off"])
        .output();
}

pub fn update_battery(label: &Label) {
    if let Ok(capacity) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
        if let Ok(status) = std::fs::read_to_string("/sys/class/power_supply/BAT0/status") {
            let capacity = capacity.trim();
            let status = status.trim();

            let status_text = match status {
                "Charging" => "Charging",
                "Discharging" => "On Battery",
                _ => status,
            };

            label.set_text(&format!("{}%  •  {}", capacity, status_text));
        }
    } else {
        label.set_text("Battery N/A");
    }
}
