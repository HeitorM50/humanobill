# 08 — Demo, vídeo e pitch

## Faturas sintéticas (item 6.9: nenhum dado real)

Marcas **fictícias** com layout realista. Clientes, endereços e números são todos inventados (ex.: "Maria Exemplo da Silva", CPF de teste inválido).

| Fixture | Marca | Cenário | Achados esperados |
|---|---|---|---|
| `telecom-1` | Vivaz Telecom (celular) | 2 SVAs escondidos ("Clube Vivaz Música" R$ 12,90, "Seguro Tela" R$ 9,90) | TEL-01 ×2 (`suspeito` → `indevido` ao responder "não contratei") |
| `telecom-2` | Vivaz Fibra (internet + TV) | Fatura paga com atraso: multa de 5% + juros de 3% ao mês; mensalidade acima do plano | TEL-03, TEL-04 (`indevido`), TEL-05 (`suspeito`) |
| `telecom-3` | Vivaz Telecom | Conta limpa | Nenhum achado. Testa o estado "tudo certo" e garante zero falso positivo |
| `energia-1` | Luzia Energia | Consumo 62% acima da média de 12 meses + leitura estimada | ENE-02, ENE-03 (`suspeito`) |
| `energia-2` | Luzia Energia | "Seguro Residencial Luzia" embutido + multa acima de 2% | ENE-07, ENE-01 |
| `energia-3` | Luzia Energia | Bandeira vermelha cobrada num mês de bandeira verde (tabela fixture) | ENE-05 |

**Como gerar:** um template HTML/CSS por marca → PDF (via navegador headless ou print). Gerar também uma versão "foto" (PNG levemente inclinado) para mostrar que a leitura funciona com imagem. Os templates ficam em `fixtures/templates/` e são versionados.

## Roteiro do vídeo (≤ 3:00, em português)

| Tempo | Cena | Fala (rascunho) |
|---|---|---|
| 0:00–0:20 | Close de uma fatura real de papel (sem dados visíveis) com um item circulado | "Cobrança por serviço que você não contratou é o problema de consumo número 1 do Brasil: quase 300 mil reclamações só em 2025. E quase ninguém confere a própria conta." |
| 0:20–0:35 | Landing do HumanoBill | "O HumanoBill lê sua fatura e te diz, em português claro, o que você está pagando e o que não deveria pagar." |
| 0:35–1:20 | Fluxo `telecom-1`: upload → progresso → resultado com R$ animado → pergunta "você contratou?" → "não" → vira `indevido` | Narrar o que aparece. Mostrar a base legal |
| 1:20–1:50 | Contestação: copiar texto + passo a passo | "Em um clique, você tem a contestação pronta e sabe exatamente onde reclamar e em quanto tempo eles têm que responder." |
| 1:50–2:15 | `energia-1`: gráfico de 12 meses, consumo atípico | "Funciona também para a conta de luz." |
| 2:15–2:40 | Diagrama da arquitetura + trecho dos testes passando | "A IA só lê a fatura. Quem decide é um motor de regras em Rust, testado, que cita a lei. Seus dados pessoais nunca são guardados." |
| 2:40–3:00 | Roadmap + fechamento | "Próximos passos: extrato bancário, cartão e acompanhamento da contestação até o dinheiro voltar. HumanoBill: sua conta, em português claro." |

**Dicas:** gravar a tela em 1080p com o cursor grande, o narrador em off com microfone decente e legendas. Subir no YouTube como **não listado** (abre sem login). Testar o link numa aba anônima.

## Pitch na final (12/12)

- O formato chega por e-mail com a lista de finalistas (item 8.3). Adaptar a duração depois.
- Estrutura: dor (com número) → demo ao vivo (com plano B: vídeo gravado) → como funciona (IA + regras) → impacto → roadmap/negócio → pedido.
- Levar a demo rodando **localmente também** (caso a rede do local falhe).

## Checklist da submissão (item 7.2): conferir em 29/11

- [ ] Nome: **HumanoBill**
- [ ] Frase de apresentação: *"Sua conta, em português claro: descubra cobranças indevidas na fatura de telecom e luz e conteste com o texto pronto."*
- [ ] Descrição (problema, solução, como funciona, tecnologias) — texto pronto em [11 — Submissão e pitch](./11-submissao-e-pitch.md)
- [ ] Trilha: **Pagamentos & finanças**
- [ ] Link do repo GitHub (público, criado depois de 04/11 às 21h, MIT)
- [ ] Link da demo (Vercel) testado em aba anônima e no celular
- [ ] Link do vídeo ≤ 3 min, abre sem login
- [ ] Aceite do regulamento no painel
- [ ] README do repo com screenshots, arquitetura e "como rodar"
- [ ] Keep-alive ativo e alerta do UptimeRobot configurado
- [ ] Último commit antes de 30/11 às 23h59
