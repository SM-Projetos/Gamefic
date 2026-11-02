// Native Slint entry point.
//
// The system layer is intentionally reused from the Tauri implementation.
// These modules are Rust-only and do not depend on Tauri APIs.
#[path = "../../src-tauri/src/system/mod.rs"]
mod system;
mod monitor;

slint::include_modules!();

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

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
        _ => name.chars().next().map(|ch| ch.to_uppercase().collect::<String>()).unwrap_or_else(|| "?".to_string()),
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


fn source_label(source: &str) -> String {
    if let Some(package) = source.strip_prefix("nativo:") {
        format!("Repositório da distribuição · {package}")
    } else if source == "flatpak" {
        "Flatpak · sistema".to_string()
    } else if source == "flatpak-user" {
        "Flatpak · usuário".to_string()
    } else if let Some(package) = source.strip_prefix("snap:") {
        format!("Snap Store · {package}")
    } else {
        source.to_string()
    }
}

fn source_cards(entries: &[system::install::FonteInfo]) -> ModelRc<AppSourceData> {
    let sources = entries
        .iter()
        .map(|entry| AppSourceData {
            id: SharedString::from(entry.fonte.as_str()),
            name: SharedString::from(source_label(&entry.fonte)),
            available: entry.disponivel,
            installed: entry.instalado,
            version: SharedString::from(entry.versao.as_deref().unwrap_or("")),
            repository_version: SharedString::from(entry.versao_repo.as_deref().unwrap_or("")),
            reason: SharedString::from(entry.motivo.as_deref().unwrap_or("")),
            indeterminate: entry.indeterminado,
        })
        .collect::<Vec<_>>();
    Rc::new(VecModel::from(sources)).into()
}

fn apply_detail(ui: &GameficWindow, detail: system::install::AppDetail) {
    ui.set_detail_name(SharedString::from(detail.nome));
    ui.set_detail_gui(detail.gui);
    ui.set_detail_vendor(SharedString::from(detail.fornecedor));
    ui.set_detail_license(SharedString::from(detail.licenca));
    ui.set_detail_summary(SharedString::from(detail.desc_curta));
    ui.set_detail_description(SharedString::from(detail.desc_longa));
    ui.set_detail_sources(source_cards(&detail.fontes));
    ui.set_detail_error(SharedString::default());
    ui.set_detail_loading(false);
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


fn format_gib(kb: u64) -> String {
    let value = kb as f64 / (1024.0 * 1024.0);
    format!("{value:.1} GiB")
}

fn format_rate(bytes_per_second: f64) -> String {
    if bytes_per_second >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bytes_per_second / (1024.0 * 1024.0))
    } else {
        format!("{:.1} KB/s", bytes_per_second / 1024.0)
    }
}

fn percent_text(value: Option<f32>) -> String {
    value.map(|number| format!("{number:.0}%")).unwrap_or_else(|| "Não disponível".to_string())
}

fn percent_int(value: Option<f32>) -> i32 {
    value.map(|number| number.round().clamp(0.0, 100.0) as i32).unwrap_or(0)
}

fn apply_metric_sample(ui: &GameficWindow, sample: monitor::MetricSample) {
    let cpu_peak = sample.cpu_peak_percent.map(|value| format!("Pico: {value:.1}%")).unwrap_or_else(|| "Pico: Não disponível".to_string());
    let cpu_frequency = sample.cpu_freq_avg_mhz
        .map(|mhz| if mhz >= 1000 { format!("Frequência média: {:.2} GHz", mhz as f32 / 1000.0) } else { format!("Frequência média: {mhz} MHz") })
        .unwrap_or_else(|| "Frequência média: Não disponível".to_string());
    let cpu_temperature = sample.temp_max_c
        .map(|value| format!("Temperatura máxima: {value:.0} °C"))
        .unwrap_or_else(|| "Temperatura: Não disponível".to_string());
    ui.set_metric_cpu_value(SharedString::from(percent_text(sample.cpu_percent)));
    ui.set_metric_cpu_detail_a(SharedString::from(cpu_temperature));
    ui.set_metric_cpu_detail_b(SharedString::from(format!("{cpu_frequency} · {cpu_peak}")));
    ui.set_metric_cpu_percent(percent_int(sample.cpu_percent));

    let network_link = match sample.net_link.as_deref() {
        Some("cabo") => "Conexão cabeada",
        Some("wifi") => "Conexão Wi-Fi",
        Some("off") => "Sem conexão",
        Some(_) => "Conexão identificada",
        None => "Conexão não identificada",
    };
    let network_label = match sample.net_link.as_deref() { Some("cabo") => "Cabo", Some("wifi") => "Wi-Fi", Some("off") => "Offline", Some(_) => "Rede ativa", None => "Não disponível" };
    ui.set_metric_network_value(SharedString::from(network_label));
    ui.set_metric_network_link(SharedString::from(network_link));
    ui.set_metric_network_detail_a(SharedString::from(sample.net_download_bps.map(|v| format!("Download: {}", format_rate(v))).unwrap_or_else(|| "Download: Não disponível".to_string())));
    ui.set_metric_network_detail_b(SharedString::from(sample.net_upload_bps.map(|v| format!("Upload: {}", format_rate(v))).unwrap_or_else(|| "Upload: Não disponível".to_string())));

    let gpu_available = sample.gpu_utilization_percent.is_some();
    ui.set_metric_gpu_available(gpu_available);
    ui.set_metric_gpu_value(SharedString::from(percent_text(sample.gpu_utilization_percent)));
    ui.set_metric_gpu_percent(percent_int(sample.gpu_utilization_percent));
    ui.set_metric_gpu_detail_a(SharedString::from(sample.gpu_temp_c.map(|v| format!("Temperatura: {v:.0} °C")).unwrap_or_else(|| "Temperatura: Não disponível".to_string())));
    ui.set_metric_gpu_detail_b(SharedString::from(sample.gpu_utilization_percent.map(|v| format!("Utilização: {v:.0}%")).unwrap_or_else(|| "Utilização não disponível neste driver".to_string())));

    let vram_percent = match (sample.gpu_mem_used_mb, sample.gpu_mem_total_mb) {
        (Some(used), Some(total)) if total > 0.0 => Some((used / total * 100.0).clamp(0.0, 100.0)),
        _ => None,
    };
    ui.set_metric_vram_available(vram_percent.is_some());
    ui.set_metric_vram_percent(percent_int(vram_percent));
    ui.set_metric_vram_value(SharedString::from(percent_text(vram_percent)));
    ui.set_metric_vram_detail_a(SharedString::from(match (sample.gpu_mem_used_mb, sample.gpu_mem_total_mb) {
        (Some(used), Some(total)) => format!("Em uso: {:.1} / {:.1} GiB", used / 1024.0, total / 1024.0),
        _ => "Memória de vídeo indisponível".to_string(),
    }));
    ui.set_metric_vram_detail_b(SharedString::from(vram_percent.map(|v| format!("Uso da VRAM: {v:.0}%")).unwrap_or_else(|| "Uso da VRAM não disponível".to_string())));

    let ram_percent = sample.mem_used_percent;
    ui.set_metric_ram_value(SharedString::from(percent_text(ram_percent)));
    ui.set_metric_ram_percent(percent_int(ram_percent));
    ui.set_metric_ram_detail_a(SharedString::from(match (sample.mem_used_kb, sample.mem_total_kb) {
        (Some(used), Some(total)) => format!("Em uso: {} / {}", format_gib(used), format_gib(total)),
        _ => "Em uso: Não disponível".to_string(),
    }));
    ui.set_metric_ram_detail_b(SharedString::from(match (sample.mem_used_kb, sample.mem_total_kb) {
        (Some(used), Some(total)) => format!("Disponível: {}", format_gib(total.saturating_sub(used))),
        _ => "Disponível: Não disponível".to_string(),
    }));

    let swap_percent = sample.swap_used_percent;
    let swap_configured = sample.swap_total_kb.is_some();
    let swap_available = sample.swap_total_kb.map(|value| value > 0).unwrap_or(false);
    ui.set_metric_swap_available(swap_available);
    ui.set_metric_swap_value(SharedString::from(if swap_available { percent_text(swap_percent) } else if swap_configured { "Sem swap ativo".to_string() } else { "Não disponível".to_string() }));
    ui.set_metric_swap_percent(percent_int(swap_percent));
    ui.set_metric_swap_detail_a(SharedString::from(match (sample.swap_used_kb, sample.swap_total_kb) {
        (Some(used), Some(total)) if total > 0 => format!("Em uso: {} / {}", format_gib(used), format_gib(total)),
        (Some(_), Some(0)) => "Sem swap ativo".to_string(),
        _ => "Swap não disponível".to_string(),
    }));
    ui.set_metric_swap_detail_b(SharedString::from(match (sample.swap_used_kb, sample.swap_total_kb) {
        (Some(used), Some(total)) if total > 0 => format!("Disponível: {}", format_gib(total.saturating_sub(used))),
        (Some(_), Some(0)) => "Sem swap ativo".to_string(),
        _ => "Disponibilidade da swap desconhecida".to_string(),
    }));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set a stable desktop identity before the window is created (Wayland/X11).
    slint::set_xdg_app_id("com.sergio.gamefic")?;

    let ui = GameficWindow::new()?;

    // Populate the first frame from the previous session, when available.
    if let Some(cached) = system::snapshot_cache_rapido() {
        apply_snapshot(&ui, &cached);
        ui.set_system_status(SharedString::from(status_from_snapshot(&cached)));
    }

    // Detalhes são consultados em background; somente a event loop Slint
    // altera propriedades da interface. O contador invalida respostas antigas.
    let detail_generation = Arc::new(AtomicU64::new(0));
    let detail_ui = ui.as_weak();
    let generation_on_select = Arc::clone(&detail_generation);
    ui.on_app_selected(move |app_id| {
        let request = generation_on_select.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = Arc::clone(&generation_on_select);
        let ui_for_worker = detail_ui.clone();
        let ui_for_failure = detail_ui.clone();
        let id = app_id.to_string();

        let worker = std::thread::Builder::new()
            .name("gamefic-app-detail".to_string())
            .spawn(move || {
                let detail = system::install::detail(&id);
                let _ = ui_for_worker.upgrade_in_event_loop(move |ui| {
                    if generation.load(Ordering::SeqCst) != request {
                        return;
                    }
                    match detail {
                        Some(detail) => apply_detail(&ui, detail),
                        None => {
                            ui.set_detail_loading(false);
                            ui.set_detail_error(SharedString::from(
                                "Não foi possível carregar os detalhes deste aplicativo.",
                            ));
                        }
                    }
                });
            });

        if worker.is_err() {
            let generation = Arc::clone(&generation_on_select);
            let _ = ui_for_failure.upgrade_in_event_loop(move |ui| {
                if generation.load(Ordering::SeqCst) == request {
                    ui.set_detail_loading(false);
                    ui.set_detail_error(SharedString::from(
                        "Não foi possível iniciar a consulta em segundo plano.",
                    ));
                }
            });
        }
    });

    let generation_on_close = Arc::clone(&detail_generation);
    ui.on_app_detail_closed(move || {
        generation_on_close.fetch_add(1, Ordering::SeqCst);
    });

    // Instalação, atualização e remoção usam exclusivamente a API validada
    // do backend. O processo roda fora da UI thread e os detalhes são relidos.
    let action_generation = Arc::new(AtomicU64::new(0));
    let action_generation_cb = Arc::clone(&action_generation);
    let action_ui = ui.as_weak();
    ui.on_source_action_requested(move |app_id, action, source| {
        let request = action_generation_cb.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = Arc::clone(&action_generation_cb);
        let weak = action_ui.clone();
        let id = app_id.to_string();
        let action = action.to_string();
        let source = source.to_string();

        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_action_busy(true);
            ui.set_action_error(false);
            ui.set_action_message(SharedString::from("Executando operação…"));
        });

        let worker = std::thread::Builder::new()
            .name("gamefic-app-action".to_string())
            .spawn(move || {
                let result = system::install::acao_app(&id, &action, &source);
                let refreshed = system::install::detail(&id);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    if generation.load(Ordering::SeqCst) != request {
                        return;
                    }
                    if let Some(detail) = refreshed {
                        apply_detail(&ui, detail);
                    }
                    ui.set_action_busy(false);
                    match result {
                        Ok(message) => {
                            ui.set_action_error(false);
                            ui.set_action_message(SharedString::from(message));
                        }
                        Err(error) => {
                            ui.set_action_error(true);
                            ui.set_action_message(SharedString::from(error.to_string()));
                        }
                    }
                });
            });

        if worker.is_err() {
            let generation = Arc::clone(&action_generation_cb);
            let _ = action_ui.upgrade_in_event_loop(move |ui| {
                if generation.load(Ordering::SeqCst) == request {
                    ui.set_action_busy(false);
                    ui.set_action_error(true);
                    ui.set_action_message(SharedString::from(
                        "Não foi possível iniciar a operação em segundo plano.",
                    ));
                }
            });
        }
    });

    // Abertura também é delegada ao backend; ele valida GUI, instalação e
    // origem (Flatpak/nativa), sem montar comandos no frontend.
    let open_generation = Arc::clone(&action_generation);
    let open_ui = ui.as_weak();
    ui.on_app_open_requested(move |app_id| {
        let request = open_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = Arc::clone(&open_generation);
        let weak = open_ui.clone();
        let id = app_id.to_string();
        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_action_busy(true);
            ui.set_action_error(false);
            ui.set_action_message(SharedString::from("Abrindo aplicativo…"));
        });
        let worker = std::thread::Builder::new()
            .name("gamefic-app-open".to_string())
            .spawn(move || {
                let result = system::install::abrir_launcher(&id, false);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    if generation.load(Ordering::SeqCst) != request {
                        return;
                    }
                    ui.set_action_busy(false);
                    match result {
                        Ok(message) => {
                            ui.set_action_error(false);
                            ui.set_action_message(SharedString::from(message));
                        }
                        Err(error) => {
                            ui.set_action_error(true);
                            ui.set_action_message(SharedString::from(error));
                        }
                    }
                });
            });
        if worker.is_err() {
            let generation = Arc::clone(&open_generation);
            let _ = open_ui.upgrade_in_event_loop(move |ui| {
                if generation.load(Ordering::SeqCst) == request {
                    ui.set_action_busy(false);
                    ui.set_action_error(true);
                    ui.set_action_message(SharedString::from(
                        "Não foi possível iniciar a abertura em segundo plano.",
                    ));
                }
            });
        }
    });

    // Monitoramento contínuo: coleta real em thread dedicada e atualização da UI pela event loop Slint.
    let monitor_controller = monitor::MonitorController::new();
    let monitor_for_interval = monitor_controller.clone();
    ui.on_monitor_interval_changed(move |interval_ms| {
        monitor_for_interval.set_interval(interval_ms.max(500) as u64);
    });
    let monitor_ui = ui.as_weak();
    monitor_controller.start(move |sample| {
        let _ = monitor_ui.upgrade_in_event_loop(move |ui| apply_metric_sample(&ui, sample));
    })?;

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
