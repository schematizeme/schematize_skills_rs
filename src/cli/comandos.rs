//! Subcomandos de SKILLS: instalar, atualizar, remover, listar, forkar, comparar,
//! criar do zero e editar.

use crate::cli::args::Cmd;
use skills_casa::mercado;
use skills_casa::{registry, skilledit, skills};

pub fn resolve(cat: &[registry::Item], names: &[String], all: bool) -> Vec<registry::Item> {
    if all || names.is_empty() {
        cat.to_vec()
    } else {
        names.iter().filter_map(|n| registry::find(cat, n)).collect()
    }
}

/// `schematize skills <sub>` — dispatcher da gestão de skills (a feature).
pub fn executar(sub: Cmd) -> Result<(), String> {
    match sub {
        Cmd::Applied { mark, json } => crate::cli::projeto::applied_cmd(mark, json),
        Cmd::Rerun { slug } => crate::cli::projeto::rerun_cmd(slug),
        Cmd::Install { names, all, with_recommended } => {
            skills_install(&names, all, with_recommended)
        }
        Cmd::Update { names, all } => skills_update(&names, all),
        Cmd::List { json } => skills_list(json),
        Cmd::Remove { name, force_core } => skills_remove(&name, force_core),
        Cmd::New { slug, name, desc, force } => skills_new(&slug, name, desc, force),
        Cmd::Edit { slug, list, file, set_from } => skills_edit(&slug, list, file, set_from),
        Cmd::Fork { slug } => skills_fork(&slug),
        Cmd::Compare { slug } => skills_compare(&slug),
        Cmd::Status { json } => crate::cli::saidajson::status(json),
        Cmd::Acoes { json } => crate::cli::saidajson::acoes(json),
        Cmd::Desktop { install, remover } => crate::cli::desktop::executar(install, remover),
    }
}

/// `schematize skills fork <slug>` — força o fork de uma skill oficial (guarda a base no stash).
pub fn skills_fork(slug: &str) -> Result<(), String> {
    if !skills::is_official(slug) {
        println!("skill {slug} não é oficial (do catálogo) — ela já edita livremente, sem fork.");
        return Ok(());
    }
    skills::fork(slug)?;
    println!(
        "skill {slug} forkada: a pasta ativa é editável e a base oficial ficou guardada no stash."
    );
    println!("compare depois com: schematize skills compare {slug}");
    Ok(())
}

/// `schematize skills compare <slug>` — mostra o diff do fork ativo vs a nova oficial (latest).
pub fn skills_compare(slug: &str) -> Result<(), String> {
    let cmp = skills::compare_update(slug)?;
    println!(
        "Comparando fork de {slug}: base v{} → nova oficial v{}",
        cmp.base_version, cmp.new_version
    );
    if cmp.files.is_empty() {
        println!("  (nenhum arquivo — nada a comparar)");
    }
    for f in &cmp.files {
        println!("  {:<10} {}", f.status, f.path);
    }
    if !cmp.diff_text.trim().is_empty() {
        println!("\n--- diff unificado (fork ativo → nova oficial) ---");
        print!("{}", cmp.diff_text);
    }
    Ok(())
}

/// `schematize skills new <slug>` — scaffolda o piso mínimo válido de uma skill nova.
pub fn skills_new(
    slug: &str,
    name: Option<String>,
    desc: Option<String>,
    force: bool,
) -> Result<(), String> {
    let name = name.unwrap_or_else(|| slug.to_string());
    let desc = desc.unwrap_or_default();
    let dest = if force {
        skilledit::scaffold_force(slug, &name, &desc)?
    } else {
        skilledit::scaffold(slug, &name, &desc)?
    };
    println!("skill criada em {}", dest.display());
    Ok(())
}

/// `schematize skills edit <slug>` — lista os arquivos, imprime um, ou o grava de um arquivo fonte.
pub fn skills_edit(
    slug: &str,
    list: bool,
    file: Option<String>,
    set_from: Option<String>,
) -> Result<(), String> {
    // Com --file: escreve (se --set-from) ou imprime o conteúdo.
    if let Some(rel) = file {
        if let Some(src) = set_from {
            let content =
                std::fs::read_to_string(&src).map_err(|e| format!("falha ao ler {src}: {e}"))?;
            skilledit::write_file(slug, &rel, &content)?;
            println!("gravado {rel}");
            return Ok(());
        }
        let content = skilledit::read_file(slug, &rel)?;
        print!("{content}");
        return Ok(());
    }
    // Sem --file: lista (o default quando nada mais é passado). `--list` é o mesmo comportamento.
    let _ = list;
    let files = skilledit::list_files(slug)?;
    println!("arquivos editáveis de schematize-{slug}:");
    for f in files {
        println!("  {f}");
    }
    Ok(())
}

/// Instala skills (ou todas com --all) e, opcionalmente, as recomendadas.
pub fn skills_install(names: &[String], all: bool, with_recommended: bool) -> Result<(), String> {
    let cat = registry::catalog();
    let selected = resolve(&cat, names, all);
    // A skill PADRÃO (engineering) entra sempre — instalar `rust` sozinha deixaria o agente sem
    // o núcleo de orquestração.
    garantir_core(&cat, &selected);
    for it in &selected {
        match skills::install(it) {
            Ok(v) => println!("✓ {} instalado v{v}", it.slug),
            Err(e) => eprintln!("✗ {}: {e}", it.slug),
        }
    }
    // Recomendações (skill BASE complementar). Nunca instala de surpresa:
    // com --all já vem tudo; senão sugere, e só instala com --with-recommended.
    if !all {
        let mut suggested: Vec<String> = Vec::new();
        for it in &selected {
            for rec in &it.recommends {
                let already = registry::find(&cat, rec)
                    .map(|r| skills::installed_version(&r).is_some())
                    .unwrap_or(false);
                let in_batch = selected.iter().any(|s| &s.slug == rec);
                if !already && !in_batch && !suggested.contains(rec) {
                    suggested.push(rec.clone());
                }
            }
        }
        if !suggested.is_empty() {
            if with_recommended {
                for rec in &suggested {
                    if let Some(r) = registry::find(&cat, rec) {
                        match skills::install(&r) {
                            Ok(v) => println!("✓ {} instalado v{v}", r.slug),
                            Err(e) => eprintln!("✗ {}: {e}", r.slug),
                        }
                    }
                }
            } else {
                println!(
                    "skills recomendadas (complementares) não instaladas: {} — use \
                     --with-recommended para instalá-las.",
                    suggested.join(", ")
                );
            }
        }
    }
    aplicar_nucleo();
    Ok(())
}

/// Instala as skills PADRÃO (`core`) que ainda faltam e não estão em `ja`. Best-effort: falha
/// de uma vira aviso, não derruba o comando.
fn garantir_core(cat: &[registry::Item], ja: &[registry::Item]) {
    for it in registry::core_items(cat) {
        if ja.iter().any(|s| s.slug == it.slug) || skills::installed_version(&it).is_some() {
            continue;
        }
        match skills::install(&it) {
            Ok(v) => println!("✓ {} instalado v{v} (skill padrão)", it.slug),
            Err(e) => eprintln!("✗ {} (skill padrão): {e}", it.slug),
        }
    }
}

/// Aplica o núcleo sempre-on chamando o hub (`schematize core install`). Best-effort e MUDO sem
/// o hub: o app das skills funciona sozinho, e o `schematize doctor --fix` corrige depois.
fn aplicar_nucleo() {
    let home = skills_casa::nucleo::util::home();
    let mut cands = vec![
        home.join(".cargo/bin/schematize"),
        home.join(".local/bin/schematize"),
        std::path::PathBuf::from("/usr/local/bin/schematize"),
        std::path::PathBuf::from("/usr/bin/schematize"),
    ];
    if let Ok(path) = std::env::var("PATH") {
        cands.extend(std::env::split_paths(&path).map(|d| d.join("schematize")));
    }
    let Some(bin) = cands.into_iter().find(|p| p.is_file()) else { return };
    match std::process::Command::new(bin).args(["core", "install"]).status() {
        Ok(st) if st.success() => {}
        Ok(_) => {
            eprintln!("aviso: `schematize core install` falhou — rode `schematize doctor --fix`.")
        }
        Err(_) => {}
    }
}

/// Atualiza skills instaladas pro latest (todas se não passar nome/--all). Skills FORKADAS
/// não são sobrescritas — `skills::update` recusa e aponta o caminho comparar/mesclar.
pub fn skills_update(names: &[String], all: bool) -> Result<(), String> {
    let cat = registry::catalog();
    let selected = resolve(&cat, names, all);
    garantir_core(&cat, &selected);
    for it in &selected {
        match skills::update(it) {
            Ok(v) => println!("✓ {} → v{v}", it.slug),
            Err(e) => eprintln!("✗ {}: {e}", it.slug),
        }
    }
    aplicar_nucleo();
    Ok(())
}

/// Lista skills: instaladas vs última disponível. Se houver rede, anexa a nota do marketplace
/// (uma única request pra todas as linhas) — offline não trava: o mapa vem vazio e a nota some.
pub fn skills_list(json: bool) -> Result<(), String> {
    if json {
        return crate::cli::saidajson::lista();
    }
    let st = skills::load_state();
    let ratings = mercado::notas(); // vazio se offline; não bloqueia a listagem
    println!("catálogo do schematize (instalado vs mais recente):");
    for it in &registry::catalog() {
        let line = linha_de_status(it, &st);
        let nota = mercado::formatar_nota(ratings.get(&it.slug).copied());
        if nota.is_empty() {
            println!("  {line}");
        } else {
            println!("  {line}  {nota}");
        }
    }
    Ok(())
}

/// Remove uma skill instalada.
pub fn skills_remove(name: &str, force_core: bool) -> Result<(), String> {
    let cat = registry::catalog();
    match registry::find(&cat, name) {
        Some(it) => {
            let r = if force_core { skills::remove_forcado(&it) } else { skills::remove(&it) };
            if force_core && it.core {
                eprintln!(
                    "aviso: {} era skill PADRÃO — o núcleo sempre-on deixa de ser atualizado.",
                    it.slug
                );
            }
            r.map(|_| println!("removido: {}", it.slug))
        }
        None => Err(format!("skill desconhecida: {name}")),
    }
}

/// **O quê:** a linha humana de uma skill: slug, versão, última, e o estado por extenso.
///
/// **Onde:** `schematize-skills list`.
///
/// **A PALAVRA mora aqui e não no domínio.** O domínio devolve [`skills::Situacao`]; quem
/// imprime escolhe como dizer. Antes a frase era montada lá dentro, já traduzida — e aí a
/// janela precisaria casar texto em português para saber o estado, que é o defeito que fez
/// uma tela irmã devolver vazio nos outros dezenove idiomas.
fn linha_de_status(it: &registry::Item, st: &skills::State) -> String {
    let (s, inst, latest) = skills::situacao(it, st, true);
    // **As palavras são as MESMAS que o hub imprimia**, e não escolhas novas. A extradição
    // troca o dono do código, não o vocabulário de quem usa: um `diff` da listagem antes e
    // depois tem de sair vazio, e foi assim que este passo foi provado.
    let estado = match s {
        skills::Situacao::EmDia => "atual",
        skills::Situacao::TemAtualizacao => "ATUALIZAR",
        skills::Situacao::NaoInstalada => "não instalado",
        skills::Situacao::Fork => "[fork]",
        skills::Situacao::Desconhecida => "",
    };
    format!(
        "{:<12} {:<8} latest={:<8} {estado}",
        it.slug,
        inst.unwrap_or_else(|| "—".into()),
        latest.unwrap_or_else(|| "?".into())
    )
}
