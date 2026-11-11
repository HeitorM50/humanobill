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

Requisitos: Rust estável, Node 24+ e Docker (para o Postgres local).

```bash
# 1. Banco local
docker run -d --name hb-pg -e POSTGRES_PASSWORD=pg -e POSTGRES_DB=humanobill -p 55432:5432 postgres:17-alpine

# 2. API em http://localhost:8099 (as migrations rodam sozinhas)
DATABASE_URL=postgres://postgres:pg@localhost:55432/humanobill \
CORS_ORIGIN=http://localhost:5173 PORT=8099 \
cargo run -p humanobill-api
# Opcional: GEMINI_API_KEY=... para analisar faturas enviadas (sem ela, só os exemplos funcionam)

# 3. Front em http://localhost:5173
cd apps/web && npm install && VITE_API_URL=http://localhost:8099 npm run dev
```

Testes: `cargo test --all` (inclui a validação das faturas sintéticas de `fixtures/`).
Teste da extração real (gasta quota): `GEMINI_API_KEY=... cargo test -p humanobill-extract -- --ignored live`.

## Documentação

Toda a documentação do projeto está em [`docs/`](./docs/README.md).

## Licença

[MIT](./LICENSE)
