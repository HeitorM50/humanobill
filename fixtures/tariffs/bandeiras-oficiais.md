# Bandeiras tarifárias oficiais (out/2025 a nov/2026)

Referência para conferir `fixtures/tariffs/bandeiras.json` (P08) e a regra ENE-05. Cada linha aponta para a notícia oficial da ANEEL em gov.br. Levantado em 11/11/2026.

Unidade do adicional: R$ a cada 100 kWh consumidos. "—" significa que a página da notícia não traz o valor (a cor foi confirmada, o valor não). Verde não tem adicional (R$ 0,00).

| Mês de vigência | Bandeira | Adicional (R$/100 kWh) | Publicada em | Notícia oficial |
|---|---|---|---|---|
| 2025-10 | Vermelha patamar 1 | — | 26/09/2025 | [Outubro terá bandeira vermelha patamar 1](https://www.gov.br/aneel/pt-br/assuntos/noticias/2025/outubro-tera-bandeira-vermelha-patamar-1) |
| 2025-11 | Vermelha patamar 1 | 4,46 | 31/10/2025 | [Bandeira vermelha patamar 1 é mantida em novembro](https://www.gov.br/aneel/pt-br/assuntos/noticias/2025/bandeira-vermelha-patamar-1-e-mantida-em-novembro) |
| 2025-12 | Amarela | — | 28/11/2025 | [ANEEL anuncia bandeira tarifária amarela em dezembro](https://www.gov.br/aneel/pt-br/assuntos/noticias/2025/aneel-anuncia-bandeira-tarifaria-amarela-em-dezembro) |
| 2026-01 | Verde | 0,00 | 23/12/2025 | [Ano novo se inicia com bandeira verde: contas de luz sem custo extra](https://www.gov.br/aneel/pt-br/assuntos/noticias/2025/ano-novo-se-inicia-com-bandeira-verde-contas-de-luz-sem-custo-extra) |
| 2026-02 | Verde | 0,00 | 30/01/2026 | [ANEEL anuncia bandeira tarifária verde em fevereiro](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/aneel-anuncia-bandeira-tarifaria-verde-em-fevereiro) |
| 2026-03 | Verde | 0,00 | 27/02/2026 | [Consumidores brasileiros terão bandeira verde em março](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/consumidores-brasileiros-terao-bandeira-verde-em-marco) |
| 2026-04 | Verde | 0,00 | 27/03/2026 | [Mês de abril terá bandeira verde nas contas de energia elétrica](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/mes-de-abril-tera-bandeira-verde-nas-contas-de-energia-eletrica) |
| 2026-05 | Amarela | — | 24/04/2026 | [Bandeira tarifária para o mês de maio será amarela](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/bandeira-tarifaria-para-o-mes-de-maio-sera-amarela) |
| 2026-06 | Amarela | — | 29/05/2026 | [Bandeira tarifária em junho permanece amarela](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/bandeira-tarifaria-em-junho-permanece-amarela) |
| 2026-07 | Amarela | 1,885 | 26/06/2026 | [Bandeira tarifária em julho permanece amarela](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/bandeira-tarifaria-em-julho-permanece-amarela-conta-de-luz-dos-consumidores-continua-tendo-acrescimo-de-r-1-885-a-cada-100-kwh) |
| 2026-08 | Amarela | 1,885 | 31/07/2026 | [Bandeira tarifária continua amarela em agosto](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026-defeso-eleitoral/bandeira-tarifaria-continua-amarela-em-agosto) |
| 2026-09 | Amarela | 1,885 | 28/08/2026 | [ANEEL mantém bandeira tarifária amarela em setembro](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026-defeso-eleitoral/aneel-mantem-bandeira-tarifaria-amarela-em-setembro) |
| 2026-10 | Verde | 0,00 | 25/09/2026 | [Outubro terá bandeira verde: contas de luz sem custo extra](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/outubro-tera-bandeira-verde-contas-de-luz-sem-custo-extra) |
| 2026-11 | Verde | 0,00 | 30/10/2026 | [ANEEL anuncia bandeira verde para vigorar em novembro](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/aneel-anuncia-bandeira-verde-para-vigorar-em-novembro) |

## Observações

- Dezembro/2026 será anunciada em 27/11/2026 e janeiro/2027 em 23/12/2026, conforme o [calendário de acionamento de 2026](https://www.gov.br/aneel/pt-br/assuntos/noticias/2026/bandeira-tarifaria-aneel-divulga-calendario-de-acionamento-para-2026).
- Valores de referência por kWh na [página de bandeiras da ANEEL](https://www.gov.br/aneel/pt-br/assuntos/tarifas/bandeiras-tarifarias) (publicada em 2022, pode estar desatualizada): amarela R$ 0,01885, vermelha patamar 1 R$ 0,04463, vermelha patamar 2 R$ 0,07877. A amarela de 2026 (R$ 1,885 por 100 kWh) e a vermelha patamar 1 de nov/2025 (R$ 4,46 por 100 kWh) batem com essa página. Vermelha patamar 2 não aparece em nenhum mês deste período.
- As notícias de maio e junho/2026 só informam a cor; o valor (R$ 1,885) aparece nas notícias de julho a setembro. Assumir R$ 1,885 para maio e junho é inferência pela página de bandeiras, não está na notícia.
- Regra de aplicação quando o ciclo de faturamento cobre dois meses (proporcional aos dias?): **não confirmada** em fonte oficial. Ver nota na ENE-05 de `docs/03-regras-negocio.md`.
- As páginas da ANEEL mostram "última modificação" em julho/2026 por reorganização do portal; a data de publicação acima é a original da notícia.
