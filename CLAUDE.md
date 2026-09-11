# CLAUDE.md

## Code philosophy

Pragmatismo e beleza acima de complexidade. Código deve ser fácil de ler antes de ser esperto.

- Legibilidade > otimização prematura. Só otimiza com motivo concreto (perf medida, não achismo).
- Prefere solução direta e óbvia a abstração genérica "pra o futuro".
- Código bonito é código que se lê como prosa: nomes claros, funções pequenas, sem indireção desnecessária.
- Simplicidade não é malandragem — é entender o problema fundo o suficiente pra escrever a versão mais simples que resolve.

## Agent skills

### Issue tracker

Issues tracked in GitHub Issues (`matheusbuniotto/caption-captain`), via `gh` CLI. See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at repo root. See `docs/agents/domain.md`.
