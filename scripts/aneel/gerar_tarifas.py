#!/usr/bin/env python3
"""Regera as tabelas embutidas no motor de regras a partir dos dados abertos da ANEEL.

Saídas (em fixtures/tariffs/):
- bandeiras.json: bandeira acionada em cada mês (ENE-05);
- tarifas.json: TE e TUSD residenciais (B1, convencional) de algumas
  distribuidoras grandes, por vigência (ENE-06).

Só usa a biblioteca padrão. Uso:

    python3 scripts/aneel/gerar_tarifas.py            # baixa do portal
    python3 scripts/aneel/gerar_tarifas.py --dir-csv pasta/   # usa CSVs já baixados

Fonte: https://dadosabertos.aneel.gov.br (conjuntos "bandeiras-tarifarias" e
"tarifas-distribuidoras-energia-eletrica"). Detalhes em scripts/aneel/README.md.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import shutil
import sys
import tempfile
import urllib.request
from pathlib import Path

PORTAL = "https://dadosabertos.aneel.gov.br"
API = PORTAL + "/api/3/action/package_show?id="

# Conjunto, nome do recurso CSV no portal e URL usada se a API não responder.
BANDEIRAS = (
    "bandeiras-tarifarias",
    "bandeira-tarifaria-acionamento.csv",
    PORTAL
    + "/dataset/7f43a020-6dc5-44b8-80b4-d97eaa94436c/resource/"
    "0591b8f6-fe54-437b-b72b-1aa2efd46e42/download/bandeira-tarifaria-acionamento.csv",
)
TARIFAS = (
    "tarifas-distribuidoras-energia-eletrica",
    "tarifas-homologadas-distribuidoras-energia-eletrica.csv",
    PORTAL
    + "/dataset/5a583f3e-1646-4f67-bf0f-69db4203e89e/resource/"
    "fcf2906c-7c32-4b9b-a637-054e7a5234f4/download/"
    "tarifas-homologadas-distribuidoras-energia-eletrica.csv",
)

# Nome da bandeira no CSV -> valor do enum TariffFlag (crates/core).
CORES = {
    "Verde": "verde",
    "Amarela": "amarela",
    "Vermelha P1": "vermelha_1",
    "Vermelha P2": "vermelha_2",
    "Escassez Hídrica": "escassez",
}

# Distribuidoras incluídas: sigla no CSV da ANEEL -> nomes que podem aparecer
# na fatura. Os nomes são curadoria nossa (não vêm do dado aberto). Ficam de fora
# nomes ambíguos, como "Enel" ou "CPFL" sozinhos, que valem para mais de uma
# distribuidora.
DISTRIBUIDORAS = {
    "CEMIG-D": ["Cemig", "Cemig Distribuição", "Cemig Distribuição S.A.", "CEMIG-D"],
    "ELETROPAULO": [
        "Enel SP",
        "Enel Distribuição São Paulo",
        "Eletropaulo",
        "Eletropaulo Metropolitana Eletricidade de São Paulo S.A.",
    ],
    "COELBA": [
        "Coelba",
        "Neoenergia Coelba",
        "Companhia de Eletricidade do Estado da Bahia",
    ],
    "COPEL-DIS": ["Copel", "Copel Distribuição", "Copel Distribuição S.A.", "COPEL-DIS"],
    "CPFL-PAULISTA": [
        "CPFL Paulista",
        "Companhia Paulista de Força e Luz",
        "CPFL-PAULISTA",
    ],
}

# Filtro das tarifas: residencial comum, baixa tensão, convencional, sem GD.
FILTRO = {
    "DscBaseTarifaria": "Tarifa de Aplicação",
    "DscSubGrupo": "B1",
    "DscModalidadeTarifaria": "Convencional",
    "DscClasse": "Residencial",
    "DscSubClasse": "Residencial",
    "DscDetalhe": "Não se aplica",
    "NomPostoTarifario": "Não se aplica",
    "DscUnidadeTerciaria": "MWh",
}

# Quantas vigências (as mais recentes) guardar por distribuidora.
VIGENCIAS = 3


def falha(msg: str) -> None:
    print(f"erro: {msg}", file=sys.stderr)
    sys.exit(1)


def url_do_recurso(conjunto: str, nome: str, reserva: str) -> str:
    """URL atual do CSV pela API CKAN do portal; usa a reserva se a API falhar."""
    try:
        with urllib.request.urlopen(API + conjunto, timeout=60) as r:
            dados = json.load(r)
        for rec in dados["result"]["resources"]:
            if rec.get("url", "").endswith("/" + nome):
                return rec["url"]
    except Exception as e:  # noqa: BLE001 (qualquer falha de rede cai na reserva)
        print(f"aviso: API do portal falhou ({e}); usando a URL conhecida", file=sys.stderr)
    return reserva


def baixar(url: str, destino: Path) -> None:
    print(f"baixando {url}", file=sys.stderr)
    req = urllib.request.Request(url, headers={"User-Agent": "humanobill-gerar-tarifas"})
    with urllib.request.urlopen(req, timeout=600) as r, destino.open("wb") as f:
        shutil.copyfileobj(r, f)


def ler_csv(caminho: Path):
    # utf-8-sig tira o BOM, se houver. Separador ";".
    with caminho.open(encoding="utf-8-sig", newline="") as f:
        yield from csv.DictReader(f, delimiter=";")


def numero(s: str) -> float:
    """'1.234,56' ou ',00' -> float."""
    s = s.strip().replace(".", "").replace(",", ".")
    if s.startswith("."):
        s = "0" + s
    return float(s)


def data(s: str) -> dt.date:
    return dt.date.fromisoformat(s.strip())


def gerar_bandeiras(csv_path: Path, desde: str, hoje: str, url: str) -> dict:
    meses: dict[str, str] = {}
    geracao = None
    for row in ler_csv(csv_path):
        mes = row["DatCompetencia"][:7]
        geracao = row.get("DatGeracaoConjuntoDados") or geracao
        if mes < desde:
            continue
        nome = row["NomBandeiraAcionada"].strip()
        if nome not in CORES:
            falha(f"bandeira desconhecida no CSV: {nome!r} ({mes})")
        cor = CORES[nome]
        if meses.get(mes, cor) != cor:
            falha(f"duas bandeiras para {mes}: {meses[mes]} e {cor}")
        meses[mes] = cor
    if not meses:
        falha("nenhum mês de bandeira encontrado")
    return {
        "fonte": (
            "ANEEL, dados abertos: conjunto 'Bandeiras Tarifárias', recurso "
            "'Bandeira Tarifária - Acionamento' (gerado pelo script scripts/aneel/gerar_tarifas.py)"
        ),
        "url": url,
        "data_geracao_conjunto": geracao,
        "coletado_em": hoje,
        "meses": dict(sorted(meses.items())),
    }


def juntar_vigencias(linhas: list[dict]) -> list[dict]:
    """Ordena e junta vigências contíguas com os mesmos valores.

    A ANEEL às vezes parte uma vigência em duas (ex.: na virada do ano) sem mudar
    a tarifa; juntar deixa a tabela menor e não muda nenhum valor.
    """
    por_periodo: dict[tuple, dict] = {}
    for v in linhas:
        chave = (v["inicio"], v["fim"])
        antigo = por_periodo.get(chave)
        if antigo and (antigo["tusd"], antigo["te"]) != (v["tusd"], v["te"]):
            falha(f"tarifas diferentes para a mesma vigência {chave}: {antigo} x {v}")
        por_periodo.setdefault(chave, v)
    ordenadas = sorted(por_periodo.values(), key=lambda v: v["inicio"])
    juntas: list[dict] = []
    for v in ordenadas:
        ult = juntas[-1] if juntas else None
        if ult and v["inicio"] <= ult["fim"]:
            falha(f"vigências sobrepostas: {ult} x {v}")
        if (
            ult
            and v["inicio"] == ult["fim"] + dt.timedelta(days=1)
            and (v["tusd"], v["te"]) == (ult["tusd"], ult["te"])
        ):
            ult["fim"] = v["fim"]
            continue
        juntas.append(dict(v))
    return juntas


def gerar_tarifas(csv_path: Path, hoje: str, url: str) -> dict:
    por_sigla: dict[str, list[dict]] = {s: [] for s in DISTRIBUIDORAS}
    cnpj: dict[str, str] = {}
    geracao = None
    for row in ler_csv(csv_path):
        if row["SigAgente"] not in por_sigla:
            continue
        if any(row[k] != v for k, v in FILTRO.items()):
            continue
        geracao = row.get("DatGeracaoConjuntoDados") or geracao
        cnpj[row["SigAgente"]] = row["NumCNPJDistribuidora"]
        por_sigla[row["SigAgente"]].append(
            {
                "inicio": data(row["DatInicioVigencia"]),
                "fim": data(row["DatFimVigencia"]),
                "tusd": numero(row["VlrTUSD"]),
                "te": numero(row["VlrTE"]),
                "resolucao": row["DscREH"].strip(),
            }
        )
    saida = []
    for sigla, nomes in DISTRIBUIDORAS.items():
        vig = juntar_vigencias(por_sigla[sigla])
        if not vig:
            falha(f"nenhuma tarifa B1 residencial para {sigla}")
        for v in vig:
            if v["tusd"] <= 0 or v["te"] <= 0:
                falha(f"tarifa não positiva em {sigla}: {v}")
        saida.append(
            {
                "sigla": sigla,
                "cnpj": cnpj[sigla],
                "nomes": nomes,
                "vigencias": [
                    {
                        "inicio": v["inicio"].isoformat(),
                        "fim": v["fim"].isoformat(),
                        "tusd_rs_mwh": v["tusd"],
                        "te_rs_mwh": v["te"],
                        "resolucao": v["resolucao"],
                    }
                    for v in vig[-VIGENCIAS:]
                ],
            }
        )
    return {
        "fonte": (
            "ANEEL, dados abertos: conjunto 'Tarifas de aplicação das distribuidoras de "
            "energia elétrica' (gerado pelo script scripts/aneel/gerar_tarifas.py)"
        ),
        "url": url,
        "data_geracao_conjunto": geracao,
        "coletado_em": hoje,
        "filtro": (
            "Tarifa de Aplicação, subgrupo B1, classe e subclasse Residencial, modalidade "
            "Convencional, sem SCEE (geração distribuída), posto 'Não se aplica'"
        ),
        "unidade": "R$/MWh, sem ICMS, PIS/COFINS e sem adicional de bandeira",
        "distribuidoras": saida,
    }


def escrever(caminho: Path, dados: dict) -> None:
    caminho.write_text(json.dumps(dados, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"gravado {caminho}", file=sys.stderr)


def main() -> None:
    raiz = Path(__file__).resolve().parents[2]
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--saida", type=Path, default=raiz / "fixtures" / "tariffs")
    p.add_argument(
        "--dir-csv",
        type=Path,
        help="pasta com os dois CSVs já baixados (não acessa a rede)",
    )
    p.add_argument("--desde", default="2024-01", help="primeiro mês de bandeira (YYYY-MM)")
    args = p.parse_args()
    hoje = dt.date.today().isoformat()

    url_b = url_do_recurso(*BANDEIRAS) if not args.dir_csv else BANDEIRAS[2]
    url_t = url_do_recurso(*TARIFAS) if not args.dir_csv else TARIFAS[2]
    with tempfile.TemporaryDirectory() as tmp:
        pasta = args.dir_csv or Path(tmp)
        csv_b = pasta / BANDEIRAS[1]
        csv_t = pasta / TARIFAS[1]
        if not args.dir_csv:
            baixar(url_b, csv_b)
            baixar(url_t, csv_t)
        bandeiras = gerar_bandeiras(csv_b, args.desde, hoje, url_b)
        tarifas = gerar_tarifas(csv_t, hoje, url_t)
    args.saida.mkdir(parents=True, exist_ok=True)
    escrever(args.saida / "bandeiras.json", bandeiras)
    escrever(args.saida / "tarifas.json", tarifas)


if __name__ == "__main__":
    main()
