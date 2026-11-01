const DPR = typeof devicePixelRatio === "number" && devicePixelRatio > 0 ? devicePixelRatio : 1;

export interface ChartStyle {
  stroke: string;
  fill: string;
}

export interface SerieChart {
  data: readonly (number | null)[];
  style: ChartStyle;
}

export interface OpcoesChart {
  /** valor do topo da escala; 100 para porcentagens, bytes/s para rede */
  maxY: number;
  /** rótulos do eixo Y (p. ex. v => `${v}%` ou taxa formatada) */
  rotuloY?: (valor: number) => string;
  /** desenha marcações de tempo (Agora, -15 s, -30 s, -1 min ...) no rodapé */
  eixoTempo?: boolean;
  /** intervalo entre amostras em ms, para calcular as marcações de tempo */
  intervaloMs?: number;
}

export class RollingData {
  private points: (number | null)[] = [];

  constructor(private readonly limit: number) {}

  push(value: number | null): void {
    this.points.push(value);
    while (this.points.length > this.limit) this.points.shift();
  }

  values(): readonly (number | null)[] {
    return this.points;
  }

  max(): number | null {
    let max: number | null = null;
    for (const v of this.points) {
      if (v != null && (max == null || v > max)) max = v;
    }
    return max;
  }

  last(): number | null {
    for (let i = this.points.length - 1; i >= 0; i -= 1) {
      if (this.points[i] != null) return this.points[i];
    }
    return null;
  }
}

/** duração em segundos -> rótulo "-45 s" / "-2 min" */
function rotuloTempo(segundos: number): string {
  if (segundos >= 60) {
    const minutos = Math.round(segundos / 60);
    return minutos >= 10 ? `-${minutos} min` : `-${minutos} min`;
  }
  return `-${Math.round(segundos)} s`;
}

/**
 * Gráfico em área com eixo Y rotulado (100% / 50% / 0% ou taxas) e,
 * opcionalmente, marcações de tempo no rodapé — o gráfico do Monitor
 * remodelado. Aceita várias séries sobrepostas (card Rede).
 */
export function drawChartEixo(
  canvas: HTMLCanvasElement,
  series: readonly SerieChart[],
  opcoes: OpcoesChart,
): void {
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  if (width === 0 || height === 0) return;

  canvas.width = Math.round(width * DPR);
  canvas.height = Math.round(height * DPR);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
  ctx.clearRect(0, 0, width, height);

  const { maxY } = opcoes;
  const alturaTempo = opcoes.eixoTempo ? 15 : 0;
  const padBordas = 6;
  const padDireita = 6;
  const eixoLargura = 40; // área reservada aos rótulos Y
  const plotX = eixoLargura;
  const plotW = Math.max(1, width - plotX - padDireita);
  const plotH = Math.max(1, height - padBordas - alturaTempo);

  const principal = series[0];
  if (!principal) return;
  const viva = principal.data.some((v) => v != null);
  // Y nos níveis 0, meio e topo
  ctx.font = "10px 'JetBrains Mono', monospace";
  ctx.fillStyle = "#9aa0b4";
  ctx.textAlign = "left";
  ctx.textBaseline = "middle";
  const meio = Math.round(maxY / 2);
  const rotulo = opcoes.rotuloY ?? ((v: number) => `${v}%`);
  ctx.fillText(rotulo(maxY), 2, padBordas / 2 + plotH * 0.08);
  ctx.fillText(rotulo(meio), 2, padBordas / 2 + plotH * 0.5 + plotH * 0.08);
  ctx.fillText(rotulo(0), 2, plotH + 1);

  if (!viva) {
    ctx.font = "12px 'Inter', sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText("sem dados", plotX + plotW / 2, plotH / 2);
    return;
  }

  // linhas-guia no meio e no topo (trilha sutil)
  for (const frac of [0.5, 1]) {
    ctx.beginPath();
    ctx.moveTo(plotX, plotH * (1 - frac) + padBordas / 2);
    ctx.lineTo(plotX + plotW, plotH * (1 - frac) + padBordas / 2);
    ctx.strokeStyle = "rgba(148, 163, 184, 0.12)";
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  for (const { data, style } of series) {
    if (data.length < 2) continue;
    const passoX = plotW / (data.length - 1);
    const yDe = (v: number) => padBordas / 2 + plotH - (Math.min(Math.max(v, 0), maxY) / maxY) * plotH;

    // segmentos separados por falhas (null)
    const segmentos: Array<Array<[number, number]>> = [];
    let atual: Array<[number, number]> = [];
    data.forEach((v, i) => {
      if (v == null) {
        if (atual.length > 1) segmentos.push(atual);
        atual = [];
        return;
      }
      atual.push([plotX + i * passoX, yDe(v)]);
    });
    if (atual.length > 1) segmentos.push(atual);

    for (const segmento of segmentos) {
      // preenchimento
      ctx.beginPath();
      segmento.forEach((p, i) => {
        if (i === 0) ctx.moveTo(p[0], p[1]);
        else ctx.lineTo(p[0], p[1]);
      });
      const ultima = segmento[segmento.length - 1];
      ctx.lineTo(ultima[0], padBordas / 2 + plotH);
      ctx.lineTo(segmento[0][0], padBordas / 2 + plotH);
      ctx.closePath();
      ctx.fillStyle = style.fill;
      ctx.fill();

      // linha
      ctx.beginPath();
      segmento.forEach((p, i) => {
        if (i === 0) ctx.moveTo(p[0], p[1]);
        else ctx.lineTo(p[0], p[1]);
      });
      ctx.strokeStyle = style.stroke;
      ctx.lineWidth = 2;
      ctx.lineJoin = "round";
      ctx.lineCap = "round";
      ctx.stroke();
    }
  }

  // eixo de tempo: Agora à direita e marcas proporcionais (amorosas com janelas curtas)
  if (opcoes.eixoTempo && opcoes.intervaloMs && opcoes.intervaloMs > 0) {
    const n = Math.max(...series.map((s) => s.data.length));
    if (n >= 2) {
      const duracaoSeg = ((n - 1) * opcoes.intervaloMs) / 1000;
      ctx.fillStyle = "#9aa0b4";
      ctx.textAlign = "center";
      ctx.textBaseline = "top";
      // marcas candidatas; desenha as que cabem na janela real
      const alvos = [15, 30, 45, 60, 90, 120, 180, 300, 600, 900, 1800, 3600];
      const marcas: number[] = [];
      for (const alvo of alvos) {
        if (alvo >= duracaoSeg) {
          marcas.push(alvo);
          break;
        }
        marcas.push(alvo);
      }
      for (const marca of marcas) {
        if (marca >= duracaoSeg) continue;
        const x = plotX + plotW * (1 - marca / duracaoSeg);
        ctx.fillText(rotuloTempo(marca), x, plotH + 5);
      }
      ctx.fillText("Agora", plotX + plotW - 2, plotH + 5);
    }
  }
}
