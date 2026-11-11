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
