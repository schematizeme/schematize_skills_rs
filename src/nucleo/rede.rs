//! A rede — PLATAFORMA: baixar texto e ler a versão publicada de um repo.
//!
//! **O quê:** o `curl` com timeout e User-Agent, e o parser da versão.
//!
//! **Onde:** o catálogo (que lê a versão de cada skill) e o instalador.
//!
//! ## Timeout SEMPRE, e por quê
//!
//! Sem `-m`, um `curl` contra um host que aceita a conexão e não responde fica pendurado para
//! sempre — e este app roda dentro de uma janela, onde "pendurado" significa uma tela que não
//! volta. O limite é curto de propósito: quem está offline recebe "não sei" em segundos, e
//! "não sei" é uma resposta que a tela sabe mostrar.

/// Segundos de teto para qualquer leitura de rede.
const TIMEOUT: &str = "6";
/// Alguns proxies e o próprio GitHub tratam pedido sem User-Agent de forma diferente.
const UA: &str = "User-Agent: schematize-skills";

/// **O quê:** baixa um texto. `None` em qualquer falha — inclusive HTTP 404 (`-f`).
///
/// **Onde:** [`versao_publicada`].
///
/// **O `-f` é o que separa "não existe" de "existe e é uma página de erro".** Sem ele, o corpo
/// do 404 do GitHub viria como sucesso, e um HTML entraria no lugar de um número de versão.
pub fn texto(url: &str) -> Option<String> {
    super::util::run("curl", &["-sfL", "-m", TIMEOUT, "-H", UA, url]).ok()
}

/// **O quê:** a versão publicada de um repo da casa.
///
/// **Onde:** o catálogo de skills.
///
/// **Cada skill guarda a versão num arquivo `VERSION` na raiz**, e não numa tag: skill é
/// conteúdo, não binário, e versionar por arquivo deixa qualquer um ler sem a API do GitHub.
pub fn versao_publicada(org: &str, repo: &str) -> Option<String> {
    let url = format!("https://raw.githubusercontent.com/{org}/{repo}/main/VERSION");
    parse_versao(&texto(&url)?)
}

/// **O quê:** a versão a partir do corpo do arquivo. PURA.
///
/// **Onde:** [`versao_publicada`], e os testes.
///
/// **Corpo vazio ou página de erro NÃO viram versão.** Uma string qualquer aqui faria o app
/// se achar desatualizado para sempre, e oferecer uma atualização que nunca conclui.
pub fn parse_versao(body: &str) -> Option<String> {
    let v = body.trim();
    let valido = !v.is_empty()
        && v.len() <= 32
        && v.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
        && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+');
    valido.then(|| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Corpo vazio, HTML ou lixo NÃO viram versão.** Uma string qualquer faria o app se
    /// achar desatualizado para sempre, oferecendo uma atualização que nunca conclui.
    #[test]
    fn corpo_ruim_nao_vira_versao() {
        assert_eq!(parse_versao("1.4.2").as_deref(), Some("1.4.2"));
        assert_eq!(parse_versao("  1.4.2\n").as_deref(), Some("1.4.2"), "espaço em volta some");
        assert_eq!(parse_versao("1.0.0-rc.1").as_deref(), Some("1.0.0-rc.1"));
        for ruim in ["", "   ", "<!DOCTYPE html>", "404: Not Found", "v1.2.3", "latest", "\u{0}"] {
            assert_eq!(parse_versao(ruim), None, "{ruim:?}");
        }
        // Um corpo enorme (a página de erro inteira) também não passa.
        assert_eq!(parse_versao(&"9".repeat(500)), None);
    }
}
