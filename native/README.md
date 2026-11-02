# Gamefic Native — Slint

Este crate inicia a migração progressiva do Gamefic desktop de Tauri para Slint.

## Executar

```sh
cargo run --manifest-path native/Cargo.toml
```

Requisitos:
- Rust 1.92 ou superior (Slint 1.18).
- Toolchain C/C++ disponível para as dependências nativas do backend Winit.
- Sessão gráfica Wayland ou X11.

## Escopo desta etapa

- Janela sem moldura nativa (`no-frame`) com barra de título própria.
- Arraste da janela por `WindowMoveArea`, minimizar, maximizar/restaurar, fechar e redimensionar.
- Temas claro/escuro e tokens centralizados em `ui/themes.slint`.
- Componentes reutilizáveis para navegação, toolbar, ícones, cards e indicadores.
- Primeira tela Apps, alimentada pelo snapshot em cache e pela detecção real do backend.
- Reuso direto dos módulos sem dependência de Tauri em `../src-tauri/src/system/`.

As demais telas, ações de instalação, monitoramento, histórico, MangoHud, preferências e empacotamento nativo serão migrados em etapas posteriores. O app Tauri antigo permanece preservado e continua sendo o caminho de execução legado enquanto a paridade não for alcançada.