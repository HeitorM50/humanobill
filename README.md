# HumanoBill

> **Sua conta, em português humano.**
> Envie a fatura de celular, internet, TV ou luz: o HumanoBill explica cada linha, aponta cobranças indevidas com base legal e entrega a contestação pronta.

Projeto do **Hack Brasil 2026**, trilha *Pagamentos & finanças*.

🚧 Em construção. Veja o [roadmap](./docs/06-roadmap.md) e o [📊 Painel do MVP](https://github.com/HeitorM50/humanobill/issues/65).

## Como funciona

1. **A IA lê** a fatura e devolve um JSON estruturado. Nome, CPF e endereço nunca são extraídos.
2. **Um motor de regras em Rust decide**: é determinístico, testado e cita a lei de cada achado.
3. **Você recebe** a explicação de cada item, quanto dá para recuperar e o texto da contestação.

## Estrutura

```
apps/web/        front-end (Vite + React + TypeScript + Tailwind)
crates/core/     tipos de domínio compartilhados
crates/rules/    motor de regras (puro, sem I/O)
crates/extract/  extração via IA (Gemini)
crates/api/      API HTTP (Axum)
fixtures/        faturas sintéticas e resultados esperados
migrations/      SQL do banco (Supabase/Postgres)
docs/            produto, requisitos, arquitetura, roadmap
```

## Rodando localmente

Requisitos: Rust estável e Node 24+.

```bash
# API em http://localhost:8080/health
cargo run -p humanobill-api

# Front em http://localhost:5173
cd apps/web && npm install && npm run dev
```

Testes: `cargo test --all`.

## Documentação

Toda a documentação do projeto está em [`docs/`](./docs/README.md).

## Licença

[MIT](./LICENSE)
