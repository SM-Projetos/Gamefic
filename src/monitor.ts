import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

import { drawChartEixo, RollingData, type ChartStyle, type OpcoesChart } from "./charts";

export interface MetricSample {
  timestampMs: number;
  cpuPercent: number | null;
  cpuFreqAvgMhz: number | null;
  memUsedPercent: number | null;
  memUsedKb: number | null;
  memTotalKb: number | null;
  swapUsedPercent: number | null;
  swapUsedKb: number | null;
  swapTotalKb: number | null;
  tempMaxC: number | null;
  tempLabel: string | null;
  gpuUtilizationPercent: number | null;
  gpuMemUsedMb: number | null;
  gpuMemTotalMb: number | null;
  gpuTempC: number | null;
  netDownloadBps: number | null;
  netUploadBps: number | null;
  netLink: "cabo" | "wifi" | "off" | null;
}

const HISTORY = 300;

const ROXO = { stroke: "#a78bfa", fill: "rgba(167, 139, 250, 0.18)" };
const CYAN = { stroke: "#22d3ee", fill: "rgba(34, 211, 238, 0.14)" };

interface CardChart {
  canvasId: string;
  series: RollingData[];
  styles: ChartStyle[];
  /** eixo de tempo no rodapé (RAM/Swap, conforme o modelo) */
  eixoTempo?: boolean;
  /** escala automática em bytes/s (card Rede); senão 100% */
  taxa?: boolean;
  opcoes?: Partial<OpcoesChart>;
}

/** bytes por segundo -> "8.4 MB/s" / "1.2 MB/s" / "850 KB/s" */
function formatarTaxa(bytesPorS: number): string {
  if (bytesPorS >= 1024 * 1024) return `${(bytesPorS / (1024 * 1024)).toFixed(1)} MB/s`;
  return `${(bytesPorS / 1024).toFixed(1)} KB/s`;
}

/** KiB -> número em GiB, sem unidade (a unidade está no rótulo/tooltip) */
function gibs(kb: number): string {
  const v = kb / (1024 * 1024);
  return v >= 100 ? v.toFixed(0) : v.toFixed(1);
}

/** MB -> número em GiB, sem unidade */
function mibs(mb: number): string {
  const v = mb / 1024;
  return v >= 100 ? v.toFixed(0) : v.toFixed(1);
}

/** MHz -> "3.27 GHz" / "950 MHz" */
function frequencia(mhz: number): string {
  return mhz >= 1000 ? `${(mhz / 1000).toFixed(2)} GHz` : `${mhz} MHz`;
}

const ROTULO_PCT = (v: number) => `${Math.round(v)}%`;

const TIPOS_CONEXAO: Record<string, string> = {
  cabo: "Conexão cabeada",
  wifi: "Conexão Wi-Fi",
  off: "Sem conexão",
};

export class Monitor {
  private cards: Record<string, CardChart>;
  private unlisten: UnlistenFn | null = null;
  private intervalMs = 2000;
  private resizeObserver: ResizeObserver | null = null;
  private picoCpu: number | null = null;

  constructor(
    private readonly onChangeInterval: (intervalMs: number) => void,
  ) {
    this.cards = {
      cpu: { canvasId: "dp-cpu-spark", series: [new RollingData(HISTORY)], styles: [ROXO] },
      rede: { canvasId: "dp-net-spark", series: [new RollingData(HISTORY), new RollingData(HISTORY)], styles: [CYAN, ROXO], taxa: true },
      gpu: { canvasId: "dp-gpu-spark", series: [new RollingData(HISTORY)], styles: [CYAN] },
      vram: { canvasId: "dp-vram-spark", series: [new RollingData(HISTORY)], styles: [ROXO] },
      ram: { canvasId: "dp-ram-spark", series: [new RollingData(HISTORY)], styles: [ROXO], eixoTempo: true },
      swap: { canvasId: "dp-swap-spark", series: [new RollingData(HISTORY)], styles: [ROXO], eixoTempo: true },
    };
  }

  private canvas(id: string): HTMLCanvasElement | null {
    const el = document.getElementById(id);
    return el instanceof HTMLCanvasElement ? el : null;
  }

  async init(): Promise<void> {
    const samples = await invoke<MetricSample[]>("get_history");
    samples.forEach((s) => this.pushSample(s));

    this.unlisten = await listen<MetricSample>("metrics", (event) => {
      this.pushSample(event.payload);
    });
  }

  getInterval(): number {
    return this.intervalMs;
  }

  setInterval(intervalMs: number): void {
    this.intervalMs = intervalMs;
    this.onChangeInterval(intervalMs);
  }

  hide(): void {
    this.resizeObserver?.disconnect();
  }

  show(canvasHost: HTMLElement | null = null): void {
    if (canvasHost) {
      if (!this.resizeObserver) {
        this.resizeObserver = new ResizeObserver(() => this.render());
      }
      this.resizeObserver.observe(canvasHost);
    }
    this.render();
  }

  dispose(): void {
    this.unlisten?.();
    this.unlisten = null;
    this.resizeObserver?.disconnect();
  }

  render(): void {
    Object.values(this.cards).forEach((card) => this.renderCard(card));
  }

  private renderCard(card: CardChart): void {
    const canvas = this.canvas(card.canvasId);
    if (!canvas || !canvas.isConnected) return;
    const opcoes: OpcoesChart = card.taxa
      ? this.opcoesRede(card)
      : {
          maxY: 100,
          rotuloY: ROTULO_PCT,
          eixoTempo: card.eixoTempo === true,
          intervaloMs: this.intervalMs,
        };
    drawChartEixo(
      canvas,
      card.series.map((serie, i) => ({
        data: serie.values(),
        style: card.styles[i] ?? ROXO,
      })),
      opcoes,
    );
  }

  /** escala do card Rede: teto "redondo" acima do pico observado (bytes/s) */
  private opcoesRede(card: CardChart): OpcoesChart {
    const pico = Math.max(
      card.series[0]?.max() ?? 0,
      card.series[1]?.max() ?? 0,
      1024 * 64, // piso: 64 KB/s, para o eixo não zerar com rede ociosa
    );
    const passo = 10 ** Math.floor(Math.log10(pico));
    const maxY = Math.ceil(pico / passo) * passo;
    return {
      // eixo já em "porcentagem da escala": desenha normalizado (dados / maxY)
      maxY,
      rotuloY: (v) => formatarTaxa(v),
      eixoTempo: false,
      intervaloMs: this.intervalMs,
    };
  }

  private pushSample(sample: MetricSample): void {
    const vramPct = sample.gpuMemUsedMb == null
      ? null
      : sample.gpuMemTotalMb && sample.gpuMemTotalMb > 0
        ? Math.min(100, (sample.gpuMemUsedMb / sample.gpuMemTotalMb) * 100)
        : null;

    this.cards.cpu.series[0]?.push(sample.cpuPercent);
    if (this.cards.rede) {
      this.cards.rede.series[0]?.push(sample.netDownloadBps);
      this.cards.rede.series[1]?.push(sample.netUploadBps);
    }
    this.cards.gpu.series[0]?.push(sample.gpuUtilizationPercent);
    this.cards.vram.series[0]?.push(vramPct);
    this.cards.ram.series[0]?.push(sample.memUsedPercent);
    this.cards.swap.series[0]?.push(sample.swapUsedPercent);

    // pico de uso da CPU nesta sessão (linha "Uso (pico)")
    if (sample.cpuPercent != null) {
      this.picoCpu = this.picoCpu == null ? sample.cpuPercent : Math.max(this.picoCpu, sample.cpuPercent);
    }

    // ---------- card CPU ----------
    setText("dp-cpu-big", sample.cpuPercent == null ? null : `${sample.cpuPercent.toFixed(0)}%`);
    setText("dp-cpu-temperatura", sample.tempMaxC == null ? null : `${sample.tempMaxC.toFixed(0)} °C`);
    setText("dp-cpu-frequencia", sample.cpuFreqAvgMhz == null ? null : frequencia(sample.cpuFreqAvgMhz));
    setText("dp-cpu-pico",
      sample.cpuPercent == null && this.picoCpu == null
        ? null
        : `${(sample.cpuPercent ?? 0).toFixed(1)} / ${(this.picoCpu ?? 0).toFixed(1)}%`);

    // ---------- card Rede ----------
    const link = sample.netLink || null;
    setText("dp-net-tipo", link ? TIPOS_CONEXAO[link] ?? link : "Não disponível");
    setText("dp-net-down", sample.netDownloadBps == null ? null : formatarTaxa(sample.netDownloadBps));
    setText("dp-net-up", sample.netUploadBps == null ? null : formatarTaxa(sample.netUploadBps));

    // ---------- card GPU ----------
    setText("dp-gpu-big", sample.gpuUtilizationPercent == null ? null : `${sample.gpuUtilizationPercent.toFixed(0)}%`);
    setText("dp-gpu-temperatura", sample.gpuTempC == null ? null : `${sample.gpuTempC.toFixed(0)} °C`);
    setText("dp-gpu-uso", sample.gpuUtilizationPercent == null ? null : `${sample.gpuUtilizationPercent.toFixed(0)}%`);
    setText("dp-gpu-vram", vramPct == null ? null : `${vramPct.toFixed(0)}%`);

    // ---------- card VRAM ----------
    setText("dp-vram-big", vramPct == null ? null : `${vramPct.toFixed(0)}%`);
    setText("dp-vram-uso", vramPct == null ? null : `${(sample.gpuMemUsedMb! / 1024).toFixed(1)} / ${(sample.gpuMemTotalMb! / 1024).toFixed(1)} GiB`);
    setText("dp-vram-disp",
      vramPct != null ? `${Math.max(0, (sample.gpuMemTotalMb! - sample.gpuMemUsedMb!) / 1024).toFixed(1)} GiB` : null);
    setText("dp-vram-pct", vramPct == null ? null : `${vramPct.toFixed(0)}%`);

    // ---------- card Memória RAM (RAM | Swap) ----------
    const totalKb = sample.memTotalKb;
    setText("dp-ram-uso",
      sample.memUsedKb == null || totalKb == null
        ? null
        : `${gib(sample.memUsedKb)} / ${gib(totalKb)}`);
    const disponivel = sample.memUsedKb != null && totalKb != null ? Math.max(0, totalKb - sample.memUsedKb) : null;
    setText("dp-ram-disp", disponivel == null ? null : gib(disponivel));
    setText("dp-ram-pct", sample.memUsedPercent == null ? null : `${sample.memUsedPercent.toFixed(0)}%`);

    const temSwap = sample.swapTotalKb != null && sample.swapTotalKb > 0;
    if (temSwap) {
      setText("dp-swap-uso",
        sample.swapUsedKb == null ? null : `${gib(sample.swapUsedKb)} / ${gib(sample.swapTotalKb!)}`);
      const swapDisp = sample.swapUsedKb != null ? Math.max(0, sample.swapTotalKb! - sample.swapUsedKb) : null;
      setText("dp-swap-disp", swapDisp == null ? null : gib(swapDisp));
      setText("dp-swap-pct", sample.swapUsedPercent == null ? null : `${sample.swapUsedPercent.toFixed(0)}%`);
    } else {
      setText("dp-swap-uso", sample.swapTotalKb == null ? null : "Sem swap ativo");
      setText("dp-swap-disp", null);
      setText("dp-swap-pct", sample.swapTotalKb == null ? null : "Sem swap ativo");
    }

    Object.values(this.cards).forEach((card) => this.renderCard(card));
  }
}

function setText(id: string, value: string | null): void {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = value ?? "Não disponível";
}
