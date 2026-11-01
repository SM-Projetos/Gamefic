import { invoke } from "@tauri-apps/api/core";
import type { AppDetail } from "./apps";

export interface DistroInfo {
  name: string;
  version?: string | null;
  family?: string | null;
}

export interface CpuInfo {
  model?: string | null;
  vendor?: string | null;
  coresPhysical?: number | null;
  threadsLogical?: number | null;
  baseFreqMhz?: number | null;
  currentFreqsMhz: number[];
  usagePercent?: number | null;
}

export interface MemoryInfo {
  totalKb?: number | null;
  usedKb?: number | null;
  availableKb?: number | null;
  swapTotalKb?: number | null;
  swapUsedKb?: number | null;
}

export interface GpuInfo {
  pciAddress?: string | null;
  vendorId?: string | null;
  vendorName?: string | null;
  deviceId?: string | null;
  deviceName?: string | null;
  driver?: string | null;
  driverVersion?: string | null;
}

export interface VulkanGpu {
  deviceName?: string | null;
  driverName?: string | null;
}

export interface VulkanInfo {
  gpus: VulkanGpu[];
}

export interface SensorReading {
  chip: string;
  label: string;
  celsius: number;
}

export interface ToolInfo {
  id: string;
  name: string;
  description: string;
  available: boolean;
  version?: string | null;
  icon?: string | null;
}

export interface StorageDevice {
  name: string;
  model?: string | null;
  kind?: string | null;
  sizeBytes?: number | null;
  usedBytes?: number | null;
  totalBytes?: number | null;
}

export interface AppEntry {
  id?: string;
  name: string;
  description: string;
  available: boolean;
  version?: string | null;
  icon?: string | null;
  estado?: string | null; // "instalado" | "nao_instalado" | "desconhecido"
  formato?: string | null;
}

export interface SystemSnapshot {
  distro?: DistroInfo | null;
  uptimeSeconds?: number | null;
  storage: StorageDevice[];
  kernel?: string | null;
  arch?: string | null;
  motherboard?: string | null;
  cpu?: CpuInfo | null;
  cpuTempC?: number | null;
  mesaVersion?: string | null;
  memory?: MemoryInfo | null;
  gpus: GpuInfo[];
  vulkan?: VulkanInfo | null;
  temperatures: SensorReading[];
  tools: ToolInfo[];
  launchers: AppEntry[];
  emuladores: AppEntry[];
  compatibilidade: AppEntry[];
  criacaoGravacao: AppEntry[];
  criacaoVideo: AppEntry[];
  criacaoAudio: AppEntry[];
  criacaoSocial: AppEntry[];
}

export const INDISPONIVEL = "Não disponível";

export function getSystemInfo(): Promise<SystemSnapshot> {
  return invoke<SystemSnapshot>("get_system_info");
}

export function mhz(mhz: number | null | undefined): string {
  if (mhz == null || mhz <= 0) return INDISPONIVEL;
  return mhz >= 1000 ? `${(mhz / 1000).toFixed(1)} GHz` : `${mhz} MHz`;
}

export function gib(kb: number | null | undefined): string {
  if (kb == null || kb <= 0) return INDISPONIVEL;
  return `${(kb / (1024 * 1024)).toFixed(1)} GiB`;
}

export function temperatura(celsius: number): string {
  return `${celsius.toFixed(1)} °C`;
}

let ultimoCatalogo: SystemSnapshot | null = null;
// (busca do catálogo removida — renderTiles mantém o termo por compatibilidade)
let appsUIwired = false;

interface InfoAtualizacao {
  nome: string;
  fonte: string;
  versao: string;
  versaoRepo: string;
}

/** apps com atualização confirmada pelos detalhes reais (fonte: popularizarCartao) */
const atualizacoesMap = new Map<string, InfoAtualizacao>();

export function renderVisaoGeral(snapshot: SystemSnapshot): void {
  ultimoCatalogo = snapshot;
  appsEstadoLimpar();
  wireAppsUI();
  renderCatalogo();
  renderDesempenho(snapshot);
}

/** erro global na detecção: estado de erro do catálogo com tentativa de novo (por main.ts) */
export function appsEstadoErro(mensagem: string): void {
  const box = document.getElementById("apps-estado");
  if (!box) return;
  box.removeAttribute("hidden");
  const texto = document.getElementById("apps-estado-texto");
  if (texto) texto.textContent = mensagem;
  document.getElementById("apps-retentar")?.removeAttribute("hidden");
}

export function appsEstadoLimpar(): void {
  document.getElementById("apps-estado")?.setAttribute("hidden", "");
}

/** contador único da topbar: total de atualizações disponíveis (aba Apps +
 * Criação de Conteúdo); recomputado a cada mudança do mapa real */
function comporContadorAtualizacoes(): void {
  setText("#topbar-atualizacoes", String(atualizacoesMap.size));
}

function atualizarIndicadores(_snapshot: SystemSnapshot): void {
  comporContadorAtualizacoes();
}

/** registra no mapa real que o app saiu da fila de atualizações (sucesso na
 * ação) e refresca o contador imediatamente, sem esperar o re-render geral */
function retirarAtualizacao(appId: string): void {
  if (atualizacoesMap.delete(appId)) comporContadorAtualizacoes();
}

/** usado também pelo fluxo da janela "Gerenciar" (apps.ts) */
export function notificarAtualizacaoFeita(appId: string): void {
  retirarAtualizacao(appId);
}

function renderCatalogo(): void {
  if (!ultimoCatalogo) return;
  atualizacoesMap.clear(); // recontada conforme os detalhes de cada card chegam
  renderTiles(ultimoCatalogo.launchers, "ov-launchers", "Nenhum launcher instalado", "");
  renderTiles(ultimoCatalogo.emuladores, "ov-emul", "Nenhum emulador detectado", "");
  renderTiles(ultimoCatalogo.tools, "ov-tools", "Nenhuma ferramenta de jogos detectada", "");
  renderTiles(ultimoCatalogo.compatibilidade, "ov-compat", "Nenhuma ferramenta de compatibilidade detectada", "");
  // aba Criação de Conteúdo: mesmos cards e funções da aba Apps
  renderTiles(ultimoCatalogo.criacaoGravacao ?? [], "ov-criacao-gravacao", "Nenhum aplicativo de gravação e stream no catálogo", "");
  renderTiles(ultimoCatalogo.criacaoVideo ?? [], "ov-criacao-video", "Nenhum aplicativo de vídeo no catálogo", "");
  renderTiles(ultimoCatalogo.criacaoAudio ?? [], "ov-criacao-audio", "Nenhum aplicativo de áudio no catálogo", "");
  renderTiles(ultimoCatalogo.criacaoSocial ?? [], "ov-criacao-social", "Nenhum aplicativo social no catálogo", "");
  inserirCardAddCompat();
  atualizarIndicadores(ultimoCatalogo);
  sincronizarModalAtualizacoes();
}

function wireAppsUI(): void {
  if (appsUIwired) return;
  appsUIwired = true;

  const scrim = document.getElementById("modal-atualizacoes");
  // botão persistente da topbar (visível em todas as abas)
  document.getElementById("btn-atualizacoes-topbar")?.addEventListener("click", abrirModalAtualizacoes);
  document.getElementById("btn-fechar-atualizacoes")?.addEventListener("click", () => {
    scrim?.setAttribute("hidden", "");
  });
  // clique no fundo do scrim fecha (só quando o alvo é o próprio scrim)
  scrim?.addEventListener("click", (ev) => {
    if (ev.target === scrim) scrim.setAttribute("hidden", "");
  });

  const lista = document.getElementById("atualizacoes-lista");
  // atualização individual (delegado)
  lista?.addEventListener("click", (ev) => {
    const botao = (ev.target as HTMLElement).closest<HTMLButtonElement>("button[data-atualizar-app]");
    if (!botao) return;
    void atualizarAppPopup(botao.dataset.appId ?? "", botao.dataset.fonte ?? "", botao);
  });
  document.getElementById("btn-atualizar-tudo")?.addEventListener("click", () => {
    void atualizarTudoPopup();
  });
}

// ---------- popup "Atualizações" ----------

function modalAtualizacoesAberto(): boolean {
  const scrim = document.getElementById("modal-atualizacoes");
  return Boolean(scrim && !scrim.hasAttribute("hidden"));
}

function abrirModalAtualizacoes(): void {
  renderizarListaAtualizacoes();
  document.getElementById("modal-atualizacoes")?.removeAttribute("hidden");
}

/** mantém o popup coerente com o mapa real quando o catálogo é revalidado */
function sincronizarModalAtualizacoes(): void {
  if (modalAtualizacoesAberto()) renderizarListaAtualizacoes();
}

function renderizarListaAtualizacoes(): void {
  const lista = document.getElementById("atualizacoes-lista");
  const vazio = document.getElementById("atualizacoes-vazio");
  const acoes = document.getElementById("atualizacoes-acoes");
  const status = document.getElementById("atualizacoes-status");
  if (!lista) return;
  lista.innerHTML = "";
  const itens = Array.from(atualizacoesMap.entries());
  if (itens.length === 0) {
    if (vazio) vazio.removeAttribute("hidden");
    if (acoes) acoes.setAttribute("hidden", "");
    return;
  }
  if (vazio) vazio.setAttribute("hidden", "");
  if (acoes) acoes.removeAttribute("hidden");
  if (status) status.textContent = "";
  const botaoTudo = document.getElementById("btn-atualizar-tudo") as HTMLButtonElement | null;
  if (botaoTudo) {
    botaoTudo.disabled = false;
    botaoTudo.textContent = "Atualizar tudo";
  }
  itens.forEach(([appId, info]) => {
    const item = document.createElement("li");
    item.className = "atualizacao-item";
    item.dataset.appId = appId;
    const dif = info.versaoRepo ? `<span class="atualizacao-item__versao num">${esc(info.versao || "—")} → ${esc(info.versaoRepo)}</span>` : "";
    item.innerHTML = `
      <span class="atualizacao-item__nome">${esc(info.nome)} ${dif}</span>
      <button class="btn btn--ghost btn--small" type="button" data-atualizar-app data-app-id="${esc(appId)}" data-fonte="${esc(info.fonte)}">Atualizar</button>
    `;
    lista.appendChild(item);
  });
}

async function atualizarAppPopup(appId: string, fonte: string, botao: HTMLButtonElement): Promise<void> {
  if (!appId) return;
  const status = document.getElementById("atualizacoes-status");
  const item = botao.closest<HTMLElement>(".atualizacao-item");
  botao.disabled = true;
  if (item) item.setAttribute("aria-busy", "true");
  if (status) status.textContent = "Atualizando...";
  try {
    const msg = await invoke<string>("app_action", { appId, acao: "atualizar", fonte });
    if (status) status.textContent = msg;
    // sucesso: o app sai da fila e o contador da topbar refresca na hora
    setAtualizadoNaLista(appId, msg);
    retirarAtualizacao(appId);
  } catch (e) {
    if (status) status.textContent = `Erro: ${String(e)}`;
    botao.disabled = false;
    if (item) item.removeAttribute("aria-busy");
  }
  window.dispatchEvent(new Event("estado-apps-mudou"));
  invalidarDetalhes();
}

/** feedback no item após atualizar (a revalidação retira o app do mapa real) */
function setAtualizadoNaLista(appId: string, msg: string): void {
  const item = document.querySelector<HTMLElement>(`.atualizacao-item[data-app-id="${CSS.escape(appId)}"]`);
  if (!item) return;
  item.setAttribute("data-concluido", "1");
  const btn = item.querySelector<HTMLButtonElement>("button[data-atualizar-app]");
  if (btn) {
    btn.disabled = true;
    btn.textContent = msg.length > 24 ? "Concluído" : msg;
  }
}

async function atualizarTudoPopup(): Promise<void> {
  const botaoTudo = document.getElementById("btn-atualizar-tudo") as HTMLButtonElement | null;
  const status = document.getElementById("atualizacoes-status");
  if (!botaoTudo || botaoTudo.disabled) return;
  const itens = Array.from(document.querySelectorAll<HTMLElement>("#atualizacoes-lista .atualizacao-item[data-app-id]:not([data-concluido])"));
  if (itens.length === 0) return;
  botaoTudo.disabled = true;
  botaoTudo.textContent = "Atualizando tudo...";
  itens.forEach((item) => {
    const btn = item.querySelector<HTMLButtonElement>("button[data-atualizar-app]");
    if (btn) btn.disabled = true;
  });
  let falhas = 0;
  for (const item of itens) {
    const appId = item.dataset.appId ?? "";
    const registro = atualizacoesMap.get(appId);
    if (!registro) continue;
    if (status) status.textContent = `Atualizando ${registro.nome}...`;
    try {
      const msg = await invoke<string>("app_action", { appId, acao: "atualizar", fonte: registro.fonte });
      setAtualizadoNaLista(appId, msg);
      retirarAtualizacao(appId);
    } catch (e) {
      falhas += 1;
      console.error(`Falha ao atualizar ${registro.nome}:`, e);
    }
  }
  if (status) status.textContent = falhas > 0 ? `Concluído, com ${falhas} falha(s).` : "Concluído.";
  if (botaoTudo) {
    botaoTudo.disabled = false;
    botaoTudo.textContent = "Atualizar tudo";
  }
  window.dispatchEvent(new Event("estado-apps-mudou"));
  invalidarDetalhes();
}

function inserirCardAddCompat(): void {
  const grid = document.getElementById("ov-compat");
  if (!grid) return;
  const add = document.createElement("div");
  add.className = "app-card app-card--add";
  add.setAttribute("role", "button");
  add.setAttribute("tabindex", "0");
  add.setAttribute("aria-label", "Adicionar ferramenta de compatibilidade (Wine ou Proton)");
  add.innerHTML = `
    <span aria-hidden="true">+</span>
    <span class="apps-add-nome">Adicionar</span>
  `;
  grid.appendChild(add);
}

export function renderDesempenho(snapshot: SystemSnapshot): void {
  // CPU: só temperatura/frequência ao vivo; modelo vai para o chip Processador
  setTextTemp("#dp-cpu-temperatura", snapshot.cpuTempC);

  // Memória e armazenamento
  renderMemoria(snapshot.memory);
  renderStorage(snapshot.storage);

  // chip "Sistema operacional" guarda arquitetura/Mesa/uptime como tooltip
  const chipDistro = document.getElementById("dp-chip-distro");
  const chipBox = document.getElementById("dp-chip-distro-box");
  if (chipDistro) {
    chipDistro.textContent = snapshot.distro?.name ?? INDISPONIVEL;
    const extras = [
      snapshot.distro?.version ? `Versão: ${snapshot.distro.version}` : null,
      snapshot.arch ? `Arquitetura: ${snapshot.arch}` : null,
      snapshot.mesaVersion?.trim() ? `Mesa ${snapshot.mesaVersion.trim().replace(/^Mesa\s*/i, "")}` : null,
      snapshot.uptimeSeconds && snapshot.uptimeSeconds > 0 ? `Ativo por ${formatarUptime(snapshot.uptimeSeconds)}` : null,
    ].filter(Boolean) as string[];
    if (chipBox) chipBox.title = extras.join(" • ");
  }
  setText("#dp-chip-kernel", snapshot.kernel ?? null);
  setText("#dp-chip-cpu", snapshot.cpu?.model ?? null);
  setText("#dp-chip-board", snapshot.motherboard ?? null);

  const gpu = snapshot.gpus[0] ?? null;
  setText("#dp-chip-gpu", gpu?.deviceName ?? gpu?.vendorName ?? (snapshot.gpus.length ? INDISPONIVEL : "Nenhuma GPU identificada"));
  const driverChip = document.getElementById("dp-chip-gpu-driver");
  if (driverChip) {
    driverChip.textContent = gpu?.driverVersion
      ? gpu.driver === "nvidia"
        ? `Driver de gráficos ${gpu.driverVersion}`
        : `Driver de gráficos ${gpu.driverVersion} (Mesa)`
      : gpu?.driver
        ? `Driver de gráficos ${gpu.driver} — versão não informada`
        : "Driver de gráficos não disponível";
  }
}

function setTextTemp(selector: string, celsius: number | null | undefined): void {
  setText(selector, celsius != null ? temperatura(celsius) : null);
}

function renderStorage(storage: StorageDevice[]): void {
  const container = document.getElementById("dp-storage");
  if (!container) return;
  container.innerHTML = "";
  if (!storage || storage.length === 0) {
    const vazio = document.createElement("p");
    vazio.className = "muted";
    vazio.textContent = `${INDISPONIVEL} — nenhuma unidade de armazenamento detectada`;
    container.appendChild(vazio);
    return;
  }
  storage.forEach((d) => {
    const usada = d.usedBytes ?? null;
    const total = d.totalBytes ?? d.sizeBytes ?? null;
    const pct = usada != null && total != null && total > 0
      ? Math.max(0, Math.min(100, (usada / total) * 100))
      : null;

    const bloco = document.createElement("div");
    bloco.className = "storage-block";

    const topo = document.createElement("div");
    topo.className = "storage-block__topo";

    const icon = document.createElement("span");
    icon.className = "storage-block__icon";
    icon.setAttribute("aria-hidden", "true");
    icon.innerHTML = `<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="3" width="16" height="18" rx="2"/><path d="M8 7h8M8 11h8M8 15h5"/><circle cx="15.5" cy="16.5" r="1.5"/></svg>`;

    const info = document.createElement("div");
    info.className = "storage-block__info";
    const rotulo = d.kind ? `${d.kind} (${d.name})` : d.name;
    const sub = usada != null && total != null && total > 0
      ? `${formatarBytes(usada) ?? INDISPONIVEL} / ${formatarBytes(total) ?? INDISPONIVEL}`
      : total != null
        ? formatarBytes(total) ?? INDISPONIVEL
        : INDISPONIVEL;
    info.innerHTML = `
      <span class="storage-block__name">${esc(rotulo)}${d.model ? ` <span class="storage-block__model">${esc(d.model)}</span>` : ""}</span>
      <span class="storage-block__sub num">${esc(sub)}</span>
    `;

    const pctEl = document.createElement("span");
    pctEl.className = "storage-block__pct num";
    pctEl.textContent = pct != null ? `${pct.toFixed(0)}%` : INDISPONIVEL;

    topo.append(icon, info, pctEl);

    const bar = document.createElement("div");
    bar.className = "bar bar--storage";
    const fill = document.createElement("div");
    fill.className = "bar__fill";
    if (pct != null) fill.style.width = `${pct.toFixed(1)}%`;
    bar.appendChild(fill);

    bloco.append(topo, bar);
    container.appendChild(bloco);
  });
}

function formatarBytes(bytes: number | null | undefined): string | null {
  if (bytes == null || bytes <= 0) return null;
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 100 || unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function renderMemoria(mem?: MemoryInfo | null): void {
  // valores grandes são os mesmos que o amostrador live escreve (cartão RAM | Swap)
  const total = mem?.totalKb ?? null;
  const usada = mem?.usedKb ?? null;
  setText("dp-ram-uso", usada != null && total != null ? `${gib(usada)} / ${gib(total)}` : INDISPONIVEL);
  setText("dp-ram-disp", mem?.availableKb != null ? gib(mem.availableKb) : null);
  const pct = usada != null && total != null && total > 0 ? (usada / total) * 100 : null;
  setText("dp-ram-pct", pct != null ? `${pct.toFixed(0)}%` : INDISPONIVEL);

  if (mem?.swapTotalKb != null && mem.swapTotalKb > 0) {
    setText("dp-swap-uso",
      mem.swapUsedKb != null ? `${gib(mem.swapUsedKb)} / ${gib(mem.swapTotalKb)}` : INDISPONIVEL);
    setText("dp-swap-disp",
      mem.swapUsedKb != null ? gib(Math.max(0, mem.swapTotalKb - mem.swapUsedKb)) : null);
    const swapPct =
      mem.swapUsedKb != null ? (mem.swapUsedKb / mem.swapTotalKb) * 100 : null;
    setText("dp-swap-pct", swapPct != null ? `${swapPct.toFixed(0)}%` : INDISPONIVEL);
  } else {
    setText("dp-swap-uso", mem?.swapTotalKb != null ? "Sem swap ativo" : INDISPONIVEL);
    setText("dp-swap-disp", null);
    setText("dp-swap-pct", mem?.swapTotalKb != null ? "Sem swap ativo" : INDISPONIVEL);
  }
}



export function classificarTemperatura(c: number): { classe: string; texto: string } {
  if (c >= 75) return { classe: "critical", texto: "Crítica" };
  if (c >= 60) return { classe: "hot", texto: "Atenção" };
  return { classe: "ok", texto: "Normal" };
}

const detalhesPromiseCache = new Map<string, Promise<AppDetail | null>>();

function detalhePromise(appId: string): Promise<AppDetail | null> {
  const cached = detalhesPromiseCache.get(appId);
  if (cached) return cached;
  const promise = invoke<AppDetail | null>("get_app_detail", { appId }).catch(() => null);
  detalhesPromiseCache.set(appId, promise);
  return promise;
}

/** reinícios de estado (instalação/remoção) invalidam o cache de detalhes no frontend */
export function invalidarDetalhes(): void {
  detalhesPromiseCache.clear();
}

/** acesso ao cache de detalhes compartilhado (cards × janela de gerenciamento) */
export function detalheDe(appId: string): Promise<AppDetail | null> {
  return detalhePromise(appId);
}

/** resultado de revalidação em segundo plano alimenta cards e janela */
export function armazenarDetalhe(appId: string, d: AppDetail): void {
  detalhesPromiseCache.set(appId, Promise.resolve(d));
}

function rotuloFonte(fonte: string): string {
  if (fonte === "flatpak") return "Flatpak";
  if (fonte === "flatpak-user") return "Flatpak (usuário)";
  if (fonte.startsWith("snap:")) return `Snap Store (${fonte.split(":")[1]})`;
  if (fonte.startsWith("nativo:")) return `Nativo (${fonte.split(":")[1]})`;
  return fonte;
}

async function popularizarConteudo(tile: HTMLElement, entry: AppEntry): Promise<void> {
  const appId = tile.dataset.appId ?? "";
  if (!appId) return;
  const detalhe = await detalhePromise(appId);
  const rodape = tile.querySelector<HTMLElement>(".app-card__rodape");
  if (!rodape || !detalhe) return;
  const algumInstalado = detalhe.fontes.some((f) => f.instalado);
  const fonteInstalada = detalhe.fontes.find((f) => f.instalado) ?? null;
  const fontesOfertaveis = detalhe.fontes.filter((f) => f.disponivel && !f.instalado);
  const semOpcoes = !algumInstalado && fontesOfertaveis.length === 0;
  // estado gui: no modal, "Abrir" só aparece com fonte instalada
  const guiAberta = Boolean(detalhe.gui) && algumInstalado && Boolean(fonteInstalada);
  const temAtualizacao =
    Boolean(fonteInstalada && fonteInstalada.versaoRepo && fonteInstalada.versaoRepo !== fonteInstalada.versao);

  // registro de atualizações reais (usado pelo indicador e pelo popup "Atualizações")
  if (temAtualizacao && fonteInstalada) {
    atualizacoesMap.set(appId, {
      nome: entry.name,
      fonte: fonteInstalada.fonte,
      versao: fonteInstalada.versao ?? "",
      versaoRepo: fonteInstalada.versaoRepo ?? "",
    });
  } else {
    atualizacoesMap.delete(appId);
  }
  comporContadorAtualizacoes();

  const btnPrincipal = rodape.querySelector<HTMLButtonElement>(
    ".app-card__controles .btn--primary:not(.fonte-dropdown-btn)",
  );
  const wrapper = rodape.querySelector<HTMLElement>("[data-fonte-wrapper]");
  const menu = tile.querySelector<HTMLElement>("[data-fonte-menu]");
  const chooser = wrapper?.querySelector<HTMLButtonElement>("[data-fonte-chooser]");

  if (wrapper) {
    wrapper.style.display = !algumInstalado && fontesOfertaveis.length > 1 ? "inline-block" : "none";
  }
  if (chooser && menu) {
    chooser.addEventListener("click", () => {
      const menuFechado = menu.toggleAttribute("hidden");
      chooser.setAttribute("aria-expanded", menuFechado ? "false" : "true");
    });
  }
  if (menu && fontesOfertaveis.length > 1) {
    menu.innerHTML = "";
    fontesOfertaveis.forEach((f) => {
      const item = document.createElement("button");
      item.type = "button";
      item.className = "fonte-dropdown-menu__item";
      item.dataset.fonte = f.fonte;
      item.textContent = rotuloFonte(f.fonte);
      item.addEventListener("click", () => {
        if (btnPrincipal) btnPrincipal.dataset.fonte = f.fonte;
        const label = wrapper?.querySelector<HTMLElement>("[data-fonte-label]");
        if (label) label.textContent = rotuloFonte(f.fonte);
        if (menu) menu.setAttribute("hidden", "");
      });
      menu.appendChild(item);
    });
  }

  // ação principal contextual: Instalar / Atualizar / Abrir conforme o estado real
  if (btnPrincipal) {
    if (!algumInstalado && fontesOfertaveis.length > 0) {
      btnPrincipal.textContent = "Instalar";
      btnPrincipal.dataset.appAcao = "instalar";
      btnPrincipal.dataset.fonte = fontesOfertaveis[0]?.fonte ?? "";
      btnPrincipal.style.display = "inline-flex";
    } else if (algumInstalado && temAtualizacao) {
      btnPrincipal.textContent = "Atualizar";
      btnPrincipal.dataset.appAcao = "atualizar";
      btnPrincipal.dataset.fonte = fonteInstalada?.fonte ?? "";
      btnPrincipal.style.display = "inline-flex";
    } else if (algumInstalado && guiAberta) {
      btnPrincipal.textContent = "Abrir";
      btnPrincipal.dataset.appAcao = "abrir";
      btnPrincipal.style.display = "inline-flex";
    } else {
      btnPrincipal.style.display = "none";
    }
  }

  const status = tile.querySelector<HTMLElement>(".app-card__status");
  if (status) {
    status.textContent = semOpcoes
      ? detalhe.fontes.some((f) => f.indeterminado)
        ? "Não foi possível verificar as fontes agora (falha temporária) — abra o gerenciamento para tentar novamente."
        : "Não encontrado em nenhuma fonte compatível com este sistema."
      : "";
  }
}

function renderTiles(
  entries: AppEntry[],
  gridId: string,
  emptyMsg: string,
  msgSemResultado: string,
  termo: string = "",
): void {
  const grid = document.getElementById(gridId);
  if (!grid) return;
  grid.innerHTML = "";
  if (!entries || entries.length === 0) {
    grid.innerHTML = `<p class="muted apps-grid__vazio">${esc(termo.trim() ? msgSemResultado : emptyMsg)}</p>`;
    return;
  }
  // ferramentas de compatibilidade mantêm apenas o fluxo próprio (modal "Adicionar"/detalhe)
  const comAcoes = gridId !== "ov-compat";
  entries.forEach((entry, i) => {
    const estado = entry.estado ?? (entry.available ? "instalado" : "nao_instalado");
    const rotuloEstado =
      estado === "instalado"
        ? "Instalado"
        : estado === "desconhecido"
          ? "Verificação indisponível"
          : "Não instalado";
    const appId = esc(entry.id ?? "");
    const tile = document.createElement("article");
    tile.className = "app-card";
    tile.dataset.appId = entry.id ?? "";
    tile.setAttribute("role", "listitem");
    const iconHtml = entry.icon
      ? `<span class="app-card__icon" aria-hidden="true"><img src="${esc(entry.icon)}" alt=""></span>`
      : `<span class="app-card__icon app-card__icon--grad" aria-hidden="true">${esc(inicial(entry.name, i))}</span>`;
    const versionHtml =
      estado === "instalado" && entry.version
        ? `<span class="app-card__versao num">${esc(entry.version)}</span>`
        : "";
    // selo de estado do próprio card, uniforme no canto inferior direito
    const seloEstado = `<span class="pill ${estado === "instalado" ? "pill--ok" : "pill--warn"}">${esc(rotuloEstado)}</span>`;
    const controlesHtml = comAcoes
      ? `
        <button class="btn btn--primary" type="button" data-app-id="${appId}" data-fonte="" hidden></button>
        <div class="fonte-dropdown-wrapper" data-fonte-wrapper hidden>
          <button type="button" class="btn btn--primary fonte-dropdown-btn" data-fonte-chooser="1" data-app-id="${appId}" aria-haspopup="true" aria-expanded="false">
            <span data-fonte-label>Flatpak</span>
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>
          </button>
          <div class="fonte-dropdown-menu" data-fonte-menu hidden></div>
        </div>
      `
      : "";
    tile.innerHTML = `
      <div class="app-card__topo">
        ${iconHtml}
        <div class="app-card__titulos">
          <h3 class="app-card__nome">${esc(entry.name)}${versionHtml}</h3>
        </div>
      </div>
      <p class="app-card__desc">${esc(entry.description)}</p>
      <div class="app-card__rodape">
        <p class="field__status app-card__status" aria-live="polite"></p>
        <div class="app-card__controles">${controlesHtml}</div>
        ${seloEstado}
      </div>
    `;
    grid.appendChild(tile);
    if (comAcoes) {
      popularizarConteudo(tile, entry).catch(() => {
        /* falha de detalhe não deve interromper a renderização do catálogo */
      });
    }
  });
}

function inicial(nome: string, indice: number): string {
  const chave = nome.trim();
  if (!chave) return "?";
  if (indice % 2 === 0) return chave.charAt(0).toUpperCase();
  return chave.charAt(0).toLowerCase();
}

export function formatarUptime(segundos: number): string {
  const h = Math.floor(segundos / 3600);
  const m = Math.floor((segundos % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${segundos}s`;
}

export function renderKernels(snapshot: SystemSnapshot): void {
  setText("#kd-distro", snapshot.distro?.name ?? null);
  setText("#kd-familia", snapshot.distro?.family ?? null);
  setText("#kd-versao-distribuicao", snapshot.distro?.version ?? null);
  setText("#kd-kernel", snapshot.kernel ?? null);
  setText("#kd-arch", snapshot.arch ?? null);

  if (snapshot.cpu) {
    setText("#kd-cpu-modelo", snapshot.cpu.model ?? null);
    setText("#kd-cpu-fisicos", contagem(snapshot.cpu.coresPhysical));
    setText("#kd-cpu-threads", contagem(snapshot.cpu.threadsLogical));
    setText("#kd-cpu-frequencia", mhz(snapshot.cpu.baseFreqMhz));
  }

  const container = document.getElementById("kd-gpu-lista");
  if (container) {
    container.innerHTML = "";
    const gpus = snapshot.gpus;
    if (gpus.length === 0) {
      container.appendChild(empty("Nenhuma GPU identificada"));
    } else {
      gpus.forEach((gpu) => {
        container.appendChild(cardGpu(gpu));
      });
    }
  }

  const vulkanContainer = document.getElementById("kd-vulkan-lista");
  if (vulkanContainer) {
    vulkanContainer.innerHTML = "";
    const vulkan = snapshot.vulkan;
    if (!vulkan || vulkan.gpus.length === 0) {
      vulkanContainer.appendChild(
        alerta("vulkaninfo não disponível no sistema — suporte Vulkan não pôde ser consultado"),
      );
    } else {
      vulkan.gpus.forEach((v) => {
        const item = document.createElement("li");
        item.innerHTML = `
          <span class="kv"><dt>Dispositivo</dt><dd>${esc(v.deviceName)}</dd></span>
          <span class="kv"><dt>Driver</dt><dd>${esc(v.driverName ?? INDISPONIVEL)}</dd></span>
        `;
        vulkanContainer.appendChild(item);
      });
    }
  }

  const ferramentasContainer = document.getElementById("kd-ferramentas-lista");
  if (ferramentasContainer) {
    ferramentasContainer.innerHTML = "";
    (snapshot.tools ?? []).forEach((tool) => {
      const li = document.createElement("li");
      li.className = "kv-lista-item";
      li.innerHTML = `
        <span class="kv"><dt>${esc(tool.name)}</dt><dd><span class="pill ${
          tool.available ? "pill--ok" : "pill--info"
        }">${tool.available ? "Instalada" : "Não instalada"}</span><span style="margin-left: 8px; color: var(--text-dim); font-size: var(--text-aux);">${esc(tool.description)}</span></dd></span>
      `;
      ferramentasContainer.appendChild(li);
    });
  }
}

function contagem(n?: number | null): string {
  return n == null || n <= 0 ? INDISPONIVEL : String(n);
}

function empty(message: string): HTMLElement {
  const item = document.createElement("p");
  item.className = "muted";
  item.textContent = message;
  return item;
}

function alerta(message: string): HTMLElement {
  const item = document.createElement("div");
  item.className = "alert alert--warn";
  item.textContent = message;
  return item;
}

function cardGpu(gpu: GpuInfo): HTMLElement {
  const card = document.createElement("div");
  card.className = "card";
  card.innerHTML = `
    <div class="card-head"><span class="pill pill--info">${esc(gpu.vendorName ?? INDISPONIVEL)}</span></div>
    <dl class="kv">
      <dt>Modelo</dt><dd>${esc(gpu.deviceName ?? INDISPONIVEL)}</dd>
      <dt>Endereço PCI</dt><dd class="num">${esc(gpu.pciAddress ?? INDISPONIVEL)}</dd>
      <dt>IDs</dt><dd class="num">${esc(gpu.vendorId ?? "?")}:${esc(gpu.deviceId ?? "?")}</dd>
      <dt>Driver</dt><dd class="num">${esc(gpu.driver ?? INDISPONIVEL)}</dd>
    </dl>
  `;
  return card;
}

function setText(selector: string, value: string | null | undefined): void {
  const element = document.querySelector(selector);
  if (!element) return;
  element.textContent = value ?? INDISPONIVEL;
}

function esc(value: string | null | undefined): string {
  if (value == null) return INDISPONIVEL;
  const div = document.createElement("div");
  div.textContent = value;
  return div.innerHTML;
}
