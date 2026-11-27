# 11 — Submissão e pitch (material pronto)

Material para as issues #45 (vídeo), #46 (submissão) e #50 (final). O que depende de ação humana está marcado com **👤**.

## 1. Textos da submissão (item 7.2) — copiar e colar no painel

**Nome:** HumanoBill

**Frase de apresentação (≤ 1 linha):**
Sua conta, em português claro: descubra cobranças indevidas na fatura de telecom e luz e conteste com o texto pronto.

**Trilha:** Pagamentos & finanças

**Descrição:**
> Cobrança por serviço não contratado é o problema de consumo nº 1 do Brasil (286 mil casos em 2025, Senacon), e quase ninguém contesta: a fatura é ilegível e o caminho é burocrático. O HumanoBill lê a fatura de celular, internet, TV ou luz (PDF ou foto), explica cada linha em português simples, aponta o que parece indevido com o trecho original e o artigo da lei, calcula quanto dá para recuperar (em dobro, se já foi pago) e entrega a contestação pronta para colar no chat da empresa, com o passo a passo e os prazos (SAC → Anatel → consumidor.gov.br; distribuidora → ouvidoria → ANEEL).
>
> Como funciona: a IA (Gemini) só lê a fatura e devolve um JSON forçado por schema, sem nome, CPF ou endereço. Quem decide é um motor de regras em Rust, determinístico e testado, com 15 regras baseadas no CDC, no RGC da Anatel e na REN ANEEL 1.000/2021. Responder ou corrigir não chama a IA de novo. O arquivo nunca é guardado e o resultado anonimizado expira em 30 dias.
>
> Stack: React + TypeScript (Vercel), API em Rust/Axum (Render), Postgres (Supabase), Gemini. 224 testes em Rust + 43 no front, critérios de aceite em BDD, acessibilidade Lighthouse 100, custo zero.

**Links:**
- Repositório: https://github.com/HeitorM50/humanobill
- Demo: https://humanobill.vercel.app
- Vídeo: https://youtu.be/BwDiTs2VVTY (2:32)

## 2. Checklist do dia da submissão (até 12h de 30/11)

| Item | Como conferir | Quem |
|---|---|---|
| Repo público | `gh repo edit HeitorM50/humanobill --visibility public --accept-visibility-change-consequences` (só com confirmação) | **👤** confirma |
| Demo no ar | `/health` responde `db: ok`; abrir a demo em aba anônima e no celular | automático + **👤** |
| Vídeo ≤ 3:00 abre sem login | abrir o link em aba anônima | **👤** |
| Link do vídeo no README | ✅ feito em 27/11 | — |
| Submissão + aceite do regulamento | painel do hackbrasil.com | **👤** |
| Último commit < 30/11 23h59 | `git log -1 --date=iso` | automático |
| Congelamento 01–11/12 | nenhum commit | **👤** |

## 3. Roteiro do vídeo (≤ 3:00) — versão final

| Tempo | Tela | Fala |
|---|---|---|
| 0:00–0:15 | Fatura de papel com uma linha circulada | "Cobrança de serviço que você não contratou é o problema de consumo número 1 do Brasil. Quase trezentas mil reclamações em 2025. E quase ninguém contesta." |
| 0:15–0:25 | Landing | "O HumanoBill lê sua conta e te diz, em português claro, o que você está pagando — e o que não deveria." |
| 0:25–1:05 | Exemplo *Celular com serviços extras*: resultado → "Você contratou Clube Vivaz Música?" → **Não contratei** → card vira **Indevido**, número sobe | "Ele achou dois serviços extras. Eu digo que não contratei… e pronto: R$ 12,90 indevidos, com o trecho da fatura e o artigo da lei." |
| 1:05–1:25 | "Você já pagou?" → **Sim** → valor dobra | "Se eu já paguei, a lei manda devolver em dobro." |
| 1:25–1:50 | **Contestar agora** → Gerar → Copiar → "Onde reclamar" | "Um clique e eu tenho o texto pronto para colar no chat da operadora, e sei onde reclamar e em quanto tempo eles têm que responder." |
| 1:50–2:15 | Exemplo *Conta de luz alta*: gráfico de 12 meses com o pico, selo "Leitura estimada" | "Funciona também para a conta de luz: consumo 62% acima da média, leitura estimada — dá para pedir revisão." |
| 2:15–2:40 | Diagrama do README + testes passando no terminal | "A IA só lê a fatura. Quem decide é um motor de regras em Rust, testado, que cita a lei. Seus dados pessoais nunca são guardados." |
| 2:40–3:00 | Landing + roadmap | "Próximos passos: extrato bancário, cartão e acompanhar a contestação até o dinheiro voltar. HumanoBill: sua conta, em português claro." |

**Dicas de gravação:** 1080p, cursor grande, legendas; **espere o contador do valor parar** antes de cortar; use os exemplos (não precisa de IA e não depende de quota).

## 4. Pitch na final (12/12) — estrutura de slides

> O formato e a duração chegam por e-mail com os finalistas (02/12). Este roteiro assume ~5 min + perguntas; corte os slides 6 e 8 se for 3 min.

1. **Gancho** — "Quanto você pagou este ano por coisas que não contratou?" + o número (286 mil casos, problema nº 1).
2. **A dor** — fatura ilegível, SVAs escondidos, contestar dá trabalho; persona: Dona Cida (celular) e Wagner (luz alta).
3. **Demo ao vivo** — mesmo fluxo do vídeo (telecom → não contratei → dobro → contestação → luz). **Plano B:** vídeo gravado e demo local.
4. **Como funciona** — "A IA lê, a regra decide": diagrama; privacidade por desenho; prompt injection testado.
5. **Por que dá para confiar** — 15 regras com base legal oficial (CDC, RGC Anatel, REN ANEEL 1.000); regra sem fonte oficial nunca acusa; 267 testes; acessibilidade 100.
6. **Impacto** — público endereçável: todo brasileiro que paga conta de telecom e luz; dinheiro devolvido em dobro; dados coletivos anonimizados viram evidência para Procon/Idec.
7. **Negócio (#64)** — freemium (análise grátis; acompanhamento do protocolo e alertas pagos) ou *success fee* sobre o valor recuperado; canais B2C + B2B2C (Procons, associações de consumidores, bancos digitais); custo marginal baixo (uma chamada de IA por fatura; regras rodam sem IA).
8. **Roadmap** — histórico e alertas → extrato e cartão → WhatsApp/e-mail → acompanhamento até o dinheiro voltar → painel coletivo.
9. **Fechamento** — "HumanoBill: sua conta, em português claro." + QR code da demo.

## 5. Perguntas prováveis do júri

| Pergunta | Resposta curta |
|---|---|
| "E se a IA ler errado?" | A IA só extrai; há validação (soma = total, schema estrito) e a tela "Conferir dados" para corrigir. A acusação vem da regra, não da IA. |
| "Isso é advocacia?" | Não. É informação ao consumidor com base legal citada; o texto é do próprio consumidor e há aviso explícito. |
| "Como vocês sabem que a regra está certa?" | Cada regra cita norma conferida em fonte oficial; regra sem fonte oficial é travada em "suspeito". 224 testes no motor/API, inclusive golden tests. |
| "Dados pessoais?" | O arquivo não é persistido; o schema não tem nome/CPF/endereço; logs só com ids (teste automatizado); resultado expira em 30 dias. |
| "E a quota/custo da IA?" | Uma chamada por fatura; respostas e correções reavaliam só o motor; rate limit por IP; demo funciona sem IA. |
| "Por que Rust?" | Determinismo, tipos fortes para dinheiro (centavos `i64`), binário pequeno com cold start baixo, e os tipos geram o TypeScript do front. |
| "Como ganha dinheiro?" | Ver slide 7. |

## 6. 👤 Logística da final

- Confirmar presença até **04/12 às 23h59** (painel).
- Documento oficial com foto; chegar com notebook carregado e a demo local rodando (README → "Rodando localmente").
- Escolher a feature para construir no dia (sugestão: marcação visual do item sobre a imagem da fatura, issue #51).
