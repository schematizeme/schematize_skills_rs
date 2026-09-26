//! **schematize-skills-gui** — a janela do `schematize-skills`.
//!
//! **O quê:** mostra o catálogo com o estado de cada skill, e manda para o TERMINAL tudo que
//! muda a máquina.
//!
//! **Onde:** o ícone do app, e a aba de Skills do hub, que delega para cá.
//!
//! ## Ela mora no MESMO repo do CLI (ADR-0020), e continua falando por `--json`
//!
//! **Estar no mesmo repo é conveniência de distribuição, não permissão para acoplar.** A
//! janela não usa o crate `skills_casa` a não ser pela `nucleo::procedencia` (plataforma, para
//! dizer de qual commit ela é): ela roda o binário e lê o `--json`, como as irmãs. Há teste
//! lendo o próprio fonte para cobrar isso.
//!
//! ## Por que NENHUMA instalação acontece aqui (D6)
//!
//! Instalar uma skill baixa um `.zip` de um repositório, descompacta em `~/.claude/skills/` e
//! sobrescreve o que estiver lá. São minutos, rede, e um caminho no disco de quem clicou. No
//! terminal, o comando aparece ANTES de rodar, o progresso é visível e o `Ctrl-C` funciona —
//! numa janela Slint, nada disso.

mod cli;
mod json;
mod telas;
mod terminal;

slint::include_modules!();

use slint::{ModelRc, SharedString, VecModel};

/// **O quê:** converte as skills do domínio nas linhas que o Slint desenha.
///
/// **Onde:** [`recarregar`].
///
/// **Os três booleanos vêm CALCULADOS do Rust**, e não de expressões no `.slint`: as regras
/// ("fork está instalado", "fork não atualiza") têm teste do lado de cá, e reimplementá-las lá
/// criaria duas leis para o mesmo fato — a que divergisse ofereceria "atualizar" a um fork,
/// que é oferecer apagar o trabalho de quem editou.
fn linhas(v: &[telas::Skill]) -> ModelRc<SkillUI> {
    ModelRc::new(VecModel::from(
        v.iter()
            .map(|s| SkillUI {
                slug: s.slug.clone().into(),
                categoria: s.categoria.clone().into(),
                verificada: s.verificada,
                situacao: s.situacao.clone().into(),
                instalada: s.instalada.clone().into(),
                ultima: s.ultima.clone().into(),
                recomenda: s.recomenda.join(", ").into(),
                e_instalada: s.instalada(),
                tem_atualizacao: s.tem_atualizacao(),
                e_fork: s.e_fork(),
            })
            .collect::<Vec<_>>(),
    ))
}

/// **O quê:** relê o catálogo e escreve as propriedades da tela.
///
/// **Onde:** a abertura e o botão «Reler».
///
/// **Lista VAZIA com o erro na tela, e nunca vazia calada:** "não há skill nenhuma" sobre uma
/// falha de leitura mandaria a pessoa reinstalar o que já tem.
fn recarregar(w: &MainWindow) {
    match cli::rodar(&["list", "--json"]).and_then(|t| telas::ler_catalogo(&t)) {
        Ok(v) => {
            w.set_resumo(telas::resumo(&v).into());
            w.set_skills(linhas(&v));
            w.set_erro(SharedString::new());
        }
        Err(e) => {
            w.set_skills(ModelRc::new(VecModel::from(Vec::<SkillUI>::new())));
            w.set_resumo(SharedString::new());
            w.set_erro(e.into());
        }
    }
}

/// **O quê:** abre um subcomando no TERMINAL e diz na tela o que aconteceu.
///
/// **Onde:** as quatro ações que mudam a máquina (D6).
///
/// **Sem terminal, a mensagem traz o COMANDO** — quem não tem um emulador instalado ainda
/// consegue fazer a coisa, colando. Um "não consegui abrir o terminal" seco é um beco sem
/// saída (§37.48).
fn no_terminal(w: &MainWindow, args: &[&str]) {
    let bin = cli::bin().display().to_string();
    let cmd = cli::comando_para_terminal(&bin, args);
    if terminal::abrir(&cmd) {
        w.set_msg("terminal aberto — clique em «Reler» quando ele terminar.".into());
    } else {
        w.set_msg(format!("não achei um terminal. Rode: {cmd}").into());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // **RESPONDE `--version` E SAI, antes de qualquer coisa gráfica.**
    //
    // Sem isto a janela IGNORA a flag e ABRE — e quem perguntou fica esperando. O
    // `debugreport` do hub pergunta a versão de cada binário do ecossistema, e as janelas
    // irmãs tinham exatamente este defeito.
    if std::env::args().skip(1).any(|a| a == "--version" || a == "-V") {
        println!("schematize-skills-gui {}", skills_casa::nucleo::procedencia::rotulo_versao());
        return Ok(());
    }

    let w = MainWindow::new()?;
    w.set_versao(
        format!("schematize-skills-gui {}", skills_casa::nucleo::procedencia::rotulo_versao())
            .into(),
    );
    recarregar(&w);
    w.set_aba(aba_dos_args(&std::env::args().skip(1).collect::<Vec<_>>()));

    {
        let weak = w.as_weak();
        w.on_recarregar(move || {
            if let Some(w) = weak.upgrade() {
                recarregar(&w);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_instalar(move |slug| {
            if let Some(w) = weak.upgrade() {
                no_terminal(&w, &["install", slug.as_str()]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_atualizar(move |slug| {
            if let Some(w) = weak.upgrade() {
                no_terminal(&w, &["update", slug.as_str()]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_comparar(move |slug| {
            if let Some(w) = weak.upgrade() {
                // `compare` é LEITURA — ele mostra o diff e não escreve nada. Vai ao terminal
                // mesmo assim porque a saída é um diff: uma caixa de texto numa janela seria
                // pior que o `less` que a pessoa já sabe usar.
                no_terminal(&w, &["compare", slug.as_str()]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_nova_skill(move || {
            if let Some(w) = weak.upgrade() {
                // Sem o slug: o `new` PERGUNTA no terminal. Um campo de texto aqui teria de
                // validar a allow-list do slug de novo — segunda lei para a mesma regra.
                no_terminal(&w, &["new", "--help"]);
            }
        });
    }
    {
        let weak = w.as_weak();
        w.on_alternar_tema(move || {
            if let Some(w) = weak.upgrade() {
                w.set_dark(!w.get_dark());
            }
        });
    }

    w.run()
}

/// **O quê:** a aba pedida em `--aba <nome>`: `0` Catálogo, `1` Instaladas.
///
/// **Onde:** [`main`], e é por aqui que o hub delega sem perder em qual aba a pessoa estava.
///
/// **Pelo NOME, e nome desconhecido cai no Catálogo.** O número é detalhe interno do `.slint`.
/// Função PURA.
fn aba_dos_args(args: &[String]) -> i32 {
    let Some(i) = args.iter().position(|a| a == "--aba") else { return 0 };
    match args.get(i + 1).map(String::as_str) {
        Some("instaladas") | Some("installed") => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::aba_dos_args;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_aba_pedida_abre_pelo_nome() {
        assert_eq!(aba_dos_args(&v(&["--aba", "instaladas"])), 1);
        assert_eq!(aba_dos_args(&v(&["--aba", "installed"])), 1);
        assert_eq!(aba_dos_args(&v(&["--aba", "catalogo"])), 0);
        assert_eq!(aba_dos_args(&v(&[])), 0);
        // Nome inventado não abre aba inventada, e a flag no fim da linha não panica.
        assert_eq!(aba_dos_args(&v(&["--aba", "turbinada"])), 0);
        assert_eq!(aba_dos_args(&v(&["--aba"])), 0);
    }

    /// **A casca não sabe NADA do domínio** (D4 do ADR-0016).
    ///
    /// `nucleo::` é permitido e o domínio não é: a janela usa `nucleo::procedencia` para dizer
    /// de qual COMMIT ela é, o que é plataforma. O que o ADR-0020 proíbe é a janela LER o
    /// domínio em vez de perguntar ao binário.
    #[test]
    fn a_casca_nao_sabe_nada_do_dominio() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        for proibido in [
            "skills_casa::skills",
            "skills_casa::registry",
            "skills_casa::skilledit",
            "skills_casa::mercado",
        ] {
            assert!(
                !producao.contains(proibido),
                "a janela usou `{proibido}`: ela fala com o BINÁRIO por `--json`, e depender do \
                 crate a faria embutir uma versão — o bug que fez a janela irmã abrir a versão \
                 antiga"
            );
        }
        // O varredor ignora COMENTÁRIO: proibir a palavra proibiria explicar a regra. Foi
        // aprendido errando na janela do database, com `Command::new`.
        let codigo: Vec<&str> =
            producao.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        for linha in &codigo {
            assert!(
                !linha.contains("Command::new"),
                "a janela rodou processo direto:\n  {linha}\nTudo passa pela casca"
            );
        }
        assert!(
            codigo.iter().any(|l| l.contains("cli::rodar")),
            "a janela fala com o binário pela casca — se esta linha sumiu, o varredor está cego"
        );
    }

    /// **Instalar, atualizar e criar passam pelo TERMINAL, e nada escreve daqui** (D6).
    ///
    /// Instalar baixa um `.zip`, descompacta em `~/.claude/skills/` e sobrescreve o que
    /// estiver lá. São minutos, rede, e um caminho no disco de quem clicou.
    #[test]
    fn toda_escrita_passa_pelo_terminal() {
        let fonte = include_str!("main.rs");
        let producao = fonte.split("#[cfg(test)]").next().expect("há código antes dos testes");
        for (callback, sub) in [
            ("on_instalar", "\"install\""),
            ("on_atualizar", "\"update\""),
            ("on_comparar", "\"compare\""),
            ("on_nova_skill", "\"new\""),
        ] {
            let i = producao.find(callback).unwrap_or_else(|| panic!("{callback} sumiu"));
            let corpo = &producao[i..producao.len().min(i + 700)];
            assert!(corpo.contains("no_terminal"), "{callback} não passa pelo terminal");
            assert!(corpo.contains(sub), "{callback} perdeu o subcomando {sub}");
        }
        let codigo: Vec<&str> =
            producao.lines().map(str::trim_start).filter(|l| !l.starts_with("//")).collect();
        for proibido in ["fs::write", "fs::create", "fs::remove", "OpenOptions"] {
            assert!(
                !codigo.iter().any(|l| l.contains(proibido)),
                "a janela escreveu no disco (`{proibido}`) — quem escreve é o binário, no terminal"
            );
        }
        assert!(codigo.iter().any(|l| l.contains("no_terminal(&w")), "o varredor está cego");
    }
}
