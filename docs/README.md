# HumanoBill — Documentação

> **Sua conta, em português claro.**
> Você envia a fatura de celular, internet, TV ou luz. O HumanoBill explica cada linha, aponta cobranças indevidas com a base legal e entrega a contestação pronta.

Projeto do Hack Brasil 2026, trilha **Pagamentos & finanças**, time solo (Heitor).
Regulamento: [`../REGULAMENTO.md`](../REGULAMENTO.md). Fonte oficial: [hackbrasil.com](https://hackbrasil.com).

## Resumo em 1 minuto

| | |
|---|---|
| **Problema** | Cobrança por serviço não contratado é o problema de consumo nº 1 do Brasil em 2025: 286 mil casos, 28,7% do total no Sindec. Faturas são ilegíveis, e contestar dá trabalho. |
| **Para quem** | Pessoa física que paga conta de telecom e de energia e não sabe se está sendo cobrada certo. |
| **Solução** | A IA lê a fatura e gera um JSON estruturado. Um **motor de regras determinístico** encontra os problemas, cita a lei e calcula quanto dá para recuperar. O app gera a contestação pronta, com o passo a passo de onde reclamar. |
| **Diferencial** | A IA não "acha" nada. Quem acusa é a regra testada, com o artigo da lei, o que dá confiança e auditabilidade. A privacidade é by design: nome, CPF e endereço nunca são extraídos. |
| **Stack** | React + TS (Vercel) · API em Rust/Axum (Render) · Supabase · Gemini |
| **Prazo** | Submissão em **30/11 às 23h59** · final em 12/12 (SP) |

## Índice

| Doc | Conteúdo |
|---|---|
| [01 — Visão de produto](./01-visao-produto.md) | Problema, dados, público, concorrência, riscos de flop, enquadramento na trilha |
| [02 — Requisitos](./02-requisitos.md) | Dores (D01–D12), RFs com MoSCoW, RNFs, critérios BDD (Gherkin) e rastreabilidade |
| [03 — Regras de negócio](./03-regras-negocio.md) | Catálogo de regras de telecom e energia, com base legal |
| [04 — Arquitetura](./04-arquitetura.md) | Componentes, fluxo, API, schema, dados, LGPD, segurança |
| [05 — Stack](./05-stack.md) | Cada tecnologia, por que foi escolhida, custos e limites |
| [06 — Roadmap](./06-roadmap.md) | Cronograma diário até 30/11, congelamento, final e expansão |
| [07 — Design e UI](./07-design-ui.md) | Guia de estilo e design system: cores, tipografia, componentes, logo, telas, estados e guia de implementação |
| [08 — Demo, vídeo e pitch](./08-demo-video-pitch.md) | Faturas sintéticas, roteiro de 3 min, checklist da submissão |
| [09 — Decisões](./09-decisoes.md) | Log de decisões (quem decidiu, quando e por quê) |
| [10 — Pontos em aberto](./10-pontos-em-aberto.md) | O que ainda falta fechar |
| [11 — Submissão e pitch](./11-submissao-e-pitch.md) | Textos da submissão, roteiro final do vídeo, slides e perguntas do júri |
