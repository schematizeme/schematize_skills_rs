//! Catálogo de skills — ÍNDICE REMOTO com fallback embutido.
//! O quê: a lista de skills instaláveis vem de um `catalog.json` remoto (fonte de
//! verdade), pra skills novas aparecerem SEM recompilar/relançar a CLI. Se a rede
//! falhar, cai no catálogo embutido. Onde: consultado por install/update/list/agent/gui.

use serde::{Deserialize, Serialize};

/// Org GitHub que hospeda o catálogo e as skills.
pub const ORG: &str = "schematizeme";

/// Autor/co-autor da skill (mostrado na GUI): a pessoa/organização por trás (credibilidade).
/// Só nome + link — quem quiser verificar a firma vai ao site do parceiro (o `url`). Sem doc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sponsor {
    pub name: String,
    pub url: String,
}

/// Uma skill/tool instalável do ecossistema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// slug curto (o que o usuário digita): "engineering", "seo", ...
    pub slug: String,
    /// nome da pasta instalada em ~/.claude/skills: "schematize-<slug>".
    pub skill_dir: String,
    /// repositório GitHub: "skill-<slug>".
    pub repo: String,
    /// nome do asset .zip do release.
    pub zip: String,
    /// grupo na GUI: "base" (arquitetura), "language" (linguagem), "external" (ferramenta).
    #[serde(default)]
    pub category: String,
    /// autor/co-autor (pessoa/org) por trás da skill.
    #[serde(default)]
    pub sponsor: Option<Sponsor>,
    /// selo de VERIFICADO — skill oficial/de parceiro vetado (vs. futura skill de comunidade).
    #[serde(default)]
    pub verified: bool,
    /// slugs de skills COMPLEMENTARES recomendadas — ex.: toda linguagem recomenda a
    /// "engineering" (a skill BASE). Consumido pelo install (sugere/instala com --with-recommended)
    /// e exposto pra GUI ler depois. `#[serde(default)]` = catálogo antigo (sem o campo) ainda parseia.
    #[serde(default)]
    pub recommends: Vec<String>,
    /// skill PADRÃO/obrigatória: instalada sempre (install/update/bootstrap) e não removível sem
    /// `--force-core`. Ausente no catálogo = `false` (catálogo antigo continua parseando).
    #[serde(default)]
    pub core: bool,
}

#[cfg(test)]
#[derive(Deserialize)]
struct Catalog {
    skills: Vec<Item>,
}

/// Catálogo embutido — fallback quando o índice remoto está indisponível (offline).
/// Mantido em dia como rede de segurança; a fonte de verdade é o `catalog.json`.
fn builtin() -> Vec<Item> {
    let sp = |name: &str, url: &str| Some(Sponsor { name: name.into(), url: url.into() });
    let me = || sp("Lucassa", "https://lucassa.me");
    // Todas as skills oficiais nascem verificadas (publisher conhecido/vetado).
    let mk = |slug: &str, cat: &str, sponsor: Option<Sponsor>, recommends: &[&str]| Item {
        slug: slug.into(),
        skill_dir: format!("schematize-{slug}"),
        repo: format!("skill-{slug}"),
        zip: format!("skill-{slug}.zip"),
        category: cat.into(),
        sponsor,
        verified: true,
        recommends: recommends.iter().map(|s| s.to_string()).collect(),
        // A engineering é o núcleo sempre-on da casa: padrão e obrigatória.
        core: slug == "engineering",
    };
    // Toda skill de LINGUAGEM recomenda a "engineering" (a skill BASE complementar).
    let lang = ["engineering"];
    vec![
        mk("engineering", "base", me(), &[]),
        mk("web", "base", me(), &[]),
        mk("go", "language", me(), &lang),
        mk("rust", "language", me(), &lang),
        mk("elixir", "language", me(), &lang),
        mk("csharp", "language", me(), &lang),
        mk("zig", "language", me(), &lang),
        mk("ruby", "language", me(), &lang),
        mk("node", "language", me(), &lang),
        mk("seo", "external", sp("Hextorn", "https://hextorn.com"), &[]),
        mk("pentest", "external", sp("Basilisk Offsec", "https://basiliskoffsec.com"), &[]),
        mk("audit", "external", me(), &[]),
        mk("ai", "base", me(), &[]),
        mk("data", "base", me(), &[]),
        mk("mobile", "base", me(), &[]),
        mk("infra", "base", me(), &[]),
        mk("docs", "base", me(), &[]),
        mk("scaffold", "base", me(), &[]),
        mk("overdev-context", "base", me(), &["engineering"]),
        mk("qa", "base", me(), &["engineering"]),
        mk("institutional", "base", me(), &["web", "seo"]),
        mk("database", "base", me(), &["data", "engineering"]),
        mk("archive", "base", me(), &["engineering", "audit"]),
    ]
}

/// O catálogo atual: SHA de `main` → raw de `main` anti-cache → cache local → embutido.
/// Ver [`crate::catalogo`].
pub fn catalog() -> Vec<Item> {
    crate::catalogo::carregar(
        &crate::catalogo::Curl,
        ORG,
        &crate::catalogo::caminho_cache(),
        crate::nucleo::util::now_unix(),
        false,
        &builtin,
    )
    .itens
}

/// Resolve os slugs PEDIDOS pelo usuário. Slug fora do catálogo = busca de novo (ignorando o
/// TTL do cache) e, se continuar fora, `Err` claro. `names` vazio devolve o catálogo todo.
pub fn resolver(names: &[String]) -> Result<(Vec<Item>, Vec<Item>), String> {
    if names.is_empty() {
        let c = catalog();
        return Ok((c.clone(), c));
    }
    crate::catalogo::resolver(
        &crate::catalogo::Curl,
        ORG,
        &crate::catalogo::caminho_cache(),
        crate::nucleo::util::now_unix(),
        names,
        &builtin,
    )
    .map(|(c, sel)| (c.itens, sel))
}

/// Resolve um slug dentro de um catálogo (clona o Item).
pub fn find(cat: &[Item], slug: &str) -> Option<Item> {
    cat.iter().find(|i| i.slug == slug).cloned()
}

/// URL do release "latest" (sempre a última versão publicada da skill).
pub fn latest_zip_url(it: &Item) -> String {
    format!("https://github.com/{ORG}/{}/releases/latest/download/{}", it.repo, it.zip)
}

/// As skills PADRÃO (`core`) do catálogo — as que nunca podem faltar numa máquina.
pub fn core_items(cat: &[Item]) -> Vec<Item> {
    cat.iter().filter(|i| i.core).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campo_core_ausente_e_false() {
        let c: Catalog = serde_json::from_str(
            r#"{"skills":[{"slug":"a","skill_dir":"schematize-a","repo":"skill-a","zip":"a.zip"},
                {"slug":"b","skill_dir":"schematize-b","repo":"skill-b","zip":"b.zip","core":true}]}"#,
        )
        .unwrap();
        assert!(!c.skills[0].core);
        assert!(c.skills[1].core);
    }

    #[test]
    fn fallback_embutido_marca_so_a_engineering_como_core() {
        let cat = builtin();
        let core = core_items(&cat);
        assert_eq!(core.len(), 1);
        assert_eq!(core[0].slug, "engineering");
    }

    /// `remove` recusa a skill padrão ANTES de tocar em qualquer arquivo (o erro orienta a flag).
    #[test]
    fn remove_recusa_skill_core() {
        let eng = core_items(&builtin()).remove(0);
        let e = crate::skills::remove(&eng).unwrap_err();
        assert!(e.contains("--force-core"), "{e}");
    }
}
