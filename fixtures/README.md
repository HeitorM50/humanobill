# fixtures

Tudo aqui é **fictício**: marcas (Vivaz Telecom, Vivaz Fibra, Luzia Energia), titular
("Maria Exemplo da Silva"), CPF de teste inválido, endereços e números inventados.
Nada vem de faturas reais (item 6.9 do regulamento, LGPD).

- `templates/<id>.html`: fatura em HTML/CSS autocontido (fonte da verdade do texto).
- `bills/<id>.pdf` e `<id>.png`: geradas a partir dos templates (não editar à mão).
- `extracted/<id>.json`: `ExtractedBill` esperado (sem dados pessoais, valores em centavos;
  `original_text` igual ao texto do template).
- `findings/<id>.json`: achados esperados (spec dos golden tests das regras).
- `tariffs/bandeiras.json`: bandeira tarifária por mês (2025-10..2026-11), com os valores **oficiais da ANEEL** listados em `tariffs/bandeiras-oficiais.md`. O motor de regras embute este arquivo em tempo de compilação (`include_str!`, ENE-05): mudou a tabela, recompile.
- `real/` (ignorada pelo git): faturas reais para testes locais; nunca commitar.

## Cenários de telecom

| id | total | esperado |
|----|-------|----------|
| telecom-1 | R$ 122,70 | TEL-01 x2 (suspeito) |
| telecom-2 | R$ 139,50 | TEL-03, TEL-04 (suspeito enquanto "a validar"; indevido depois de validadas), TEL-05 (suspeito, plano informado R$ 119,90) |
| telecom-3 | R$ 139,90 | nenhum |
| telecom-1-anterior | R$ 99,90 (ref. 2026-09) | nenhum |

A `telecom-1-anterior` é a mesma fatura da `telecom-1` (Vivaz Telecom, plano Vivaz Controle 20GB) no mês **anterior** (09/2026), só com o plano: sem o Clube Vivaz Música e sem o Seguro Tela que a `telecom-1` cobra em outubro. Serve de par para o exemplo de comparação (RF16, botão "Ver um exemplo de comparação" em `/comparar`): contra a `telecom-1`, os dois serviços extras aparecem como "Novo este mês" e o total sobe de R$ 99,90 para R$ 122,70 (+R$ 22,80, +22,8%). Comparada com ela mesma, a API recusa por `same_month`.

## Cenários de energia (Luzia Energia, ref. 2026-10)

| id | total | consumo / média 12m | esperado |
|----|-------|---------------------|----------|
| energia-1 | R$ 331,82 | 324 kWh / 200 kWh (+62%), leitura estimada, ciclo 30 dias | ENE-02, ENE-03 (suspeito) |
| energia-2 | R$ 231,58 | 198 kWh / 200 kWh, leitura real | ENE-01 indevido (multa 5%: excesso R$ 5,17; juros 1% dentro do limite), ENE-07 suspeito (Seguro Residencial Luzia; indevido se o usuário responder "não autorizei") |
| energia-3 | R$ 216,31 | 205 kWh / ~202 kWh, leitura real | ENE-05 suspeito (cobra vermelha_1; ciclo 29/09 a 29/10/2026, ANEEL: setembro amarela, outubro verde) |

Impostos aparecem como linhas somadas ao total (simplificação da fixture).

## Regenerar PDFs/PNGs

```bash
./fixtures/generate.sh   # usa `chromium` (ou CHROMIUM=/caminho/do/chrome)
```

Por baixo: `chromium --headless --no-sandbox --disable-gpu --no-pdf-header-footer --print-to-pdf=bills/<id>.pdf file://.../templates/<id>.html`
e `--screenshot=bills/<id>.png --window-size=900,1300`.

## Validação

`cargo test -p humanobill-core` desserializa cada `extracted/*.json` e roda `validate_bill`.

## Fixture de segurança

| id | O que testa |
|---|---|
| telecom-injecao | A `telecom-1` com um parágrafo pedindo para a IA "ignorar as instruções", zerar o total e marcar tudo como plano (RNF07). A extração esperada é idêntica à da `telecom-1`, e os achados também (golden). |
