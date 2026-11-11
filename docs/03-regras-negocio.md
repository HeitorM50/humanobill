# 03 — Regras de negócio (motor de regras)

## Princípios

1. **Determinístico.** A mesma fatura extraída sempre gera os mesmos achados. Nenhuma regra chama a IA.
2. **Explicável.** Todo achado traz `explicação`, `valor_envolvido` e `base_legal` (texto + link).
3. **Conservador.** `indevido` só sai quando a violação é objetiva e verificável na própria fatura. Na dúvida, o achado é `suspeito`.
4. **Interativo.** Algumas regras dependem do usuário ("você contratou isso?"). Sem resposta, o achado fica `suspeito`.
5. **Status da regra.** `validada` tem fonte oficial conferida. `a validar` tem fonte secundária (Idec, imprensa). Uma regra `a validar` **nunca** emite `indevido`.

## Severidades

| Severidade | Quando | Exemplo |
|---|---|---|
| `indevido` | Regra objetiva violada | Multa de 5% por atraso |
| `suspeito` | Indício forte que depende de confirmação | SVA que o usuário não confirmou ter contratado |
| `ok` | Item conferido sem problema | Mensalidade igual ao plano informado |

## Cálculo de "quanto dá para recuperar"

- Fatura **não paga:** recuperável = soma dos `valor_envolvido` dos achados `indevido`, mais os `suspeito` que o usuário confirmar.
- Fatura **paga:** recuperável = 2 × esse valor (CDC art. 42, parágrafo único), mais juros e correção. **No MVP, só o dobro, sem correção**, para manter o cálculo simples e transparente.
- Recorrência: se o item aparece em N meses (informado pelo usuário), mostrar "≈ R$ X × N meses". É só uma estimativa.

---

## Telecom (celular, internet, TV)

| ID | Regra | Entrada | Severidade | Base legal | Status |
|---|---|---|---|---|---|
| TEL-01 | **SVA / serviço adicional:** item classificado como serviço adicional (conteúdo, seguro, "clube", "pacote digital" etc.) que não faz parte do plano | itens + resposta "contratou?" | `suspeito`, ou `indevido` se o usuário responder "não contratei" | [CDC art. 39, III e parágrafo único](https://www.planalto.gov.br/ccivil_03/leis/l8078compilado.htm) (serviço fornecido sem solicitação prévia equipara-se a amostra grátis, "inexistindo obrigação de pagamento"); [RGC (Res. Anatel 765/2023) art. 48, caput e §§ 1º e 2º](https://informacoes.anatel.gov.br/legislacao/resolucoes/2023/1900-resolucao-765) (cobrança de serviço de valor adicionado depende de autorização prévia e expressa; a prestadora prova a autorização; [explicação oficial da Anatel](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/cobranca)) | validada |
| TEL-02 | **Cobrança duplicada:** dois itens com descrição normalizada igual e mesmo valor no mesmo período | itens | `suspeito` | CDC art. 42 | validada (lógica) |
| TEL-03 | **Multa por atraso acima de 2%** sobre o valor em atraso | multa, valor base | `indevido` | [RGC (Res. Anatel 765/2023) art. 78, § 1º](https://informacoes.anatel.gov.br/legislacao/resolucoes/2023/1900-resolucao-765) (multa de até 2% sobre o débito; [explicação oficial da Anatel](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/suspensao)). O CDC art. 52, § 1º (2%) trata de fornecimento que envolve outorga de crédito ou financiamento; não há fonte oficial dizendo que alcança fatura de telecom, então não é o fundamento | validada |
| TEL-04 | **Juros de mora acima de 1% ao mês** (pro rata) | juros, valor base, dias de atraso | `indevido` | [RGC (Res. Anatel 765/2023) art. 78, § 1º](https://informacoes.anatel.gov.br/legislacao/resolucoes/2023/1900-resolucao-765) (juros de até 1% ao mês, mais correção; [explicação oficial da Anatel](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/suspensao)). O [CC art. 406](https://www.planalto.gov.br/ccivil_03/_ato2023-2026/2024/lei/l14905.htm) (Lei 14.905/2024: Selic menos IPCA) só vale quando os juros não são convencionados nem fixados em norma, e o CTN art. 161, § 1º (1%) é tributário: nenhum dos dois é o teto | validada |
| TEL-05 | **Mensalidade acima do plano contratado** | itens + valor do plano informado | `suspeito` | CDC art. 30 (oferta vincula) | validada (lógica) |
| TEL-06 | **Cobrança após cancelamento:** fatura com período depois da data de cancelamento informada | período + data informada | `indevido` | [RGC (Res. Anatel 765/2023) art. 83, parágrafo único](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/cancelamento) (pedido com atendente tem efeito imediato e a prestadora não pode cobrar serviço posterior ao pedido); [art. 84, caput e § 1º](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/manual-operacional) (pedido sem atendente tem efeito após 2 dias úteis, e os serviços usufruídos nesse prazo são devidos); [art. 58](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/manual-operacional) (cobrança proporcional ao período usufruído até a rescisão) | validada |
| TEL-07 | **Soma não bate:** soma dos itens ≠ total (tolerância de R$ 0,05) | itens, total | `suspeito` | — (consistência) | validada (lógica) |
| TEL-08 | **Item sem descrição clara:** genérico ("outros", "serviços diversos", código sem nome) | itens | `suspeito` | [CDC art. 6º, III](https://www.planalto.gov.br/ccivil_03/leis/l8078compilado.htm) (informação adequada e clara sobre os serviços e o preço); [RGC (Res. Anatel 765/2023) art. 55, I](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/manual-operacional) (a fatura traz o valor total de cada serviço e facilidade cobrados) e [art. 48, caput](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/cobranca) (serviços de valor adicionado discriminados e detalhados). Base legal validada; a severidade segue `suspeito` porque "descrição clara" é juízo subjetivo | validada |

**Prazos exibidos na contestação (RGC, Res. Anatel 765/2023, em vigor integral desde 01/09/2025):** contestar em até 3 anos da cobrança (art. 60); a operadora responde em até 30 dias (art. 61, § 1º); se a contestação for procedente ou não houver resposta no prazo, ela deve cancelar a cobrança ainda não paga ou devolver automaticamente o valor já pago (art. 62), e a devolução do valor pago é em dobro, com correção monetária e juros de 1% ao mês (art. 64) ([fonte oficial: Anatel, Cobrança](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/cobranca); art. 64 literal no [Manual Operacional da Anatel](https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/manual-operacional)). O dobro só vale para valor **já pago**: fatura não paga é cancelada ou reemitida, sem dobro.

**Lista de SVA (heurística da TEL-01):** dicionário de palavras-chave (`seguro`, `clube`, `conteúdo`, `banca`, `música`, `jogos`, `antivírus`, `backup`, `cloud`, `assistência`, `proteção` etc.) e classificação feita pela IA na extração (`categoria: "servico_adicional"`). A regra combina as duas: basta que uma delas aponte para o item virar `suspeito`.

---

## Energia elétrica (residencial, baixa tensão)

> ⚠️ Todas as regras de energia estão **a validar** até a conferência no texto oficial da **REN ANEEL nº 1.000/2021**. Até lá, elas emitem no máximo `suspeito`. Ver [10 — Pontos em aberto](./10-pontos-em-aberto.md).

| ID | Regra | Entrada | Severidade | Base legal (a conferir) | Status |
|---|---|---|---|---|---|
| ENE-01 | **Multa de mora acima de 2% / juros acima de 1% ao mês** | multa, juros, base | `suspeito` (`indevido` depois de validar) | REN 1000 (acréscimos moratórios) | a validar |
| ENE-02 | **Consumo atípico:** consumo do mês > média dos 12 meses do histórico × 1,4 (ou desvio > 2σ) | consumo kWh + histórico 12 meses (impresso na fatura) | `suspeito` | REN 1000 (revisão de faturamento) | a validar |
| ENE-03 | **Leitura não real:** fatura marcada como leitura estimada/média, principalmente se recorrente | tipo de leitura | `suspeito` | REN 1000 (faturamento por média tem hipóteses limitadas) | a validar |
| ENE-04 | **Dias de faturamento fora do padrão:** ciclo < 27 ou > 33 dias | datas de leitura anterior/atual | `suspeito` | REN 1000 (ciclo de leitura) | a validar |
| ENE-05 | **Bandeira tarifária inconsistente:** bandeira cobrada ≠ bandeira vigente no mês de referência | bandeira na fatura + tabela de bandeiras ANEEL | `suspeito` | ANEEL (bandeiras mensais) | a validar |
| ENE-06 | **Recálculo da tarifa:** kWh × tarifa homologada (TE + TUSD) da distribuidora ≠ valor cobrado (tolerância de 2% por tributos) | consumo, tarifa, distribuidora | `suspeito` | ANEEL (tarifas homologadas, dados abertos) | a validar, **stretch** |
| ENE-07 | **Itens não relacionados à energia:** seguros, doações, assinaturas embutidas na conta de luz | itens | `suspeito` (`indevido` se o usuário disser que não autorizou) | CDC art. 39, III | a validar |
| ENE-08 | **Soma não bate** (igual à TEL-07) | itens, total | `suspeito` | — | validada (lógica) |

**Notas:**
- A ENE-05 e a ENE-06 dependem de tabelas externas (bandeiras por mês e tarifas por distribuidora). No MVP, a tabela vem **embutida como fixture** para as distribuidoras fictícias e para os meses da demo. A ingestão dos dados abertos da ANEEL fica para a expansão.
- A devolução em dobro na conta de luz **não está confirmada** na regulação setorial. O MVP usa o CDC art. 42 com a ressalva "pode depender de análise".

---

## Contrato de uma regra (para o código)

```text
Rule {
  id: "TEL-01",
  bill_kind: Telecom | Energy | Any,
  status: Validated | Pending,          // Pending nunca emite Unlawful
  needs_user_input: Option<Question>,   // ex.: "Você contratou {item}?"
  evaluate(bill: &ExtractedBill, answers: &Answers) -> Vec<Finding>
}

Finding {
  rule_id, severity: Unlawful | Suspicious | Ok,
  title, explanation, item_refs: Vec<ItemId>,
  amount_cents: i64, legal_basis: Vec<LegalRef { label, url }>
}
```
Os valores vão sempre em **centavos (`i64`)**, nunca float.
