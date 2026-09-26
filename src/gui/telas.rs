//! O que cada tela mostra — lido do `--json` do binário headless, nunca de saída humana.
//!
//! **O quê:** transforma os documentos de `list --json` e `status --json` nas linhas que a
//! janela desenha.
//!
//! **Onde:** [`crate::gui`], que liga isto às propriedades do Slint.
//!
//! ## Funções PURAS, e a razão tem nome
//!
//! Tudo aqui entra texto e sai lista. O subprocesso fica em [`crate::gui::cli`]. Assim o
//! comportamento é afirmável com JSON inválido, truncado, hostil ou de outro idioma — sem
//! depender de haver skill instalada ou rede na máquina de quem roda a suíte.
//!
//! **E a SITUAÇÃO chega como slug, nunca como frase.** A janela desenha a cor e a palavra a
//! partir de `"em_dia"`/`"tem_atualizacao"`; casar texto traduzido aqui é o defeito que fez
//! uma tela irmã devolver vazio nos outros dezenove idiomas, sem erro nenhum.

use super::json::ler;

/// Uma skill do catálogo, com o estado dela nesta máquina.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Skill {
    pub slug: String,
    pub categoria: String,
    pub verificada: bool,
    /// O slug do estado: `em_dia`, `tem_atualizacao`, `nao_instalada`, `fork`, `desconhecida`.
    pub situacao: String,
    /// Vazio quando não está instalada.
    pub instalada: String,
    /// Vazio quando não deu para perguntar.
    pub ultima: String,
    /// Os slugs que esta skill recomenda junto.
    pub recomenda: Vec<String>,
}

impl Skill {
    /// **O quê:** esta skill está instalada?
    ///
    /// **Onde:** a aba Instaladas, e a contagem do topo.
    ///
    /// **Olha a VERSÃO e não a situação**, porque um fork está instalado e a situação dele é
    /// `fork`; medir pela situação deixaria os forks de fora da lista do que se tem.
    pub fn instalada(&self) -> bool {
        !self.instalada.is_empty()
    }

    /// **O quê:** há atualização a oferecer?
    ///
    /// **Onde:** o destaque da linha.
    ///
    /// **Fork NÃO conta**, e é o ponto: uma skill editada não "atualiza" no sentido normal —
    /// ela compara e mescla. Oferecer "atualizar" a um fork é oferecer apagar o trabalho de
    /// quem editou.
    pub fn tem_atualizacao(&self) -> bool {
        self.situacao == "tem_atualizacao"
    }

    /// **O quê:** é um fork editável?
    pub fn e_fork(&self) -> bool {
        self.situacao == "fork"
    }
}

/// **O quê:** o catálogo do `list --json`.
///
/// **Onde:** as abas Catálogo e Instaladas.
///
/// **`Err` e lista vazia são coisas diferentes:** um catálogo que não carregou é `Err`, e a
/// tela mostra o erro. Uma lista vazia diria "não há skill nenhuma" sobre uma falha de leitura.
pub fn ler_catalogo(texto: &str) -> Result<Vec<Skill>, String> {
    let doc = ler(texto)?;
    Ok(doc
        .arr("skills")
        .iter()
        .map(|s| Skill {
            slug: s.str_ou_vazio("slug"),
            categoria: s.str_ou_vazio("categoria"),
            verificada: s.bool("verificada") == Some(true),
            situacao: s.str_ou_vazio("situacao"),
            instalada: s.str_ou_vazio("instalada"),
            ultima: s.str_ou_vazio("ultima"),
            recomenda: s
                .arr("recomenda")
                .iter()
                .filter_map(|r| r.como_str().map(str::to_string))
                .collect(),
        })
        .collect())
}

/// **O quê:** a frase do topo da aba Catálogo.
///
/// **Onde:** a aba Catálogo.
///
/// **Conta o que é ACIONÁVEL**, e não o total: "24 skills" não diz nada a quem abriu a tela;
/// "3 com atualização" diz o que fazer com ela.
pub fn resumo(skills: &[Skill]) -> String {
    let inst = skills.iter().filter(|s| s.instalada()).count();
    let novas = skills.iter().filter(|s| s.tem_atualizacao()).count();
    let forks = skills.iter().filter(|s| s.e_fork()).count();
    let mut partes = vec![format!("{} de {} instalada(s)", inst, skills.len())];
    if novas > 0 {
        partes.push(format!("{novas} com atualização"));
    }
    if forks > 0 {
        partes.push(format!("{forks} fork(s)"));
    }
    partes.join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{"skills":[
      {"slug":"rust","dir":"schematize-rust","repo":"skill-rust","categoria":"language",
       "verificada":true,"recomenda":["engineering"],"situacao":"em_dia",
       "instalada":"1.11.0","ultima":"1.11.0"},
      {"slug":"web","dir":"schematize-web","repo":"skill-web","categoria":"base",
       "verificada":true,"recomenda":[],"situacao":"tem_atualizacao",
       "instalada":"1.15.0","ultima":"1.16.0"},
      {"slug":"seo","dir":"schematize-seo","repo":"skill-seo","categoria":"external",
       "verificada":false,"recomenda":[],"situacao":"nao_instalada",
       "instalada":null,"ultima":"0.3.0"},
      {"slug":"qa","dir":"schematize-qa","repo":"skill-qa","categoria":"base",
       "verificada":true,"recomenda":[],"situacao":"fork",
       "instalada":"0.9.0","ultima":"1.0.0"}
    ]}"#;

    #[test]
    fn le_o_catalogo() {
        let s = ler_catalogo(DOC).expect("lê");
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].slug, "rust");
        assert_eq!(s[0].recomenda, ["engineering"]);
        assert!(s[0].verificada);
        assert!(!s[2].verificada);
        assert_eq!(s[2].instalada, "", "`null` vira vazio, e vazio é um estado");
    }

    /// **Fork está INSTALADO, e não tem atualização a oferecer.**
    ///
    /// Medir "instalada" pela situação deixaria os forks de fora da lista do que se tem; e
    /// oferecer "atualizar" a um fork é oferecer apagar o trabalho de quem editou.
    #[test]
    fn fork_conta_como_instalada_e_nao_como_atualizavel() {
        let s = ler_catalogo(DOC).expect("lê");
        let qa = s.iter().find(|x| x.slug == "qa").expect("há o qa");
        assert!(qa.e_fork());
        assert!(qa.instalada(), "um fork ESTÁ instalado");
        assert!(!qa.tem_atualizacao(), "fork compara e mescla, não atualiza");
        // E a que não está instalada não conta como instalada, mesmo tendo `ultima`.
        let seo = s.iter().find(|x| x.slug == "seo").expect("há o seo");
        assert!(!seo.instalada());
    }

    /// **O resumo conta o que é ACIONÁVEL.** "24 skills" não diz nada a quem abriu a tela.
    #[test]
    fn o_resumo_conta_o_que_pede_acao() {
        let s = ler_catalogo(DOC).expect("lê");
        let r = resumo(&s);
        assert!(r.contains("3 de 4 instalada(s)"), "{r}");
        assert!(r.contains("1 com atualização"), "{r}");
        assert!(r.contains("1 fork(s)"), "{r}");
        // Sem nada pendente, a frase NÃO inventa zeros — "0 com atualização" faria a pessoa
        // procurar um problema que não existe.
        let calmo = resumo(&[Skill {
            slug: "x".into(),
            instalada: "1.0.0".into(),
            situacao: "em_dia".into(),
            ..Default::default()
        }]);
        assert_eq!(calmo, "1 de 1 instalada(s)");
    }

    /// **Vazio e ERRO são estados diferentes.**
    #[test]
    fn vazio_e_erro_nao_se_confundem() {
        assert_eq!(ler_catalogo(r#"{"skills":[]}"#).expect("vazio é Ok"), vec![]);
        assert!(ler_catalogo("{ isto nao e json").is_err(), "ilegível é Err, não lista vazia");
        assert_eq!(ler_catalogo("{}").expect("objeto vazio"), vec![]);
    }

    /// **A situação é SLUG, e a janela desenha a partir dele.** Um documento com a frase
    /// traduzida no lugar do slug não vira "em dia" por acaso — ele cai no estado neutro.
    #[test]
    fn frase_traduzida_no_lugar_do_slug_nao_vira_estado() {
        let s = ler_catalogo(
            r#"{"skills":[{"slug":"x","situacao":"atualização disponível","instalada":"1.0"}]}"#,
        )
        .expect("lê");
        assert!(!s[0].tem_atualizacao(), "só o SLUG conta");
        assert!(!s[0].e_fork());
        assert!(s[0].instalada(), "mas a versão instalada continua valendo");
    }

    /// Entrada hostil não panica — janela que morre ao abrir é pior que lista vazia.
    #[test]
    fn entrada_hostil_nao_panica() {
        let fundo = format!("{}{}", "[".repeat(3000), "]".repeat(3000));
        for lixo in [
            "",
            "null",
            "[]",
            "0",
            "\"t\"",
            "\u{0}",
            &fundo,
            r#"{"skills":"nao e lista"}"#,
            r#"{"skills":[null]}"#,
            r#"{"skills":[{"slug":42,"recomenda":7,"verificada":"talvez"}]}"#,
        ] {
            let _ = ler_catalogo(lixo);
        }
    }
}
