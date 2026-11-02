fn main() {
    slint_build::compile("ui/main.slint")
        .expect("falha ao compilar a interface Slint");

    println!("cargo:rerun-if-changed=ui");
    println!("cargo:rerun-if-changed=../src-tauri/assets/icons");
    println!("cargo:rerun-if-changed=../src/assets/brand");
}
