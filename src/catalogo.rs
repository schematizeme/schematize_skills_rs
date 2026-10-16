//! Busca do catálogo remoto — ANTI-CACHE, com degradação em camadas e erro claro.
//!
//! **O quê:** obtém o `catalog.json` do repo `schematize-cli` sem sofrer o cache de ~5 min do
//! `raw.githubusercontent.com` na URL de `main`, e explica por extenso quando um slug pedido
//! não existe.
//!
//! **Onde:** [`crate::registry`], que consulta o catálogo em install/update/remove/list.
//!
//! ## A ordem das fontes
//!
//! 1. **Por SHA.** Pergunta à API qual é o commit atual de `main` (`Accept:
//!    application/vnd.github.sha`, sem token) e baixa `raw/<sha>/catalog.json`. URL por SHA é
//!    imutável: não existe versão velha dela no cache.
//! 2. **`main` com query anti-cache** (`?t=<agora>`), se a API falhar — inclusive 403 por
//!    rate limit (60/h anônimo). A falha da API NUNCA derruba o comando.
//! 3. **Cache local** (`catalog-cache.json`), se a rede toda falhar.
//! 4. **Embutido**, por último.
//!
//! ## O cache local tem TTL curto e uma válvula
//!
//! Dentro do TTL o cache serve sem rede. Mas um slug que NÃO está nele não é "inexistente":
//! pode ser skill publicada há 1 minuto. Por isso [`resolver`] busca de novo, uma vez, ignorando
//! o TTL, antes de desistir — e só então devolve o erro.

use crate::registry::Item;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Repo (da org da casa) que hospeda o `catalog.json`.
const REPO: &str = "schematize-cli";
/// Tempo em que o cache local vale sem consultar a rede, em segundos.
pub const TTL_SEGUNDOS: u64 = 120;
const UA: &str = "User-Agent: schematize-skills";

/// De onde veio o catálogo que está em uso — entra na mensagem de erro, para o usuário saber
/// o quão fresca é a resposta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origem {
    /// Raw por SHA do commit atual de `main` — a fonte exata.
    Sha,
    /// Raw de `main` com query anti-cache.
    Main,
    /// Cache local (rede indisponível, ou TTL ainda válido).
    Cache,
    /// Catálogo embutido no binário.
    Embutido,
}

impl Origem {
    fn texto(self) -> &'static str {
        match self {
            Origem::Sha => "GitHub, commit atual",
            Origem::Main => "GitHub, branch main",
            Origem::Cache => "cache local",
            Origem::Embutido => "catálogo embutido, sem rede",
        }
    }
}

/// O catálogo resolvido, com o carimbo de quando foi obtido.
#[derive(Debug, Clone)]
pub struct Catalogo {
    pub itens: Vec<Item>,
    /// Segundos Unix de quando o conteúdo foi baixado (do cache: de quando foi gravado).
    pub atualizado: u64,
    pub origem: Origem,
}

/// A fronteira de rede. Injetável: os testes nunca tocam a internet.
pub trait Rede {
    /// GET com cabeçalhos extras. `None` em qualquer falha (rede, 403, 404, timeout).
    fn get(&self, url: &str, cabecalhos: &[&str]) -> Option<String>;
}

/// A rede de verdade: `curl`.
pub struct Curl;

impl Rede for Curl {
    fn get(&self, url: &str, cabecalhos: &[&str]) -> Option<String> {
        let mut args: Vec<&str> = vec!["-sfL", "-m", "8", "-H", UA];
        for h in cabecalhos {
            args.push("-H");
            args.push(h);
        }
        args.push(url);
        crate::nucleo::util::run("curl", &args).ok()
    }
}

#[derive(Serialize, Deserialize)]
struct Doc {
    skills: Vec<Item>,
}

#[derive(Serialize, Deserialize)]
struct Gravado {
    atualizado: u64,
    skills: Vec<Item>,
}

/// Onde o cache mora: `<dados>/catalog-cache.json`.
pub fn caminho_cache() -> PathBuf {
    crate::nucleo::util::dados_dir().join("catalog-cache.json")
}

/// **O quê:** o SHA de 40 hex a partir do corpo da API. PURA.
///
/// Corpo que não é exatamente um SHA (página de erro, JSON de rate limit) vira `None` — assim
/// ele nunca entra numa URL.
pub fn parse_sha(corpo: &str) -> Option<String> {
    let s = corpo.trim();
    (s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())).then(|| s.to_ascii_lowercase())
}

fn interpretar(corpo: &str) -> Option<Vec<Item>> {
    let d: Doc = serde_json::from_str(corpo).ok()?;
    (!d.skills.is_empty()).then_some(d.skills)
}

fn ler_cache(cache: &Path) -> Option<Gravado> {
    let g: Gravado = serde_json::from_str(&std::fs::read_to_string(cache).ok()?).ok()?;
    (!g.skills.is_empty()).then_some(g)
}

fn gravar_cache(cache: &Path, itens: &[Item], agora: u64) {
    // Best-effort: cache que não grava não pode impedir o comando. Escrita atômica
    // (tmp + rename) para que um Ctrl-C no meio nunca deixe JSON truncado.
    let Ok(txt) = serde_json::to_string(&Gravado { atualizado: agora, skills: itens.to_vec() })
    else {
        return;
    };
    if let Some(dir) = cache.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = cache.with_extension("tmp");
    if std::fs::write(&tmp, txt).is_ok() {
        let _ = std::fs::rename(&tmp, cache);
    }
}

/// **O quê:** baixa da rede, na ordem SHA → main anti-cache. `None` se as duas falharem.
fn baixar(rede: &dyn Rede, org: &str, agora: u64) -> Option<(Vec<Item>, Origem)> {
    let api = format!("https://api.github.com/repos/{org}/{REPO}/commits/main");
    if let Some(sha) =
        rede.get(&api, &["Accept: application/vnd.github.sha"]).and_then(|c| parse_sha(&c))
    {
        let url = format!("https://raw.githubusercontent.com/{org}/{REPO}/{sha}/catalog.json");
        if let Some(itens) = rede.get(&url, &[]).and_then(|c| interpretar(&c)) {
            return Some((itens, Origem::Sha));
        }
    }
    let url = format!("https://raw.githubusercontent.com/{org}/{REPO}/main/catalog.json?t={agora}");
    let itens = rede.get(&url, &["Cache-Control: no-cache"]).and_then(|c| interpretar(&c))?;
    Some((itens, Origem::Main))
}

/// **O quê:** o catálogo, com a cadeia de degradação inteira.
///
/// `forcar = true` ignora o TTL do cache (vai à rede). Nunca falha: no pior caso devolve o
/// embutido.
pub fn carregar(
    rede: &dyn Rede,
    org: &str,
    cache: &Path,
    agora: u64,
    forcar: bool,
    embutido: &dyn Fn() -> Vec<Item>,
) -> Catalogo {
    let local = ler_cache(cache);
    if !forcar {
        if let Some(g) = &local {
            if agora.saturating_sub(g.atualizado) < TTL_SEGUNDOS {
                return Catalogo {
                    itens: g.skills.clone(),
                    atualizado: g.atualizado,
                    origem: Origem::Cache,
                };
            }
        }
    }
    if let Some((itens, origem)) = baixar(rede, org, agora) {
        gravar_cache(cache, &itens, agora);
        return Catalogo { itens, atualizado: agora, origem };
    }
    if let Some(g) = local {
        return Catalogo { itens: g.skills, atualizado: g.atualizado, origem: Origem::Cache };
    }
    Catalogo { itens: embutido(), atualizado: agora, origem: Origem::Embutido }
}

/// **O quê:** resolve os slugs pedidos. Slug desconhecido → refaz a busca UMA vez (ignorando o
/// TTL) antes de desistir; se ainda não existe, `Err` com mensagem completa.
///
/// `names` vazio devolve o catálogo todo.
pub fn resolver(
    rede: &dyn Rede,
    org: &str,
    cache: &Path,
    agora: u64,
    names: &[String],
    embutido: &dyn Fn() -> Vec<Item>,
) -> Result<(Catalogo, Vec<Item>), String> {
    let mut cat = carregar(rede, org, cache, agora, false, embutido);
    let falta = |c: &Catalogo| names.iter().any(|n| !c.itens.iter().any(|i| &i.slug == n));
    // `atualizado == agora` = acabou de vir da rede (ou do embutido, que não melhora tentando
    // de novo): repetir só gastaria o timeout.
    if falta(&cat) && cat.atualizado != agora {
        cat = carregar(rede, org, cache, agora, true, embutido);
    }
    let desconhecidos: Vec<&String> =
        names.iter().filter(|n| !cat.itens.iter().any(|i| &i.slug == *n)).collect();
    if let Some(slug) = desconhecidos.first() {
        return Err(erro_desconhecido(slug, &cat));
    }
    let sel =
        names.iter().filter_map(|n| cat.itens.iter().find(|i| &i.slug == n).cloned()).collect();
    Ok((cat, sel))
}

/// **O quê:** a mensagem de slug inexistente. PURA.
pub fn erro_desconhecido(slug: &str, cat: &Catalogo) -> String {
    let slugs: Vec<&str> = cat.itens.iter().map(|i| i.slug.as_str()).collect();
    let mut msg = format!(
        "skill '{slug}' não existe no catálogo (atualizado em {}, fonte: {}). Disponíveis: {}.",
        formatar_utc(cat.atualizado),
        cat.origem.texto(),
        slugs.join(", ")
    );
    let sug = sugestoes(slug, &slugs);
    if !sug.is_empty() {
        msg.push_str(&format!(" Você quis dizer: {}?", sug.join(", ")));
    }
    if matches!(cat.origem, Origem::Cache | Origem::Embutido) {
        msg.push_str(" Sem rede para confirmar no GitHub — verifique a conexão e tente de novo.");
    }
    msg
}

/// Até 3 slugs parecidos: distância de edição pequena ou contém/está contido.
pub fn sugestoes(slug: &str, slugs: &[&str]) -> Vec<String> {
    let limite = (slug.chars().count() / 3).clamp(1, 3);
    let mut v: Vec<(usize, &str)> = slugs
        .iter()
        .filter_map(|s| {
            let d = distancia(slug, s);
            let contem = !slug.is_empty() && (s.contains(slug) || slug.contains(s));
            (d <= limite || contem).then_some((if contem { d.min(1) } else { d }, *s))
        })
        .collect();
    v.sort();
    v.into_iter().take(3).map(|(_, s)| s.to_string()).collect()
}

/// Distância de Levenshtein.
pub fn distancia(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let custo = usize::from(a[i - 1] != b[j - 1]);
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + custo));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// `AAAA-MM-DD HH:MM UTC` a partir de segundos Unix. PURA (algoritmo civil de Hinnant).
pub fn formatar_utc(secs: u64) -> String {
    let dias = (secs / 86_400) as i64;
    let resto = secs % 86_400;
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = doy - (153 * mp + 2) / 5 + 1;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 };
    let ano = yoe + era * 400 + i64::from(mes <= 2);
    format!("{ano:04}-{mes:02}-{dia:02} {:02}:{:02} UTC", resto / 3600, resto % 3600 / 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn doc(slugs: &[&str]) -> String {
        let itens: Vec<String> = slugs
            .iter()
            .map(|s| {
                format!(r#"{{"slug":"{s}","skill_dir":"schematize-{s}","repo":"skill-{s}","zip":"skill-{s}.zip"}}"#)
            })
            .collect();
        format!(r#"{{"skills":[{}]}}"#, itens.join(","))
    }

    /// Rede falsa: responde por trecho da URL e registra as chamadas.
    struct Falsa {
        respostas: Vec<(&'static str, Option<String>)>,
        chamadas: RefCell<Vec<String>>,
    }

    impl Falsa {
        fn new(r: Vec<(&'static str, Option<String>)>) -> Self {
            Self { respostas: r, chamadas: RefCell::new(vec![]) }
        }
    }

    impl Rede for Falsa {
        fn get(&self, url: &str, _: &[&str]) -> Option<String> {
            self.chamadas.borrow_mut().push(url.to_string());
            self.respostas.iter().find(|(t, _)| url.contains(t)).and_then(|(_, r)| r.clone())
        }
    }

    fn tmp(nome: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("skills-catalogo-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d.join("catalog-cache.json")
    }

    fn emb() -> Vec<Item> {
        interpretar(&doc(&["embutida"])).unwrap()
    }

    #[test]
    fn sha_so_aceita_40_hex() {
        assert_eq!(parse_sha(&format!("{SHA}\n")).as_deref(), Some(SHA));
        for ruim in ["", "main", "<html>", r#"{"message":"API rate limit exceeded"}"#, &SHA[..39]] {
            assert_eq!(parse_sha(ruim), None, "{ruim:?}");
        }
    }

    #[test]
    fn baixa_pela_url_do_sha() {
        let r =
            Falsa::new(vec![("/commits/main", Some(SHA.into())), (SHA, Some(doc(&["a", "b"])))]);
        let c = carregar(&r, "org", &tmp("sha"), 1000, false, &emb);
        assert_eq!(c.origem, Origem::Sha);
        assert_eq!(c.itens.len(), 2);
        assert!(r.chamadas.borrow().iter().any(|u| u.contains(&format!("/{SHA}/catalog.json"))));
        assert!(!r.chamadas.borrow().iter().any(|u| u.contains("/main/catalog.json")));
    }

    /// API falha (rate limit): cai no raw de main com query anti-cache, sem quebrar.
    #[test]
    fn api_falha_cai_no_main_com_query() {
        let r = Falsa::new(vec![
            ("/commits/main", None),
            ("/main/catalog.json?t=777", Some(doc(&["a"]))),
        ]);
        let c = carregar(&r, "org", &tmp("main"), 777, false, &emb);
        assert_eq!(c.origem, Origem::Main);
        assert_eq!(c.itens[0].slug, "a");
    }

    /// API devolve lixo (JSON de rate limit com 200): não vira URL, cai no main.
    #[test]
    fn api_com_lixo_nao_vira_url() {
        let r = Falsa::new(vec![
            ("/commits/main", Some(r#"{"message":"limit"}"#.into())),
            ("/main/catalog.json", Some(doc(&["a"]))),
        ]);
        let c = carregar(&r, "org", &tmp("lixo"), 5, false, &emb);
        assert_eq!(c.origem, Origem::Main);
    }

    #[test]
    fn sem_rede_usa_cache_depois_embutido() {
        let cache = tmp("cache");
        let ok = Falsa::new(vec![("/commits/main", Some(SHA.into())), (SHA, Some(doc(&["a"])))]);
        carregar(&ok, "org", &cache, 1000, false, &emb);
        let fora = Falsa::new(vec![]);
        // Fora do TTL e sem rede: serve o cache velho.
        let c = carregar(&fora, "org", &cache, 1000 + TTL_SEGUNDOS + 50, false, &emb);
        assert_eq!((c.origem, c.itens[0].slug.as_str(), c.atualizado), (Origem::Cache, "a", 1000));
        // Sem cache nenhum: embutido.
        let c = carregar(&fora, "org", &tmp("vazio"), 1, false, &emb);
        assert_eq!((c.origem, c.itens[0].slug.as_str()), (Origem::Embutido, "embutida"));
    }

    #[test]
    fn dentro_do_ttl_nao_vai_a_rede() {
        let cache = tmp("ttl");
        let ok = Falsa::new(vec![("/commits/main", Some(SHA.into())), (SHA, Some(doc(&["a"])))]);
        carregar(&ok, "org", &cache, 1000, false, &emb);
        let r = Falsa::new(vec![]);
        let c = carregar(&r, "org", &cache, 1010, false, &emb);
        assert_eq!(c.origem, Origem::Cache);
        assert!(r.chamadas.borrow().is_empty());
    }

    /// O caso do bug: cache sem o slug `regression` → refaz a busca (ignora TTL) → acha.
    #[test]
    fn catalogo_sem_o_slug_refaz_a_busca_e_acha() {
        let cache = tmp("refresh");
        let velho = Falsa::new(vec![("/commits/main", Some(SHA.into())), (SHA, Some(doc(&["a"])))]);
        carregar(&velho, "org", &cache, 1000, false, &emb);
        let novo = Falsa::new(vec![
            ("/commits/main", Some(SHA.into())),
            (SHA, Some(doc(&["a", "regression"]))),
        ]);
        let (c, sel) =
            resolver(&novo, "org", &cache, 1010, &["regression".to_string()], &emb).expect("acha");
        assert_eq!(sel[0].slug, "regression");
        assert_eq!(c.origem, Origem::Sha);
        assert!(!novo.chamadas.borrow().is_empty(), "tem de ter ido à rede apesar do TTL");
    }

    #[test]
    fn slug_inexistente_e_erro_claro_com_sugestao() {
        let r = Falsa::new(vec![
            ("/commits/main", Some(SHA.into())),
            (SHA, Some(doc(&["regression", "rust"]))),
        ]);
        let e = resolver(&r, "org", &tmp("erro"), 86_400 * 365, &["regresion".to_string()], &emb)
            .unwrap_err();
        assert!(e.contains("skill 'regresion' não existe no catálogo"), "{e}");
        assert!(e.contains("atualizado em 1971-01-01"), "{e}");
        assert!(e.contains("Disponíveis: regression, rust"), "{e}");
        assert!(e.contains("Você quis dizer: regression?"), "{e}");
    }

    /// Sem rede nenhuma o erro avisa que não pôde confirmar — não afirma "não existe" no escuro.
    #[test]
    fn erro_sem_rede_avisa() {
        let r = Falsa::new(vec![]);
        let e = resolver(&r, "org", &tmp("offline"), 10, &["zzz".to_string()], &emb).unwrap_err();
        assert!(e.contains("Sem rede para confirmar"), "{e}");
    }

    #[test]
    fn distancia_e_sugestoes() {
        assert_eq!(distancia("rust", "rust"), 0);
        assert_eq!(distancia("rust", "rost"), 1);
        assert_eq!(distancia("", "abc"), 3);
        assert_eq!(sugestoes("reg", &["regression", "rust", "web"]), ["regression"]);
        assert!(sugestoes("xyzxyzxyz", &["rust", "web"]).is_empty());
    }

    #[test]
    fn formata_data() {
        assert_eq!(formatar_utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(formatar_utc(1_784_073_600 + 3_600 * 14 + 60 * 5), "2026-07-15 14:05 UTC");
    }
}
