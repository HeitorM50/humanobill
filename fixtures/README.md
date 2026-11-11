# fixtures

Tudo aqui é **fictício**: marcas (Vivaz Telecom, Vivaz Fibra, Luzia Energia), titular
("Maria Exemplo da Silva"), CPF de teste inválido, endereços e números inventados.
Nada vem de faturas reais (item 6.9 do regulamento, LGPD).

- `templates/<id>.html`: fatura em HTML/CSS autocontido (fonte da verdade do texto).
- `bills/<id>.pdf` e `<id>.png`: geradas a partir dos templates (não editar à mão).
- `extracted/<id>.json`: `ExtractedBill` esperado (sem dados pessoais, valores em centavos;
  `original_text` igual ao texto do template).
- `findings/<id>.json`: achados esperados (spec dos golden tests das regras).
- `tariffs/`: tarifas de referência.
- `real/` (ignorada pelo git): faturas reais para testes locais; nunca commitar.

## Cenários de telecom

| id | total | esperado |
|----|-------|----------|
| telecom-1 | R$ 122,70 | TEL-01 x2 (suspeito) |
| telecom-2 | R$ 139,50 | TEL-03, TEL-04 (indevido), TEL-05 (suspeito, plano informado R$ 119,90) |
| telecom-3 | R$ 139,90 | nenhum |

## Regenerar PDFs/PNGs

```bash
./fixtures/generate.sh   # usa `chromium` (ou CHROMIUM=/caminho/do/chrome)
```

Por baixo: `chromium --headless --no-sandbox --disable-gpu --no-pdf-header-footer --print-to-pdf=bills/<id>.pdf file://.../templates/<id>.html`
e `--screenshot=bills/<id>.png --window-size=900,1300`.

## Validação

`cargo test -p humanobill-core` desserializa cada `extracted/*.json` e roda `validate_bill`.
