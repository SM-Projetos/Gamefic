// Native Slint entry point.
//
// The system layer is intentionally reused from the Tauri implementation.
// These modules are Rust-only and do not depend on Tauri APIs.
#[path = "../../src-tauri/src/system/mod.rs"]
mod system;

slint::include_modules!();

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;

fn glyph_for(id: &str, name: &str) -> String {
    match id.to_ascii_lowercase().as_str() {
        "steam" => "S".to_string(),
        "lutris" => "L".to_string(),
        "heroic" | "heroic-games-launcher" => "H".to_string(),
        "itch" | "itchio" => "i".to_string(),
        "minigalaxy" => "M".to_string(),
        "retroarch" => "RA".to_string(),
        "dolphin" => "D".to_string(),
        "pcsx2" => "P2".to_string(),
        "ppsspp" => "P".to_string(),
        "duckstation" => "DS".to_string(),
        "mangohud" => "MH".to_string(),
        "gamemode" => "GM".to_string(),
        "protontricks" => "PT".to_string(),
        "wine" => "W".to_string(),
        _ => name.chars().next().unwrap_or('?').to_uppercase().to_string(),
    }
}

fn has_raster_icon(id: &str) -> bool {
    matches!(
        id.to_ascii_lowercase().as_str(),
        "steam"
            | "heroic"
            | "heroic-games-launcher"
            | "itch"
            | "itchio"
            | "minigalaxy"
            | "pcsx2"
            | "ppsspp"
            | "duckstation"
            | "gamemode"
            | "protontricks"
    )
}

fn app_cards(entries: &[system::AppEntry]) -> ModelRc<AppCardData> {
    let cards = entries
        .iter()
        .map(|entry| AppCardData {
            id: SharedString::from(entry.id.as_str()),
            name: SharedString::from(entry.name.as_str()),
            description: SharedString::from(entry.description.as_str()),
            state: SharedString::from(entry.estado.as_str()),
            glyph: SharedString::from(glyph_for(&entry.id, &entry.name)),
            raster_icon: has_raster_icon(&entry.id),
        })
        .collect::<Vec<_>>();

    Rc::new(VecModel::from(cards)).into()
}

fn tool_cards(entries: &[system::ToolInfo]) -> ModelRc<AppCardData> {
    let cards = entries
        .iter()
        .map(|entry| AppCardData {
            id: SharedString::from(entry.id.as_str()),
            name: SharedString::from(entry.name.as_str()),
            description: SharedString::from(entry.description.as_str()),
            // Availability is a capability/detection signal, not proof that a
            // package can be installed. Unknown is safer than inventing a state.
            state: SharedString::from(if entry.available {
                "instalado"
            } else {
                "desconhecido"
            }),
            glyph: SharedString::from(glyph_for(&entry.id, &entry.name)),
            raster_icon: has_raster_icon(&entry.id),
        })
        .collect::<Vec<_>>();

    Rc::new(VecModel::from(cards)).into()
}

fn apply_snapshot(ui: &GameficWindow, snapshot: &system::SystemSnapshot) {
    ui.set_launchers(app_cards(&snapshot.launchers));
    ui.set_emulators(app_cards(&snapshot.emuladores));
    ui.set_game_tools(tool_cards(&snapshot.tools));
    ui.set_compatibility(app_cards(&snapshot.compatibilidade));
    ui.set_catalog_loaded(true);
}

fn status_from_snapshot(snapshot: &system::SystemSnapshot) -> String {
    match snapshot.distro.as_ref() {
        Some(distro) => format!("Sistema detectado · {}", distro.name),
        None => "Sistema detectado".to_string(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set a stable desktop identity before the window is created (Wayland/X11).
    slint::set_xdg_app_id("com.sergio.gamefic");

    let ui = GameficWindow::new()?;

    // Populate the first frame from the previous session, when available.
    if let Some(cached) = system::snapshot_cache_rapido() {
        apply_snapshot(&ui, &cached);
        ui.set_system_status(SharedString::from(status_from_snapshot(&cached)));
    }

    // A fresh detection runs outside the UI thread. Only the Slint event loop
    // is allowed to mutate component properties.
    let ui_weak = ui.as_weak();
    std::thread::Builder::new()
        .name("gamefic-system-detection".to_string())
        .spawn(move || {
            let snapshot = system::snapshot();
            let status = SharedString::from(status_from_snapshot(&snapshot));

            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                apply_snapshot(&ui, &snapshot);
                ui.set_system_status(status);
            });
        })?;

    ui.run()?;
    Ok(())
}
