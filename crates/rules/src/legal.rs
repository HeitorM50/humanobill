//! Bases legais citadas pelos achados (rótulo + link para a fonte).
//!
//! Ponto único de troca: a revisão da base legal (docs/03, docs/10) muda só
//! este arquivo e a tabela `RULES` em `lib.rs`.

use humanobill_core::LegalRef;

/// Referência legal estática; vira `LegalRef` (com `String`) no achado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Legal {
    pub label: &'static str,
    pub url: &'static str,
}

impl Legal {
    pub fn to_ref(self) -> LegalRef {
        LegalRef {
            label: self.label.to_owned(),
            url: self.url.to_owned(),
        }
    }
}

const URL_CDC: &str = "https://www.planalto.gov.br/ccivil_03/leis/l8078compilado.htm";
// RGC (Res. Anatel 765/2023): páginas oficiais da Anatel. O texto integral
// (informacoes.anatel.gov.br/.../1900-resolucao-765) estava fora do ar na validação (11/11/2026).
const URL_RGC_COBRANCA: &str =
    "https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/cobranca";
const URL_RGC_SUSPENSAO: &str =
    "https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/suspensao";
const URL_RGC_CANCELAMENTO: &str =
    "https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/cancelamento";
const URL_RGC_MANUAL: &str =
    "https://www.gov.br/anatel/pt-br/consumidor/conheca-seus-direitos/manual-operacional";

// REN ANEEL 1.000/2021 (texto compilado). Artigos conferidos em 11/11/2026 (docs/03, Energia).
const URL_REN_1000: &str = "https://www2.aneel.gov.br/cedoc/ren20211000.pdf";
const URL_ANEEL_TARIFAS: &str =
    "https://dadosabertos.aneel.gov.br/dataset/tarifas-distribuidoras-energia-eletrica";
const URL_ANEEL_BANDEIRAS: &str =
    "https://www.gov.br/aneel/pt-br/assuntos/tarifas/bandeiras-tarifarias";

/// CDC art. 39, III e par. único: serviço sem solicitação = amostra grátis.
pub const CDC_39_III: Legal = Legal {
    label: "CDC art. 39, III e par. único",
    url: URL_CDC,
};

/// CDC art. 42: cobrança indevida / devolução em dobro.
pub const CDC_42: Legal = Legal {
    label: "CDC art. 42",
    url: URL_CDC,
};

/// CDC art. 6º, III: informação clara.
pub const CDC_6_III: Legal = Legal {
    label: "CDC art. 6º, III",
    url: URL_CDC,
};

/// CDC art. 52, § 1º: multa de mora de até 2% no fornecimento com outorga de crédito.
pub const CDC_52_P1: Legal = Legal {
    label: "CDC art. 52, § 1º (multa de atraso de até 2% em compra com crédito)",
    url: URL_CDC,
};

/// CDC art. 30: a oferta vincula o fornecedor.
pub const CDC_30: Legal = Legal {
    label: "CDC art. 30",
    url: URL_CDC,
};

/// RGC art. 48: SVA só com autorização prévia e expressa.
pub const RGC_48: Legal = Legal {
    label: "RGC Anatel art. 48 (autorização prévia e expressa)",
    url: URL_RGC_COBRANCA,
};

/// RGC art. 55, I: a fatura traz o valor de cada serviço.
pub const RGC_55_I: Legal = Legal {
    label: "RGC Anatel art. 55, I (valor de cada serviço)",
    url: URL_RGC_MANUAL,
};

/// RGC art. 78, § 1º: multa de até 2% e juros de até 1% ao mês.
pub const RGC_78_P1: Legal = Legal {
    label: "RGC Anatel art. 78, § 1º (multa até 2%, juros até 1% a.m.)",
    url: URL_RGC_SUSPENSAO,
};

/// RGC arts. 83 e 84: sem cobrança após o pedido de cancelamento.
pub const RGC_83_84: Legal = Legal {
    label: "RGC Anatel arts. 83 e 84 (cancelamento)",
    url: URL_RGC_CANCELAMENTO,
};

/// REN 1000 art. 343: multa de até 2%, juros de 1% ao mês pro rata die, IPCA à parte.
pub const REN_343: Legal = Legal {
    label: "REN ANEEL 1.000/2021, art. 343 (multa até 2%, juros 1% a.m. pro rata die, IPCA)",
    url: URL_REN_1000,
};

/// REN 1000 arts. 401 V, 408, 413 e 416: procedimento de contestação de fatura.
pub const REN_401_416: Legal = Legal {
    label: "REN ANEEL 1.000/2021, arts. 401 V, 408, 413 e 416 (contestação de fatura)",
    url: URL_REN_1000,
};

/// REN 1000 arts. 283, 288 e 289: hipóteses de faturamento sem leitura.
pub const REN_283_289: Legal = Legal {
    label: "REN ANEEL 1.000/2021, arts. 283, 288 e 289 (faturamento sem leitura)",
    url: URL_REN_1000,
};

/// REN 1000 arts. 260 e 285: ciclo de leitura de 27 a 33 dias (grupo B).
pub const REN_260_285: Legal = Legal {
    label: "REN ANEEL 1.000/2021, arts. 260 e 285 (ciclo de 27 a 33 dias)",
    url: URL_REN_1000,
};

/// REN 1000 arts. 331 e 413: doações e serviços na fatura só com autorização.
pub const REN_331_413: Legal = Legal {
    label: "REN ANEEL 1.000/2021, arts. 331 e 413 (serviços e doações na fatura)",
    url: URL_REN_1000,
};

/// Bandeiras mensais (ENE-05, "a validar"): cor de cada mês nas notícias da ANEEL.
pub const ANEEL_BANDEIRAS: Legal = Legal {
    label: "Bandeiras tarifárias mensais da ANEEL",
    url: URL_ANEEL_BANDEIRAS,
};

/// REN 1000 art. 173: cobrar as tarifas homologadas (caput), tarifa proporcional
/// quando muda no ciclo (§ 2º) e aplicação por subgrupo, classe e modalidade (§ 3º).
pub const REN_173: Legal = Legal {
    label: "REN ANEEL 1.000/2021, art. 173 (cobrança das tarifas homologadas pela ANEEL)",
    url: URL_REN_1000,
};

/// Tarifas homologadas das distribuidoras (ENE-06): dados abertos da ANEEL.
pub const ANEEL_TARIFAS: Legal = Legal {
    label: "Tarifas homologadas das distribuidoras (dados abertos da ANEEL)",
    url: URL_ANEEL_TARIFAS,
};

// Res. CMN 3.919/2010 (tarifas bancárias), versão consolidada do BCB (v7, última
// alteração pela Res. CMN 4.925, vigente desde 1º/6/2022). Lida em 22/11/2026 (docs/03, Banco).
const URL_CMN_3919: &str =
    "https://normativos.bcb.gov.br/Lists/Normativos/Attachments/49514/Res_3919_v7_P.pdf";

/// Res. CMN 3.919/2010, art. 1º: tarifa só com previsão em contrato ou serviço
/// previamente autorizado ou solicitado.
pub const CMN_3919_PREVIA: Legal = Legal {
    label: "Res. CMN 3.919/2010, art. 1º (tarifa só com contrato ou serviço pedido)",
    url: URL_CMN_3919,
};

/// Res. CMN 3.919/2010, art. 2º, I: serviços essenciais gratuitos da conta
/// corrente (até 4 saques, 2 extratos e 10 folhas de cheque por mês).
pub const CMN_3919_ESSENCIAIS: Legal = Legal {
    label: "Res. CMN 3.919/2010, art. 2º, I (serviços essenciais gratuitos: até 4 saques, 2 extratos e 10 folhas de cheque por mês)",
    url: URL_CMN_3919,
};

/// Res. CMN 3.919/2010, arts. 8º e 9º, I: pacote só por contrato específico, e o
/// cliente pode usar e pagar só serviços avulsos.
pub const CMN_3919_PACOTE: Legal = Legal {
    label: "Res. CMN 3.919/2010, arts. 8º e 9º, I (pacote só com contrato específico)",
    url: URL_CMN_3919,
};

/// Res. CMN 3.919/2010, art. 3º, V e Tabela I, item 5: tarifas do cartão de crédito
/// (anuidade, 2ª via, saque, pagamento de contas e avaliação emergencial de crédito).
pub const CMN_3919_CARTAO: Legal = Legal {
    label: "Res. CMN 3.919/2010, art. 3º, V e Tabela I, item 5 (tarifas do cartão de crédito)",
    url: URL_CMN_3919,
};
