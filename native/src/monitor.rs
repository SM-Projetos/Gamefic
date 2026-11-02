use std::collections::VecDeque;
use std::fs;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::system::{cpu, gpu, memory, sensors};

const HISTORY_LIMIT: usize = 600;
const DEFAULT_INTERVAL_MS: u64 = 2_000;

#[derive(Debug, Clone, Default)]
pub struct MetricSample {
    pub cpu_percent: Option<f32>,
    pub cpu_freq_avg_mhz: Option<u32>,
    pub temp_max_c: Option<f32>,
    pub temp_label: Option<String>,
    pub mem_used_percent: Option<f32>,
    pub mem_used_kb: Option<u64>,
    pub mem_total_kb: Option<u64>,
    pub swap_used_percent: Option<f32>,
    pub swap_used_kb: Option<u64>,
    pub swap_total_kb: Option<u64>,
    pub gpu_utilization_percent: Option<f32>,
    pub gpu_mem_used_mb: Option<f32>,
    pub gpu_mem_total_mb: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub net_download_bps: Option<f64>,
    pub net_upload_bps: Option<f64>,
    pub net_link: Option<String>,
}

#[derive(Clone)]
pub struct MonitorController {
    running: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    history: Arc<Mutex<VecDeque<MetricSample>>>,
}

impl MonitorController {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            interval_ms: Arc::new(AtomicU64::new(DEFAULT_INTERVAL_MS)),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(HISTORY_LIMIT))),
        }
    }

    pub fn set_interval(&self, interval_ms: u64) {
        let interval = match interval_ms {
            1_000 | 2_000 | 5_000 => interval_ms,
            _ => DEFAULT_INTERVAL_MS,
        };
        self.interval_ms.store(interval, Ordering::SeqCst);
    }

    pub fn start<F>(&self, on_sample: F) -> io::Result<()>
    where
        F: Fn(MetricSample) + Send + 'static,
    {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let running = Arc::clone(&self.running);
        let interval = Arc::clone(&self.interval_ms);
        let history = Arc::clone(&self.history);

        std::thread::Builder::new()
            .name("gamefic-monitor".to_string())
            .spawn(move || {
                let mut net_previous: Option<(u64, u64, u64)> = None;

                while running.load(Ordering::SeqCst) {
                    let sample = collect_sample(&mut net_previous);
                    {
                        let mut samples = history
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        samples.push_back(sample.clone());
                        while samples.len() > HISTORY_LIMIT {
                            samples.pop_front();
                        }
                    }
                    on_sample(sample);

                    let wait_ms = interval.load(Ordering::SeqCst).max(500);
                    let mut elapsed = 0;
                    while elapsed < wait_ms && running.load(Ordering::SeqCst) {
                        let step = 100.min(wait_ms - elapsed);
                        std::thread::sleep(Duration::from_millis(step));
                        elapsed += step;
                    }
                }
            })?;

        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn history(&self) -> Vec<MetricSample> {
        self.history
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .cloned()
            .collect()
    }
}

fn collect_sample(net_previous: &mut Option<(u64, u64, u64)>) -> MetricSample {
    let mem = memory::detect();
    let mem_total_kb = mem.as_ref().and_then(|value| value.total_kb);
    let mem_used_kb = mem.as_ref().and_then(|value| value.used_kb);
    let mem_used_percent = percent(mem_used_kb, mem_total_kb);

    let swap_total_kb = mem.as_ref().and_then(|value| value.swap_total_kb).filter(|v| *v > 0);
    let swap_used_kb = mem.as_ref().and_then(|value| value.swap_used_kb);
    let swap_used_percent = percent(swap_used_kb, swap_total_kb);

    let temperatures = sensors::detect();
    let hottest = temperatures
        .iter()
        .filter(|reading| reading.celsius > 0.0)
        .max_by(|left, right| left.celsius.total_cmp(&right.celsius));
    let gpu_metrics = gpu::metrics();

    let timestamp_ms = now_ms();
    let mut net_download_bps = None;
    let mut net_upload_bps = None;
    if let Some((rx, tx)) = read_network_counters() {
        if let Some((previous_rx, previous_tx, previous_timestamp)) = *net_previous {
            let seconds = timestamp_ms.saturating_sub(previous_timestamp) as f64 / 1_000.0;
            if seconds >= 0.1 {
                net_download_bps = Some(rx.saturating_sub(previous_rx) as f64 / seconds);
                net_upload_bps = Some(tx.saturating_sub(previous_tx) as f64 / seconds);
            }
        }
        *net_previous = Some((rx, tx, timestamp_ms));
    }

    MetricSample {
        cpu_percent: cpu::usage_percent(),
        cpu_freq_avg_mhz: average_cpu_frequency(),
        temp_max_c: hottest.map(|reading| reading.celsius),
        temp_label: hottest.map(|reading| format!("{} · {}", reading.chip, reading.label)),
        mem_used_percent,
        mem_used_kb,
        mem_total_kb,
        swap_used_percent,
        swap_used_kb,
        swap_total_kb,
        gpu_utilization_percent: gpu_metrics.as_ref().and_then(|value| value.utilization_percent),
        gpu_mem_used_mb: gpu_metrics.as_ref().and_then(|value| value.mem_used_mb),
        gpu_mem_total_mb: gpu_metrics.as_ref().and_then(|value| value.mem_total_mb),
        gpu_temp_c: gpu_metrics.as_ref().and_then(|value| value.temp_c),
        net_download_bps,
        net_upload_bps,
        net_link: network_link().map(str::to_string),
    }
}

fn percent(used: Option<u64>, total: Option<u64>) -> Option<f32> {
    match (used, total) {
        (Some(used), Some(total)) if total > 0 => Some((used as f32 / total as f32) * 100.0),
        _ => None,
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn read_network_counters() -> Option<(u64, u64)> {
    let content = fs::read_to_string("/proc/net/dev").ok()?;
    let mut rx_total = 0_u64;
    let mut tx_total = 0_u64;

    for line in content.lines().skip(2) {
        let (interface, counters) = line.split_once(':')?;
        if interface.trim() == "lo" {
            continue;
        }
        let columns: Vec<&str> = counters.split_whitespace().collect();
        if columns.len() < 9 {
            continue;
        }
        rx_total = rx_total.checked_add(columns[0].parse::<u64>().ok()?)?;
        tx_total = tx_total.checked_add(columns[8].parse::<u64>().ok()?)?;
    }

    Some((rx_total, tx_total))
}

fn network_link() -> Option<&'static str> {
    let interfaces = fs::read_dir("/sys/class/net").ok()?;
    let mut wired = false;
    let mut wireless = false;
    let mut wired_present = false;
    let mut wireless_present = false;

    for entry in interfaces.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }
        let path = entry.path();
        let is_wireless = path.join("wireless").exists();
        let active = fs::read_to_string(path.join("operstate"))
            .map(|value| value.trim() == "up")
            .unwrap_or(false);

        if is_wireless {
            wireless_present = true;
            wireless |= active;
        } else {
            wired_present = true;
            wired |= active;
        }
    }

    if wired {
        Some("cabo")
    } else if wireless {
        Some("wifi")
    } else if wired_present || wireless_present {
        Some("off")
    } else {
        None
    }
}

fn average_cpu_frequency() -> Option<u32> {
    let entries = fs::read_dir("/sys/devices/system/cpu").ok()?;
    let mut frequencies = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("cpu") || !name[3..].chars().all(|character| character.is_ascii_digit()) {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(entry.path().join("cpufreq/scaling_cur_freq")) {
            if let Ok(khz) = raw.trim().parse::<u32>() {
                if khz > 0 {
                    frequencies.push(khz / 1_000);
                }
            }
        }
    }

    if frequencies.is_empty() {
        None
    } else {
        Some((frequencies.iter().map(|value| *value as u64).sum::<u64>()
            / frequencies.len() as u64) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::percent;

    #[test]
    fn calculates_percent_only_with_valid_total() {
        assert_eq!(percent(Some(25), Some(100)), Some(25.0));
        assert_eq!(percent(Some(25), Some(0)), None);
        assert_eq!(percent(None, Some(100)), None);
    }
}
