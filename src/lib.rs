//! **schematize-skills** — o catálogo, o que está instalado, e a edição.
//!
//! **O quê:** instala, atualiza, remove e edita skills do Claude Code; conhece o catálogo da
//! casa e o que cada projeto declara precisar.
//!
//! **Onde:** o binário `schematize-skills`, a janela dele, e o hub — que delega a aba de
//! Skills para cá (E5 do ADR-0018, ADR-0012 F4).
//!
//! ## Por que é um app à parte
//!
//! Skill é conteúdo de terceiro: qualquer pessoa publica uma, e o contrato dela com a
//! interface (um botão declarado no `SKILL.md`) é software de fora. Um domínio que aceita
//! extensão de terceiro tem ciclo próprio de release — prendê-lo ao ciclo do hub faz um
//! conserto de catálogo esperar por um conserto de janela.

pub mod nucleo;

pub mod catalogo;
pub mod guiactions;
pub mod mercado;
pub mod registry;
pub mod skilledit;
pub mod skills;
pub mod skillsproj;
