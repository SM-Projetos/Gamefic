//! Diagnóstico manual: revalida os ids passados COM consulta de repositório
//! (mesma lógica do comando `revalidar_detalhe`) — útil para semear o cache
//! persistido `consultas_remotas.json` com resultados reais. Sem argumentos,
//! revalida TODO o catálogo (rotina anti "sem fonte de instalação").
//! Uso: cargo run --release --example dump_revalidar [-- id1 id2 ...]
fn main() {
    let ids: Vec<String> = std::env::args().skip(1).collect();
    let ids = if ids.is_empty() {
        [
            // apps
            "steam", "lutris", "heroic", "bottles", "itch", "minigalaxy",
            // ferramentas
            "gamemode", "mangohud", "gamescope", "lact", "goverlay", "winetricks",
            "protontricks",
            // compatibilidade
            "wine",
            // emuladores
            "retroarch", "dolphin", "pcsx2", "rpcs3", "ppsspp", "duck", "mgba", "mame",
            // criação de conteúdo
            "obs", "gpu-screen-recorder", "kdenlive", "losslesscut", "audacity",
            "easyeffects", "discord", "teamspeak",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    } else {
        ids
    };
    for id in &ids {
        let t = std::time::Instant::now();
        match gamefic_lib::system::install::revalidar(id) {
            Some(d) => {
                for f in &d.fontes {
                    println!(
                        "{id}: {} disp={} inst={} vrepo={:?} ({:?})",
                        f.fonte, f.disponivel, f.instalado, f.versao_repo, t.elapsed()
                    );
                }
            }
            None => println!("{id}: NENHUM detalhe"),
        }
    }
}
