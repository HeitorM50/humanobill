# scripts/aneel: tabelas de bandeiras e tarifas a partir dos dados abertos da ANEEL

O script `gerar_tarifas.py` baixa dois conjuntos do portal de dados abertos da ANEEL e regera os arquivos que o motor de regras embute com `include_str!` (sem I/O na hora de avaliar uma fatura):

| Arquivo gerado | Regra | Conteúdo |
|---|---|---|
| `fixtures/tariffs/bandeiras.json` | ENE-05 | Bandeira acionada em cada mês, desde 2024-01 (`--desde` muda o início) |
| `fixtures/tariffs/tarifas.json` | ENE-06 | TE e TUSD residenciais (B1, convencional) das 3 vigências mais recentes de 5 distribuidoras grandes: Cemig, Enel SP (Eletropaulo), Coelba, Copel e CPFL Paulista |

## Como rodar

Python 3.9 ou mais novo, só a biblioteca padrão. Da raiz do repositório:

```bash
python3 scripts/aneel/gerar_tarifas.py
cargo test -p humanobill-rules   # confere a tabela de bandeiras contra bandeiras-oficiais.md
```

O CSV de tarifas tem cerca de 80 MB e é baixado para uma pasta temporária, que o script apaga no fim. Para não baixar de novo, passe uma pasta com os dois CSVs já baixados: `--dir-csv pasta/`.

Se uma bandeira ou uma tarifa vier num formato que o script não conhece, ele para com erro e não grava nada. Não há valor inventado nem preenchido à mão: tudo vem do CSV, exceto a lista de nomes de cada distribuidora (veja abaixo).

## Fonte

- Bandeiras: conjunto [Bandeiras Tarifárias](https://dadosabertos.aneel.gov.br/dataset/bandeiras-tarifarias), recurso "Bandeira Tarifária - Acionamento" (`bandeira-tarifaria-acionamento.csv`).
- Tarifas: conjunto [Tarifas de aplicação das distribuidoras de energia elétrica](https://dadosabertos.aneel.gov.br/dataset/tarifas-distribuidoras-energia-eletrica), recurso `tarifas-homologadas-distribuidoras-energia-eletrica.csv`.

O script acha a URL de cada CSV pela API do portal (`/api/3/action/package_show`). Se a API não responder, ele usa a URL conhecida. Cada JSON guarda a `url`, a data de geração do conjunto pela ANEEL (`data_geracao_conjunto`) e a data da coleta (`coletado_em`).

**Última coleta:** 20/11/2026 (bandeiras geradas pela ANEEL em 16/11/2026; tarifas em 20/11/2026). A bandeira de dezembro/2026 sai em 27/11/2026: rode o script de novo depois disso.

## Filtro das tarifas

Linhas com: base tarifária "Tarifa de Aplicação", subgrupo B1, classe e subclasse "Residencial", modalidade "Convencional", detalhe "Não se aplica" (tira a tarifa de quem tem geração distribuída, SCEE) e posto "Não se aplica". Os valores ficam em R$/MWh, sem ICMS, sem PIS/COFINS e sem o adicional de bandeira.

A ANEEL às vezes divide uma vigência em duas sem mudar o valor (por exemplo, na virada do ano). O script junta essas vigências. Se encontrar duas tarifas diferentes para o mesmo período, ou períodos sobrepostos, ele para com erro.

## Nomes das distribuidoras

O dado aberto traz só a sigla (por exemplo, `CEMIG-D` ou `ELETROPAULO`) e o CNPJ. Os nomes que aparecem na fatura ("Cemig Distribuição S.A.", "Enel SP") estão no próprio script, em `DISTRIBUIDORAS`. Para incluir outra distribuidora, acrescente a sigla do CSV e os nomes dela. Não use nomes ambíguos, como "Enel" ou "CPFL" sozinhos, porque eles valem para mais de uma distribuidora.
