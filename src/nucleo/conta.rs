//! O endereço da plataforma — PLATAFORMA, e nada de domínio.
//!
//! **O quê:** a URL base da API, com a variável de ambiente que a sobrescreve.
//!
//! **Onde:** a consulta de versões do catálogo.
//!
//! **A variável existe para DESENVOLVER contra outro ambiente**, e é a mesma do hub: mudar o
//! nome dela aqui faria quem já a exporta descobrir, em silêncio, que este app fala com
//! produção enquanto o resto da casa fala com a staging.

/// O endereço de produção.
const PADRAO: &str = "https://api.schematize.net";

/// **O quê:** a base da API, sem a barra final.
///
/// **Onde:** a consulta de versões.
///
/// **Apara a barra final** porque quem compõe a URL escreve `{base}/versions`; com a barra,
/// sairia `//versions` — que alguns servidores aceitam e outros devolvem 404, e descobrir de
/// qual lado se está no meio de um incidente é o tipo de tempo que não se recupera.
pub fn api_base() -> String {
    std::env::var("SCHEMATIZE_API_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| PADRAO.to_string())
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    /// **A barra final é aparada.** Quem compõe escreve `{base}/versions`; com a barra sairia
    /// `//versions`, que alguns servidores aceitam e outros devolvem 404.
    #[test]
    fn a_base_nunca_termina_em_barra() {
        // A env é global ao processo: o teste a restaura para não contaminar os vizinhos.
        let antes = std::env::var("SCHEMATIZE_API_URL").ok();
        // SAFETY: teste de processo único; a env é restaurada ao fim.
        unsafe { std::env::set_var("SCHEMATIZE_API_URL", "https://staging.exemplo/") };
        assert_eq!(super::api_base(), "https://staging.exemplo");
        unsafe { std::env::set_var("SCHEMATIZE_API_URL", "   ") };
        assert!(!super::api_base().ends_with('/'), "vazio cai no padrão, e ele também não tem");
        match antes {
            Some(v) => unsafe { std::env::set_var("SCHEMATIZE_API_URL", v) },
            None => unsafe { std::env::remove_var("SCHEMATIZE_API_URL") },
        }
    }
}
