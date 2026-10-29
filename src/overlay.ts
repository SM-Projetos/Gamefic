// aba Overlay — gerenciador próprio de configuração do MangoHud.
// Lê/formulário → grava ~/.local/... via comandos overlay_*; avisa quando o
// MangoHud não está instalado (nada é instalado automaticamente). O "ativo"
// (MANGOHUD=1 ao iniciar jogos de dentro do Gamefic) fica nas preferências.
import { invoke } from "@tauri-apps/api/core";
import { loadPrefs, savePrefs, type Prefs } from "./prefs";

// ---------- tipos espelhando src-tauri/src/system/mangohud.rs ----------
interface Metricas {
  fps: boolean;
  frametime: boolean;
  cpuStats: boolean;
  gpuStats: boolean;
  cpuTemp: boolean;
  gpuTemp: boolean;
  vram: boolean;
  ram: boolean;
  coreLoad: boolean;
  cpuPower: boolean;
  gpuPower: boolean;
  gpuCoreClock: boolean;
  gpuMemClock: boolean;
  throttlingStatus: boolean;
  noDisplay: boolean;
}
interface Aparencia {
  posicao: string;
  tamanhoFonte: number;
  alphaFundo: number;
  corTexto: string;
  corGpu: string;
  corCpu: string;
  corVram: string;
  corRam: string;
  horizontal: boolean;
  titulo: string | null;
  hudCompact: boolean;
}
interface Graficos {
  frame_timing: boolean;
  graficos: string[];
}
interface Atalhos {
  toggleHud: string | null;
  toggleHudPosition: string | null;
  toggleFpsLimit: string | null;
  toggleLogging: string | null;
  reloadCfg: string | null;
}
interface ConfigMangoHud {
  metricas: Metricas;
  aparencia: Aparencia;
  graficos: Graficos;
  atalhos: Atalhos;
}
interface Carregado {
  config: ConfigMangoHud;
  preservadas: number;
  // Fase 3: o arquivo `<perfil>.conf` existe? e quantos campos herdam
  // do global (linhas comentadas = herdam a preferência global)
  perfilExiste: boolean;
  camposHerdados: number;
}
interface Estado {
  instalado: boolean;
  perfil: string;
  confCaminho: string;
  confExiste: boolean;
}

const CHAVES_METRICAS: Array<{ chave: keyof Metricas; rotulo: string; ajuda?: string }> = [
  { chave: "fps", rotulo: "FPS" },
  { chave: "frametime", rotulo: "Frametime (ms por quadro)" },
  { chave: "cpuStats", rotulo: "Uso de CPU" },
  { chave: "gpuStats", rotulo: "Uso de GPU" },
  { chave: "cpuTemp", rotulo: "Temperatura da CPU" },
  { chave: "gpuTemp", rotulo: "Temperatura da GPU" },
  { chave: "vram", rotulo: "VRAM" },
  { chave: "ram", rotulo: "RAM" },
  { chave: "coreLoad", rotulo: "Carga por núcleo" },
  { chave: "cpuPower", rotulo: "Consumo da CPU (W)" },
  { chave: "gpuPower", rotulo: "Consumo da GPU (W)" },
  { chave: "gpuCoreClock", rotulo: "Frequência do núcleo da GPU" },
  { chave: "gpuMemClock", rotulo: "Frequência da memória da GPU" },
  { chave: "throttlingStatus", rotulo: "Status de throttling" },
  { chave: "noDisplay", rotulo: "Oculto por padrão", ajuda: "mostra o HUD durante o jogo com o atalho de toggle" },
];

const POSICOES = [
  ["top-left", "Superior à esquerda"],
  ["top-center", "Superior ao centro"],
  ["top-right", "Superior à direita"],
  ["middle-left", "Meio à esquerda"],
  ["middle-center", "Centro"],
  ["middle-right", "Meio à direita"],
  ["bottom-left", "Inferior à esquerda"],
  ["bottom-center", "Inferior ao centro"],
  ["bottom-right", "Inferior à direita"],
] as const;

const CHAVES_GRAFICOS = ["gpu_load", "cpu_load", "gpu_core_clock", "gpu_mem_clock", "vram", "ram", "cpu_temp", "gpu_temp"] as const;

const CHAVES_ATAOS: Array<{ chave: keyof Atalhos; id: string; rotulo: string }> = [
  { chave: "toggleHud", id: "atalho-toggle-hud", rotulo: "Mostrar/ocultar o HUD" },
  { chave: "toggleHudPosition", id: "atalho-toggle-posicao", rotulo: "Mudar a posição" },
  { chave: "toggleFpsLimit", id: "atalho-toggle-limite", rotulo: "Alternar limite de FPS" },
  { chave: "toggleLogging", id: "atalho-toggle-log", rotulo: "Alternar gravação de log" },
  { chave: "reloadCfg", id: "atalho-reload-cfg", rotulo: "Recarregar configuração" },
];

function esc(t: string): string {
  return t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

// estado em memória: usado também por apps.ts ao iniciar um jogo
let configBase: ConfigMangoHud | null = null;
let overlayAtivoAgora = false;
// perfil sendo editado na aba ("global" = arquivo comum do MangoHud,
// válido para todos os jogos; perfil individual = arquivo por processo)
let perfilAtual = "global";

const NOME_PERFIL_RE = /^[a-z0-9_-]{1,48}$/;

export function overlayAtivo(): boolean {
  return overlayAtivoAgora;
}

// ---------- seção de perfis (Fase 3) ----------
function perfisHtml(perfis: string[], dados: Carregado): string {
  const opcoes = ["global", ...perfis.filter((p) => p !== "global")]
    .map(
      (p) => `<option value="${esc(p)}" ${p === perfilAtual ? "selected" : ""}>${p === "global" ? "global — todos os jogos" : p}</option>`,
    )
    .join("");
  const herdaTexto =
    perfilAtual === "global"
      ? "Este é o arquivo global: vale para todos os jogos que não tenham perfil próprio com o mesmo nome de processo."
      : `${dados.camposHerdados} campo(s) herdam o global (linhas comentadas no arquivo) — mudanças no global se propagam para esses campos. Campos divergentes ficam explícitos no perfil.`;
  return `
    <h2 class="page__section-title">Perfil</h2>
    <div class="field" style="max-width:30rem">
      <label class="field__label" for="ov-perfil">Configuração editada</label>
      <select class="field__control" id="ov-perfil">${opcoes}</select>
      <p class="muted">O perfil é escolhido pelo MangoHud pelo nome do processo do jogo (ex.: <span class="num">wine</span>, o nome do executável do emulador ou do jogo). Um perfil nunca altera o arquivo global.</p>
      <div style="display:flex; gap:var(--space-2); margin-top:var(--space-2)">
        <input type="text" class="field__control" id="ov-perfil-novo" placeholder="novo perfil (a-z, 0-9, _ ou -)" maxlength="48" style="flex:1">
        <button class="btn btn--ghost" id="ov-perfil-criar" type="button">Editar novo</button>
        ${dados.perfilExiste && perfilAtual !== "global" ? `<button class="btn btn--ghost" id="ov-perfil-remover" type="button">Remover perfil</button>` : ""}
      </div>
      <p class="muted" style="margin-top:var(--space-2)">${esc(herdaTexto)}</p>
      ${perfilAtual !== "global" && !dados.perfilExiste ? `<p class="muted">Este perfil ainda não está gravado — use “Salvar configuração” para criá-lo.</p>` : ""}
    </div>`;
}

// ---------- construção do formulário ----------
function switchHtml(id: string, rotulo: string, marcado: boolean, ajuda?: string): string {
  return `
    <div class="field">
      <label class="switch">
        <input type="checkbox" id="${id}" ${marcado ? "checked" : ""}>
        <span class="switch__track"></span>
        <span class="switch__label">${esc(rotulo)}</span>
      </label>
      ${ajuda ? `<p class="muted">${esc(ajuda)}</p>` : ""}
    </div>`;
}

function corHtml(id: string, rotulo: string, valor: string): string {
  return `
    <div class="field">
      <label class="field__label" for="${id}">${esc(rotulo)}</label>
      <input type="color" class="field__control" id="${id}" value="#${esc(valor)}" style="height:2.2rem; padding:.2rem;">
    </div>`;
}

function metricasHtml(m: Metricas): string {
  const lista = CHAVES_METRICAS.map((c) => switchHtml(`ovm-${c.chave}`, c.rotulo, m[c.chave], c.ajuda)).join("");
  return `
    <h2 class="page__section-title">Métricas exibidas</h2>
    <div class="grid">${lista}</div>`;
}

function aparenciaHtml(a: Aparencia): string {
  return `
    <h2 class="page__section-title">Aparência</h2>
    <div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(220px,1fr))">
      <div class="field">
        <label class="field__label" for="ova-posicao">Posição na tela</label>
        <select class="field__control" id="ova-posicao">
          ${POSICOES.map(([v, r]) => `<option value="${v}" ${a.posicao === v ? "selected" : ""}>${esc(r)}</option>`).join("")}
        </select>
      </div>
      <div class="field">
        <label class="field__label" for="ova-titulo">Título exibido no HUD</label>
        <input type="text" class="field__control" id="ova-titulo" value="${esc(a.titulo ?? "")}" placeholder="(sem título)">
        <p class="muted">Texto fixo exibido pelo MangoHud dentro do jogo (chave <span class="num">custom_text</span>); vazio = sem título.</p>
      </div>
      <div class="field">
        <label class="field__label" for="ova-fonte">Tamanho da fonte (8–128)</label>
        <input type="number" min="8" max="128" class="field__control" id="ova-fonte" value="${a.tamanhoFonte}">
      </div>
      <div class="field">
        <label class="field__label" for="ova-alpha">Transparência do fundo (<span id="ova-alpha-valor">${a.alphaFundo.toFixed(2)}</span>)</label>
        <input type="range" min="0" max="1" step="0.05" class="field__control" id="ova-alpha" value="${a.alphaFundo}">
      </div>
    </div>
    <div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(140px,1fr))">
      ${corHtml("ova-cor-texto", "Cor do texto", a.corTexto)}
      ${corHtml("ova-cor-gpu", "Cor da GPU", a.corGpu)}
      ${corHtml("ova-cor-cpu", "Cor da CPU", a.corCpu)}
      ${corHtml("ova-cor-vram", "Cor da VRAM", a.corVram)}
      ${corHtml("ova-cor-ram", "Cor da RAM", a.corRam)}
    </div>
    <div class="grid">
      ${switchHtml("ova-horizontal", "Layout horizontal", a.horizontal)}
      ${switchHtml("ova-hud-compact", "HUD compacto", a.hudCompact)}
    </div>`;
}

function graficosHtml(g: Graficos): string {
  return `
    <h2 class="page__section-title">Gráficos de desempenho</h2>
    <div class="grid">
      ${switchHtml("ovg-frame-timing", "Gráfico de linha do frametime", g.frame_timing)}
    </div>
    <div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(180px,1fr))">
      ${CHAVES_GRAFICOS.map((k) => switchHtml(`ovg-${k}`, k, g.graficos.includes(k))).join("")}
    </div>
    <p class="muted">Gráficos são desenhados no HUD; "vram", "ram" e temperaturas também exigem a métrica correspondente ativa.</p>`;
}

function atalhosHtml(a: Atalhos): string {
  return `
    <h2 class="page__section-title">Atalhos de teclado</h2>
    <p class="muted">Vazio = o MangoHud mantém o padrão do programa. Formato na sintaxe do X11 (ex.: Shift_R+F12); um atalho definido aqui substitui a entrada correspondente do arquivo.</p>
    <div class="grid" style="grid-template-columns:repeat(auto-fit,minmax(220px,1fr))">
      ${CHAVES_ATAOS.map((x) => `<div class="field">
        <label class="field__label" for="${x.id}">${esc(x.rotulo)}</label>
        <input type="text" class="field__control" id="${x.id}" value="${esc(a[x.chave] ?? "")}" placeholder="(padrão do MangoHud)">
      </div>`).join("")}
    </div>`;
}

function secaoAtivacaoHtml(prefs: Prefs): string {
  // a ativação +"MANGOHUD=1" vale para jogos abertos de dentro do Gamefic
  // (Steam, Lutris, Heroic etc.); o arquivo não tem uma chave para isso
  return `
    <h2 class="page__section-title">Ativação</h2>
    <div class="grid">${switchHtml("ovi-ativo", "Overlay ligado nos jogos iniciados pelo Gamefic (MANGOHUD=1)", prefs.mangohudOverlay === true)}</div>
    <p class="muted">Como funciona: a variável <span class="num">MANGOHUD=1</span> é repassada ao launcher aberto pelo Gamefic e os jogos herdam o ambiente dele — Steam nativo e launchers nativos alcançam jogos Vulkan/OpenGL e Proton/DXVK. Em aplicativos Flatpak a variável entra por <span class="num">--env</span> e só aparece o overlay se o runtime incluir o MangoHud. Jogos Direct3D sem DXVK/Proton não exibem o overlay; Wine puro não é iniciado diretamente pelo Gamefic. Para jogos abertos por fora (Steam do sistema), utilize o wrapper <span class="num">mangohud</span> da linha de comando.</p>`;
}

// ---------- leitura / gravação ----------
function lerFormulario(): ConfigMangoHud {
  const base = JSON.parse(JSON.stringify(configBase)) as ConfigMangoHud;
  const bool = (id: string): boolean => document.getElementById(id) instanceof HTMLInputElement
    && (document.getElementById(id) as HTMLInputElement).checked;
  const num = (id: string): number => Number((document.getElementById(id) as HTMLInputElement).value);
  const txt = (id: string): string => (document.getElementById(id) as HTMLInputElement).value;
  for (const c of CHAVES_METRICAS) {
    base.metricas[c.chave] = bool(`ovm-${c.chave}`);
  }
  base.aparencia.posicao = txt("ova-posicao");
  base.aparencia.tamanhoFonte = num("ova-fonte");
  base.aparencia.alphaFundo = num("ova-alpha");
  base.aparencia.corTexto = txt("ova-cor-texto").replace(/^#/, "").toUpperCase();
  base.aparencia.corGpu = txt("ova-cor-gpu").replace(/^#/, "").toUpperCase();
  base.aparencia.corCpu = txt("ova-cor-cpu").replace(/^#/, "").toUpperCase();
  base.aparencia.corVram = txt("ova-cor-vram").replace(/^#/, "").toUpperCase();
  base.aparencia.corRam = txt("ova-cor-ram").replace(/^#/, "").toUpperCase();
  {
    const t = txt("ova-titulo").trim();
    base.aparencia.titulo = t.length > 0 ? t : null;
  }
  base.aparencia.horizontal = bool("ova-horizontal");
  base.aparencia.hudCompact = bool("ova-hud-compact");
  base.graficos.frame_timing = bool("ovg-frame-timing");
  base.graficos.graficos = CHAVES_GRAFICOS.filter((k) => bool(`ovg-${k}`));
  for (const x of CHAVES_ATAOS) {
    const v = txt(x.id).trim();
    base.atalhos[x.chave] = v.length > 0 ? v : null;
  }
  return base;
}

function setStatus(msg: string, tipo: "ok" | "erro"): void {
  const el = document.getElementById("ov-status");
  if (!el) return;
  el.textContent = msg;
  el.className = `field__status ${tipo === "ok" ? "pill pill--ok" : "pill pill--error"}`;
}

async function salvarConfig(): Promise<void> {
  if (!configBase) return;
  const botao = document.getElementById("ov-salvar");
  try {
    if (botao instanceof HTMLButtonElement) botao.disabled = true;
    setStatus("Validando e gravando…", "ok");
    const msg = await invoke<string>("overlay_salvar", {
      perfil: perfilAtual,
      config: lerFormulario(),
    });
    const estado = await invoke<Estado>("overlay_estado", {});
    statusEstado(estado, true);
    setStatus(msg, "ok");
  } catch (e) {
    setStatus(String(e), "erro");
  } finally {
    if (botao instanceof HTMLButtonElement) botao.disabled = false;
  }
}

async function alternarAtivo(marcado: boolean): Promise<void> {
  try {
    const prefs = await loadPrefs();
    prefs.mangohudOverlay = marcado;
    savePrefs(prefs);
    overlayAtivoAgora = marcado;
  } catch (e) {
    console.error("Falha ao salvar a ativação do overlay:", e);
  }
}

function statusEstado(estado: Estado, temEstadoAnterior: boolean): void {
  const alvo = document.getElementById("ov-estado");
  if (!alvo) return;
  if (!estado.instalado) {
    alvo.className = "pill pill--warn";
    alvo.textContent = "MangoHud não detectado no sistema";
    return;
  }
  if (temEstadoAnterior) {
    alvo.className = "pill pill--ok";
    alvo.textContent = `MangoHud presente · config em ${estado.confCaminho}`;
  } else {
    alvo.className = "pill pill--info";
    alvo.textContent = `MangoHud presente · sem arquivo de config (o salvar cria em ${estado.confCaminho})`;
  }
}

// ---------- ponto de entrada ----------
export async function initOverlay(): Promise<void> {
  const conteudo = document.getElementById("overlay-conteudo");
  if (!conteudo) return;
  const fallback = document.getElementById("overlay-nota");
  if (fallback) fallback.hidden = true;

  let estado: Estado;
  try {
    estado = await invoke<Estado>("overlay_estado", {});
  } catch (e) {
    conteudo.innerHTML = `<div class="card"><p class="field__status">Falha ao consultar o estado do MangoHud: ${esc(String(e))}</p></div>`;
    return;
  }

  if (!estado.instalado) {
    conteudo.innerHTML = `
      <div class="card">
        <span class="pill pill--warn">MangoHud não detectado no sistema</span>
        <p style="margin-top:var(--space-3)">Instale o pacote <strong>mangohud</strong> pela distribuição (o Gamefic não instala pacotes por conta própria). A interface de configuração fica disponível assim que o binário <span class="num">mangohud</span> esteja acessível.</p>
        <p class="muted">Sem alterações no sistema foram feitas por esta aba.</p>
      </div>`;
    return;
  }

  await montarOverlay(conteudo, estado);
}

// monta (ou remonta) o formulário com o perfil selecionado no momento;
// chamada na abertura da aba e a cada troca/criação/remoção de perfil
async function montarOverlay(conteudo: HTMLElement, estado: Estado): Promise<void> {
  let dados: Carregado;
  try {
    dados = await invoke<Carregado>("overlay_carregar", { perfil: perfilAtual });
  } catch (e) {
    conteudo.innerHTML = `<div class="card"><p class="field__status">Falha ao ler a configuração: ${esc(String(e))}</p></div>`;
    return;
  }
  configBase = dados.config;

  const prefs = await loadPrefs();
  overlayAtivoAgora = prefs.mangohudOverlay === true;

  let perfis: string[] = [];
  try {
    perfis = await invoke<string[]>("overlay_perfis");
  } catch (e) {
    console.error("Falha ao listar perfis do MangoHud:", e);
  }

  const caminhoGravacao =
    perfilAtual === "global"
      ? estado.confCaminho
      : `o arquivo ${perfilAtual}.conf na pasta do MangoHud`;

  conteudo.innerHTML = `
    <div class="card" style="margin-bottom:var(--space-4)">
      <span class="pill pill--info" id="ov-estado">MangoHud</span>
      <span class="muted">${dados.preservadas > 0 ? `${dados.preservadas} opção(ões) avançadas fora do conjunto gerenciado serão preservadas no arquivo.` : "Opções fora do conjunto gerenciado, se existirem, são preservadas."}</span>
      <div class="field__status" id="ov-status" style="margin-top:var(--space-2)"></div>
    </div>
    ${perfisHtml(perfis, dados)}
    ${secaoAtivacaoHtml(prefs)}
    ${metricasHtml(dados.config.metricas)}
    ${aparenciaHtml(dados.config.aparencia)}
    ${graficosHtml(dados.config.graficos)}
    ${atalhosHtml(dados.config.atalhos)}
    <div style="margin-top:var(--space-4); display:flex; gap:var(--space-3); align-items:center">
      <button class="btn btn--primary" id="ov-salvar" type="button">Salvar configuração</button>
      <span class="muted">Grava em ${esc(caminhoGravacao)}</span>
    </div>`;

  statusEstado(estado, estado.confExiste);
  const alpha = document.getElementById("ova-alpha") as HTMLInputElement | null;
  const alphaValor = document.getElementById("ova-alpha-valor");
  alpha?.addEventListener("input", () => {
    if (alphaValor) alphaValor.textContent = Number(alpha.value).toFixed(2);
  });
  document.getElementById("ovi-ativo")?.addEventListener("change", (ev) => {
    const alvo = ev.target as HTMLInputElement;
    void alternarAtivo(alvo.checked);
  });
  document.getElementById("ov-salvar")?.addEventListener("click", () => void salvarConfig());
  document.getElementById("ov-perfil")?.addEventListener("change", (ev) => {
    perfilAtual = (ev.target as HTMLSelectElement).value;
    void montarOverlay(conteudo, estado);
  });
  document.getElementById("ov-perfil-criar")?.addEventListener("click", () => {
    const campo = document.getElementById("ov-perfil-novo") as HTMLInputElement | null;
    const nome = campo?.value.trim() ?? "";
    if (!NOME_PERFIL_RE.test(nome)) {
      setStatus("Nome de perfil inválido — use 1–48 caracteres de a-z, 0-9, _ ou -", "erro");
      return;
    }
    perfilAtual = nome;
    void montarOverlay(conteudo, estado);
  });
  document.getElementById("ov-perfil-remover")?.addEventListener("click", () => {
    if (!window.confirm(`Remover o perfil "${perfilAtual}"? Jogos com esse nome de processo voltarão a usar o arquivo global.`)) {
      return;
    }
    const nome = perfilAtual;
    void invoke<string>("overlay_remover_perfil", { perfil: nome })
      .then(async (msg) => {
        perfilAtual = "global";
        await montarOverlay(conteudo, estado);
        setStatus(msg, "ok");
      })
      .catch((e) => setStatus(String(e), "erro"));
  });
}
