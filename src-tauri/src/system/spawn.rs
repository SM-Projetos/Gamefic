//! Execução de comandos de sistema com consciência de sandbox.
//!
//! Quando o Gamefic roda como Flatpak (env `FLATPAK_ID` presente), os
//! comandos de detecção, gerenciamento e abertura de aplicativos precisam
//! acontecer no HOST — dentro do sandbox não há pacman/flatpak/sensors e
//! qualquer spawn ficaria limitado ao isolamento. O aplicativo já pede a
//! permissão `org.freedesktop.Flatpak` no manifesto; fora do Flatpak, o
//! comportamento nativo fica idêntico ao de sempre.

use std::process::Command;

/// `Command` para executar `programa` no lugar certo: direto (nativo) ou via
/// `flatpak-spawn --host --watch` quando o app está no sandbox do Flatpak.
pub fn comando(programa: &str) -> Command {
    if std::env::var_os("FLATPAK_ID").is_some() {
        let mut c = Command::new("flatpak-spawn");
        c.arg("--host").arg("--watch").arg(programa);
        c
    } else {
        Command::new(programa)
    }
}
