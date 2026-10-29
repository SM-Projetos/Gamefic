//! Gerenciador próprio de configuração do MangoHud (aba Overlay).
//!
//! Lê, edita e grava o arquivo `MangoHud.conf` respeitando a sintaxe
//! documentada pelo projeto MangoHud (gabarito `data/MangoHud.conf`
//! do upstream):
//! - opção ativada    = linha livre `chave` ou `chave=valor`;
//! - opção desativada = linha comentada `# chave=...`.
//! O Gamefic gerencia UM CONJUNTO FIXO de chaves; qualquer outra linha
//! (chaves avançadas, comentários do usuário) é preservada verbatim no
//! lugar original. O bloco gerenciado é regenerado no fim do arquivo e
//! as marcas antigas do Gamefic são recortadas para não duplicar.
//!
//! Não executa comandos privilegiados e não instala pacotes: aqui só
//! existe leitura/validação/gravação de arquivo do próprio usuário e
//! verificação da presença do binário `mangohud` (mesmo critério de
//! `tools.rs`, via `which`, sem shell).
//!
//! Perfis: além do arquivo global, cada aplicativo pode ter uma
//! configuração própria em `MangoHud/<nome-do-processo>.conf` — é a
//! convenção do próprio MangoHud (o Vulkan/OpenGL layer busca a
//! configuração do processo executando antes do global; linha comentada
//! no perfil = campo herdado do global). Perfis válidos: 1–48 caracteres
//! `a-z`, `0-9`, `_`, `-` (não alteram o global ao serem gravados).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// único perfil implementado nesta fase
pub const PERFIL_GLOBAL: &str = "global";

/// marca que identifica os blocos escritos pelo Gamefic (recortados na
/// próxima regeneração para o arquivo não crescer indefinidamente)
pub const MARCA_INI: &str = "### [gamefic] inicio das opcoes gerenciadas";
pub const MARCA_FIM: &str = "### [gamefic] fim das opcoes gerenciadas";

pub const POSICOES: &str = "top-left,top-right,top-center,middle-left,middle-center,middle-right,bottom-left,bottom-right,bottom-center";
pub const GRAFICOS_DISPONIVEIS: &str =
    "gpu_load,cpu_load,gpu_core_clock,gpu_mem_clock,vram,ram,cpu_temp,gpu_temp";

// ---------------------------------------------------------------------------
// modelo das configurações (serde camelCase para a interface)

/// métricas exibidas no overlay (ligado/desligado como no MangoHud.conf;
/// os padrões seguem o gabarito upstream: fps/frametime/cpu/gpu ativos)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Metricas {
    pub fps: bool,
    pub frametime: bool,
    pub cpu_stats: bool,
    pub gpu_stats: bool,
    pub cpu_temp: bool,
    pub gpu_temp: bool,
    pub vram: bool,
    pub ram: bool,
    pub core_load: bool,
    pub cpu_power: bool,
    pub gpu_power: bool,
    pub gpu_core_clock: bool,
    pub gpu_mem_clock: bool,
    pub throttling_status: bool,
    /// HUD oculto por padrão (mostrar com o atalho de toggle)
    pub no_display: bool,
}

impl Default for Metricas {
    fn default() -> Self {
        Metricas {
            fps: true,
            frametime: true,
            cpu_stats: true,
            gpu_stats: true,
            cpu_temp: false,
            gpu_temp: false,
            vram: false,
            ram: false,
            core_load: false,
            cpu_power: false,
            gpu_power: false,
            gpu_core_clock: false,
            gpu_mem_clock: false,
            throttling_status: true,
            no_display: false,
        }
    }
}

/// aparência: posição, tamanho, cores, transparência e orientação.
/// cores em hex RRGGBB (sintaxe do MangoHud, sem `#`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Aparencia {
    pub posicao: String,
    pub tamanho_fonte: u32,
    /// 0.0 (invisível) a 1.0 (opaco)
    pub alpha_fundo: f32,
    pub cor_texto: String,
    pub cor_gpu: String,
    pub cor_cpu: String,
    pub cor_vram: String,
    pub cor_ram: String,
    /// layout horizontal (linha única)
    pub horizontal: bool,
    /// título exibido no HUD (chave `custom_text`); None = não gerencia
    pub titulo: Option<String>,
    /// versão compacta do HUD
    pub hud_compact: bool,
}

impl Default for Aparencia {
    fn default() -> Self {
        // valores do gabarito upstream
        Aparencia {
            posicao: "top-left".to_string(),
            tamanho_fonte: 24,
            alpha_fundo: 0.5,
            cor_texto: "FFFFFF".to_string(),
            cor_gpu: "2E9762".to_string(),
            cor_cpu: "2E97CB".to_string(),
            cor_vram: "AD64C1".to_string(),
            cor_ram: "C26693".to_string(),
            horizontal: false,
            // título padrão do HUD: Gamefic (chave documentada `custom_text`)
            titulo: Some("Gamefic".to_string()),
            hud_compact: false,
        }
    }
}

/// gráficos de desempenho: linha contínua (`graphs=...`) e gráfico de
/// frametime (`frame_timing`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Graficos {
    pub frame_timing: bool,
    /// chaves aceitas pelo MangoHud, separadas por vírgula
    pub graficos: Vec<String>,
}

impl Default for Graficos {
    fn default() -> Self {
        Graficos {
            frame_timing: true,
            graficos: Vec::new(),
        }
    }
}

/// atalhos de teclado. `Some("")`/vazio é tratado como "não gerenciar"
/// (o escritor regrava a linha comentada, preservando o padrão do MangoHud)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Atalhos {
    pub toggle_hud: Option<String>,
    pub toggle_hud_position: Option<String>,
    pub toggle_fps_limit: Option<String>,
    pub toggle_logging: Option<String>,
    pub reload_cfg: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ConfigMangoHud {
    pub metricas: Metricas,
    pub aparencia: Aparencia,
    pub graficos: Graficos,
    pub atalhos: Atalhos,
}

// ---------------------------------------------------------------------------
// parser (função pura, testável)

/// resultado da leitura de uma linha do conf
struct LinhaConf<'a> {
    chave: &'a str,
    /// `true` quando a opção está comentada (desativada)
    comentada: bool,
    valor: Option<&'a str>,
}

/// decompõe uma linha na forma `chave[=valor]`, com ou sem comentário.
/// Não casa textos à esquerda de outras palavras: depois de tirar o `#`,
/// a linha tem que COMEÇAR com a chave seguida de `=`, fim de linha ou
/// espaço (comentários de documentação do gabarito não são opções).
fn linha_conf(linha: &str) -> Option<LinhaConf<'_>> {
    let mut comentada = false;
    let mut corpo = linha.trim_start();
    if let Some(resto) = corpo.strip_prefix('#') {
        comentada = true;
        corpo = resto.trim_start();
    }
    if corpo.is_empty() {
        return None;
    }
    let fim = corpo.find(['=', ' ', '\t']).unwrap_or(corpo.len());
    let chave = &corpo[..fim];
    if chave.is_empty()
        || !chave
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return None;
    }
    let valor = {
        let apos = &corpo[fim..];
        if let Some(v) = apos.strip_prefix('=') {
            let v = v.trim();
            let v = v.split_whitespace().next().unwrap_or("");
            let v = v.trim_matches('"');
            if v.is_empty() { None } else { Some(v) }
        } else {
            None
        }
    };
    Some(LinhaConf {
        chave,
        comentada,
        valor,
    })
}

/// popula o modelo com o que está no texto; o que falta fica no default
pub fn parse_conf(texto: &str) -> (ConfigMangoHud, usize) {
    let mut cfg = ConfigMangoHud::default();
    let mut preservadas = 0usize;
    for linha in texto.lines() {
        let Some(l) = linha_conf(linha) else {
            continue;
        };
        let ativa = !l.comentada;
        match l.chave {
            "fps" => cfg.metricas.fps = ativa,
            "frametime" => cfg.metricas.frametime = ativa,
            "cpu_stats" => cfg.metricas.cpu_stats = ativa,
            "gpu_stats" => cfg.metricas.gpu_stats = ativa,
            "cpu_temp" => cfg.metricas.cpu_temp = ativa,
            "gpu_temp" => cfg.metricas.gpu_temp = ativa,
            "vram" => cfg.metricas.vram = ativa,
            "ram" => cfg.metricas.ram = ativa,
            "core_load" => cfg.metricas.core_load = ativa,
            "cpu_power" => cfg.metricas.cpu_power = ativa,
            "gpu_power" => cfg.metricas.gpu_power = ativa,
            "gpu_core_clock" => cfg.metricas.gpu_core_clock = ativa,
            "gpu_mem_clock" => cfg.metricas.gpu_mem_clock = ativa,
            "throttling_status" => cfg.metricas.throttling_status = ativa,
            "no_display" => cfg.metricas.no_display = ativa,
            "position" => {
                if let Some(v) = l.valor {
                    cfg.aparencia.posicao = v.to_string();
                }
            }
            "font_size" => {
                if let Some(v) = l.valor.and_then(|v| v.parse().ok()) {
                    cfg.aparencia.tamanho_fonte = v;
                }
            }
            "background_alpha" => {
                if let Some(v) = l.valor.and_then(|v| v.parse().ok()) {
                    cfg.aparencia.alpha_fundo = v;
                }
            }
            "text_color" => {
                if let Some(v) = l.valor {
                    cfg.aparencia.cor_texto = v.to_string();
                }
            }
            "gpu_color" => {
                if let Some(v) = l.valor {
                    cfg.aparencia.cor_gpu = v.to_string();
                }
            }
            "cpu_color" => {
                if let Some(v) = l.valor {
                    cfg.aparencia.cor_cpu = v.to_string();
                }
            }
            "vram_color" => {
                if let Some(v) = l.valor {
                    cfg.aparencia.cor_vram = v.to_string();
                }
            }
            "ram_color" => {
                if let Some(v) = l.valor {
                    cfg.aparencia.cor_ram = v.to_string();
                }
            }
            "horizontal" => cfg.aparencia.horizontal = ativa,
            "hud_compact" => cfg.aparencia.hud_compact = ativa,
            "custom_text" => {
                cfg.aparencia.titulo = if ativa {
                    l.valor
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                } else {
                    None
                };
            }
            // variante centralizada (usada pelo GOverlay); entra no mesmo
            // campo "título" na interface
            "custom_text_center" => {
                cfg.aparencia.titulo = if ativa {
                    l.valor
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                } else {
                    None
                };
            }
            "frame_timing" => cfg.graficos.frame_timing = ativa,
            "graphs" => {
                if !ativa {
                    continue;
                }
                if let Some(v) = l.valor {
                    cfg.graficos.graficos = v
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                }
            }
            "toggle_hud" => {
                if ativa {
                    cfg.atalhos.toggle_hud = l.valor.map(str::to_string);
                }
            }
            "toggle_hud_position" => {
                if ativa {
                    cfg.atalhos.toggle_hud_position = l.valor.map(str::to_string);
                }
            }
            "toggle_fps_limit" => {
                if ativa {
                    cfg.atalhos.toggle_fps_limit = l.valor.map(str::to_string);
                }
            }
            "toggle_logging" => {
                if ativa {
                    cfg.atalhos.toggle_logging = l.valor.map(str::to_string);
                }
            }
            "reload_cfg" => {
                if ativa {
                    cfg.atalhos.reload_cfg = l.valor.map(str::to_string);
                }
            }
            _ => {
                // opção avançada desconhecida: preservada pelo reconstrutor
                preservadas += 1;
            }
        }
    }
    (cfg, preservadas)
}

// ---------------------------------------------------------------------------
// validação (a interface NÃO é fronteira de segurança: valide no Rust)

fn validar(cfg: &ConfigMangoHud) -> Result<(), String> {
    let a = &cfg.aparencia;
    let mut erros: Vec<String> = Vec::new();
    if !POSICOES.split(',').any(|p| p == a.posicao) {
        erros.push(format!(
            "posicao '{}' não é reconhecida pelo MangoHud",
            a.posicao
        ));
    }
    if !(8..=128).contains(&a.tamanho_fonte) {
        erros.push("tamanho da fonte deve ficar entre 8 e 128".to_string());
    }
    if !(0.0..=1.0).contains(&a.alpha_fundo) {
        erros.push("transparência deve ficar entre 0,0 e 1,0".to_string());
    }
    for (nome, cor) in [
        ("texto", &a.cor_texto),
        ("GPU", &a.cor_gpu),
        ("CPU", &a.cor_cpu),
        ("VRAM", &a.cor_vram),
        ("RAM", &a.cor_ram),
    ] {
        if cor.len() != 6 || !cor.bytes().all(|b| b.is_ascii_hexdigit()) {
            erros.push(format!(
                "cor de {nome} deve ter 6 dígitos hexadecimais (ex.: 2E9762)"
            ));
        }
    }
    for g in &cfg.graficos.graficos {
        if !GRAFICOS_DISPONIVEIS.split(',').any(|p| p == g) {
            erros.push(format!(
                "gráfico '{g}' não é suportado (suportados: {GRAFICOS_DISPONIVEIS})"
            ));
        }
    }
    for (nome, atalho) in chaves_atalho(&cfg.atalhos) {
        if let Some(t) = atalho {
            if t.is_empty()
                || t.len() > 40
                || !t
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'_')
            {
                erros.push(format!(
                    "atalho de {nome} parece inválido (ex.: Shift_R+F12)"
                ));
            }
        }
    }
    if erros.is_empty() {
        Ok(())
    } else {
        Err(erros.join("; "))
    }
}

// ---------------------------------------------------------------------------
// reconstrutor do arquivo (preserva o que o Gamefic não gerencia)

fn formato_alpha(v: f32) -> String {
    let s = format!("{v}");
    if s.contains('.') { s } else { format!("{s}.0") }
}

fn chaves_metricas(c: &Metricas) -> Vec<(&'static str, bool)> {
    vec![
        ("fps", c.fps),
        ("frametime", c.frametime),
        ("cpu_stats", c.cpu_stats),
        ("gpu_stats", c.gpu_stats),
        ("cpu_temp", c.cpu_temp),
        ("gpu_temp", c.gpu_temp),
        ("vram", c.vram),
        ("ram", c.ram),
        ("core_load", c.core_load),
        ("cpu_power", c.cpu_power),
        ("gpu_power", c.gpu_power),
        ("gpu_core_clock", c.gpu_core_clock),
        ("gpu_mem_clock", c.gpu_mem_clock),
        ("throttling_status", c.throttling_status),
        ("no_display", c.no_display),
    ]
}

fn chaves_atalho(a: &Atalhos) -> Vec<(&'static str, Option<String>)> {
    vec![
        ("toggle_hud", a.toggle_hud.clone()),
        ("toggle_hud_position", a.toggle_hud_position.clone()),
        ("toggle_fps_limit", a.toggle_fps_limit.clone()),
        ("toggle_logging", a.toggle_logging.clone()),
        ("reload_cfg", a.reload_cfg.clone()),
    ]
}

/// reescreve o texto: linhas gerenciadas antigas (livres ou comentadas) saem
/// do corpo, marcas antigas do Gamefic são recortadas e o bloco gerenciado
/// é anexado no fim. Linhas desconhecidas e comentários do usuário permanecem.
pub fn reconstruir_conf(original: Option<&str>, cfg: &ConfigMangoHud) -> String {
    let mut gerenciadas: Vec<(&'static str, String)> = Vec::new();
    for (k, ativa) in chaves_metricas(&cfg.metricas) {
        // toggle de métrica: linha livre quando ativa, comentada quando não
        gerenciadas.push((
            k,
            if ativa {
                k.to_string()
            } else {
                format!("#{k}")
            },
        ));
    }
    let a = &cfg.aparencia;
    gerenciadas.push(("position", format!("position={}", a.posicao)));
    gerenciadas.push(("font_size", format!("font_size={}", a.tamanho_fonte)));
    gerenciadas.push((
        "background_alpha",
        format!("background_alpha={}", formato_alpha(a.alpha_fundo)),
    ));
    gerenciadas.push(("text_color", format!("text_color={}", a.cor_texto)));
    gerenciadas.push(("gpu_color", format!("gpu_color={}", a.cor_gpu)));
    gerenciadas.push(("cpu_color", format!("cpu_color={}", a.cor_cpu)));
    gerenciadas.push(("vram_color", format!("vram_color={}", a.cor_vram)));
    gerenciadas.push(("ram_color", format!("ram_color={}", a.cor_ram)));
    gerenciadas.push(linha_titulo(
        chave_titulo(original),
        a.titulo.as_deref(),
    ));
    gerenciadas.push((
        "horizontal",
        if a.horizontal {
            "horizontal".into()
        } else {
            "#horizontal".into()
        },
    ));
    gerenciadas.push((
        "hud_compact",
        if a.hud_compact {
            "hud_compact".into()
        } else {
            "#hud_compact".into()
        },
    ));
    gerenciadas.push((
        "frame_timing",
        if cfg.graficos.frame_timing {
            "frame_timing".into()
        } else {
            "#frame_timing".into()
        },
    ));
    if cfg.graficos.graficos.is_empty() {
        gerenciadas.push(("graphs", "#graphs".into()));
    } else {
        gerenciadas.push((
            "graphs",
            format!("graphs={}", cfg.graficos.graficos.join(",")),
        ));
    }
    for (k, v) in chaves_atalho(&cfg.atalhos) {
        match v {
            Some(t) if !t.trim().is_empty() => gerenciadas.push((k, format!("{k}={t}"))),
            _ => gerenciadas.push((k, format!("#{k}"))),
        }
    }

    let chaves: Vec<&str> = gerenciadas.iter().map(|(k, _)| *k).collect();
    let mut corpo: Vec<&str> = Vec::new();
    match original {
        Some(texto) => {
            for linha in texto.lines() {
                let t = linha.trim();
                if t == MARCA_INI || t == MARCA_FIM {
                    continue;
                }
                if let Some(l) = linha_conf(linha) {
                    if chaves.contains(&l.chave) {
                        continue;
                    }
                }
                corpo.push(linha);
            }
            // remove linhas em branco do fim: o bloco novo será anexado ali
            while corpo.last().is_some_and(|l| l.trim().is_empty()) {
                corpo.pop();
            }
        }
        None => {
            // arquivo observado ao abrir uma configuração do Gamefic pela primeira vez:
            // cabeçalho curto após as posições originais
            corpo.push("### MangoHud.conf — editado pelo Gamefic (aba Overlay)");
            corpo.push("### Documentação: https://github.com/flightlessmango/MangoHud");
            corpo.push("### Linhas fora do bloco abaixo são preservadas exatamente como estão.");
        }
    }

    let mut saida = String::new();
    saida.push_str(&corpo.join("\n"));
    saida.push_str("\n\n");
    saida.push_str(MARCA_INI);
    saida.push('\n');
    for (_, linha) in gerenciadas {
        saida.push_str(&linha);
        saida.push('\n');
    }
    saida.push_str(MARCA_FIM);
    saida.push('\n');
    saida
}

// ---------------------------------------------------------------------------
// arquivos e estado

/// diretório de configuração do usuário (XDG). A variável de teste evita
/// que os testes toquem no MangoHud.conf real do usuário
pub fn base_config_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("GAMEFIC_TEST_CONFIG_ROOT") {
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".config"))
        })
        .ok_or_else(|| "não foi possível localizar o diretório de configuração".to_string())
}

pub fn conf_caminho(perfil: &str) -> Result<PathBuf, String> {
    perfil_valido(perfil)?;
    if perfil == PERFIL_GLOBAL {
        return Ok(base_config_dir()?.join("MangoHud").join("MangoHud.conf"));
    }
    // convenção upstream: arquivo por nome do processo executando
    Ok(base_config_dir()?.join("MangoHud").join(format!("{perfil}.conf")))
}

fn conf_dir() -> Result<PathBuf, String> {
    Ok(base_config_dir()?.join("MangoHud"))
}

/// perfis válidos: 1–48 caracteres de a-z/0-9/_/-; "global" é reservado ao
/// arquivo global (o nome do perfil corresponde ao nome do executável do jogo)
fn perfil_valido(perfil: &str) -> Result<(), String> {
    if perfil == PERFIL_GLOBAL {
        return Ok(());
    }
    let ok = !perfil.is_empty()
        && perfil.len() <= 48
        && perfil
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
        && !perfil.starts_with('-') // não pode virar opção (também evita paths)
        && perfil != "mangohud.conf"; // reservado ao arquivo global
    if ok {
        Ok(())
    } else {
        Err(format!(
            "nome de perfil '{perfil}' inválido — use 1–48 caracteres de a-z, 0-9, _ ou - (ex.: 'wine', 'steam')"
        ))
    }
}

/// perfis já presentes no diretório (arquivos `<nome>.conf` distintos do
/// global); usado pela aba para preencher o seletor sem adivinhação
pub fn perfis_existentes() -> Vec<String> {
    let Ok(dir) = conf_dir() else {
        return Vec::new();
    };
    let Ok(entradas) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut nomes: Vec<String> = entradas
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let nome = e.file_name().to_string_lossy().to_string();
            let stem = nome.strip_suffix(".conf")?.to_string();
            if stem == "MangoHud" {
                // arquivo global não entra na lista de perfis
                return None;
            }
            perfil_valido(&stem).ok().map(|_| stem)
        })
        .collect();
    nomes.sort();
    nomes
}

/// MangoHud presente? (binário da distribuição; mesmo critério de tools.rs,
/// que verifica o binário "mangohud" — presente nos pacotes Vulkan/GL)
pub fn instalado() -> bool {
    let Ok(o) = std::process::Command::new("which").arg("mangohud").output() else {
        return false;
    };
    o.status.success()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Estado {
    pub instalado: bool,
    pub perfil: String,
    pub conf_caminho: String,
    pub conf_existe: bool,
}

pub fn estado(perfil: &str) -> Estado {
    let caminho = conf_caminho(perfil).ok();
    Estado {
        instalado: instalado(),
        perfil: perfil.to_string(),
        conf_caminho: caminho
            .as_ref()
            .map(|c| c.display().to_string())
            .unwrap_or_default(),
        conf_existe: caminho.is_some_and(|c| c.is_file()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Carregado {
    /// valores resolvidos: explicitos do perfil sobrepõem; o ausente herda o global
    pub config: ConfigMangoHud,
    /// número de opções avançadas preservadas (não gerenciadas)
    pub preservadas: usize,
    /// arquivo `<perfil>.conf` já existe? (false = visão é só a do global)
    pub perfil_existe: bool,
    /// quantos campos gerenciados ficaram herdados do global (o perfil
    /// não tem linha para eles)
    pub campos_herdados: usize,
}

fn eh_gerenciada(chave: &str) -> bool {
    matches!(
        chave,
        "fps" | "frametime" | "cpu_stats" | "gpu_stats" | "cpu_temp" | "gpu_temp"
            | "vram" | "ram" | "core_load" | "cpu_power" | "gpu_power" | "gpu_core_clock"
            | "gpu_mem_clock" | "throttling_status" | "no_display" | "position" | "font_size"
            | "background_alpha" | "text_color" | "gpu_color" | "cpu_color" | "vram_color"
            | "ram_color" | "horizontal" | "hud_compact" | "frame_timing" | "graphs"
            | "toggle_hud" | "toggle_hud_position" | "toggle_fps_limit" | "toggle_logging"
            | "reload_cfg" | "custom_text" | "custom_text_center"
    )
}

fn eh_bool(chave: &str) -> bool {
    matches!(
        chave,
        "fps" | "frametime" | "cpu_stats" | "gpu_stats" | "cpu_temp" | "gpu_temp" | "vram"
            | "ram" | "core_load" | "cpu_power" | "gpu_power" | "gpu_core_clock"
            | "gpu_mem_clock" | "throttling_status" | "no_display" | "horizontal"
            | "hud_compact" | "frame_timing"
    )
}

/// opções DEFINIDAS no texto (linhas livres): chaves gerenciadas só;
/// comentadas/ausentes = herdadas do global (semântica upstream do
/// perfil por processo). Toggles com `=0` são off explícito (o gabarito
/// do MangoHud documenta `chave=0` para desativar)
fn definidas_no_texto(texto: &str) -> std::collections::HashMap<String, String> {
    let mut mapa = std::collections::HashMap::new();
    for linha in texto.lines() {
        let Some(l) = linha_conf(linha) else {
            continue;
        };
        if l.comentada {
            // comentada = herdada do global (não é um override)
            continue;
        }
        if !eh_gerenciada(l.chave) {
            continue;
        }
        if eh_bool(l.chave) {
            let valor = match l.valor {
                Some(v) if v == "0" || v.eq_ignore_ascii_case("false") => "0".to_string(),
                _ => "1".to_string(),
            };
            mapa.insert(l.chave.to_string(), valor);
        } else if let Some(v) = l.valor {
            mapa.insert(l.chave.to_string(), v.to_string());
        }
    }
    mapa
}


/// aplica as definições de perfil (mapa) sobre uma base (global resolvida)
/// — só a instância retorna valores efetivos
fn resolver(base: &ConfigMangoHud, definidos: &std::collections::HashMap<String, String>) -> (ConfigMangoHud, usize) {
    let mut cfg = base.clone();
    for (chave, valor) in definidos {
        match chave.as_str() {
            "fps" => cfg.metricas.fps = valor == "1",
            "frametime" => cfg.metricas.frametime = valor == "1",
            "cpu_stats" => cfg.metricas.cpu_stats = valor == "1",
            "gpu_stats" => cfg.metricas.gpu_stats = valor == "1",
            "cpu_temp" => cfg.metricas.cpu_temp = valor == "1",
            "gpu_temp" => cfg.metricas.gpu_temp = valor == "1",
            "vram" => cfg.metricas.vram = valor == "1",
            "ram" => cfg.metricas.ram = valor == "1",
            "core_load" => cfg.metricas.core_load = valor == "1",
            "cpu_power" => cfg.metricas.cpu_power = valor == "1",
            "gpu_power" => cfg.metricas.gpu_power = valor == "1",
            "gpu_core_clock" => cfg.metricas.gpu_core_clock = valor == "1",
            "gpu_mem_clock" => cfg.metricas.gpu_mem_clock = valor == "1",
            "throttling_status" => cfg.metricas.throttling_status = valor == "1",
            "no_display" => cfg.metricas.no_display = valor == "1",
            "horizontal" => cfg.aparencia.horizontal = valor == "1",
            "hud_compact" => cfg.aparencia.hud_compact = valor == "1",
            "custom_text" => {
                let t = valor.trim();
                cfg.aparencia.titulo = if t.is_empty() { None } else { Some(t.to_string()) };
            }
            "custom_text_center" => {
                let t = valor.trim();
                cfg.aparencia.titulo = if t.is_empty() { None } else { Some(t.to_string()) };
            }
            "frame_timing" => cfg.graficos.frame_timing = valor == "1",
            "position" => cfg.aparencia.posicao = valor.clone(),
            "font_size" => if let Ok(n) = valor.parse() { cfg.aparencia.tamanho_fonte = n; },
            "background_alpha" => if let Ok(n) = valor.parse() { cfg.aparencia.alpha_fundo = n; },
            "text_color" => cfg.aparencia.cor_texto = valor.clone(),
            "gpu_color" => cfg.aparencia.cor_gpu = valor.clone(),
            "cpu_color" => cfg.aparencia.cor_cpu = valor.clone(),
            "vram_color" => cfg.aparencia.cor_vram = valor.clone(),
            "ram_color" => cfg.aparencia.cor_ram = valor.clone(),
            "graphs" => {
                cfg.graficos.graficos = valor
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "toggle_hud" => cfg.atalhos.toggle_hud = Some(valor.trim().to_string()),
            "toggle_hud_position" => cfg.atalhos.toggle_hud_position = Some(valor.trim().to_string()),
            "toggle_fps_limit" => cfg.atalhos.toggle_fps_limit = Some(valor.trim().to_string()),
            "toggle_logging" => cfg.atalhos.toggle_logging = Some(valor.trim().to_string()),
            "reload_cfg" => cfg.atalhos.reload_cfg = Some(valor.trim().to_string()),
            _ => {}
        }
    }
    let herdadas = CHAVES_TODAS
        .iter()
        .filter(|k| !definidos.contains_key(**k))
        .count();
    (cfg, herdadas)
}

/// todas as chaves gerenciadas (contagem de herança)
const CHAVES_TODAS: &[&str] = &[
    "fps", "frametime", "cpu_stats", "gpu_stats", "cpu_temp", "gpu_temp", "vram", "ram",
    "core_load", "cpu_power", "gpu_power", "gpu_core_clock", "gpu_mem_clock", "throttling_status",
    "no_display", "position", "font_size", "background_alpha", "text_color", "gpu_color",
    "cpu_color", "vram_color", "ram_color", "horizontal", "hud_compact", "frame_timing", "graphs",
    "toggle_hud", "toggle_hud_position", "toggle_fps_limit", "toggle_logging", "reload_cfg",
    "custom_text", "custom_text_center",
];

/// texto do conf correlato resolvido: global + defaults (o que o perfil herda)
fn global_resolvida() -> (ConfigMangoHud, String) {
    let texto = fs::read_to_string(conf_caminho(PERFIL_GLOBAL).unwrap_or_default()).unwrap_or_default();
    let (cfg, _) = parse_conf(&texto);
    (cfg, texto)
}

/// leitura + parse; conf ausente = defaults do gabarito (não grava nada)
/// leitura com herança: global resolvido primeiro; o perfil (quando existe)
/// sobrepõe campo a campo; linha ausente/comentada = herda. Em modo global
/// o resultado é o comportamento da Fase 1 (comentada = desligada).
pub fn carregar(perfil: &str) -> Result<Carregado, String> {
    perfil_valido(perfil)?;
    if perfil == PERFIL_GLOBAL {
        let caminho = conf_caminho(perfil)?;
        match fs::read_to_string(&caminho) {
            Ok(texto) => {
                let (config, preservadas) = parse_conf(&texto);
                return Ok(Carregado {
                    config,
                    preservadas,
                    perfil_existe: true,
                    campos_herdados: 0,
                });
            }
            Err(_) => {
                return Ok(Carregado {
                    config: ConfigMangoHud::default(),
                    preservadas: 0,
                    perfil_existe: false,
                    campos_herdados: 0,
                });
            }
        }
    }
    let (base, _) = global_resolvida();
    let caminho = conf_caminho(perfil)?;
    match fs::read_to_string(&caminho) {
        Ok(texto) => {
            let definidos = definidas_no_texto(&texto);
            let (config, campos_herdados) = resolver(&base, &definidos);
            let preservadas = texto
                .lines()
                .filter_map(linha_conf)
                .filter(|l| !eh_gerenciada(l.chave))
                .count();
            Ok(Carregado {
                config,
                preservadas,
                perfil_existe: true,
                campos_herdados,
            })
        }
        Err(_) => {
            // arquivo ainda não existe: visão editável do estado global
            Ok(Carregado {
                config: base,
                preservadas: 0,
                perfil_existe: false,
                campos_herdados: CHAVES_TODAS.len(),
            })
        }
    }
}

/// validação + gravação (cria pastas; não executa nada externo).
/// No global, o comportamento é o da Fase 1 (todas as linhas explícitas).
/// Em um perfil, campos iguais ao global ficam comentados (herdam); os
/// divergentes vão explícitos — toggle desligado vira `chave=0`, sintaxe
/// documentada pelo gabarito do MangoHud para desativar. O arquivo global
/// NUNCA é alterado ao salvar um perfil (§ isolamento).
pub fn salvar(perfil: &str, cfg: &ConfigMangoHud) -> Result<String, String> {
    validar(cfg)?;
    let caminho = conf_caminho(perfil)?;
    let atual = fs::read_to_string(&caminho).ok();
    let texto = if perfil == PERFIL_GLOBAL {
        reconstruir_conf(atual.as_deref(), cfg)
    } else {
        let (base, _) = global_resolvida();
        reconstruir_perfil(atual.as_deref(), cfg, &base)
    };
    if let Some(dir) = caminho.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("falha ao criar {dir:?}: {e}"))?;
    }
    fs::write(&caminho, &texto).map_err(|e| format!("falha ao gravar {caminho:?}: {e}"))?;
    if perfil == PERFIL_GLOBAL {
        Ok(format!("Configuração gravada em {}", caminho.display()))
    } else {
        Ok(format!(
            "Perfil '{perfil}' gravado em {}; campos iguais ao global ficam comentados e herdam o global",
            caminho.display()
        ))
    }
}

/// remove o ARQUIVO de um perfil (nunca o global); remover um perfil que
/// já não existe é inofensivo e volta a mensagem correspondente
pub fn remover(perfil: &str) -> Result<String, String> {
    perfil_valido(perfil)?;
    if perfil == PERFIL_GLOBAL {
        return Err(
            "o arquivo global não pode ser removido por aqui — use a própria aba para editá-lo"
                .to_string(),
        );
    }
    let caminho = conf_caminho(perfil)?;
    match fs::remove_file(&caminho) {
        Ok(()) => Ok(format!(
            "Perfil '{perfil}' removido: jogos com esse nome de processo voltam a usar o global"
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(format!("Perfil '{perfil}' já não existia — nada a remover"))
        }
        Err(e) => Err(format!("falha ao remover {caminho:?}: {e}")),
    }
}

/// variante de título usada pelo arquivo original: as duas chaves
/// (`custom_text` e `custom_text_center`) alimentam o mesmo campo na
/// interface; a gravação reescreve a chave que o arquivo usa para não
/// duplicar o título no HUD nem abandonar a linha legada
fn chave_titulo(original: Option<&str>) -> &'static str {
    let usa_centro = original
        .map(|t| {
            t.lines()
                .any(|l| linha_conf(l).is_some_and(|c| c.chave == "custom_text_center"))
        })
        .unwrap_or(false);
    if usa_centro {
        "custom_text_center"
    } else {
        "custom_text"
    }
}

fn linha_titulo(chave: &'static str, titulo: Option<&str>) -> (&'static str, String) {
    if titulo.is_some_and(|t| !t.trim().is_empty()) {
        (fixa(chave), format!("{chave}={}", titulo.unwrap_or_default()))
    } else {
        (fixa(chave), format!("#{chave}"))
    }
}

/// reescrita de um ARQUIVO DE PERFIL: linha comentada = herdada do global;
/// divergências vão explícitas (ver salvar). Linhas desconhecidas do
/// o global não é modificado pela escrita de um perfil
pub fn reconstruir_perfil(
    original: Option<&str>,
    cfg: &ConfigMangoHud,
    base: &ConfigMangoHud,
) -> String {
    let mut gerenciadas: Vec<(&'static str, String)> = Vec::new();
    let m = &cfg.metricas;
    let b = &base.metricas;
    for (chave, valor, referencia) in [
        ("fps", m.fps, b.fps),
        ("frametime", m.frametime, b.frametime),
        ("cpu_stats", m.cpu_stats, b.cpu_stats),
        ("gpu_stats", m.gpu_stats, b.gpu_stats),
        ("cpu_temp", m.cpu_temp, b.cpu_temp),
        ("gpu_temp", m.gpu_temp, b.gpu_temp),
        ("vram", m.vram, b.vram),
        ("ram", m.ram, b.ram),
        ("core_load", m.core_load, b.core_load),
        ("cpu_power", m.cpu_power, b.cpu_power),
        ("gpu_power", m.gpu_power, b.gpu_power),
        ("gpu_core_clock", m.gpu_core_clock, b.gpu_core_clock),
        ("gpu_mem_clock", m.gpu_mem_clock, b.gpu_mem_clock),
        ("throttling_status", m.throttling_status, b.throttling_status),
        ("no_display", m.no_display, b.no_display),
    ] {
        let linha = if valor == referencia {
            format!("#{chave}")
        } else if valor {
            chave.to_string()
        } else {
            // sintaxe documentada pelo gabarito: chave=0 desativa
            format!("{chave}=0")
        };
        gerenciadas.push((chave, linha));
    }
    gerenciadas.extend(linhas_aparencia(
        &cfg.aparencia,
        &base.aparencia,
        chave_titulo(original),
    ));
    {
        let (g, r) = (&cfg.graficos, &base.graficos);
        gerenciadas.push((
            "frame_timing",
            if g.frame_timing == r.frame_timing {
                "#frame_timing".into()
            } else if g.frame_timing {
                "frame_timing".into()
            } else {
                "frame_timing=0".into()
            },
        ));
        if g.graficos == r.graficos {
            gerenciadas.push(("graphs", "#graphs".into()));
        } else if g.graficos.is_empty() {
            gerenciadas.push(("graphs", "graphs=0".into()));
        } else {
            gerenciadas.push(("graphs", format!("graphs={}", g.graficos.join(","))));
        }
    }
    gerenciadas.extend(linhas_atalhos(&cfg.atalhos, &base.atalhos));

    let chaves: Vec<&str> = gerenciadas.iter().map(|(k, _)| *k).collect();
    let mut corpo: Vec<&str> = Vec::new();
    match original {
        Some(texto) => {
            for linha in texto.lines() {
                let t = linha.trim();
                if t == MARCA_INI || t == MARCA_FIM {
                    continue;
                }
                if let Some(l) = linha_conf(linha) {
                    if chaves.contains(&l.chave) {
                        continue;
                    }
                }
                corpo.push(linha);
            }
            while corpo.last().is_some_and(|l| l.trim().is_empty()) {
                corpo.pop();
            }
        }
        None => {
            corpo.push(
                "### MangoHud.conf — perfil de jogo editado pelo Gamefic (aba Overlay)",
            );
            corpo
                .push("### Linhas comentadas herdam o arquivo global; divergentes ficam explícitas.");
            corpo.push("### Linhas fora do bloco abaixo são preservadas exatamente como estão.");
        }
    }

    let mut saida = String::new();
    saida.push_str(&corpo.join("\n"));
    saida.push_str("\n\n");
    saida.push_str(MARCA_INI);
    saida.push('\n');
    for (_, linha) in gerenciadas {
        saida.push_str(&linha);
        saida.push('\n');
    }
    saida.push_str(MARCA_FIM);
    saida.push('\n');
    saida
}

fn linhas_aparencia(a: &Aparencia, r: &Aparencia, titulo_key: &'static str) -> Vec<(&'static str, String)> {
    let pares: Vec<(&'static str, String, String)> = vec![
        ("position", a.posicao.clone(), r.posicao.clone()),
        ("font_size", a.tamanho_fonte.to_string(), r.tamanho_fonte.to_string()),
        (
            "background_alpha",
            formato_alpha(a.alpha_fundo),
            formato_alpha(r.alpha_fundo),
        ),
        ("text_color", a.cor_texto.clone(), r.cor_texto.clone()),
        ("gpu_color", a.cor_gpu.clone(), r.cor_gpu.clone()),
        ("cpu_color", a.cor_cpu.clone(), r.cor_cpu.clone()),
        ("vram_color", a.cor_vram.clone(), r.cor_vram.clone()),
        ("ram_color", a.cor_ram.clone(), r.cor_ram.clone()),
        (
            titulo_key,
            a.titulo.clone().unwrap_or_default(),
            r.titulo.clone().unwrap_or_default(),
        ),
    ];
    let mut linhas: Vec<(&'static str, String)> = pares
        .into_iter()
        .map(|(k, v, rv)| {
            if v == rv {
                (fixa(k), format!("#{k}"))
            } else {
                (fixa(k), format!("{k}={v}"))
            }
        })
        .collect();
    for (k, valor, referencia) in [
        ("horizontal", a.horizontal, r.horizontal),
        ("hud_compact", a.hud_compact, r.hud_compact),
    ] {
        let linha = if valor == referencia {
            format!("#{k}")
        } else if valor {
            k.to_string()
        } else {
            format!("{k}=0")
        };
        linhas.push((fixa(k), linha));
    }
    linhas
}

/// as chaves aqui são fixas do conjunto gerenciado; devolve a string estática
fn fixa(k: &str) -> &'static str {
    CHAVES_TODAS.iter().find(|c| **c == k).copied().unwrap_or("")
}

fn linhas_atalhos(a: &Atalhos, r: &Atalhos) -> Vec<(&'static str, String)> {
    chaves_atalho(a)
        .into_iter()
        .zip(chaves_atalho(r))
        .map(|((k, av), (_, rv))| {
            if av == rv || av.as_deref().is_none_or(|t| t.trim().is_empty()) {
                // igual ao global OU vazio (não gerenciar): herda
                (k, format!("#{k}"))
            } else {
                (k, format!("{k}={}", av.unwrap_or_default()))
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// testes (parser/round-trip/validação — sem tocar no conf real do usuário)

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn parser_tristate_linhas_livres_e_comentadas() {
        let (cfg, _) = parse_conf("fps\n# frametime\n#  gpu_temp\ncpu_stats\n");
        assert!(cfg.metricas.fps);
        assert!(!cfg.metricas.frametime);
        assert!(!cfg.metricas.gpu_temp);
        assert!(cfg.metricas.cpu_stats);
    }

    #[test]
    fn parser_nao_confunde_comentario_de_documentacao() {
        // linhas do gabarito devem preservar os defaults do modelo
        let (cfg, preserv) =
            parse_conf("### Enable most of the toggleable parameters\n### fps_limit=0\n");
        assert!(cfg.metricas.fps, "comentário não pode desligar uma métrica");
        assert_eq!(preserv, 0, "não é opção: nada a contar aqui");
    }

    #[test]
    fn parser_captura_valores_e_atalhos() {
        let (cfg, _) = parse_conf(
            "position=top-right\nfont_size=32\ntext_color=FF8800\ntoggle_hud=Shift_R+F12\n",
        );
        assert_eq!(cfg.aparencia.posicao, "top-right");
        assert_eq!(cfg.aparencia.tamanho_fonte, 32);
        assert_eq!(cfg.aparencia.cor_texto, "FF8800");
        assert_eq!(cfg.atalhos.toggle_hud.as_deref(), Some("Shift_R+F12"));
    }

    #[test]
    fn parser_graficos_com_lista() {
        let (cfg, _) = parse_conf("graphs=cpu_load,gpu_temp,vram\n");
        assert_eq!(cfg.graficos.graficos, vec!["cpu_load", "gpu_temp", "vram"]);
        let (cfg2, _) = parse_conf("#graphs\n");
        assert!(
            cfg2.graficos.graficos.is_empty(),
            "comentado = sem gráficos"
        );
    }

    #[test]
    fn reconstrutor_preserva_linhas_desconhecidas_e_comentarios() {
        let original = "### uso pessoal\nhelp\nfps_limit=60\n# minha nota\ngpu_stats\n";
        let mut cfg = ConfigMangoHud::default();
        cfg.metricas.gpu_stats = false;
        cfg.metricas.ram = true;
        let saida = reconstruir_conf(Some(original), &cfg);
        assert!(saida.contains("### uso pessoal"));
        assert!(saida.contains("help"));
        assert!(saida.contains("fps_limit=60"), "não gerenciada: intacta");
        assert!(saida.contains("# minha nota"));
        let gerenciada_comentada = saida.lines().any(|l| l == "#gpu_stats");
        assert!(
            gerenciada_comentada,
            "gpu_stats desligado vai comentado: {saida}"
        );
        assert!(saida.contains("\nram\n"), "ram ligado vai livre: {saida}");
    }

    #[test]
    fn reconstrutor_remove_marcas_antigas_sem_duplicar() {
        let original = format!("{MARCA_INI}\nfps\n{MARCA_FIM}\n");
        let saida = reconstruir_conf(Some(&original), &ConfigMangoHud::default());
        assert_eq!(saida.matches(MARCA_INI).count(), 1, "marca única: {saida}");
        // a linha antiga "fps" (livre) é recortada do corpo — só no bloco novo
        assert_eq!(saida.matches("fps").count() - 1, 1);
    }

    #[test]
    fn roundtrip_estavel() {
        let primeiro = reconstruir_conf(None, &ConfigMangoHud::default());
        let segundo = reconstruir_conf(Some(&primeiro), &ConfigMangoHud::default());
        assert_eq!(
            primeiro, segundo,
            "regravar sem mudança não pode alterar o arquivo"
        );
    }

    #[test]
    fn validacao_rejeita_fora_de_faixa() {
        let mut cfg = ConfigMangoHud::default();
        cfg.aparencia.tamanho_fonte = 300;
        assert!(validar(&cfg).is_err());
        cfg.aparencia.tamanho_fonte = 24;
        cfg.aparencia.cor_gpu = "XYZ".into();
        assert!(validar(&cfg).is_err());
        cfg.aparencia = Aparencia::default();
        cfg.atalhos.toggle_hud = Some("abrir overlay!?".into());
        assert!(validar(&cfg).is_err());
        cfg.atalhos = Atalhos::default();
        assert!(validar(&cfg).is_ok());
    }

    #[test]
    fn salvar_e_reler_em_diretorio_isolado() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = std::env::temp_dir().join("gamefic-test-mangohud");
        let _ = std::fs::remove_dir_all(&dir);
        // edition 2024: set_var é unsafe; em testes é seguro pois nenhum
        // outro teste depende do valor desta variável
        unsafe { std::env::set_var("GAMEFIC_TEST_CONFIG_ROOT", &dir) };
        let mut cfg = ConfigMangoHud::default();
        cfg.metricas.gpu_temp = true;
        cfg.aparencia.posicao = "bottom-right".into();
        let msg = salvar(PERFIL_GLOBAL, &cfg).expect("gravação isolada");
        assert!(msg.contains("gravada"));
        let lido = carregar(PERFIL_GLOBAL).expect("leitura isolada");
        assert!(lido.config.metricas.gpu_temp);
        assert_eq!(lido.config.aparencia.posicao, "bottom-right");
        unsafe { std::env::set_var("GAMEFIC_TEST_CONFIG_ROOT", "") };
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn titulo_legado_do_goverlay_e_rescrito_mantendo_a_variante() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = isolado("titulo");
        // arquivo legado escrito pelo GOverlay: título centralizado
        let caminho = conf_caminho(PERFIL_GLOBAL).unwrap();
        fs::create_dir_all(caminho.parent().unwrap()).unwrap();
        fs::write(
            &caminho,
            "################### File Generated by Goverlay 1.9.4 stable ###################\n\ncustom_text_center=Goverlay\nposition=top-left\n",
        )
        .unwrap();
        let lido = carregar(PERFIL_GLOBAL).unwrap();
        assert_eq!(
            lido.config.aparencia.titulo.as_deref(),
            Some("Goverlay"),
            "título lido da variante centralizada"
        );

        // gravação pelo Gamefic substitui o valor na MESMA variante
        let mut cfg = ConfigMangoHud::default();
        cfg.aparencia.titulo = Some("GAMEFIC".to_string());
        salvar(PERFIL_GLOBAL, &cfg).expect("regravação");
        let texto = fs::read_to_string(&caminho).unwrap();
        assert!(texto.contains("custom_text_center=GAMEFIC"), "{texto}");
        assert!(
            !texto.lines().any(|l| l.trim_start().starts_with("custom_text_center=Goverlay")),
            "valor legado substituído: {texto}"
        );
        assert!(
            !texto.lines().any(|l| l.trim_start().starts_with("custom_text=")),
            "sem duplicar variante: {texto}"
        );
        let depois = carregar(PERFIL_GLOBAL).unwrap();
        assert_eq!(depois.config.aparencia.titulo.as_deref(), Some("GAMEFIC"));
        limpa(&dir);
    }

    #[test]
    fn nomes_invalidos_de_perfil_sao_recusados() {
        // Fase 3: perfis individuais são válidos, MAS o nome é restrito
        // (evita escape de caminho e nomes que colidem com o global)
        for invalido in ["", "Nome Maior", "a/b", "../escape", "ab.", "Game", "perfis própios"] {
            assert!(conf_caminho(invalido).is_err(), "deve recusar: {invalido}");
        }
        assert!(conf_caminho(PERFIL_GLOBAL).is_ok());
        assert!(conf_caminho("wine").is_ok());
        assert!(conf_caminho("steam-native_2").is_ok());
    }

    // ---------- Fase 3: isolamento e herança ----------
    // testes dependem do diretório isolado; serializados para não trocar
    // a variável de ambiente entre si
    static AMBIENTE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn isolado(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gamefic-f3-{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        unsafe { std::env::set_var("GAMEFIC_TEST_CONFIG_ROOT", &dir) };
        dir
    }

    fn limpa(dir: &std::path::Path) {
        unsafe { std::env::set_var("GAMEFIC_TEST_CONFIG_ROOT", "") };
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn perfil_inexistente_devolve_o_global_sem_criar_arquivo() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = isolado("inexistente");
        let cfg_global = ConfigMangoHud::default();
        salvar(PERFIL_GLOBAL, &cfg_global).expect("global gravado");
        let global_caminho = conf_caminho(PERFIL_GLOBAL).unwrap();

        let carreg = carregar("csgo").expect("carregar sem perfil");
        assert!(!carreg.perfil_existe);
        assert_eq!(carreg.config, cfg_global);
        assert_eq!(carreg.campos_herdados, CHAVES_TODAS.len());
        // nenhum arquivo de perfil foi criado pela leitura
        assert!(!conf_caminho("csgo").unwrap().exists());
        assert!(global_caminho.exists());
        limpa(&dir);
    }

    #[test]
    fn salvar_perfil_não_altera_o_arquivo_global() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = isolado("isolamento");
        let mut g = ConfigMangoHud::default();
        g.aparencia.posicao = "top-right".into();
        salvar(PERFIL_GLOBAL, &g).expect("global");

        let mut p = ConfigMangoHud::default();
        p.aparencia.posicao = "bottom-center".into();
        p.aparencia.tamanho_fonte = 40;
        salvar("wine", &p).expect("perfil");

        let texto_global = std::fs::read_to_string(conf_caminho(PERFIL_GLOBAL).unwrap()).unwrap();
        assert!(
            !texto_global.contains("bottom-center"),
            "global intocado: {texto_global}"
        );
        let global_lido = carregar(PERFIL_GLOBAL).unwrap();
        assert_eq!(global_lido.config.aparencia.posicao, "top-right");

        // perfil tem só as divergências explícitas
        let texto_perfil = std::fs::read_to_string(conf_caminho("wine").unwrap()).unwrap();
        assert!(texto_perfil.contains("position=bottom-center"));
        assert!(texto_perfil.contains("font_size=40"));
        assert!(texto_perfil.contains("#fps"), "igual ao global = herda: {texto_perfil}");
        limpa(&dir);
    }

    #[test]
    fn herança_campo_a_campo_e_fallback_global() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = isolado("heranca");
        // global: gpu_temp LIGADO, position top-right
        let mut g = ConfigMangoHud::default();
        g.metricas.gpu_temp = true;
        g.aparencia.posicao = "top-right".into();
        salvar(PERFIL_GLOBAL, &g).expect("global");

        // perfil desativa o que o global liga (toggle off explícito);
        // position fica inalterado (= global) para testar a herança
        let mut p = ConfigMangoHud::default();
        p.metricas.gpu_temp = false; // diverge do global ligado
        p.aparencia.posicao = "top-right".into(); // igual ao global: herda
        salvar("dolphin", &p).expect("perfil");

        let lido = carregar("dolphin").unwrap();
        // herdado do global
        assert_eq!(lido.config.aparencia.posicao, "top-right");
        // divergente no perfil
        assert!(!lido.config.metricas.gpu_temp);
        assert!(lido.perfil_existe);
        let texto = std::fs::read_to_string(conf_caminho("dolphin").unwrap()).unwrap();
        assert!(texto.contains("gpu_temp=0"), "off explícito: {texto}");
        assert!(texto.contains("#position") && texto.lines().all(|l| !l.starts_with("position")), "position herdado: {texto}");

        // mudar o global DEPOIS propaga para campos herdados
        let mut g2 = ConfigMangoHud::default();
        g2.metricas.gpu_temp = true;
        g2.aparencia.posicao = "middle-center".into();
        g2.metricas.fps = false; // novo global desliga o fps
        salvar(PERFIL_GLOBAL, &g2).expect("global regravado");
        let dps = carregar("dolphin").unwrap();
        assert_eq!(
            dps.config.aparencia.posicao,
            "middle-center",
            "herda mudança do global"
        );
        assert!(
            !dps.config.metricas.fps,
            "fps herdado do novo global (desligado)"
        );
        assert!(
            !dps.config.metricas.gpu_temp,
            "divergência do perfil se mantém"
        );
        limpa(&dir);
    }

    #[test]
    fn perfis_existentes_lista_e_remover() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = isolado("listagem");
        let mut p = ConfigMangoHud::default();
        p.aparencia.tamanho_fonte = 30;
        salvar("retroarch", &p).expect("1");
        salvar("steam", &p).expect("2");
        assert_eq!(
            perfis_existentes(),
            vec!["retroarch".to_string(), "steam".to_string()]
        );
        let msg = remover("steam").expect("remoção");
        assert!(msg.contains("removido"));
        assert_eq!(perfis_existentes(), vec!["retroarch".to_string()]);
        // remover de novo é inofensivo
        assert!(remover("steam").is_ok());
        // global NUNCA sai por aqui
        assert!(remover(PERFIL_GLOBAL).is_err());
        // nome inválido nunca passa
        assert!(remover("../x").is_err());
        limpa(&dir);
    }

    #[test]
    fn validacao_continua_no_perfil() {
        let _guardo = AMBIENTE.lock().unwrap();
        let dir = isolado("validar");
        let mut p = ConfigMangoHud::default();
        p.aparencia.tamanho_fonte = 999;
        assert!(salvar("wine", &p).is_err(), "validação da Fase 1 vale no perfil");
        assert!(!conf_caminho("wine").unwrap().exists(), "falha não grava arquivo");
        limpa(&dir);
    }
}
