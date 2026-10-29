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
//! Perfis: a API aceita um identificador de perfil para permitir perfis
//! por jogo no futuro; nesta fase apenas `global` é suportado e qualquer
//! outro perfil é recusado com mensagem explícita.

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
    if perfil != PERFIL_GLOBAL {
        return Err(format!(
            "perfil '{perfil}' será suportado em etapa futura — nesta fase só 'global' está disponível"
        ));
    }
    Ok(base_config_dir()?.join("MangoHud").join("MangoHud.conf"))
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
    pub config: ConfigMangoHud,
    /// número de opções avançadas preservadas (não gerenciadas)
    pub preservadas: usize,
}

/// leitura + parse; conf ausente = defaults do gabarito (não grava nada)
pub fn carregar(perfil: &str) -> Result<Carregado, String> {
    let caminho = conf_caminho(perfil)?;
    match fs::read_to_string(&caminho) {
        Ok(texto) => {
            let (config, preservadas) = parse_conf(&texto);
            Ok(Carregado {
                config,
                preservadas,
            })
        }
        Err(_) => Ok(Carregado {
            config: ConfigMangoHud::default(),
            preservadas: 0,
        }),
    }
}

/// validação + gravação (cria pastas; não executa nada externo)
pub fn salvar(perfil: &str, cfg: &ConfigMangoHud) -> Result<String, String> {
    validar(cfg)?;
    let caminho = conf_caminho(perfil)?;
    let atual = fs::read_to_string(&caminho).ok();
    let texto = reconstruir_conf(atual.as_deref(), cfg);
    if let Some(dir) = caminho.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("falha ao criar {dir:?}: {e}"))?;
    }
    fs::write(&caminho, &texto).map_err(|e| format!("falha ao gravar {caminho:?}: {e}"))?;
    Ok(format!("Configuração gravada em {}", caminho.display()))
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
    fn perfis_fora_da_fase_sao_recusados() {
        assert!(conf_caminho(PERFIL_GLOBAL).is_ok());
        match conf_caminho("meu-jogo") {
            Err(e) => assert!(e.contains("etapa futura")),
            Ok(_) => panic!("perfil individual não pode ser aceito nesta fase"),
        }
    }
}
