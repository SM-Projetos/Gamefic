use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::system::{cpu, gpu, memory, sensors};

pub const HISTORY_LIMIT: usize = 600;
pub const DEFAULT_INTERVAL_MS: u64 = 2000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricSample {
    pub timestamp_ms: u64,
    pub cpu_percent: Option<f32>,
    pub cpu_freq_avg_mhz: Option<u32>,
    pub mem_used_percent: Option<f32>,
    pub mem_used_kb: Option<u64>,
    pub mem_total_kb: Option<u64>,
    pub swap_used_percent: Option<f32>,
    pub swap_used_kb: Option<u64>,
    pub swap_total_kb: Option<u64>,
    pub temp_max_c: Option<f32>,
    pub temp_label: Option<String>,
    pub gpu_utilization_percent: Option<f32>,
    pub gpu_mem_used_mb: Option<f32>,
    pub gpu_mem_total_mb: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub net_download_bps: Option<f64>,
    pub net_upload_bps: Option<f64>,
    /// "cabo" | "wifi" | "off" (None: impossível determinar).
    pub net_link: Option<String>,
}

#[derive(Debug, Default)]
pub struct MonitorInner {
    running: AtomicBool,
    generation: AtomicU64,
    lifecycle: Mutex<()>,
    history: Mutex<VecDeque<MetricSample>>,
}

pub type MonitorState = Arc<MonitorInner>;

fn generation_is_active(state: &MonitorInner, generation: u64) -> bool {
    state.running.load(Ordering::SeqCst)
        && state.generation.load(Ordering::SeqCst) == generation
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Soma dos contadores cumulativos de bytes (rx, tx) de todas as interfaces
/// físicas de rede (/proc/net/dev, ignorando `lo`). None: impossível ler.
fn ler_rede() -> Option<(u64, u64)> {
    let conteudo = std::fs::read_to_string("/proc/net/dev").ok()?;
    let mut rx_total = 0u64;
    let mut tx_total = 0u64;
    for linha in conteudo.lines().skip(2) {
        let (iface, resto) = linha.split_once(':')?;
        if iface.trim() == "lo" {
            continue;
        }
        // colunas: rx_bytes rx_packets rx_errs rx_drop rx_fifo rx_frame rx_comp rx_multi tx_bytes ...
        let colunas: Vec<&str> = resto.split_whitespace().collect();
        if colunas.len() < 9 {
            continue;
        }
        let rx = colunas[0].parse::<u64>().ok()?;
        let tx = colunas[8].parse::<u64>().ok()?;
        rx_total = rx_total.checked_add(rx)?;
        tx_total = tx_total.checked_add(tx)?;
    }
    Some((rx_total, tx_total))
}

/// Tipo de conexão ativa pelos operstates de /sys/class/net:
/// cabo (interface não-wireless ativa) tem prioridade; "off" quando existem
/// interfaces, mas nenhuma ativa; None quando /sys não cooperar.
fn tipo_conexao() -> Option<&'static str> {
    let entradas = std::fs::read_dir("/sys/class/net").ok()?;
    let mut cabo_ativo = false;
    let mut wifi_ativo = false;
    let mut cabo = false;
    let mut wifi = false;
    for entrada in entradas.flatten() {
        let nome = entrada.file_name().to_string_lossy().to_string();
        if nome == "lo" {
            continue;
        }
        let caminho = entrada.path();
        let sem_fio = caminho.join("wireless").exists();
        let operstate = std::fs::read_to_string(caminho.join("operstate"))
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
            == "up";
        if sem_fio {
            wifi = true;
            if operstate {
                wifi_ativo = true;
            }
        } else {
            cabo = true;
            if operstate {
                cabo_ativo = true;
            }
        }
    }
    if cabo_ativo {
        Some("cabo")
    } else if wifi_ativo {
        Some("wifi")
    } else if cabo || wifi {
        Some("off")
    } else {
        None
    }
}

fn collect_sample(net_prev: &mut Option<(u64, u64, u64)>) -> MetricSample {
    let mem = memory::detect();

    let mem_total_kb = mem.as_ref().and_then(|m| m.total_kb);
    let mem_used_kb = mem.as_ref().and_then(|m| m.used_kb);

    let mem_used_percent = match (mem_used_kb, mem_total_kb) {
        (Some(used), Some(total)) if total > 0 => Some((used as f32 / total as f32) * 100.0),
        _ => None,
    };

    let swap_total_kb = mem.as_ref().and_then(|m| m.swap_total_kb).filter(|t| *t > 0);
    let swap_used_kb = mem.as_ref().and_then(|m| m.swap_used_kb);
    let swap_used_percent = match (swap_used_kb, swap_total_kb) {
        (Some(used), Some(total)) if total > 0 => Some((used as f32 / total as f32) * 100.0),
        _ => None,
    };

    let temps = sensors::detect();
    let hottest = temps
        .iter()
        .filter(|t| t.celsius > 0.0)
        .max_by(|a, b| a.celsius.total_cmp(&b.celsius));

    let gpu_metrics = gpu::metrics();

    // Vazão de rede: diferença entre os contadores acumulados desta leitura e
    // da amostra anterior, dividida pelo tempo entre amostras.
    let timestamp_ms = now_ms();
    let mut net_download_bps = None;
    let mut net_upload_bps = None;
    if let Some((rx, tx)) = ler_rede() {
        if let Some((rx0, tx0, ts0)) = *net_prev {
            let decorrido = timestamp_ms.saturating_sub(ts0) as f64 / 1000.0;
            if decorrido >= 0.1 {
                net_download_bps = Some(rx.saturating_sub(rx0) as f64 / decorrido);
                net_upload_bps = Some(tx.saturating_sub(tx0) as f64 / decorrido);
            }
        }
        *net_prev = Some((rx, tx, timestamp_ms));
    }

    MetricSample {
        timestamp_ms,
        cpu_percent: cpu::usage_percent(),
        cpu_freq_avg_mhz: cpu_avg_freq(),
        mem_used_percent,
        mem_used_kb,
        mem_total_kb,
        swap_used_percent,
        swap_used_kb,
        swap_total_kb,
        temp_max_c: hottest.map(|t| t.celsius),
        temp_label: hottest.map(|t| format!("{}·{}", t.chip, t.label)),
        gpu_utilization_percent: gpu_metrics.as_ref().and_then(|g| g.utilization_percent),
        gpu_mem_used_mb: gpu_metrics.as_ref().and_then(|g| g.mem_used_mb),
        gpu_mem_total_mb: gpu_metrics.as_ref().and_then(|g| g.mem_total_mb),
        gpu_temp_c: gpu_metrics.as_ref().and_then(|g| g.temp_c),
        net_download_bps,
        net_upload_bps,
        net_link: tipo_conexao().map(|t| t.to_string()),
    }
}

fn cpu_avg_freq() -> Option<u32> {
    let mut freqs: Vec<u32> = Vec::new();

    if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !(name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit())) {
                continue;
            }
            if let Ok(raw) = std::fs::read_to_string(entry.path().join("cpufreq/scaling_cur_freq"))
            {
                let khz: u32 = raw.trim().parse().unwrap_or(0);
                if khz > 0 {
                    freqs.push(khz / 1000);
                }
            }
        }
    }

    if freqs.is_empty() {
        None
    } else {
        let sum: u64 = freqs.iter().map(|f| *f as u64).sum();
        Some((sum / freqs.len() as u64) as u32)
    }
}

/// eslint-disable: cada coleta acontece na thread de fundo, nunca na interface.
#[tauri::command]
pub fn start_monitoring(
    app: AppHandle,
    state: State<'_, MonitorState>,
    interval_ms: u64,
) -> Result<(), String> {
    let interval = interval_ms.max(500);
    let _lifecycle = state
        .lifecycle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // Chamadas repetidas não devem iniciar outro trabalhador.
    if state.running.load(Ordering::SeqCst) {
        return Ok(());
    }

    let generation = state
        .generation
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1);
    state.running.store(true, Ordering::SeqCst);

    let shared = Arc::clone(&state);
    let app_handle = app.clone();

    std::thread::Builder::new()
        .name("gamefic-monitor".to_string())
        .spawn(move || {
            // contadores de rede da amostra anterior (rx acumulado, tx, timestamp)
            let mut net_prev: Option<(u64, u64, u64)> = None;
            loop {
                if !generation_is_active(&shared, generation) {
                    break;
                }

                let sample = collect_sample(&mut net_prev);

                {
                    // Serializa a publicação da amostra com stop/start. Se esta
                    // geração foi invalidada durante a coleta, descarta a amostra.
                    let _lifecycle = shared
                        .lifecycle
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if !generation_is_active(&shared, generation) {
                        break;
                    }

                    let mut history = shared
                        .history
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    history.push_back(sample.clone());
                    while history.len() > HISTORY_LIMIT {
                        history.pop_front();
                    }
                    drop(history);
                    let _ = app_handle.emit("metrics", sample);
                }

                let mut slept = 0u64;
                while slept < interval && generation_is_active(&shared, generation) {
                    let step = 200.min(interval - slept);
                    std::thread::sleep(Duration::from_millis(step));
                    slept += step;
                }
            }
        })
        .map_err(|e| {
            state.running.store(false, Ordering::SeqCst);
            state.generation.fetch_add(1, Ordering::SeqCst);
            format!("falha ao iniciar a coleta: {e}")
        })?;

    Ok(())
}

#[tauri::command]
pub fn stop_monitoring(state: State<'_, MonitorState>) {
    let _lifecycle = state
        .lifecycle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.running.store(false, Ordering::SeqCst);
    // Invalida qualquer trabalhador antigo, inclusive se um novo iniciar logo depois.
    state.generation.fetch_add(1, Ordering::SeqCst);
}

#[tauri::command]
pub fn get_history(state: State<'_, MonitorState>) -> Vec<MetricSample> {
    let history = state
        .history
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    history.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::{generation_is_active, MonitorInner};
    use std::sync::atomic::Ordering;

    #[test]
    fn current_generation_is_active_while_running() {
        let state = MonitorInner::default();
        state.generation.store(7, Ordering::SeqCst);
        state.running.store(true, Ordering::SeqCst);

        assert!(generation_is_active(&state, 7));
    }

    #[test]
    fn stopped_or_replaced_generation_is_inactive() {
        let state = MonitorInner::default();
        state.generation.store(7, Ordering::SeqCst);
        state.running.store(true, Ordering::SeqCst);
        assert!(generation_is_active(&state, 7));

        state.generation.store(8, Ordering::SeqCst);
        assert!(!generation_is_active(&state, 7));
        assert!(generation_is_active(&state, 8));

        state.running.store(false, Ordering::SeqCst);
        assert!(!generation_is_active(&state, 8));
    }
}
