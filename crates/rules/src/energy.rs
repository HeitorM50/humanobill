//! Regras de energia ENE-01 a ENE-08 (docs/03, seção Energia).
//! ENE-06 (recálculo da tarifa) é Could: conservadora, só para distribuidoras da
//! tabela de tarifas da ANEEL (a demo, com a fictícia Luzia Energia, não muda).
//!
//! Cada função devolve a severidade **pretendida** pela regra. O rebaixamento
//! de regras "a validar" (hoje ENE-05 e ENE-06) é feito pelo motor (`crate::cap`).
//!
//! Contas em centavos e kWh inteiros. Estimativas de "valor envolvido" (ENE-02,
//! ENE-04) arredondam para baixo, para nunca inflar o valor contestado.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use chrono::{Datelike, NaiveDate};
use humanobill_core::{
    Answer, Answers, BillItem, Cents, ExtractedBill, ItemCategory, Question, QuestionKind,
    ReadingType, Severity, TariffFlag,
};

use crate::Output;
use crate::shared::{dias, draft, items_of, late_fee_items, pct_round, sum, sum_mismatch};
use crate::text::{brl, fold, has_word, tokens};

/// Folga de arredondamento na multa e nos juros (docs/03, "Tolerâncias recomendadas").
pub(crate) const ENE01_TOLERANCE_CENTS: Cents = 5;

/// ENE-02: consumo atípico se passar de média × 14/10 (heurística do produto).
const SPIKE_NUM: i64 = 14;
const SPIKE_DEN: i64 = 10;
/// ENE-02: meses de histórico exigidos e usados.
const MIN_HISTORY: usize = 3;
const MAX_HISTORY: usize = 12;

/// ENE-04: ciclo do grupo B (REN 1000 art. 260) e exceção do § 1º.
const CYCLE_MIN: i64 = 27;
const CYCLE_MAX: i64 = 33;
const CYCLE_EXCEPTION_MIN: i64 = 15;
const CYCLE_EXCEPTION_MAX: i64 = 47;

pub(crate) const Q_ENE04_CYCLE: &str = "ENE-04:ciclo";

/// ENE-07: palavras (sem acento) que indicam cobrança que não é energia.
const EXTRA_KEYWORDS: &[&str] = &[
    "seguro",
    "doacao",
    "doacoes",
    "assinatura",
    "clube",
    "contribuicao",
    "contribuicoes",
    "assistencia",
    "protecao",
    "revista",
    "capitalizacao",
];

/// ENE-07: palavras da contribuição de iluminação pública (CIP/COSIP), que é devida.
const PUBLIC_LIGHTING_WORDS: &[&str] = &["iluminacao", "ilum", "cip", "cosip"];

/// Tabela de bandeiras embutida em tempo de compilação (fonte: ANEEL,
/// fixtures/tariffs/bandeiras-oficiais.md). Sem I/O em tempo de execução.
const FLAGS_JSON: &str = include_str!("../../../fixtures/tariffs/bandeiras.json");

/// Tarifas homologadas (TE e TUSD, B1 residencial convencional) de algumas
/// distribuidoras, geradas do dado aberto da ANEEL por scripts/aneel. Sem I/O.
const TARIFFS_JSON: &str = include_str!("../../../fixtures/tariffs/tarifas.json");

/// ENE-06: folga sobre a tarifa homologada (docs/03).
const ENE06_TOLERANCE_PCT: i64 = 2;
/// ENE-06: ciclo máximo avaliado (dias), para não varrer períodos absurdos.
const ENE06_MAX_DAYS: i64 = 400;
/// ENE-06: palavras de outra modalidade (tarifa branca, geração distribuída).
const ENE06_SKIP_WORDS: &[&str] = &[
    "branca",
    "ponta",
    "intermediario",
    "scee",
    "injetada",
    "compensada",
];

fn q_ene07(item_id: &str) -> String {
    format!("ENE-07:{item_id}")
}

fn energy_lines(bill: &ExtractedBill) -> Vec<&BillItem> {
    items_of(bill, ItemCategory::Energia)
}

/// `floor(value * num / den)` para `value, num >= 0`, `den > 0`.
fn share_floor(value: Cents, num: i64, den: i64) -> Cents {
    if den <= 0 || value <= 0 || num <= 0 {
        return 0;
    }
    let v = i128::from(value) * i128::from(num) / i128::from(den);
    Cents::try_from(v).unwrap_or(Cents::MAX)
}

// ---------- ENE-01 ----------

/// ENE-01: multa acima de 2% ou juros acima de 1% ao mês pro rata die
/// (REN 1000 art. 343), sobre a base declarada em `late_fees`.
pub(crate) fn ene01(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(fees) = bill.late_fees.as_ref().filter(|f| f.base_cents > 0) else {
        return out;
    };
    let fine_cap = pct_round(fees.base_cents, 2, 100);
    let days = Cents::from(fees.days_late);
    let interest_cap = pct_round(fees.base_cents, days, 3_000);

    let fine_excess = (fees.fine_cents > fine_cap.saturating_add(ENE01_TOLERANCE_CENTS))
        .then(|| fees.fine_cents - fine_cap);
    let interest_excess = (fees.interest_cents
        > interest_cap.saturating_add(ENE01_TOLERANCE_CENTS))
    .then(|| fees.interest_cents - interest_cap);

    let fine_items = late_fee_items(bill, &["multa"]);
    let interest_items = late_fee_items(bill, &["juro", "mora"]);
    let same_refs = fine_items
        .iter()
        .map(|i| &i.id)
        .eq(interest_items.iter().map(|i| &i.id));

    match (fine_excess, interest_excess) {
        // Mesmo item (ou nenhum) para multa e juros: um achado só, para o id ser único.
        (Some(f), Some(j)) if same_refs => out.findings.push(draft(
            Severity::Unlawful,
            "Multa e juros de atraso acima do limite".into(),
            format!(
                "Multa de {} e juros de {} passam do limite da ANEEL: multa até 2% e juros de \
                 1% ao mês.",
                brl(fees.fine_cents),
                brl(fees.interest_cents)
            ),
            &fine_items,
            f.saturating_add(j),
        )),
        (f, j) => {
            if let Some(f) = f {
                out.findings.push(draft(
                    Severity::Unlawful,
                    "Multa por atraso acima de 2%".into(),
                    format!(
                        "A multa foi {}, mais que 2% do valor em atraso ({}). A ANEEL permite \
                         no máximo 2%.",
                        brl(fees.fine_cents),
                        brl(fine_cap)
                    ),
                    &fine_items,
                    f,
                ));
            }
            if let Some(j) = j {
                out.findings.push(draft(
                    Severity::Unlawful,
                    "Juros de atraso acima de 1% ao mês".into(),
                    format!(
                        "Os juros foram {}. Para {} de atraso, 1% ao mês daria no máximo {}.",
                        brl(fees.interest_cents),
                        dias(days),
                        brl(interest_cap)
                    ),
                    &interest_items,
                    j,
                ));
            }
        }
    }
    out
}

// ---------- ENE-02 ----------

/// ENE-02: consumo do mês acima de 1,4 × a média do histórico (até 12 meses,
/// mínimo 3, sem o próprio mês de referência). Sempre `suspeito`.
pub(crate) fn ene02(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(e) = bill.energy.as_ref() else {
        return out;
    };
    let mut hist: Vec<_> = e
        .history
        .iter()
        .filter(|p| p.month != bill.reference_month)
        .collect();
    hist.sort_by(|a, b| a.month.cmp(&b.month));
    let recent = &hist[hist.len().saturating_sub(MAX_HISTORY)..];
    if recent.len() < MIN_HISTORY {
        return out;
    }
    let n = i64::try_from(recent.len()).unwrap_or(i64::MAX);
    let total: i64 = recent.iter().map(|p| i64::from(p.kwh)).sum();
    let kwh = i64::from(e.consumption_kwh);
    // kwh > (total / n) × 14/10  <=>  kwh × n × 10 > total × 14 (sem divisão).
    if kwh * n * SPIKE_DEN <= total * SPIKE_NUM {
        return out;
    }
    let energy = energy_lines(bill);
    // Parte do valor da energia acima da média: valor × (kwh − média) / kwh.
    let excess = share_floor(sum(&energy), kwh * n - total, kwh * n);
    let avg = (total + n / 2) / n;
    let explanation = if total == 0 {
        format!(
            "Você gastou {kwh} kWh, e a sua média era 0 kWh. Pode ser erro de leitura: peça a \
             revisão à distribuidora."
        )
    } else {
        let pct = (kwh * n - total) * 100 / total;
        format!(
            "Você gastou {kwh} kWh, {pct}% acima da sua média de {avg} kWh. Pode ser erro de \
             leitura: peça a revisão à distribuidora."
        )
    };
    out.findings.push(draft(
        Severity::Suspicious,
        "Consumo muito acima da sua média".into(),
        explanation,
        &energy,
        excess,
    ));
    out
}

// ---------- ENE-03 ----------

/// ENE-03: leitura estimada ou pela média (REN 1000 arts. 283, 288 e 289). Sempre `suspeito`.
pub(crate) fn ene03(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(e) = bill.energy.as_ref() else {
        return out;
    };
    let (title, how) = match e.reading_type {
        ReadingType::Real => return out,
        ReadingType::Estimada => ("Leitura estimada, não real", "leitura estimada"),
        ReadingType::Media => ("Leitura pela média, não real", "leitura pela média"),
    };
    let energy = energy_lines(bill);
    out.findings.push(draft(
        Severity::Suspicious,
        title.into(),
        format!(
            "Esta conta usou {how}, não a do seu medidor. Isso só vale em casos específicos: \
             peça a leitura real."
        ),
        &energy,
        sum(&energy),
    ));
    out
}

// ---------- ENE-04 ----------

/// ENE-04: ciclo de leitura fora de 27 a 33 dias (REN 1000 art. 260). Pergunta se
/// é a primeira conta ou mudança de rota (§ 1º: 15 a 47 dias). Sempre `suspeito`.
pub(crate) fn ene04(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(e) = bill.energy.as_ref() else {
        return out;
    };
    let days = (e.reading_curr_date - e.reading_prev_date).num_days();
    if days <= 0 || (CYCLE_MIN..=CYCLE_MAX).contains(&days) {
        return out;
    }
    out.questions.push(Question {
        id: Q_ENE04_CYCLE.into(),
        rule_id: "ENE-04".into(),
        item_id: None,
        text: "Esta é a sua primeira conta, ou a distribuidora avisou que mudou a data de \
               leitura?"
            .into(),
        kind: QuestionKind::YesNo,
    });
    let exception = matches!(answers.get(Q_ENE04_CYCLE), Some(Answer::Yes))
        && (CYCLE_EXCEPTION_MIN..=CYCLE_EXCEPTION_MAX).contains(&days);
    if exception {
        return out;
    }
    let energy = energy_lines(bill);
    let (explanation, amount) = if days > CYCLE_MAX {
        (
            format!(
                "A leitura cobriu {}; o normal é de 27 a 33. Acima disso, a cobrança deve ser \
                 limitada a 33 dias.",
                dias(days)
            ),
            // Art. 285: proporcionaliza a 33 dias; excesso = valor × (dias − 33) / dias.
            share_floor(sum(&energy), days - CYCLE_MAX, days),
        )
    } else {
        (
            format!(
                "A leitura cobriu {}; o normal é de 27 a 33. Confira se a distribuidora avisou a \
                 mudança de data.",
                dias(days)
            ),
            0,
        )
    };
    out.findings.push(draft(
        Severity::Suspicious,
        "Período de leitura fora do padrão".into(),
        explanation,
        &energy,
        amount,
    ));
    out
}

// ---------- ENE-05 ----------

/// Bandeira de cada mês ("YYYY-MM"), lida uma vez da tabela embutida.
/// Tabela malformada vira tabela vazia (ENE-05 não avalia); um teste garante que ela é válida.
pub(crate) fn flag_table() -> &'static BTreeMap<String, TariffFlag> {
    static TABLE: OnceLock<BTreeMap<String, TariffFlag>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(FLAGS_JSON) else {
            return BTreeMap::new();
        };
        let Some(months) = v.get("meses").and_then(|m| m.as_object()) else {
            return BTreeMap::new();
        };
        months
            .iter()
            .filter_map(|(m, f)| {
                let flag = serde_json::from_value::<TariffFlag>(f.clone()).ok()?;
                Some((m.clone(), flag))
            })
            .collect()
    })
}

fn flag_rank(f: TariffFlag) -> u8 {
    match f {
        TariffFlag::Verde => 0,
        TariffFlag::Amarela => 1,
        TariffFlag::Vermelha1 => 2,
        TariffFlag::Vermelha2 => 3,
        TariffFlag::Escassez => 4,
    }
}

fn flag_name(f: TariffFlag) -> &'static str {
    match f {
        TariffFlag::Verde => "verde",
        TariffFlag::Amarela => "amarela",
        TariffFlag::Vermelha1 => "vermelha patamar 1",
        TariffFlag::Vermelha2 => "vermelha patamar 2",
        TariffFlag::Escassez => "escassez hídrica",
    }
}

/// Cor da bandeira escrita no texto ("vermelha patamar 1", "VERMELHA P1", "amarela").
/// `None` se não houver cor, se houver mais de uma, ou se for vermelha sem patamar.
pub(crate) fn parse_flag(text: &str) -> Option<TariffFlag> {
    let t = tokens(text);
    let colors: Vec<&str> = ["escassez", "amarela", "verde", "vermelha", "vermelho"]
        .into_iter()
        .filter(|c| has_word(&t, c))
        .collect();
    match colors.as_slice() {
        ["escassez"] => Some(TariffFlag::Escassez),
        ["amarela"] => Some(TariffFlag::Amarela),
        ["verde"] => Some(TariffFlag::Verde),
        ["vermelha"] | ["vermelho"] => red_level(&fold(text)),
        _ => None,
    }
}

/// Patamar logo depois de "vermelha": "patamar 1", "pat. 2", "p1", "1", "ii".
/// Número seguido de vírgula/ponto e dígito é valor ("vermelha 1,00"), não patamar.
fn red_level(folded: &str) -> Option<TariffFlag> {
    let start = folded.find("vermelh")?;
    let rest = &folded[start..];
    let rest = rest.trim_start_matches(|c: char| c.is_alphabetic());
    let mut rest = skip_separators(rest);
    for prefix in ["patamar", "pat", "p"] {
        if let Some(r) = rest.strip_prefix(prefix) {
            rest = skip_separators(r);
            break;
        }
    }
    let mut chars = rest.chars();
    let level = match (chars.next(), chars.next(), chars.next()) {
        (Some('1'), next, after) if !is_number_tail(next, after) => TariffFlag::Vermelha1,
        (Some('2'), next, after) if !is_number_tail(next, after) => TariffFlag::Vermelha2,
        (Some('i'), Some('i'), next) if !next.is_some_and(char::is_alphanumeric) => {
            TariffFlag::Vermelha2
        }
        (Some('i'), next, _) if !next.is_some_and(char::is_alphanumeric) => TariffFlag::Vermelha1,
        _ => return None,
    };
    Some(level)
}

/// Pula espaços, pontos e hífens no começo.
fn skip_separators(s: &str) -> &str {
    s.trim_start_matches(|c: char| c.is_whitespace() || c == '.' || c == '-')
}

/// O dígito continua um número ("12", "1,00", "1.5")?
fn is_number_tail(next: Option<char>, after: Option<char>) -> bool {
    match next {
        Some(c) if c.is_ascii_digit() => true,
        Some(',' | '.') => after.is_some_and(|c| c.is_ascii_digit()),
        _ => false,
    }
}

/// Meses (YYYY-MM) que o ciclo de leitura toca, do mês da leitura anterior ao da atual.
fn cycle_months(prev: NaiveDate, curr: NaiveDate) -> Vec<String> {
    let first = prev.year() * 12 + prev.month0() as i32;
    let last = curr.year() * 12 + curr.month0() as i32;
    (first..=last)
        .map(|m| format!("{}-{:02}", m.div_euclid(12), m.rem_euclid(12) + 1))
        .collect()
}

/// "a", "a ou b", "a, b ou c".
fn or_list(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{} ou {last}", init.join(", ")),
    }
}

/// ENE-05: bandeira cobrada que não é a de nenhum mês tocado pelo ciclo de
/// leitura (tolerância de docs/03 até a proporcionalidade ser validada).
/// Regra "a validar": o motor nunca deixa sair `indevido`.
pub(crate) fn ene05(bill: &ExtractedBill, _answers: &Answers) -> Output {
    let mut out = Output::default();
    let Some(e) = bill.energy.as_ref() else {
        return out;
    };
    if e.reading_curr_date < e.reading_prev_date {
        return out;
    }
    let table = flag_table();
    let mut allowed: Vec<TariffFlag> = Vec::new();
    for m in cycle_months(e.reading_prev_date, e.reading_curr_date) {
        // Mês sem dado na tabela: não avalia.
        let Some(&f) = table.get(&m) else {
            return out;
        };
        if !allowed.contains(&f) {
            allowed.push(f);
        }
    }

    // Bandeira de cada item: a escrita nele; senão, a declarada na fatura.
    // Sem item de bandeira, compara só a declarada (ex.: verde, que não tem adicional).
    let flag_items = items_of(bill, ItemCategory::Bandeira);
    let mut charged: BTreeMap<u8, (TariffFlag, Vec<&BillItem>)> = BTreeMap::new();
    for item in flag_items.iter().copied() {
        let flag = parse_flag(&item.description)
            .or_else(|| parse_flag(&item.original_text))
            .or(e.tariff_flag);
        if let Some(f) = flag {
            charged
                .entry(flag_rank(f))
                .or_insert((f, Vec::new()))
                .1
                .push(item);
        }
    }
    if let (true, Some(f)) = (flag_items.is_empty(), e.tariff_flag) {
        charged.insert(flag_rank(f), (f, Vec::new()));
    }

    let names: Vec<&str> = allowed.iter().map(|f| flag_name(*f)).collect();
    // Só acusa bandeira MAIS CARA que todas as vigentes no ciclo: cobrar uma mais
    // barata não prejudica o consumidor.
    let max_allowed = allowed.iter().map(|f| flag_rank(*f)).max().unwrap_or(0);
    for (flag, items) in charged
        .into_values()
        .filter(|(f, _)| !allowed.contains(f) && flag_rank(*f) > max_allowed)
    {
        out.findings.push(draft(
            Severity::Suspicious,
            "Bandeira diferente da definida pela ANEEL".into(),
            format!(
                // No máximo 23 palavras: 4 cores vigentes + a cobrada.
                "A conta cobra bandeira {}, mas a ANEEL definiu {} no período da leitura.",
                flag_name(flag),
                or_list(&names)
            ),
            &items,
            sum(&items),
        ));
    }
    out
}

// ---------- ENE-06 ----------

/// Uma vigência de tarifa homologada, em centavos por MWh (sem impostos e sem bandeira).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Validity {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub tusd: i64,
    pub te: i64,
}

/// Distribuidora da tabela de tarifas: nome de exibição, nomes normalizados
/// aceitos na fatura e vigências em ordem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Distributor {
    pub name: String,
    pub keys: Vec<String>,
    pub validities: Vec<Validity>,
}

/// Lê a tabela de tarifas (formato de `fixtures/tariffs/tarifas.json`).
/// Entrada malformada é ignorada: a ENE-06 só não avalia aquela distribuidora.
pub(crate) fn parse_tariffs(json: &str) -> Vec<Distributor> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(list) = v.get("distribuidoras").and_then(|d| d.as_array()) else {
        return Vec::new();
    };
    list.iter().filter_map(parse_distributor).collect()
}

fn parse_distributor(d: &serde_json::Value) -> Option<Distributor> {
    let names: Vec<&str> = d
        .get("nomes")?
        .as_array()?
        .iter()
        .filter_map(|n| n.as_str())
        .collect();
    let keys: Vec<String> = names
        .iter()
        .map(|n| distributor_key(n))
        .filter(|k| !k.is_empty())
        .collect();
    let mut validities: Vec<Validity> = d
        .get("vigencias")?
        .as_array()?
        .iter()
        .map(parse_validity)
        .collect::<Option<_>>()?;
    validities.sort_by_key(|v| v.start);
    let name = (*names.first()?).to_owned();
    (!keys.is_empty() && !validities.is_empty()).then_some(Distributor {
        name,
        keys,
        validities,
    })
}

fn parse_validity(v: &serde_json::Value) -> Option<Validity> {
    let day = |k: &str| NaiveDate::parse_from_str(v.get(k)?.as_str()?, "%Y-%m-%d").ok();
    // R$/MWh com 2 casas -> centavos/MWh.
    let cents = |k: &str| {
        let x = v.get(k)?.as_f64()?;
        let c = (x * 100.0).round();
        (c > 0.0 && c < 1e12).then_some(c as i64)
    };
    let (start, end) = (day("inicio")?, day("fim")?);
    (start <= end).then_some(Validity {
        start,
        end,
        tusd: cents("tusd_rs_mwh")?,
        te: cents("te_rs_mwh")?,
    })
}

/// Tabela de tarifas embutida (dado aberto da ANEEL, lida uma vez).
pub(crate) fn tariff_table() -> &'static [Distributor] {
    static TABLE: OnceLock<Vec<Distributor>> = OnceLock::new();
    TABLE.get_or_init(|| parse_tariffs(TARIFFS_JSON))
}

/// Nome comparável: sem acento, caixa e pontuação, e sem "S.A."/"Ltda".
fn distributor_key(name: &str) -> String {
    tokens(name)
        .into_iter()
        .filter(|t| !matches!(t.as_str(), "s" | "a" | "sa" | "ltda"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_distributor<'a>(table: &'a [Distributor], provider: &str) -> Option<&'a Distributor> {
    let key = distributor_key(provider);
    if key.is_empty() {
        return None;
    }
    table.iter().find(|d| d.keys.contains(&key))
}

/// Tarifa proporcional do ciclo (REN 1000 art. 173, § 2º): soma de tarifa × dias de
/// cada vigência, para TUSD e TE, e o número de dias. `None` se algum dia do ciclo
/// não tem vigência na tabela, ou se o ciclo é vazio ou longo demais.
fn cycle_tariff(
    validities: &[Validity],
    prev: NaiveDate,
    curr: NaiveDate,
) -> Option<(i64, i64, i64)> {
    let days = (curr - prev).num_days();
    if !(1..=ENE06_MAX_DAYS).contains(&days) {
        return None;
    }
    let (mut tusd, mut te) = (0_i64, 0_i64);
    for day in prev.iter_days().take(usize::try_from(days).ok()?) {
        let v = validities.iter().find(|v| v.start <= day && day <= v.end)?;
        tusd += v.tusd;
        te += v.te;
    }
    Some((tusd, te, days))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Component {
    Tusd,
    Te,
    /// Linha única com TE + TUSD.
    Both,
}

impl Component {
    fn label(self) -> &'static str {
        match self {
            Component::Tusd => "TUSD",
            Component::Te => "TE",
            Component::Both => "TE + TUSD",
        }
    }
}

fn item_words(item: &BillItem) -> Vec<String> {
    let mut t = tokens(&item.description);
    t.extend(tokens(&item.original_text));
    t
}

fn component(item: &BillItem) -> Option<Component> {
    let t = item_words(item);
    match (has_word(&t, "tusd"), t.iter().any(|w| w == "te")) {
        (true, true) => Some(Component::Both),
        (true, false) => Some(Component::Tusd),
        (false, true) => Some(Component::Te),
        (false, false) => None,
    }
}

/// Décimos de milésimo de real (R$ 0,0001) -> "R$ 0,5931".
fn brl4(v: i64) -> String {
    format!("R$ {},{:04}", v / 10_000, v % 10_000)
}

/// ENE-06: valor de TE/TUSD acima da tarifa homologada da distribuidora (com 2% de
/// folga), sem impostos. Só avalia distribuidora da tabela, com impostos destacados
/// em outras linhas e tarifa convencional. Sempre `suspeito`.
pub(crate) fn ene06(bill: &ExtractedBill, _answers: &Answers) -> Output {
    ene06_with(tariff_table(), bill)
}

fn ene06_with(table: &[Distributor], bill: &ExtractedBill) -> Output {
    let mut out = Output::default();
    let Some(e) = bill.energy.as_ref() else {
        return out;
    };
    let kwh = i64::from(e.consumption_kwh);
    let Some(dist) = find_distributor(table, &bill.provider_name) else {
        return out;
    };
    let energy = energy_lines(bill);
    // A tabela é sem impostos: sem imposto destacado, o preço da linha pode incluí-los.
    let has_taxes = bill
        .items
        .iter()
        .any(|i| i.category == ItemCategory::Imposto);
    // Tarifa branca e geração distribuída têm outras tarifas: fora da tabela.
    let other_modality = energy.iter().any(|i| {
        let t = item_words(i);
        ENE06_SKIP_WORDS.iter().any(|w| has_word(&t, w))
    });
    if kwh <= 0 || !has_taxes || other_modality {
        return out;
    }
    let Some((tusd, te, days)) =
        cycle_tariff(&dist.validities, e.reading_prev_date, e.reading_curr_date)
    else {
        return out;
    };

    let mut by_component: BTreeMap<Component, Vec<&BillItem>> = BTreeMap::new();
    for item in energy {
        if let Some(c) = component(item) {
            by_component.entry(c).or_default().push(item);
        }
    }
    for (comp, items) in by_component {
        // Tarifa × dias somada no ciclo, em centavos/MWh.
        let tariff_days = match comp {
            Component::Tusd => tusd,
            Component::Te => te,
            Component::Both => tusd + te,
        };
        let charged = i128::from(sum(&items));
        // Esperado (centavos) = tarifa média × kWh / 1000 = tariff_days × kWh / (1000 × dias).
        let num = i128::from(tariff_days) * i128::from(kwh);
        let den = 1_000 * i128::from(days);
        // Cobrado > esperado × (100 + 2)/100, sem divisão.
        if charged * den * 100 <= num * (100 + i128::from(ENE06_TOLERANCE_PCT)) {
            continue;
        }
        let expected_ceil = (num + den - 1) / den;
        let excess = Cents::try_from(charged - expected_ceil).unwrap_or(Cents::MAX);
        let charged_unit = i64::try_from(charged * 100 / i128::from(kwh)).unwrap_or(i64::MAX);
        let tariff_unit = (tariff_days + 5 * days) / (10 * days);
        out.findings.push(draft(
            Severity::Suspicious,
            format!("{} acima da tarifa da ANEEL", comp.label()),
            format!(
                "A conta cobra {} por kWh de {}. A tarifa da ANEEL para {} é {}, sem impostos. \
                 Peça a revisão.",
                brl4(charged_unit),
                comp.label(),
                dist.name,
                brl4(tariff_unit)
            ),
            &items,
            excess,
        ));
    }
    out
}

// ---------- ENE-07 ----------

/// ENE-07: cobrança que não é energia (seguro, doação, assinatura...). Sem resposta:
/// suspeito. "Não autorizei": indevido (pretendido). "Autorizei": ok.
pub(crate) fn ene07(bill: &ExtractedBill, answers: &Answers) -> Output {
    let mut out = Output::default();
    for item in bill.items.iter().filter(|i| is_extra(i)) {
        let qid = q_ene07(&item.id);
        out.questions.push(Question {
            id: qid.clone(),
            rule_id: "ENE-07".into(),
            item_id: Some(item.id.clone()),
            text: format!("Você autorizou {} na sua conta de luz?", item.description),
            kind: QuestionKind::YesNo,
        });
        let (severity, title, explanation) = match answers.get(&qid) {
            Some(Answer::No) => (
                Severity::Unlawful,
                format!("Cobrança não autorizada: {}", item.description),
                "Você disse que não autorizou esta cobrança. Serviço sem pedido não precisa ser \
                 pago e pode ser retirado da conta."
                    .to_owned(),
            ),
            Some(Answer::Yes) => (
                Severity::Ok,
                format!("Cobrança autorizada: {}", item.description),
                "Você confirmou que autorizou esta cobrança. Nada a contestar aqui.".to_owned(),
            ),
            _ => (
                Severity::Suspicious,
                format!("Cobrança extra na conta de luz: {}", item.description),
                "Isto não é energia. Na conta de luz, só pode entrar com sua autorização. Se não \
                 autorizou, peça a retirada."
                    .to_owned(),
            ),
        };
        out.findings.push(draft(
            severity,
            title,
            explanation,
            &[item],
            item.amount_cents,
        ));
    }
    out
}

fn is_extra(item: &BillItem) -> bool {
    match item.category {
        ItemCategory::Energia
        | ItemCategory::Bandeira
        | ItemCategory::Imposto
        | ItemCategory::IluminacaoPublica
        | ItemCategory::EncargoAtraso => false,
        ItemCategory::ServicoAdicional => true,
        ItemCategory::Plano | ItemCategory::Outros => {
            let t = tokens(&item.description);
            let lighting = PUBLIC_LIGHTING_WORDS.iter().any(|w| has_word(&t, w));
            !lighting && EXTRA_KEYWORDS.iter().any(|k| has_word(&t, k))
        }
    }
}

// ---------- ENE-08 ----------

/// ENE-08: soma dos itens diferente do total (igual à TEL-07).
pub(crate) fn ene08(bill: &ExtractedBill, _answers: &Answers) -> Output {
    sum_mismatch(bill)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use humanobill_core::{
        Answer, BillItem, ItemCategory as C, QuestionKind, ReadingType, Severity, TariffFlag as F,
    };

    fn ans(pairs: &[(&str, Answer)]) -> Answers {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    fn sev_refs(out: &Output) -> Vec<(Severity, Vec<String>, i64)> {
        out.findings
            .iter()
            .map(|d| (d.severity, d.item_refs.clone(), d.amount_cents))
            .collect()
    }

    fn refs(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_owned()).collect()
    }

    fn with_items(extra: Vec<BillItem>) -> ExtractedBill {
        let mut items = energy_items();
        items.extend(extra);
        energy_bill(items)
    }

    // ---------- ENE-01: multa acima de 2% / juros acima de 1% ao mês ----------

    fn com_encargos(fine: i64, interest: i64, base: i64, days: u32) -> ExtractedBill {
        let mut b = with_items(vec![
            item("i7", "Multa por atraso", fine, C::EncargoAtraso),
            item("i8", "Juros por atraso", interest, C::EncargoAtraso),
        ]);
        b.late_fees = fees(base, fine, interest, days);
        b
    }

    #[test]
    fn ene01_multa_de_5_por_cento_como_na_energia_2_e_indevido() {
        // Base R$ 172,40: 2% = 344,8 -> 345 centavos; multa R$ 8,62 excede R$ 5,17.
        let out = ene01(&com_encargos(862, 172, 17_240, 30), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i7"]), 517)]
        );
        assert_eq!(
            out.findings[0].original_text.as_deref(),
            Some("MULTA POR ATRASO 862")
        );
    }

    #[test]
    fn ene01_multa_de_exatamente_2_por_cento_nao_acusa() {
        assert!(
            ene01(&com_encargos(200, 0, 10_000, 0), &Answers::new())
                .findings
                .is_empty()
        );
    }

    #[test]
    fn ene01_tolerancia_de_5_centavos_na_multa() {
        assert!(
            ene01(&com_encargos(205, 0, 10_000, 0), &Answers::new())
                .findings
                .is_empty()
        );
        let out = ene01(&com_encargos(206, 0, 10_000, 0), &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Unlawful, refs(&["i7"]), 6)]);
    }

    #[test]
    fn ene01_juros_de_1_por_cento_em_30_dias_nao_acusa() {
        // energia-2: 1% de R$ 172,40 = 172,4 -> 172 centavos cobrados.
        let out = ene01(&com_encargos(0, 172, 17_240, 30), &Answers::new());
        assert!(out.findings.is_empty(), "{:?}", out.findings);
    }

    #[test]
    fn ene01_juros_pro_rata_die_com_tolerancia() {
        // R$ 100,00 por 15 dias: limite R$ 0,50; folga até R$ 0,55.
        assert!(
            ene01(&com_encargos(0, 55, 10_000, 15), &Answers::new())
                .findings
                .is_empty()
        );
        let out = ene01(&com_encargos(0, 56, 10_000, 15), &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Unlawful, refs(&["i8"]), 6)]);
        assert!(out.findings[0].explanation.contains("15 dias"));
    }

    #[test]
    fn ene01_multa_e_juros_acima_geram_dois_achados() {
        let out = ene01(&com_encargos(500, 300, 10_000, 30), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![
                (Severity::Unlawful, refs(&["i7"]), 300),
                (Severity::Unlawful, refs(&["i8"]), 200),
            ]
        );
    }

    /// Borda: um único item de encargos (ou nenhum) para multa e juros daria
    /// dois achados com o mesmo id; viram um achado só, com os excessos somados.
    #[test]
    fn ene01_multa_e_juros_no_mesmo_item_viram_um_achado() {
        let mut b = with_items(vec![item("i7", "Encargos", 800, C::EncargoAtraso)]);
        b.late_fees = fees(10_000, 500, 300, 30);
        let out = ene01(&b, &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Unlawful, vec![], 500)]);
        let mut b = with_items(vec![item(
            "i7",
            "Multa e juros de mora",
            800,
            C::EncargoAtraso,
        )]);
        b.late_fees = fees(10_000, 500, 300, 30);
        let out = ene01(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i7"]), 500)]
        );
    }

    #[test]
    fn ene01_sem_encargos_nao_avalia() {
        let b = with_items(vec![item("i7", "Multa por atraso", 900, C::EncargoAtraso)]);
        assert!(ene01(&b, &Answers::new()).findings.is_empty());
        let mut b = with_items(vec![]);
        b.late_fees = fees(0, 900, 900, 30);
        assert!(ene01(&b, &Answers::new()).findings.is_empty());
    }

    // ---------- ENE-02: consumo atípico ----------

    fn com_consumo(kwh: u32, hist: &[u32]) -> ExtractedBill {
        let mut b = energy_bill(energy_items());
        let e = b.energy.as_mut().unwrap();
        e.consumption_kwh = kwh;
        e.history = history(hist);
        b
    }

    #[test]
    fn ene02_consumo_62_por_cento_acima_da_media_e_suspeito() {
        let out = ene02(&com_consumo(324, &[200; 12]), &Answers::new());
        // Valor envolvido: parte da energia acima da média, 16.000 × 124/324.
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1", "i2"]), 6_123)]
        );
        let d = &out.findings[0];
        assert!(d.explanation.contains("324 kWh"), "{}", d.explanation);
        assert!(d.explanation.contains("62%"), "{}", d.explanation);
        assert!(d.explanation.contains("200 kWh"), "{}", d.explanation);
    }

    #[test]
    fn ene02_limiar_de_1_4_vezes_a_media() {
        assert!(
            ene02(&com_consumo(280, &[200; 12]), &Answers::new())
                .findings
                .is_empty()
        );
        assert_eq!(
            ene02(&com_consumo(281, &[200; 12]), &Answers::new())
                .findings
                .len(),
            1
        );
    }

    #[test]
    fn ene02_consumo_normal_nao_acusa() {
        let out = ene02(
            &com_consumo(198, &[190, 185, 210, 220, 205]),
            &Answers::new(),
        );
        assert_eq!(out, Output::default());
    }

    #[test]
    fn ene02_exige_ao_menos_3_meses_de_historico() {
        assert!(
            ene02(&com_consumo(900, &[200, 200]), &Answers::new())
                .findings
                .is_empty()
        );
        assert!(
            ene02(&com_consumo(900, &[]), &Answers::new())
                .findings
                .is_empty()
        );
        assert_eq!(
            ene02(&com_consumo(900, &[200, 200, 200]), &Answers::new())
                .findings
                .len(),
            1
        );
    }

    #[test]
    fn ene02_ignora_o_proprio_mes_de_referencia_no_historico() {
        // Histórico 2026-08 e 2026-09 + o próprio 2026-10 (900): só 2 meses válidos.
        let mut b = com_consumo(900, &[200, 200]);
        let e = b.energy.as_mut().unwrap();
        e.history.push(humanobill_core::ConsumptionPoint {
            month: "2026-10".into(),
            kwh: 900,
        });
        assert!(ene02(&b, &Answers::new()).findings.is_empty());
    }

    #[test]
    fn ene02_usa_so_os_12_meses_mais_recentes() {
        // 14 meses: os 2 mais antigos (1000 kWh) ficam de fora; média 200.
        let mut hist = vec![1_000, 1_000];
        hist.extend([200; 12]);
        let out = ene02(&com_consumo(300, &hist), &Answers::new());
        assert_eq!(out.findings.len(), 1);
    }

    #[test]
    fn ene02_media_zero_com_consumo_e_suspeito() {
        let out = ene02(&com_consumo(150, &[0, 0, 0]), &Answers::new());
        assert_eq!(out.findings.len(), 1);
        assert_eq!(out.findings[0].severity, Severity::Suspicious);
        assert_eq!(out.findings[0].amount_cents, 16_000);
    }

    #[test]
    fn ene02_sem_bloco_de_energia_nao_avalia() {
        let mut b = com_consumo(900, &[200; 12]);
        b.energy = None;
        assert_eq!(ene02(&b, &Answers::new()), Output::default());
    }

    // ---------- ENE-03: leitura não real ----------

    fn com_leitura(rt: ReadingType) -> ExtractedBill {
        let mut b = energy_bill(energy_items());
        b.energy.as_mut().unwrap().reading_type = rt;
        b
    }

    #[test]
    fn ene03_leitura_estimada_e_suspeito() {
        let out = ene03(&com_leitura(ReadingType::Estimada), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1", "i2"]), 16_000)]
        );
        assert!(out.findings[0].explanation.contains("estimada"));
    }

    #[test]
    fn ene03_leitura_pela_media_e_suspeito() {
        let out = ene03(&com_leitura(ReadingType::Media), &Answers::new());
        assert_eq!(out.findings.len(), 1);
        assert_eq!(out.findings[0].severity, Severity::Suspicious);
        assert!(out.findings[0].explanation.contains("média"));
    }

    #[test]
    fn ene03_leitura_real_nao_acusa() {
        assert_eq!(
            ene03(&com_leitura(ReadingType::Real), &Answers::new()),
            Output::default()
        );
    }

    // ---------- ENE-04: ciclo fora de 27 a 33 dias ----------

    fn com_ciclo(prev: (i32, u32, u32), curr: (i32, u32, u32)) -> ExtractedBill {
        let mut b = energy_bill(energy_items());
        let e = b.energy.as_mut().unwrap();
        e.reading_prev_date = date(prev.0, prev.1, prev.2);
        e.reading_curr_date = date(curr.0, curr.1, curr.2);
        b
    }

    #[test]
    fn ene04_ciclo_de_27_a_33_dias_nao_acusa_nem_pergunta() {
        for curr in [(2026, 10, 25), (2026, 10, 28), (2026, 10, 31)] {
            // 27, 30 e 33 dias desde 28/09.
            let out = ene04(&com_ciclo((2026, 9, 28), curr), &Answers::new());
            assert_eq!(out, Output::default(), "{curr:?}");
        }
    }

    #[test]
    fn ene04_ciclo_de_26_dias_e_suspeito_sem_valor() {
        let out = ene04(&com_ciclo((2026, 9, 28), (2026, 10, 24)), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1", "i2"]), 0)]
        );
        assert!(out.findings[0].explanation.contains("26 dias"));
    }

    #[test]
    fn ene04_ciclo_de_34_dias_estima_o_excesso() {
        // Acima de 33 dias, proporcionaliza a 33 (art. 285): 16.000 × 1/34.
        let out = ene04(&com_ciclo((2026, 9, 28), (2026, 11, 1)), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1", "i2"]), 470)]
        );
    }

    #[test]
    fn ene04_pergunta_sobre_primeira_conta_ou_mudanca_de_rota() {
        let out = ene04(&com_ciclo((2026, 9, 28), (2026, 11, 10)), &Answers::new());
        assert_eq!(out.questions.len(), 1);
        let q = &out.questions[0];
        assert_eq!(q.id, "ENE-04:ciclo");
        assert_eq!(q.rule_id, "ENE-04");
        assert_eq!(q.item_id, None);
        assert_eq!(q.kind, QuestionKind::YesNo);
    }

    #[test]
    fn ene04_primeira_conta_ou_mudanca_de_rota_aceita_15_a_47_dias() {
        let sim = ans(&[("ENE-04:ciclo", Answer::Yes)]);
        // 47 dias (28/09 -> 14/11): aceito com a exceção do § 1º.
        let out = ene04(&com_ciclo((2026, 9, 28), (2026, 11, 14)), &sim);
        assert!(out.findings.is_empty());
        // 48 dias: nem a exceção cobre.
        let out = ene04(&com_ciclo((2026, 9, 28), (2026, 11, 15)), &sim);
        assert_eq!(out.findings.len(), 1);
        // 15 dias aceito, 14 não.
        assert!(
            ene04(&com_ciclo((2026, 10, 1), (2026, 10, 16)), &sim)
                .findings
                .is_empty()
        );
        assert_eq!(
            ene04(&com_ciclo((2026, 10, 1), (2026, 10, 15)), &sim)
                .findings
                .len(),
            1
        );
        // "Não" mantém o suspeito.
        let nao = ans(&[("ENE-04:ciclo", Answer::No)]);
        assert_eq!(
            ene04(&com_ciclo((2026, 9, 28), (2026, 11, 14)), &nao)
                .findings
                .len(),
            1
        );
    }

    #[test]
    fn ene04_datas_invertidas_ou_iguais_nao_avalia() {
        for curr in [(2026, 9, 1), (2026, 9, 28)] {
            assert_eq!(
                ene04(&com_ciclo((2026, 9, 28), curr), &Answers::new()),
                Output::default()
            );
        }
    }

    // ---------- ENE-05: bandeira tarifária ----------

    /// P08: `bandeiras.json` (gerado do dado aberto da ANEEL) tem, para cada mês
    /// de `bandeiras-oficiais.md` (notícias da ANEEL), a mesma cor.
    #[test]
    fn ene05_tabela_embutida_confere_com_bandeiras_oficiais() {
        let md = include_str!("../../../fixtures/tariffs/bandeiras-oficiais.md");
        let mut meses = 0;
        for linha in md.lines().filter(|l| l.starts_with("| 20")) {
            let cols: Vec<&str> = linha.split('|').map(str::trim).collect();
            let (mes, cor) = (cols[1], cols[2]);
            let esperado = parse_flag(cor).unwrap_or_else(|| panic!("cor ilegível: {cor}"));
            assert_eq!(flag_table().get(mes), Some(&esperado), "{mes}");
            meses += 1;
        }
        // out/2025 a nov/2026.
        assert_eq!(meses, 14);
        assert!(flag_table().len() >= meses);
    }

    #[test]
    fn ene05_le_a_bandeira_do_texto_do_item() {
        for (txt, f) in [
            ("Bandeira tarifária vermelha patamar 1", Some(F::Vermelha1)),
            ("ADIC BANDEIRA VERMELHA P1 205 KWH 0,91", Some(F::Vermelha1)),
            ("Bandeira Vermelha 2", Some(F::Vermelha2)),
            ("BAND. VERMELHA PATAMAR II", Some(F::Vermelha2)),
            ("Adicional bandeira amarela", Some(F::Amarela)),
            ("Bandeira verde", Some(F::Verde)),
            ("Bandeira escassez hídrica", Some(F::Escassez)),
            ("Bandeira vermelha 1,00", None),
            ("Adicional bandeira", None),
        ] {
            assert_eq!(parse_flag(txt), f, "{txt}");
        }
    }

    fn com_bandeira(
        prev: (i32, u32, u32),
        curr: (i32, u32, u32),
        flag: Option<F>,
        item_desc: Option<&str>,
    ) -> ExtractedBill {
        let mut items = energy_items();
        if let Some(d) = item_desc {
            items.push(item("i4", d, 91, C::Bandeira));
        }
        let mut b = com_ciclo(prev, curr);
        b.items = items;
        b.total_cents = b.items.iter().map(|i| i.amount_cents).sum();
        b.energy.as_mut().unwrap().tariff_flag = flag;
        b
    }

    #[test]
    fn ene05_vermelha_em_ciclo_de_setembro_a_outubro_e_suspeito() {
        // energia-3: setembro amarela, outubro verde; vermelha 1 não é nenhuma.
        let b = com_bandeira(
            (2026, 9, 29),
            (2026, 10, 29),
            Some(F::Vermelha1),
            Some("Bandeira tarifária vermelha patamar 1"),
        );
        let out = ene05(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i4"]), 91)]
        );
        let e = &out.findings[0].explanation;
        assert!(e.contains("vermelha patamar 1"), "{e}");
        assert!(e.contains("amarela ou verde"), "{e}");
    }

    #[test]
    fn ene05_aceita_bandeira_de_qualquer_mes_do_ciclo() {
        for flag in [F::Amarela, F::Verde] {
            let b = com_bandeira((2026, 9, 29), (2026, 10, 29), Some(flag), None);
            assert_eq!(ene05(&b, &Answers::new()), Output::default(), "{flag:?}");
        }
    }

    #[test]
    fn ene05_ciclo_so_de_outubro_cobrando_amarela_e_suspeito() {
        let b = com_bandeira(
            (2026, 10, 1),
            (2026, 10, 30),
            Some(F::Amarela),
            Some("Adicional bandeira amarela"),
        );
        let out = ene05(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i4"]), 91)]
        );
    }

    #[test]
    fn ene05_so_a_bandeira_declarada_sem_item_acusa_sem_referencia() {
        let b = com_bandeira((2026, 10, 1), (2026, 10, 30), Some(F::Vermelha2), None);
        let out = ene05(&b, &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Suspicious, vec![], 0)]);
    }

    #[test]
    fn ene05_item_sem_cor_usa_a_bandeira_declarada() {
        let b = com_bandeira(
            (2026, 10, 1),
            (2026, 10, 30),
            Some(F::Vermelha1),
            Some("Adicional bandeira"),
        );
        let out = ene05(&b, &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i4"]), 91)]
        );
        // Sem cor no item nem bandeira declarada: nada a comparar.
        let b = com_bandeira(
            (2026, 10, 1),
            (2026, 10, 30),
            None,
            Some("Adicional bandeira"),
        );
        assert_eq!(ene05(&b, &Answers::new()), Output::default());
    }

    /// Com item de bandeira, vale a cor do item (é onde está o dinheiro); a bandeira
    /// declarada no cabeçalho só entra quando não há item ou o item não diz a cor.
    #[test]
    fn ene05_cor_do_item_prevalece_sobre_a_declarada() {
        let b = com_bandeira(
            (2026, 9, 29),
            (2026, 10, 29),
            Some(F::Vermelha1),
            Some("Adicional bandeira amarela"),
        );
        assert_eq!(ene05(&b, &Answers::new()), Output::default());
    }

    #[test]
    fn ene05_mes_fora_da_tabela_nao_avalia() {
        // Mês ainda não anunciado (a tabela é regerada todo mês).
        let b = com_bandeira(
            (2099, 11, 28),
            (2099, 12, 28),
            Some(F::Vermelha2),
            Some("Bandeira vermelha patamar 2"),
        );
        assert_eq!(ene05(&b, &Answers::new()), Output::default());
    }

    /// Borda: ciclo longo tocando 4 meses com 4 cores diferentes (out/2025 a jan/2026
    /// têm vermelha 1, amarela e verde); a explicação continua com até 25 palavras.
    #[test]
    fn ene05_explicacao_curta_mesmo_com_muitas_bandeiras() {
        let b = com_bandeira((2025, 10, 1), (2026, 1, 20), Some(F::Escassez), None);
        let out = ene05(&b, &Answers::new());
        assert_eq!(out.findings.len(), 1);
        let e = &out.findings[0].explanation;
        assert!(e.contains("vermelha patamar 1, amarela ou verde"), "{e}");
        assert!(e.split_whitespace().count() <= 25, "{e}");
    }

    #[test]
    fn ene05_nunca_pretende_indevido() {
        let b = com_bandeira((2026, 10, 1), (2026, 10, 30), Some(F::Escassez), None);
        for d in ene05(&b, &Answers::new()).findings {
            assert_eq!(d.severity, Severity::Suspicious);
        }
    }

    // ---------- ENE-06: tarifa acima da homologada ----------

    /// Tabela fictícia: reajuste em 01/10/2026 (TUSD 500 -> 600 R$/MWh, TE 300).
    const TARIFAS_TESTE: &str = r#"{
        "distribuidoras": [{
            "sigla": "FORCA-T",
            "cnpj": "00000000000000",
            "nomes": ["Força Teste", "Força Teste Distribuição S.A."],
            "vigencias": [
                {"inicio": "2026-01-01", "fim": "2026-09-30",
                 "tusd_rs_mwh": 500.0, "te_rs_mwh": 300.0, "resolucao": "REH teste 1"},
                {"inicio": "2026-10-01", "fim": "2027-09-30",
                 "tusd_rs_mwh": 600.0, "te_rs_mwh": 300.0, "resolucao": "REH teste 2"}
            ]
        }]
    }"#;

    /// 200 kWh, TUSD e TE nos valores dados, ICMS à parte, ciclo de `prev` a `curr`.
    fn com_tarifa(
        provider: &str,
        tusd: i64,
        te: i64,
        prev: (i32, u32, u32),
        curr: (i32, u32, u32),
    ) -> ExtractedBill {
        let mut b = com_ciclo(prev, curr);
        b.provider_name = provider.into();
        b.items = vec![
            item("i1", "Uso da rede (TUSD) 200 kWh", tusd, C::Energia),
            item("i2", "Energia consumida (TE) 200 kWh", te, C::Energia),
            item("i3", "ICMS 18%", 3_000, C::Imposto),
        ];
        b.total_cents = b.items.iter().map(|i| i.amount_cents).sum();
        b
    }

    fn ene06_teste(b: &ExtractedBill) -> Output {
        ene06_with(&parse_tariffs(TARIFAS_TESTE), b)
    }

    const OUT: (i32, u32, u32) = (2026, 10, 1);
    const OUT_FIM: (i32, u32, u32) = (2026, 10, 31);

    #[test]
    fn ene06_tusd_acima_da_homologada_e_suspeito() {
        // Homologada: 600 R$/MWh × 200 kWh = R$ 120,00; cobrado R$ 130,00.
        let b = com_tarifa("Força Teste", 13_000, 6_000, OUT, OUT_FIM);
        let out = ene06_teste(&b);
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i1"]), 1_000)]
        );
        let e = &out.findings[0].explanation;
        assert!(e.contains("R$ 0,6500"), "{e}");
        assert!(e.contains("R$ 0,6000"), "{e}");
        assert!(e.contains("TUSD"), "{e}");
        assert!(e.contains("Força Teste"), "{e}");
    }

    #[test]
    fn ene06_tarifa_igual_ou_abaixo_da_homologada_nao_acusa() {
        for (tusd, te) in [(12_000, 6_000), (11_000, 5_500)] {
            let b = com_tarifa("Força Teste", tusd, te, OUT, OUT_FIM);
            assert_eq!(ene06_teste(&b), Output::default(), "{tusd} {te}");
        }
    }

    /// Borda: tolerância de 2% (R$ 120,00 -> até R$ 122,40 não acusa).
    #[test]
    fn ene06_tolerancia_de_2_por_cento() {
        let b = com_tarifa("Força Teste", 12_240, 6_120, OUT, OUT_FIM);
        assert_eq!(ene06_teste(&b), Output::default());
        let b = com_tarifa("Força Teste", 12_241, 6_121, OUT, OUT_FIM);
        assert_eq!(
            sev_refs(&ene06_teste(&b)),
            vec![
                (Severity::Suspicious, refs(&["i1"]), 241),
                (Severity::Suspicious, refs(&["i2"]), 121),
            ]
        );
    }

    /// REN 1000 art. 173, § 2º: reajuste no meio do ciclo usa a tarifa proporcional.
    /// 15 dias a 500 e 15 a 600: média 550, R$ 110,00. Cobrar R$ 115,00 acusa,
    /// mesmo abaixo da tarifa nova (R$ 120,00).
    #[test]
    fn ene06_reajuste_no_ciclo_usa_tarifa_proporcional() {
        let b = com_tarifa("Força Teste", 11_500, 6_000, (2026, 9, 16), (2026, 10, 16));
        assert_eq!(
            sev_refs(&ene06_teste(&b)),
            vec![(Severity::Suspicious, refs(&["i1"]), 500)]
        );
        let b = com_tarifa("Força Teste", 11_000, 6_000, (2026, 9, 16), (2026, 10, 16));
        assert_eq!(ene06_teste(&b), Output::default());
    }

    #[test]
    fn ene06_nome_da_distribuidora_normalizado() {
        for nome in [
            "FORÇA TESTE",
            "Força Teste Distribuição S/A",
            "forca  teste",
        ] {
            let b = com_tarifa(nome, 13_000, 6_000, OUT, OUT_FIM);
            assert_eq!(ene06_teste(&b).findings.len(), 1, "{nome}");
        }
        // Parecido não basta: a comparação é do nome inteiro.
        for nome in ["Força", "Força Teste Norte", "Luzia Energia"] {
            let b = com_tarifa(nome, 13_000, 6_000, OUT, OUT_FIM);
            assert_eq!(ene06_teste(&b), Output::default(), "{nome}");
        }
    }

    #[test]
    fn ene06_sem_vigencia_que_cubra_o_ciclo_nao_avalia() {
        // Começa antes da primeira vigência da tabela.
        let b = com_tarifa("Força Teste", 50_000, 50_000, (2025, 12, 15), (2026, 1, 14));
        assert_eq!(ene06_teste(&b), Output::default());
    }

    /// Sem imposto destacado, o preço da linha pode já incluir tributos: não compara.
    #[test]
    fn ene06_sem_imposto_destacado_nao_avalia() {
        let mut b = com_tarifa("Força Teste", 13_000, 6_000, OUT, OUT_FIM);
        b.items.retain(|i| i.category != C::Imposto);
        assert_eq!(ene06_teste(&b), Output::default());
    }

    /// Tarifa branca (postos horários) não é a convencional da tabela.
    #[test]
    fn ene06_tarifa_branca_nao_avalia() {
        let mut b = com_tarifa("Força Teste", 13_000, 6_000, OUT, OUT_FIM);
        b.items[0].description = "TUSD ponta 40 kWh".into();
        assert_eq!(ene06_teste(&b), Output::default());
    }

    #[test]
    fn ene06_linha_unica_de_te_mais_tusd_compara_a_soma() {
        let mut b = com_tarifa("Força Teste", 0, 0, OUT, OUT_FIM);
        b.items = vec![
            item("i1", "Consumo TE + TUSD 200 kWh", 19_000, C::Energia),
            item("i3", "ICMS 18%", 3_000, C::Imposto),
        ];
        // Homologada: (600 + 300) × 0,2 = R$ 180,00.
        assert_eq!(
            sev_refs(&ene06_teste(&b)),
            vec![(Severity::Suspicious, refs(&["i1"]), 1_000)]
        );
    }

    #[test]
    fn ene06_demo_luzia_energia_nunca_avalia() {
        for f in ["energia-1", "energia-2", "energia-3"] {
            let path = format!(
                "{}/../../fixtures/extracted/{f}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let b: ExtractedBill =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            assert_eq!(ene06(&b, &Answers::new()), Output::default(), "{f}");
        }
    }

    #[test]
    fn ene06_sem_bloco_de_energia_ou_sem_consumo_nao_avalia() {
        let mut b = com_tarifa("Força Teste", 13_000, 6_000, OUT, OUT_FIM);
        b.energy.as_mut().unwrap().consumption_kwh = 0;
        assert_eq!(ene06_teste(&b), Output::default());
        b.energy = None;
        assert_eq!(ene06_teste(&b), Output::default());
    }

    /// A tabela embutida (dado aberto da ANEEL) é válida: 5 distribuidoras, nomes,
    /// vigências em ordem e tarifas positivas.
    #[test]
    fn ene06_tabela_embutida_e_valida() {
        let t = tariff_table();
        assert_eq!(t.len(), 5);
        for d in t {
            assert!(!d.name.is_empty() && !d.keys.is_empty(), "{d:?}");
            assert!(!d.validities.is_empty(), "{}", d.name);
            for v in &d.validities {
                assert!(
                    v.start <= v.end && v.tusd > 0 && v.te > 0,
                    "{}: {v:?}",
                    d.name
                );
            }
            for w in d.validities.windows(2) {
                assert!(w[0].end < w[1].start, "{}", d.name);
            }
        }
        // Valor conferido no CSV da ANEEL (REH 3.589/2026, Cemig, B1 residencial).
        let cemig = t.iter().find(|d| d.name == "Cemig").unwrap();
        let v = cemig
            .validities
            .iter()
            .find(|v| v.start == date(2026, 5, 28))
            .unwrap();
        assert_eq!((v.tusd, v.te), (59_308, 31_021));
    }

    // ---------- ENE-07: itens que não são energia ----------

    fn seguro() -> BillItem {
        item("i6", "Seguro Residencial Luzia", 1_290, C::ServicoAdicional)
    }

    #[test]
    fn ene07_servico_adicional_sem_resposta_e_suspeito() {
        let out = ene07(&with_items(vec![seguro()]), &Answers::new());
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Suspicious, refs(&["i6"]), 1_290)]
        );
        assert_eq!(
            out.findings[0].original_text.as_deref(),
            Some("SEGURO RESIDENCIAL LUZIA 1290")
        );
    }

    #[test]
    fn ene07_gera_pergunta_por_item() {
        let out = ene07(&with_items(vec![seguro()]), &Answers::new());
        assert_eq!(out.questions.len(), 1);
        let q = &out.questions[0];
        assert_eq!(q.id, "ENE-07:i6");
        assert_eq!(q.rule_id, "ENE-07");
        assert_eq!(q.item_id.as_deref(), Some("i6"));
        assert_eq!(
            q.text,
            "Você autorizou Seguro Residencial Luzia na sua conta de luz?"
        );
        assert_eq!(q.kind, QuestionKind::YesNo);
    }

    #[test]
    fn ene07_nao_autorizei_pretende_indevido() {
        let out = ene07(
            &with_items(vec![seguro()]),
            &ans(&[("ENE-07:i6", Answer::No)]),
        );
        assert_eq!(
            sev_refs(&out),
            vec![(Severity::Unlawful, refs(&["i6"]), 1_290)]
        );
    }

    #[test]
    fn ene07_autorizei_e_ok() {
        let out = ene07(
            &with_items(vec![seguro()]),
            &ans(&[("ENE-07:i6", Answer::Yes)]),
        );
        assert_eq!(sev_refs(&out), vec![(Severity::Ok, refs(&["i6"]), 1_290)]);
    }

    #[test]
    fn ene07_resposta_de_tipo_errado_conta_como_sem_resposta() {
        let out = ene07(
            &with_items(vec![seguro()]),
            &ans(&[("ENE-07:i6", Answer::Amount(1))]),
        );
        assert_eq!(out.findings[0].severity, Severity::Suspicious);
    }

    #[test]
    fn ene07_palavra_chave_em_item_outros() {
        for desc in [
            "Doação Hospital Esperança",
            "DOAÇÕES",
            "Assinatura Revista Casa",
            "Clube de Vantagens Luzia",
            "Contribuição Associação Amigos",
            "Contribuições voluntárias",
            "Assistência 24h",
            "Proteção elétrica",
            "Título de capitalização",
        ] {
            let out = ene07(
                &with_items(vec![item("i9", desc, 500, C::Outros)]),
                &Answers::new(),
            );
            assert_eq!(
                sev_refs(&out),
                vec![(Severity::Suspicious, refs(&["i9"]), 500)],
                "{desc}"
            );
        }
    }

    #[test]
    fn ene07_ignora_energia_bandeira_imposto_iluminacao_e_encargo() {
        let b = with_items(vec![
            item("i4", "Bandeira amarela", 300, C::Bandeira),
            item(
                "i5",
                "Contribuição de iluminação pública",
                1_430,
                C::IluminacaoPublica,
            ),
            item("i6", "Multa seguro atraso", 100, C::EncargoAtraso),
            item("i7", "Seguro (imposto)", 100, C::Imposto),
            // IA classificou a CIP como "outros": continua não sendo extra.
            item("i8", "Contribuição Ilum. Pública (CIP)", 1_430, C::Outros),
            item("i9", "COSIP contribuição municipal", 1_430, C::Outros),
            item("i10", "Ajuste de faturamento", 100, C::Outros),
        ]);
        let out = ene07(&b, &Answers::new());
        assert_eq!(out, Output::default());
    }

    // ---------- ENE-08: soma não bate ----------

    #[test]
    fn ene08_soma_diferente_do_total_e_suspeito() {
        let mut b = energy_bill(energy_items());
        b.total_cents += 100;
        let out = ene08(&b, &Answers::new());
        assert_eq!(sev_refs(&out), vec![(Severity::Suspicious, vec![], 100)]);
    }

    #[test]
    fn ene08_tolerancia_de_5_centavos() {
        let mut b = energy_bill(energy_items());
        b.total_cents += 5;
        assert!(ene08(&b, &Answers::new()).findings.is_empty());
        b.total_cents += 1;
        assert_eq!(ene08(&b, &Answers::new()).findings.len(), 1);
    }

    #[test]
    fn ene08_soma_igual_ao_total_nao_marca() {
        assert_eq!(
            ene08(&energy_bill(energy_items()), &Answers::new()),
            Output::default()
        );
    }

    #[test]
    fn ene05_bandeira_mais_barata_que_a_vigente_nao_acusa() {
        let mut b: ExtractedBill = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/extracted/energia-3.json"
        )))
        .unwrap();
        // Ciclo todo em agosto de 2026 (amarela); a conta cobra verde (mais barata).
        if let Some(e) = b.energy.as_mut() {
            e.tariff_flag = Some(TariffFlag::Verde);
            e.reading_prev_date = chrono::NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
            e.reading_curr_date = chrono::NaiveDate::from_ymd_opt(2026, 8, 31).unwrap();
        }
        b.items
            .retain(|i| i.category != humanobill_core::ItemCategory::Bandeira);
        b.total_cents = b.items.iter().map(|i| i.amount_cents).sum();
        assert!(ene05(&b, &Answers::new()).findings.is_empty());
        // Contraprova: no mesmo ciclo, cobrar vermelha 1 (mais cara) acusa.
        if let Some(e) = b.energy.as_mut() {
            e.tariff_flag = Some(TariffFlag::Vermelha1);
        }
        assert_eq!(ene05(&b, &Answers::new()).findings.len(), 1);
    }
}
