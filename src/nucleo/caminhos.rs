//! Onde as coisas moram — PLATAFORMA, e nada de domínio.
//!
//! **O quê:** os diretórios do Claude Code e os do app, mais a escrita atômica.
//!
//! **Onde:** o domínio de skills, que instala em `~/.claude/skills` e guarda estado em
//! `~/.claude/schematize/`.
//!
//! **Por que é copiado e não uma lib comum (D4 do ADR-0016):** uma `commons` de domínio é
//! vetada pelo piso 6, e uma de plataforma amarraria a versão de oito repos por causa de
//! duzentas linhas. Nada aqui sabe o que é skill, catálogo ou fork.
//!
//! ## A regra "LER AMBOS" veio junto, e não é detalhe
//!
//! Houve um período curto em que o app se chamou **Overflow** e escreveu em
//! `~/.claude/overflow/`. Apagar esse caminho da resolução tornaria INVISÍVEL o estado de
//! quem instalou naquela janela — as skills continuariam no disco e o app diria que não há
//! nenhuma. Ler os dois e escrever no novo é o que migra sem perguntar.

use std::path::{Path, PathBuf};

/// Os nomes de diretório de projeto aceitos, do canônico ao legado.
const DIRS_ACEITOS: [&str; 2] = [".schematize", ".overdev"];

/// **O quê:** o `~/.claude`. **Onde:** tudo que segue.
pub fn claude_dir() -> PathBuf {
    super::util::home().join(".claude")
}

/// **O quê:** onde as skills são instaladas (`~/.claude/skills`).
pub fn skills_dir() -> PathBuf {
    claude_dir().join("skills")
}

/// **O quê:** onde os comandos achatados moram (`~/.claude/commands`).
pub fn commands_dir() -> PathBuf {
    claude_dir().join("commands")
}

/// **O quê:** o dir de DADOS do app, pela regra "ler ambos".
///
/// Canônico é `~/.claude/schematize/`; `overflow/` ainda é LIDO — ver o cabeçalho.
pub fn dados_dir() -> PathBuf {
    let c = claude_dir();
    for nome in ["schematize", "overflow"] {
        let d = c.join(nome);
        if d.is_dir() {
            return d;
        }
    }
    c.join("schematize")
}

/// **O quê:** o `state.json` — que skill está instalada, em que versão, e se virou fork.
pub fn state_path() -> PathBuf {
    dados_dir().join("state.json")
}

/// **O quê:** o dir do schematize DENTRO de um projeto, pela mesma regra "ler ambos".
///
/// **Onde:** o arquivo de skills do projeto (`.schematize/skills.json`).
pub fn schematize_dir_at(root: &Path) -> PathBuf {
    for nome in DIRS_ACEITOS {
        let d = root.join(nome);
        if d.is_dir() {
            return d;
        }
    }
    root.join(DIRS_ACEITOS[0])
}

/// Escrita atômica (temporário único + `sync_all` + `rename`): vem de `schematize_plataforma`
/// (ADR-0026; esta função foi a origem da versão da lib). Usada pelo arquivo de skills do
/// projeto e pelo `state.json`.
pub use schematize_plataforma::arquivo::escreve_atomico;

#[cfg(test)]
mod tests {
    use super::*;

    /// **A regra "ler ambos" não é enfeite.** Apagar o caminho legado tornaria invisível o
    /// estado de quem instalou quando o app se chamava Overflow.
    #[test]
    fn o_dir_do_projeto_le_o_legado_e_escreve_no_novo() {
        let base = std::env::temp_dir().join(format!("sk-cam-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).expect("cria");

        // Nenhum dos dois existe: devolve o CANÔNICO, para quem for criar criar o certo.
        assert!(schematize_dir_at(&base).ends_with(".schematize"));

        // Só o legado existe: devolve o legado, senão o estado de lá some da vista.
        std::fs::create_dir_all(base.join(".overdev")).expect("cria");
        assert!(schematize_dir_at(&base).ends_with(".overdev"));

        // Os dois existem: o canônico ganha.
        std::fs::create_dir_all(base.join(".schematize")).expect("cria");
        assert!(schematize_dir_at(&base).ends_with(".schematize"));
        let _ = std::fs::remove_dir_all(&base);
    }
}
