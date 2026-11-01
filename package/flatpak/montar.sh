#!/bin/sh
# Monta o pacote Flatpak do Gamefic a partir do binário de release já
# construído (cargo build --release) — fluxo flatpak build-init/export/bundle,
# sem flatpak-builder e sem download de SDK: só precisa do runtime
# org.gnome.Platform//48 (webkit2gtk-4.1 embutido nele).
set -eu

RAIZ="$(cd "$(dirname "$0")/../.." && pwd)"
BINARIO="$RAIZ/src-tauri/target/release/gamefic"
ICONS="$RAIZ/src-tauri/icons"
[ -x "$BINARIO" ] || { echo "binário de release ausente: $BINARIO"; exit 1; }

PACOTE="$RAIZ/package/flatpak"
TRABALHO="$PACOTE/flatpak-build"
REPO="$PACOTE/flatpak-repo"
APP_ID="com.sergio.gamefic"

rm -rf "$TRABALHO" "$REPO" && mkdir -p "$TRABALHO/files"

# metadata escrito à mão em vez de flatpak build-init (que exigiria o SDK
# instalado apenas para copiar um gabarito) — montagem usa só o runtime
cat > "$TRABALHO/metadata" <<EOF
[Application]
name=$APP_ID
runtime=org.gnome.Platform/x86_64/48
command=gamefic

[Context]
shared=ipc;network;
sockets=x11;wayland;fallback-x11;
devices=dri;
filesystems=home;
talk=org.freedesktop.Flatpak;

[Environment]
LC_ALL=
LANG=

[Extension org.freedesktop.Platform.GL]
version=1.4
directory=lib/gl
subdirectories=true
EOF

# binário + ícones + entrada de desktop
mkdir -p "$TRABALHO/files/bin" \
         "$TRABALHO/files/share/applications" \
         "$TRABALHO/files/share/icons/hicolor/32x32/apps" \
         "$TRABALHO/files/share/icons/hicolor/128x128/apps" \
         "$TRABALHO/files/share/icons/hicolor/512x512/apps"
cp "$BINARIO" "$TRABALHO/files/bin/gamefic"
cp "$PACOTE/com.sergio.gamefic.desktop" "$TRABALHO/files/share/applications/"
cp "$ICONS/32x32.png"                 "$TRABALHO/files/share/icons/hicolor/32x32/apps/$APP_ID.png"
cp "$ICONS/128x128.png"               "$TRABALHO/files/share/icons/hicolor/128x128/apps/$APP_ID.png"
cp "$ICONS/128x128@2x.png"            "$TRABALHO/files/share/icons/hicolor/512x512/apps/$APP_ID.png"

# comandos de fechamento do sandbox:
#  - UI: X11/Wayland + DRI + ipc
#  - rede: consultas remotas do backend (remote-info/forgejo)
#  - host: os comandos de detecção/gerenciamento falam com o host via
#          flatpak-spawn --host (spawn.rs), exigindo talk ao serviço
#  - casa: leitura/edição de configs reais do usuário (MangoHud, Steam,
#          compatibilitytools.d) — o app é um otimizador de sistema
# (permissões já definidas no metadata; nada a acrescentar aqui)
mkdir -p "$TRABALHO/files/lib/gl"
# build-export exige marcador de finalização (build-finish sem opções: só
# carimba o diretório; as permissões ficam no metadata)
flatpak build-finish --no-attributes-sharing "$TRABALHO" 2>/dev/null || flatpak build-finish "$TRABALHO"
flatpak build-export "$REPO" "$TRABALHO" stable
OUT="$PACOTE/Gamefic_0.1.0_amd64.flatpak"
flatpak build-bundle "$REPO" "$OUT" "$APP_ID" stable
echo "bundle pronto: $OUT"
# NOTA: ao instalar o bundle, o `talk=org.freedesktop.Flatpak` do metadata
# não é aplicado pelo cliente neste fluxo manual (com SDK ele seria). Se a
# detecção/gerenciamento na sua máquina usar o pacote Flatpak, aplique a
# permissão uma vez (sem root):
#   flatpak override --user com.sergio.gamefic \
#     --talk-name=org.freedesktop.Flatpak
