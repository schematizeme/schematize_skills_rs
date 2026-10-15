//! A superfície da CLI.
//!
//! **Onde:** `main`, na raiz da árvore do clap.
//!
//! **Os nomes vieram do `schematize skills <sub>` do hub, de propósito** (E5 do ADR-0018):
//! quem já digitava `skills install rust` digita `schematize-skills install rust`. Mudar a
//! forma do comando no MESMO dia em que ele muda de dono cobraria duas mudanças de quem usa —
//! e `schematize skills …` continua funcionando, encaminhado pelo hub.

use clap::{Parser, Subcommand};

/// `schematize-skills` — o catálogo, o que está instalado, e a edição.
#[derive(Parser)]
#[command(
    name = "schematize-skills",
    // `version = <fn>` e nao `version` puro: o numero sozinho nao distingue dois binarios com o
    // mesmo `Cargo.toml` e comportamento diferente. Ver `nucleo/procedencia.rs`.
    version = skills_casa::nucleo::procedencia::rotulo_versao(),
    about = "schematize skills — the catalogue, what is installed, and editing",
    long_about = "Install, update, remove and edit Claude Code skills; know the house \
                  catalogue and what each project declares it needs.\n\
                  Works standalone; the schematize hub delegates its Skills tab to it."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Which skill VERSION shaped THIS project, and which fell behind the installed one.
    Applied {
        /// Record that <slug> was just applied here (the agent calls this when it finishes).
        #[arg(long)]
        mark: Option<String>,
        /// Machine-readable output (the hub reads this to show what fell behind).
        #[arg(long)]
        json: bool,
    },
    /// Re-run a skill over this project to refresh its precepts (opens an agent).
    Rerun {
        /// Skill slug; omit to re-run every outdated one.
        slug: Option<String>,
    },
    /// Install one or more skills (or all with --all) from the latest release.
    Install {
        names: Vec<String>,
        #[arg(long)]
        all: bool,
        /// Also install any recommended (complementary) skills, e.g. engineering.
        #[arg(long)]
        with_recommended: bool,
    },
    /// Update installed skills to latest (all if no name/--all).
    Update {
        names: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    /// List skills: installed vs latest available.
    List {
        /// Machine-readable output (the window reads this).
        #[arg(long)]
        json: bool,
    },
    /// Remove an installed skill (default skills like engineering need --force-core).
    Remove {
        name: String,
        /// Remove even a DEFAULT skill (engineering). The agent loses the always-on core.
        #[arg(long)]
        force_core: bool,
    },
    /// Create a new skill scaffold in ~/.claude/skills/schematize-<slug>/.
    New {
        /// slug (allow-list [a-z0-9-]) — vira a pasta schematize-<slug>.
        slug: String,
        /// Human name (title/descriptions); defaults to the slug.
        #[arg(long)]
        name: Option<String>,
        /// Dense description for the SKILL.md frontmatter.
        #[arg(long)]
        desc: Option<String>,
        /// Overwrite if the skill already exists.
        #[arg(long)]
        force: bool,
    },
    /// Edit an installed skill: --list its files, or --file <rel> to print one (or set it).
    Edit {
        slug: String,
        /// List the editable files (relative paths). Default action if nothing else is given.
        #[arg(long)]
        list: bool,
        /// A file relative to the skill root: prints it (or writes it with --set-from).
        #[arg(long)]
        file: Option<String>,
        /// Write the file from this source path (needs --file <rel>).
        #[arg(long)]
        set_from: Option<String>,
    },
    /// Fork an OFFICIAL skill (stash its base) so it can be edited without losing the original.
    Fork { slug: String },
    /// Compare a forked skill against the latest official (files changed + unified diff).
    Compare { slug: String },
    /// What this machine knows: installed skills, versions, and forks.
    Status {
        /// Machine-readable output (the window reads this).
        #[arg(long)]
        json: bool,
    },
    /// Buttons that installed skills declare for the hub's project tab (their `gui.json`).
    ///
    /// **É o ÚNICO ponto em que uma skill de terceiro declara UI**, e por isso ele é um
    /// subcomando e não um detalhe interno: quem lê o `gui.json` tem de ser UM só, e o
    /// formato é contrato com software de fora.
    Acoes {
        /// Machine-readable output (the hub reads this to draw the buttons).
        #[arg(long)]
        json: bool,
    },
    /// Desktop integration: put this app in the applications menu, or take it out.
    Desktop {
        #[arg(long)]
        install: bool,
        #[arg(long)]
        remover: bool,
    },
}
