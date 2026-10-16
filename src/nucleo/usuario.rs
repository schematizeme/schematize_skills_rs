//! O usuário REAL por trás do processo — "prever macacos" para `sudo`.
//!
//! **O quê:** decide em qual HOME o app age e a quem devolver a posse do que criou.
//!
//! **Onde:** [`super::util::home`] (todo caminho de `~/.claude` nasce dele), a casca da CLI
//! (aviso + devolução de posse) e a chamada ao `schematize core install`.
//!
//! Rodando como root via `sudo`, `$HOME` costuma ser `/root`; gravar lá instalaria as skills
//! para ninguém. O critério é o MESMO do `schematize_cli_rs` (`nucleo::home_real`): `SUDO_USER`
//! não vazio e diferente de `root`, o processo ser root de fato, e o HOME vir do `/etc/passwd`.
//! A decisão é PURA ([`resolver_alvo`]) para ser testada sem `sudo` e sem mexer no ambiente.

use std::path::{Path, PathBuf};

/// Variável que liga o escape: instalar para root de propósito.
pub const ENV_PARA_ROOT: &str = "SCHEMATIZE_ALLOW_ROOT";

/// Onde o app age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alvo {
    pub home: PathBuf,
    /// `Some((uid, gid))` só quando estamos agindo por outro usuário sob sudo.
    pub dono: Option<(u32, u32)>,
    /// O nome do usuário real, quando redirecionado.
    pub usuario: Option<String>,
}

/// O que a decisão precisa saber do mundo (injetável).
pub struct Mundo<'a> {
    pub home_env: Option<String>,
    pub sudo_user: Option<String>,
    pub euid_root: bool,
    pub passwd: &'a str,
    pub para_root: bool,
}

/// **O quê:** a decisão pura de HOME alvo.
pub fn resolver_alvo(m: &Mundo<'_>) -> Alvo {
    let padrao = Alvo {
        home: PathBuf::from(m.home_env.clone().unwrap_or_else(|| ".".into())),
        dono: None,
        usuario: None,
    };
    if m.para_root || !m.euid_root {
        return padrao;
    }
    let Some(user) = m.sudo_user.as_deref().filter(|u| !u.is_empty() && *u != "root") else {
        return padrao;
    };
    match entrada_passwd(m.passwd, user) {
        Some((home, uid, gid)) => {
            Alvo { home, dono: Some((uid, gid)), usuario: Some(user.to_string()) }
        }
        None => padrao,
    }
}

/// `(home, uid, gid)` de `user` num texto no formato do `/etc/passwd`. PURA.
pub fn entrada_passwd(passwd: &str, user: &str) -> Option<(PathBuf, u32, u32)> {
    passwd.lines().find_map(|l| {
        let c: Vec<&str> = l.split(':').collect();
        if c.len() < 6 || c[0] != user || c[5].is_empty() {
            return None;
        }
        Some((PathBuf::from(c[5]), c[2].parse().ok()?, c[3].parse().ok()?))
    })
}

fn ligado(v: Option<String>) -> bool {
    matches!(v.as_deref(), Some("1") | Some("true") | Some("yes") | Some("sim"))
}

/// O alvo do processo atual (lê o ambiente real).
pub fn alvo() -> Alvo {
    let ler = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let home_env = ler("HOME")
        .or_else(|| ler("USERPROFILE"))
        .or_else(|| Some(format!("{}{}", ler("HOMEDRIVE")?, ler("HOMEPATH")?)));
    #[cfg(unix)]
    let (euid_root, passwd) = (
        // SAFETY: geteuid não tem pré-condição nem efeito colateral.
        unsafe { libc::geteuid() } == 0,
        std::fs::read_to_string("/etc/passwd").unwrap_or_default(),
    );
    #[cfg(not(unix))]
    let (euid_root, passwd) = (false, String::new());
    resolver_alvo(&Mundo {
        home_env,
        sudo_user: ler("SUDO_USER"),
        euid_root,
        passwd: &passwd,
        para_root: ligado(ler(ENV_PARA_ROOT)),
    })
}

/// A linha de aviso, quando há redirecionamento.
pub fn aviso(a: &Alvo) -> Option<String> {
    let u = a.usuario.as_deref()?;
    Some(format!("rodando via sudo: instalando para {u} em {}", a.home.display()))
}

/// **O quê:** devolve ao usuário real o que ficou como root sob `raizes`.
///
/// Só toca entradas cujo dono é root (uid 0) — o que o app criou; nada do usuário muda — e não
/// segue symlinks. Retorna quantas entradas mudou.
pub fn devolver_posse(raizes: &[PathBuf], dono: (u32, u32)) -> usize {
    #[cfg(unix)]
    {
        fn anda(p: &Path, d: (u32, u32), n: &mut usize) {
            use std::os::unix::fs::MetadataExt;
            let Ok(md) = std::fs::symlink_metadata(p) else { return };
            if md.uid() == 0 && std::os::unix::fs::lchown(p, Some(d.0), Some(d.1)).is_ok() {
                *n += 1;
            }
            if md.is_dir() {
                if let Ok(rd) = std::fs::read_dir(p) {
                    for e in rd.flatten() {
                        anda(&e.path(), d, n);
                    }
                }
            }
        }
        let mut n = 0;
        for r in raizes {
            anda(r, dono, &mut n);
        }
        n
    }
    #[cfg(not(unix))]
    {
        let _ = (raizes, dono);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str =
        "root:x:0:0:root:/root:/bin/bash\nfulano:x:1001:1002:F:/home/fulano:/bin/bash\n";

    fn mundo(sudo: Option<&str>, root: bool, escape: bool) -> Mundo<'static> {
        Mundo {
            home_env: Some("/root".into()),
            sudo_user: sudo.map(Into::into),
            euid_root: root,
            passwd: PASSWD,
            para_root: escape,
        }
    }

    #[test]
    fn sudo_user_fulano_vai_para_o_home_do_fulano() {
        let a = resolver_alvo(&mundo(Some("fulano"), true, false));
        assert_eq!(a.home, PathBuf::from("/home/fulano"));
        assert_eq!(a.dono, Some((1001, 1002)));
        assert_eq!(aviso(&a).unwrap(), "rodando via sudo: instalando para fulano em /home/fulano");
    }

    #[test]
    fn sem_sudo_user_vale_o_home_do_ambiente() {
        let mut m = mundo(None, true, false);
        m.home_env = Some("/home/outro".into());
        let a = resolver_alvo(&m);
        assert_eq!(a.home, PathBuf::from("/home/outro"));
        assert_eq!(a.dono, None);
        assert!(aviso(&a).is_none());
    }

    #[test]
    fn sudo_user_root_ou_vazio_fica_em_root() {
        for u in ["root", ""] {
            let a = resolver_alvo(&mundo(Some(u), true, false));
            assert_eq!(a.home, PathBuf::from("/root"), "SUDO_USER={u:?}");
            assert_eq!(a.dono, None);
        }
    }

    #[test]
    fn escape_instala_para_root_de_proposito() {
        let a = resolver_alvo(&mundo(Some("fulano"), true, true));
        assert_eq!(a.home, PathBuf::from("/root"));
        assert_eq!(a.dono, None);
    }

    #[test]
    fn sem_ser_root_de_fato_o_sudo_user_herdado_nao_conta() {
        let a = resolver_alvo(&mundo(Some("fulano"), false, false));
        assert_eq!(a.home, PathBuf::from("/root"));
    }

    #[test]
    fn usuario_ausente_do_passwd_cai_no_ambiente() {
        let a = resolver_alvo(&mundo(Some("fantasma"), true, false));
        assert_eq!(a.home, PathBuf::from("/root"));
        assert_eq!(a.dono, None);
    }

    #[test]
    fn escape_le_o_env() {
        assert!(ligado(Some("1".into())));
        assert!(!ligado(Some("0".into())));
        assert!(!ligado(None));
    }
}
