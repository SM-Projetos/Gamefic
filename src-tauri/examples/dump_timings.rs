//! Diagnóstico manual: mede cada `detail()` do catálogo no sistema atual,
//! em duas rodadas (cache frio → quente), replicando o cenário real de
//! pré-carregamento da aba Apps.
//! Uso: cargo run --release --example dump_timings [-- id1 id2 ...]
fn main() {
    let ids: Vec<String> = std::env::args().skip(1).collect();
    let ids = if ids.is_empty() {
        vec![
            "steam".to_string(), "lutris".to_string(), "heroic".to_string(),
            "bottles".to_string(), "itch".to_string(), "minigalaxy".to_string(),
            "gamemode".to_string(), "mangohud".to_string(), "gamescope".to_string(),
            "lact".to_string(), "goverlay".to_string(), "winetricks".to_string(),
            "protontricks".to_string(), "wine".to_string(), "retroarch".to_string(),
            "dolphin".to_string(), "pcsx2".to_string(), "rpcs3".to_string(),
            "ppsspp".to_string(), "duck".to_string(), "mgba".to_string(),
            "mame".to_string(),
        ]
    } else {
        ids
    };
    let total = std::time::Instant::now();
    println!("== rodada 1 (cache frio, simulando pré-carga) ==");
    for id in &ids {
        let t = std::time::Instant::now();
        let ok = gamefic_lib::system::install::detail(id).is_some();
        println!("  {id:>13} {:>8.2} s{}", t.elapsed().as_secs_f32(), if ok { "" } else { " (None!)" });
    }
    println!("  total: {:.2} s", total.elapsed().as_secs_f32());

    println!("== rodada 2 (cache quente, reabertura imediata) ==");
    let t2 = std::time::Instant::now();
    for id in &ids {
        let ok = gamefic_lib::system::install::detail(id).is_some();
        assert!(ok, "detail({id}) era Some na rodada 1");
    }
    println!("  total: {:.2} s", t2.elapsed().as_secs_f32());
}
