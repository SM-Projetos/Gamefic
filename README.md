# Gamefic

Aplicativo desktop para Linux voltado à otimização do computador para jogos: detecta hardware e sistema, monitora desempenho, consulta kernels/drivers e organiza ajustes com histórico de eventos.

- **Licença:** GPL-3.0-only (`LICENSE`)
- **Stack:** Tauri 2 · Rust · TypeScript · HTML/CSS

## Requisitos de desenvolvimento (Arch/CachyOS)

- Node.js + npm
- Rust estável (rustup)
- Bibliotecas dev: `gtk3`, `webkit2gtk-4.1`, `libudev` (com o toolchain base da distro)

## Comandos

```sh
npm install          # dependências do frontend
npm run tauri dev    # desenvolvimento (Vite + janela do app)
npm run tauri build  # empacotamento (.deb, .rpm, AppImage)
```

Verificações pontuais:

```sh
npx tsc --noEmit                  # tipos do frontend
(cd src-tauri && cargo check)     # backend
(cd src-tauri && cargo run --example dump_system)  # imprime o snapshot de detecção
(cd src-tauri && cargo run --example dump_revalidar)  # revalida as fontes de todo o catálogo
```

## Estrutura

- `src/` — frontend (TypeScript, tokens de design, componentes, gráficos)
- `src-tauri/src/system/` — detecção real (distro, kernel, CPU, memória, GPU, Vulkan, sensores, ferramentas)
- `src-tauri/src/system/appinfo.rs` — catálogo de apps e ferramentas (aba Apps e Criação de Conteúdo), com fontes nativas/Flatpak/Snap confirmadas de verdade
- `src-tauri/src/system/install.rs` — instalação/atualização/remoção real (flatpak install/uninstall/update por escopo, pacman, snap) com cache de detalhes e validação por consulta aos remotos configurados
- `src-tauri/src/system/icons.rs` — ícones embutidos da interface (catálogo e marca)
- `src-tauri/src/monitor.rs` — coleta periódica em thread de fundo com histórico da sessão
- `src-tauri/src/events.rs` — histórico persistido de eventos
- `src-tauri/src/recommendations.rs` — análise e recomendações justificadas pelos dados detectados
- `src-tauri/src/system/mangohud.rs` — gerenciador próprio de configuração do MangoHud (aba Overlay): leitura/edição/gravação do `MangoHud.conf` preservando opções não gerenciadas, perfis individuais por nome de processo (`MangoHud/<processo>.conf`) com herança do global campo a campo, e ativação por ambiente nos jogos iniciados pelo app

Dados indisponíveis são exibidos como "Não disponível" — nenhum valor fictício é apresentado como medição real.

## Dados do usuário

`~/.local/share/com.<usuário>.gamefic/` — preferências (JSON), eventos e o cache do último snapshot (`snapshot_cache.json`) usado para abrir a tela inicial já com conteúdo (a detecção fresca sempre roda em background depois e substitui os dados), onde `<usuário>` é o seu usuário do Linux.
