//! **schematize-skills** — o catálogo, o que está instalado, e a edição.
//!
//! **Onde:** o ícone do app, a janela (por `--json`), e a mão de quem digita.

mod cli;

use clap::Parser;
use cli::args::Cli;

/// **O quê:** devolve o `SIGPIPE` ao padrão do Unix.
///
/// **Onde:** a primeira linha de `main`.
///
/// **Por quê:** o Rust ignora `SIGPIPE`, então `schematize-skills list | head` faz o processo
/// PANICAR ao escrever num cano fechado, em vez de terminar em silêncio como todo comando de
/// Unix termina. Este app imprime listas, que é o tipo de saída que se lê com `head`.
#[cfg(unix)]
fn restaurar_sigpipe() {
    // SAFETY: chamada única, antes de qualquer thread, restaurando o handler padrão do SO.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}
#[cfg(not(unix))]
fn restaurar_sigpipe() {}

fn main() {
    restaurar_sigpipe();
    let cli = Cli::parse();
    if cli.para_root {
        // Único thread ainda; o escape vale para o resto do processo e para o filho (`core install`).
        std::env::set_var(skills_casa::nucleo::usuario::ENV_PARA_ROOT, "1");
    }
    let alvo = skills_casa::nucleo::usuario::alvo();
    if let Some(msg) = skills_casa::nucleo::usuario::aviso(&alvo) {
        eprintln!("{msg}");
    }
    // O gestor de atualizações viaja com todo programa da casa (contrato market-parasita).
    schematize_plataforma::parasita::garantir(&schematize_plataforma::parasita::Config::new(
        "schematize",
    ));
    let r = cli::comandos::executar(cli.cmd);
    // O que o app criou como root volta a ser do usuário real (mesmo se o comando falhou).
    if let Some(dono) = alvo.dono {
        let raizes = [alvo.home.join(".claude"), alvo.home.join(".schematize")];
        skills_casa::nucleo::usuario::devolver_posse(&raizes, dono);
    }
    if let Err(e) = r {
        // Piso 4: erro NUNCA é engolido. Vai para o stderr, e o código de saída é 1 — quem
        // encadeia este comando num script precisa dos dois.
        eprintln!("erro: {e}");
        std::process::exit(1);
    }
}
