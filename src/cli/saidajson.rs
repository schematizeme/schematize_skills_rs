//! A saída de MÁQUINA — o contrato que a janela e o hub leem.
//!
//! **O quê:** `list --json` (o catálogo com o estado de cada skill) e `status --json` (o que
//! esta máquina tem instalado).
//!
//! **Onde:** a janela deste app, e o hub quando delega a aba.
//!
//! ## Escrito à MÃO, como nos irmãos
//!
//! Nada de `#[derive(Serialize)]` numa struct de domínio. O documento é um CONTRATO com outro
//! processo: derivar faria o nome de um campo mudar quando alguém renomeia um campo do
//! domínio, em silêncio e sem passar por revisão. Escrito à mão, a mudança de forma aparece no
//! diff — e há teste que fixa cada nome.
//!
//! **O estado vem como SLUG, nunca traduzido** (`"em_dia"`, `"tem_atualizacao"`). Quem lê é um
//! programa; um programa que casa palavra traduzida quebra ao trocar o idioma da máquina — e
//! esse defeito já aconteceu aqui: uma tela casava rótulo em português e devolvia vazio nos
//! outros dezenove idiomas, sem erro nenhum.

use skills_casa::{registry, skills};

/// **O quê:** escapa uma string para dentro de JSON.
///
/// **Onde:** todo campo de texto deste módulo.
///
/// **Cobre os cinco de controle além das aspas e da barra.** Uma descrição de skill vem de
/// terceiro: um `\n` cru no meio dela produziria um documento que o leitor recusa, e a janela
/// mostraria "resposta ilegível" sobre um catálogo perfeitamente bom.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

/// `null` ou uma string entre aspas. **Ausência é `null`, nunca `""`**: a janela distingue
/// "não tem versão instalada" de "instalada com versão vazia", e as duas telas são diferentes.
fn opt(v: Option<&String>) -> String {
    match v {
        Some(s) => format!("\"{}\"", esc(s)),
        None => "null".into(),
    }
}

/// **O quê:** uma entrada do catálogo com o estado dela nesta máquina.
///
/// **Onde:** [`lista`]. Separada para o teste de contrato poder fixar os nomes de campo sem
/// depender de rede nem de catálogo instalado.
pub fn item(
    it: &registry::Item,
    situacao: &str,
    inst: Option<&String>,
    ultima: Option<&String>,
) -> String {
    format!(
        "{{\"slug\":\"{}\",\"dir\":\"{}\",\"repo\":\"{}\",\"categoria\":\"{}\",\
         \"verificada\":{},\"recomenda\":[{}],\"situacao\":\"{}\",\"instalada\":{},\"ultima\":{}}}",
        esc(&it.slug),
        esc(&it.skill_dir),
        esc(&it.repo),
        esc(&it.category),
        it.verified,
        it.recommends.iter().map(|r| format!("\"{}\"", esc(r))).collect::<Vec<_>>().join(","),
        esc(situacao),
        opt(inst),
        opt(ultima),
    )
}

/// **O quê:** `schematize-skills list --json`.
///
/// **Onde:** a janela e o hub.
pub fn lista() -> Result<(), String> {
    let st = skills::load_state();
    let itens: Vec<String> = registry::catalog()
        .iter()
        .map(|it| {
            let (s, inst, ultima) = skills::situacao(it, &st, true);
            item(it, s.slug(), inst.as_ref(), ultima.as_ref())
        })
        .collect();
    println!("{{\"skills\":[{}]}}", itens.join(","));
    Ok(())
}

/// **O quê:** `schematize-skills status [--json]` — o que ESTA máquina tem.
///
/// **Onde:** o hub, para a pílula da aba.
///
/// **Sem rede.** A pergunta é "o que eu tenho", e responder isso não pode depender de um
/// servidor estar no ar — é a mesma razão de o `disco` do optimizer não consultar nada.
pub fn status(json: bool) -> Result<(), String> {
    let st = skills::load_state();
    let cat = registry::catalog();
    if !json {
        println!("{} skill(s) instalada(s):", st.skills.len());
        for (slug, e) in &st.skills {
            let marca = if e.forked { " [fork]" } else { "" };
            println!("  {slug:<16} v{}{marca}", e.version);
        }
        return Ok(());
    }
    let itens: Vec<String> = st
        .skills
        .iter()
        .map(|(slug, e)| {
            format!(
                "{{\"slug\":\"{}\",\"versao\":\"{}\",\"fork\":{},\"base_do_fork\":{}}}",
                esc(slug),
                esc(&e.version),
                e.forked,
                opt(e.fork_base_version.as_ref())
            )
        })
        .collect();
    println!("{{\"instaladas\":[{}],\"no_catalogo\":{}}}", itens.join(","), cat.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item_teste() -> registry::Item {
        registry::Item {
            slug: "rust".into(),
            skill_dir: "schematize-rust".into(),
            repo: "skill-rust".into(),
            zip: "skill-rust.zip".into(),
            category: "language".into(),
            sponsor: None,
            verified: true,
            recommends: vec!["engineering".into()],
        }
    }

    /// **O CONTRATO: cada nome de campo, fixado.**
    ///
    /// Quem lê este documento é outro processo. Um campo renomeado em silêncio quebra a janela
    /// e o hub ao mesmo tempo, e o sintoma aparece longe da causa — uma tela vazia sem erro.
    #[test]
    fn o_contrato_do_item_fixa_cada_campo() {
        let d =
            item(&item_teste(), "tem_atualizacao", Some(&"1.2.0".into()), Some(&"1.3.0".into()));
        for campo in [
            "\"slug\":\"rust\"",
            "\"dir\":\"schematize-rust\"",
            "\"repo\":\"skill-rust\"",
            "\"categoria\":\"language\"",
            "\"verificada\":true",
            "\"recomenda\":[\"engineering\"]",
            "\"situacao\":\"tem_atualizacao\"",
            "\"instalada\":\"1.2.0\"",
            "\"ultima\":\"1.3.0\"",
        ] {
            assert!(d.contains(campo), "faltou {campo} em {d}");
        }
        // E é JSON de verdade — não só uma string que parece.
        let v: serde_json::Value = serde_json::from_str(&d).expect("documento válido");
        assert_eq!(v["situacao"], "tem_atualizacao");
    }

    /// **Ausência é `null`, nunca `""`.** A janela distingue "não instalada" de "instalada com
    /// versão vazia", e as duas telas são diferentes.
    #[test]
    fn ausencia_e_null_e_nao_string_vazia() {
        let d = item(&item_teste(), "nao_instalada", None, Some(&"1.3.0".into()));
        assert!(d.contains("\"instalada\":null"), "{d}");
        assert!(!d.contains("\"instalada\":\"\""), "{d}");
        let v: serde_json::Value = serde_json::from_str(&d).expect("válido");
        assert!(v["instalada"].is_null());
    }

    /// **A SITUAÇÃO é slug, nunca frase traduzida.** Quem lê é um programa, e um programa que
    /// casa palavra traduzida quebra ao trocar o idioma da máquina — defeito já visto aqui.
    #[test]
    fn a_situacao_nao_e_traduzida() {
        use skills_casa::skills::Situacao::*;
        for s in [EmDia, TemAtualizacao, NaoInstalada, Fork, Desconhecida] {
            let slug = s.slug();
            assert!(
                slug.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "`{slug}` não é um slug estável"
            );
        }
        assert_eq!(EmDia.slug(), "em_dia");
    }

    /// **Texto de terceiro não quebra o documento.** Descrição de skill vem de fora: um `\n`
    /// cru produziria um JSON que o leitor recusa, e a janela diria "resposta ilegível" sobre
    /// um catálogo perfeitamente bom.
    #[test]
    fn texto_hostil_continua_json_valido() {
        let mut it = item_teste();
        it.slug = "as\"pas\\barra\nquebra\ttab".into();
        it.category = "\u{1}controle".into();
        let d = item(&it, "em_dia", None, None);
        let v: serde_json::Value = serde_json::from_str(&d).expect("continua válido");
        assert_eq!(v["slug"], "as\"pas\\barra\nquebra\ttab", "e o valor volta inteiro");
    }
}
