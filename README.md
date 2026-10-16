# schematize-skills

O catálogo de skills do Claude Code, o que está instalado nesta máquina, e a edição.

Extraído do `schematize` na **E5** da extradição (ADR-0018, ADR-0012 F4). Funciona sozinho; o
hub delega a aba de Skills para ele.

```
schematize-skills list            # o catálogo, com o que está instalado
schematize-skills list --json     # o mesmo, para outro programa ler
schematize-skills install rust    # instala uma skill
schematize-skills fork rust       # torna uma skill oficial editável, guardando a base
schematize-skills compare rust    # o que mudou entre o seu fork e a oficial nova
schematize-skills applied         # qual VERSÃO de skill moldou ESTE projeto
schematize-skills desktop --install
```

`schematize skills …` continua funcionando: o hub encaminha para cá.

## Por que é um app à parte

Skill é conteúdo de **terceiro**: qualquer pessoa publica uma, e o contrato dela com a interface
(um botão declarado no `SKILL.md`) é software de fora. Um domínio que aceita extensão de terceiro
tem ciclo próprio de release — prendê-lo ao ciclo do hub faz um conserto de catálogo esperar por
um conserto de janela.

## Rodando com `sudo`

Sob `sudo`, o app detecta o usuário real (`SUDO_USER`, HOME do `/etc/passwd`), instala nele e
devolve a posse do que criou; avisa em uma linha. Para instalar para o root de propósito, use
`--para-root` ou `SCHEMATIZE_ALLOW_ROOT=1`.
