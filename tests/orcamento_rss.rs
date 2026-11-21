//! Orcamento de memoria (piso de eficiencia de runtime) — ver `BUDGET.md`.
//!
//! **O que faz:** sobe o binario `schematize-skills` de verdade, com `HOME` isolado e um `curl`
//! falso no `PATH` que demora 300 ms por chamada (isso mantem o processo vivo tempo suficiente
//! para amostrar `VmHWM` em `/proc/<pid>/status`), e FALHA se o pico passar do teto do BUDGET.
//!
//! **Onde:** `cargo test` (gate local). Sem `/proc` (nao-Linux) o teste avisa e sai — nunca
//! passa mudo e nunca e `#[ignore]`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Teto de RSS pico da CLI, em KiB: medido (~5,6 MiB) + folga larga p/ maquina carregada,
/// muito abaixo dos 256 MiB do piso.
const TETO_KIB: u64 = 16 * 1024;

/// **O que:** le `VmHWM` (pico de RSS, KiB) de `/proc/<pid>/status`. `None` se indisponivel.
///
/// **Onde:** [`medir_pico`].
fn vmhwm_kib(pid: u32) -> Option<u64> {
    let s = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let linha = s.lines().find(|l| l.starts_with("VmHWM:"))?;
    linha.split_whitespace().nth(1)?.parse().ok()
}

/// **O que:** cria `HOME` temporario e um `curl` falso (so Unix) e devolve `(home, bin_dir)`.
///
/// **Onde:** [`medir_pico`].
fn preparar(tag: &str) -> std::io::Result<(PathBuf, PathBuf)> {
    let base = std::env::temp_dir().join(format!("orcamento-rss-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let home = base.join("home");
    let bin = base.join("bin");
    fs::create_dir_all(&home)?;
    fs::create_dir_all(&bin)?;
    let curl = bin.join("curl");
    fs::write(&curl, "#!/bin/sh\nsleep 0.3\necho 1.0.0\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o755))?;
    }
    Ok((home, bin))
}

/// **O que:** roda o binario com `args` e devolve o maior `VmHWM` amostrado (KiB).
///
/// **Onde:** os testes deste arquivo.
fn medir_pico(tag: &str, args: &[&str]) -> Result<u64, String> {
    let (home, bin) = preparar(tag).map_err(|e| format!("preparo: {e}"))?;
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default());
    let mut filho = Command::new(env!("CARGO_BIN_EXE_schematize-skills"))
        .args(args)
        .env("HOME", &home)
        .env("PATH", path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn: {e}"))?;
    let pid = filho.id();
    let (mut pico, limite) = (0u64, Instant::now() + Duration::from_secs(30));
    loop {
        if let Some(v) = vmhwm_kib(pid) {
            pico = pico.max(v);
        }
        if filho.try_wait().map_err(|e| format!("wait: {e}"))?.is_some() {
            break;
        }
        if Instant::now() > limite {
            let _ = filho.kill();
            return Err("processo passou de 30 s".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let _ = fs::remove_dir_all(Path::new(&home).parent().unwrap_or(&home));
    Ok(pico)
}

/// **O que:** afirma o teto para `args`, ou avisa e sai se `/proc` nao existe.
///
/// **Onde:** os testes deste arquivo.
fn afirmar_teto(tag: &str, args: &[&str]) {
    if !Path::new("/proc/self/status").exists() {
        eprintln!("orcamento_rss: PULADO — /proc indisponivel nesta plataforma");
        return;
    }
    let pico = medir_pico(tag, args).expect("medicao");
    eprintln!("orcamento_rss[{tag}]: pico VmHWM = {pico} KiB (teto {TETO_KIB} KiB)");
    assert!(pico > 0, "nenhuma amostra de VmHWM: o teste estaria cego");
    assert!(pico <= TETO_KIB, "{tag}: pico {pico} KiB > teto {TETO_KIB} KiB (ver BUDGET.md)");
}

/// `list --json` (catalogo com consulta de versoes) respeita o orcamento.
#[test]
fn list_json_respeita_orcamento() {
    afirmar_teto("list-json", &["list", "--json"]);
}

/// `list` (texto) respeita o orcamento.
#[test]
fn list_texto_respeita_orcamento() {
    afirmar_teto("list", &["list"]);
}

/// Self-check: a amostragem enxerga um pico plausivel (> 1 MiB); se devolvesse 0 ou lixo, o
/// teste de teto estaria cego e nunca falharia.
#[test]
fn amostragem_enxerga_pico() {
    if !Path::new("/proc/self/status").exists() {
        eprintln!("orcamento_rss: PULADO — /proc indisponivel nesta plataforma");
        return;
    }
    let pico = medir_pico("self-check", &["list", "--json"]).expect("medicao");
    assert!(pico > 1024, "pico {pico} KiB implausivel (< 1 MiB): amostragem quebrada");
}
