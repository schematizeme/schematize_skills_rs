# BUDGET — orcamento de memoria (piso de eficiencia de runtime)

Referencia: `skill_engineering_src/references/eficiencia-runtime.md`. Teto = medido + folga, nunca
acima de 256 MiB (CLI) / 400 MiB (GUI). Faz valer: `tests/orcamento_rss.rs` (le `VmHWM` de
`/proc/<pid>/status` do binario real, `HOME` isolado, `curl` falso de 300 ms; falha acima do teto).

| Operacao | RSS pico alvo (teto) | RSS medido antes | RSS medido depois | Como medir |
|---|---|---|---|---|
| CLI `schematize-skills list --json` | 16 MiB | 5,4 MiB | 5,4 MiB (sem mudanca de codigo) | `cargo test --no-default-features --test orcamento_rss -- --nocapture` |
| CLI `schematize-skills list` | 16 MiB | 5,6 MiB | 5,6 MiB | idem |
| GUI `schematize-skills-gui` | 400 MiB | nao medido: precisa de display | nao medido | abrir a janela e ler `VmHWM` do pid |

Notas:
- Nao houve reducao: o diagnostico (ver relatorio de 2026-11-21) nao achou causa real a corrigir na CLI
  (43 pacotes, 1,5 MB de binario, `install` ja faz streaming via `curl -o`, leituras sao de arquivos
  de skill pequenos). Orcamento registra o estado e impede regressao.
- GUI: 469 pacotes (Slint+winit+femtovg+accessibility). Reduzir features mudaria comportamento
  (acessibilidade) — fica como plano: medir `VmHWM` com display antes de qualquer corte.
