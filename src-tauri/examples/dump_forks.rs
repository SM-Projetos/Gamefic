//! Diagnóstico manual: timing de forks_wine (local) e forks_proton (rede),
//! agora paralelizados por fork dentro de forks_proton.
fn main() {
    let t0 = std::time::Instant::now();
    let wines = gamefic_lib::system::install::forks_wine();
    println!("forks_wine (local): {:.2} s → {}", t0.elapsed().as_secs_f32(), wines.len());
    let t1 = std::time::Instant::now();
    let protons = gamefic_lib::system::install::forks_proton();
    println!("forks_proton (rede, paralelo): {:.2} s → {}", t1.elapsed().as_secs_f32(), protons.len());
    for p in &protons {
        println!("  {:<28} repo={:?}", p.nome, p.versao_repo.as_deref().unwrap_or("—"));
    }
}
