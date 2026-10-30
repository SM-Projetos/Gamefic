# CHANGELOG — Gamefic

Formato baseado em [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), versionado por SemVer. Histórico completo de desenvolvimento nos commits `SNAPSHOT-xxxx` do `git log`.

## [Não lançado]

### Adicionado
- **Fase 3 — perfis por jogo com herança do global:** cada jogo/emulador pode ter uma configuração própria no arquivo `~/.config/MangoHud/<nome-do-processo>.conf` (convenção do próprio MangoHud, que resolve o arquivo pelo nome do processo executando). Campos idênticos às preferências globais ficam comentados no arquivo do perfil (herdam o global — mudanças no global se propagam para esses campos); divergências vão explícitas, com toggle desligado em `chave=0` (sintaxe documentada pelo gabarito). Salvar um perfil NUNCA altera o `MangoHud.conf` global; perfis válidos: 1–48 caracteres de `a-z`, `0-9`, `_`, `-` (validados no Rust contra escape de caminho e colisão com o global). Perfil inexistente devolve a visão editável do global, e remover um perfil devolve os jogos correspondentes ao global (remoção do global é recusada).
- Seletor de perfil na aba Overlay: perfil global + perfis existentes, criação por nome de processo, remoção com confirmação e contagem de campos herdados exibida na interface.
- Título do HUD gerenciado (chave documentada `custom_text`): campo "Título exibido no HUD" com padrão `Gamefic` quando o usuário não informar outro; vazio = sem título (linha comentada, herdando o global em perfis).
- Diagnóstico nos eventos do histórico: gravação/remoção de perfil e falha de ativação na abertura de aplicativos (categorias Configuração/Diagnóstico, sem dados sensíveis).
- Novos comandos Tauri: `overlay_perfis` (lista perfis existentes) e `overlay_remover_perfil`.
- Novos testes: nomes de perfil inválidos recusados, isolamento global × perfil, herança campo a campo com fallback do global (incl. propagação de mudança posterior no global), listagem/remoção de perfis, validação da Fase 1 válida também em perfis, e teste antigo isolado por mutex de ambiente.
- **Fase 2 — ativação do MangoHud por caminho de inicio:** no Steam/Flatpak a variável vai como `--env=MANGOHUD=1` ao `flatpak run` (o ambiente do processo cliente flatpak não atravessa o sandbox); em launchers nativos, segue por herança de ambiente ao spawn. Início de jogo nunca é bloqueado por problema de MangoHud: a mensagem devolvida pela abertura informa ausência do binário e a limitação de runtime Flatpak. Helpers puros cobertos por testes (argumentos, preferência on/off, mensagens de falha).
- Nota de compatibilidade honesta na aba Overlay: Vulkan/OpenGL e Proton/DXVK exibem o overlay; Direct3D sem DXVK não; Wine puro não é iniciado diretamente pelo Gamefic.
- **Gerenciador próprio de configuração do MangoHud (aba Overlay):** leitura, edição e gravação respeitando a sintaxe do arquivo `MangoHud.conf` (`~/.config/MangoHud/MangoHud.conf`), preservando verbatim qualquer opção não gerenciada (chaves avançadas e comentários); controle de métricas (FPS, frametime, CPU/GPU, VRAM/RAM, temperaturas, frequências, consumo, cargas por núcleo, throttling), aparência (posição, tamanho da fonte, transparência, cores, horizontal/compacto), gráficos de desempenho (`graphs`/`frame_timing`), atalhos de teclado e ocultação padrão; validação no Rust (posições/rangos/hex/atalhos); estado transparente do arquivo e aviso claro quando o MangoHud não está instalado (nada é instalado automaticamente). Perfis por jogo ficaram preparados na API desde então (ativados de fato na Fase 3).
- Ativação do overlay nos jogos iniciados pelo Gamefic: `abrir_app` aceita `overlay` e injeta `MANGOHUD=1` no processo, preservando argumentos e variáveis de ambiente anteriores; a escolha fica nas preferências (`mangohudOverlay`).
- Novos comandos Tauri: `overlay_estado`, `overlay_carregar`, `overlay_salvar` (I/O em `spawn_blocking`).
- Testes unitários do parser/gravador do conf (tristate, round-trip estável, preservação de linhas desconhecidas, validação e perfis recusados).
- Detectores de família ampliados (Alpine/Void/Gentoo/Solus/NixOS), arquitetura real (uname) e catálogo com pacotes mapeados por família (`PkgNat`): `wine-cachyos` só é candidato no CachyOS/Arch.
- Fonte Snap Store distinta: só é ofertada com `snapd` funcionalmente em execução, com identificadores verificados na API oficial da Snap Store (steam e retroarch).
- Cache de detalhes com TTL, invalidado por qualquer ação de instalação/remoção, e exemplo `examples/dump_fontes.rs` para diagnóstico manual.
- Método de consulta de remotos Flatpak reais (`flatpak remotes`): disponibilidade concedida apenas por confirmação do remoto configurado, sem presumir flathub.
- Novos testes: mapeamento por família de catálogo, snap verificado, parser de remotos, idioma localizado e estados de consulta.

### Modificado
- Tema claro: tokens de status verde (`--accent-green` → `#047857`) e âmbar (`--accent-amber` → `#c2410c` laranja escuro) redefinidos com translúcidos próprios, corrigindo o contraste fraco das cores de fundo escuro aplicadas ao tema claro. Alcance global via tokens — pílulas "Instalado"/"Não instalado" dos cards, indicadores "Instalados"/"Atualizações" na aba Apps, status "Sistema detectado" da barra superior e demais indicadores verdes/laranja das outras abas. Tema escuro preservado intacto em `:root`.
- Interface da aba Apps reconstruída: novo cabeçalho com indicadores "No catálogo", "Instalados" e "Atualizações" (contados a partir dos detalhes reais de cada card; o de Atualizações é clicável e abre um popup com a lista de atualizações, com atualização individual e "Atualizar tudo"), campo de busca na mesma linha dos indicadores com estado de "sem resultados" em cada categoria, estado de erro na detecção com botão "Tentar novamente", e catálogo em cards compactos (ícone ampliado, nome, versão, descrição e pílula textual de estado no canto inferior direito — verde circulado para instalado e âmbar para não instalado ou verificação indisponível, reutilizando os indicadores globais) com grids CSS Grid responsivos. A ação principal de cada card é contextual ao estado real: Instalar (com seletor de fonte Flatpak/Snap/nativo quando houver mais de uma), Atualizar (só quando há versão nova no repositório) ou Abrir (apps com GUI); clicar no próprio card abre o gerenciador existente, onde ficam Remover, atualização e a seleção completa de fontes. Cards de Compatibilidade (Wine/Proton) e o card "+ Adicionar" mantêm o fluxo próprio pelo modal existente. Cards do catálogo são navegáveis por teclado (Enter/Espaço) com rótulos ARIA.

### Corrigido
- Consultas bem-sucedidas sem análise multi-idioma (`Versão:` PT-BR e `Version:`), evitando esconder fontes disponíveis do Flatpak em sistemas em português.
- Diferenciação de ausência confirmada, remoto ausente no escopo, falha de consulta (ex.: rede) e snapd parado; falhas temporárias mostram novo estado "indeterminado" na interface
- Interface da aba Apps: fontes não verificadas/viáveis não aparecem mais como opções no seletor; diagnóstico detalhado permanece na listagem de fontes.
- Listener de remoção de ferramentas de compatibilidade do Steam perdido no SNAPSHOT-0044 (botões Remover sem efeito).

## [0.1.0] — versão inicial (2026-10-22)

Primeira versão funcional do app: fundação visual, detecção real de sistema, monitoramento, histórico de eventos e análise de otimização.

### Fundação (SNAPSHOT-0001, 0002)

- Projeto Tauri 2 com frontend TypeScript vanilla (sem framework JS, sem Electron/GTK/Qt)
- Janela com 5 abas: Visão geral, Desempenho, Kernels, Otimização, Histórico e Diagnósticos
- Sistema de design centralizado (`src/styles/`): paleta, tipografia (Inter + JetBrains Mono com licenças OFL embutidas), espaçamento, componentes
- Temas: escuro e claro, mais modo auto (segue o sistema), com persistência em disco
- Painel de configurações com preferências JSON

### Detecção real do sistema (SNAPSHOT-0003)

- Snapshot real do sistema: distribuição, kernel, CPU (núcleos/threads/frequências), memória e swap, GPU e driver, Vulkan, sensores de temperatura
- Degradação honesta: dados indisponíveis retornam `null` e aparecem como "Não disponível" — nenhum valor fictício
- Exemplo `examples/dump_system.rs` para diagnóstico
- Estado de detecção exibido no topo da janela

### Monitoramento com gráficos (SNAPSHOT-0004)

- Coleta periódica em thread de fundo (histórico da sessão de 600 amostras)
- Intervalo configurável e persistido
- Uso de CPU (agregado), memória, temperatura, GPU (disponível quando o driver NVIDIA fornece via `nvidia-smi`)
- Gráficos em canvas na aba Desempenho, sem dependências externas no frontend
- Início e parada do monitoramento registrados como eventos

### Histórico de eventos e análise de otimização (SNAPSHOT-0005)

- Eventos persistidos em JSON com limite de 500 registros — sobrevivem ao fechamento do app
- Abras de filtro: por categoria (sistema, detecção, monitoramento, otimização) e por resultado (concluído, erro, aviso)
- Análise da aba Otimização: estados consultáveis do sistema (governador de CPU, Gamemode/MangoHud, Vulkan, swap) e recomendações justificadas pelos dados detectados, com classificação de risco (baixo/médio/alto)
- Perfis Equilibrado/Gamer/Economia de energia estruturados (0 ajustes implementáveis nesta versão — nenhuma ação sistêmica é executada; os textos informam isso)
- Limpeza do histórico com confirmação e registro do evento

### Build e acabamento (SNAPSHOT-0006)

- Build de produção: `.deb`, `.rpm` e AppImage
- Remoção do plugin `opener` (não usado) — permissões do webview restritas a `core:default`
- README com requisitos, comandos e estrutura

### Segurança e licenças

- Nenhuma execução genérica de shell: a detecção usa apenas binários fixos com argumentos fixos, e tudo é validado no Rust
- Sem operações com sudo/root; interface nunca é a fronteira de segurança
- Licença do projeto: GPL-3.0-only — todas as dependências são licenças livres (MIT/Apache-2.0/BSD/Zlib/MPL-2.0/Unicode/OFL), verificadas individualmente

### Limitações conhecidas

- Recomendações são apenas análise — nenhuma otimização é aplicada ao sistema nesta versão
- Métricas de GPU dependem do driver NVIDIA; em GPU sem suporte a `nvidia-smi`/AMD, campo aparece como "Não disponível"
- Testes de portabilidade limitados ao host de desenvolvimento (CachyOS, Wayland)
