//! As NOTAS da plataforma — quantas estrelas cada skill tem, e de quantas pessoas.
//!
//! **O quê:** lê `{api}/skills` e devolve `slug → (média, votos)`.
//!
//! **Onde:** a listagem da CLI e o catálogo da janela.
//!
//! ## A nota é ENFEITE, e o código trata assim
//!
//! Sem rede, sem API, com JSON de outra versão — a resposta é um mapa vazio, e a listagem
//! aparece inteira sem nota nenhuma. **Nunca um erro que interrompe.** Uma skill instalável
//! que não lista porque o servidor de avaliações caiu seria o caso mais claro de dependência
//! que o piso 10 proíbe: a ausência de um serviço não pode derrubar outro.

use std::collections::HashMap;

/// **O quê:** as notas de todas as skills. Mapa VAZIO em qualquer falha.
///
/// **Onde:** a listagem.
pub fn notas() -> HashMap<String, (f32, u32)> {
    let url = format!("{}/skills", crate::nucleo::conta::api_base());
    match crate::nucleo::rede::texto(&url) {
        Some(b) => parse_notas(&b),
        None => HashMap::new(),
    }
}

/// **O quê:** as notas a partir do corpo. PURA, e é onde os testes entram.
///
/// **Onde:** [`notas`].
///
/// **Parser estreito de propósito.** Ele lê os três campos que interessam e ignora o resto:
/// uma resposta que ganha campos novos continua funcionando, e uma que perde um campo perde
/// só a nota daquela skill — não a listagem inteira.
pub fn parse_notas(body: &str) -> HashMap<String, (f32, u32)> {
    let mut out = HashMap::new();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else { return out };
    let itens = v.get("skills").and_then(|s| s.as_array()).or_else(|| v.as_array());
    let Some(itens) = itens else { return out };
    for it in itens {
        let Some(slug) = it.get("slug").and_then(|s| s.as_str()) else { continue };
        let media = it.get("rating").or_else(|| it.get("avg")).and_then(|r| r.as_f64());
        let votos = it.get("votes").or_else(|| it.get("count")).and_then(|c| c.as_u64());
        // **Média sem votos não é nota.** Uma skill com `rating: 5.0` e nenhum voto mostraria
        // cinco estrelas vindas do nada — e cinco estrelas é exatamente o que faz alguém
        // instalar sem olhar.
        if let (Some(m), Some(c)) = (media, votos) {
            if c > 0 && (0.0..=5.0).contains(&m) {
                out.insert(slug.to_string(), (m as f32, c as u32));
            }
        }
    }
    out
}

/// **O quê:** a nota em texto (`★4.7 (12)`), ou vazio quando não há.
///
/// **Onde:** a listagem. Vazio, e não "sem nota": uma coluna em branco se lê; uma frase
/// repetida em vinte linhas vira ruído.
pub fn formatar_nota(nota: Option<(f32, u32)>) -> String {
    match nota {
        Some((avg, count)) => format!("★{avg:.1} ({count})"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_as_notas_das_duas_formas_do_documento() {
        let a = parse_notas(r#"{"skills":[{"slug":"rust","rating":4.7,"votes":12}]}"#);
        assert_eq!(a.get("rust"), Some(&(4.7, 12)));
        // Lista no topo, sem envelope: a API já respondeu assim.
        let b = parse_notas(r#"[{"slug":"qa","avg":3.5,"count":4}]"#);
        assert_eq!(b.get("qa"), Some(&(3.5, 4)));
    }

    /// **Média sem votos NÃO é nota.** Cinco estrelas vindas do nada é exatamente o que faz
    /// alguém instalar sem olhar.
    #[test]
    fn media_sem_votos_nao_vira_estrela() {
        assert!(parse_notas(r#"[{"slug":"x","rating":5.0,"votes":0}]"#).is_empty());
        assert!(parse_notas(r#"[{"slug":"x","rating":5.0}]"#).is_empty());
        // E uma média fora da escala é dado corrompido, não uma skill excelente.
        assert!(parse_notas(r#"[{"slug":"x","rating":9.9,"votes":3}]"#).is_empty());
        assert!(parse_notas(r#"[{"slug":"x","rating":-1,"votes":3}]"#).is_empty());
    }

    /// **A nota é ENFEITE: falha vira mapa vazio, nunca erro.** Uma skill instalável que não
    /// lista porque o servidor de avaliações caiu é a dependência que o piso 10 proíbe.
    #[test]
    fn documento_ruim_vira_mapa_vazio() {
        let fundo = format!("{}{}", "[".repeat(3000), "]".repeat(3000));
        for lixo in
            ["", "null", "0", "{ nao e json", "\u{0}", &fundo, r#"{"skills":"nao e lista"}"#]
        {
            assert!(parse_notas(lixo).is_empty(), "{lixo:?}");
        }
        // Uma skill sem slug é ignorada, e as outras da mesma lista continuam.
        let m = parse_notas(r#"[{"rating":4.0,"votes":2},{"slug":"ok","rating":4.0,"votes":2}]"#);
        assert_eq!(m.len(), 1);
        assert!(m.contains_key("ok"));
    }

    #[test]
    fn a_nota_vazia_nao_vira_frase() {
        assert_eq!(formatar_nota(None), "");
        assert_eq!(formatar_nota(Some((4.65, 12))), "★4.7 (12)");
    }
}
