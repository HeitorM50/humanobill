//! JSON Schema enviado ao modelo (`responseJsonSchema`).
//! Espelha `humanobill_core::ExtractedBill` SEM nenhum campo pessoal (RNF04).

use serde_json::{Value, json};

pub fn extracted_bill_schema() -> Value {
    let date = json!({"type": "string", "description": "YYYY-MM-DD"});
    let cents = json!({"type": "integer", "description": "valor em centavos"});
    json!({
        "type": "object",
        "properties": {
            "kind": {"type": "string", "enum": ["telecom", "energy"]},
            "provider_name": {"type": "string"},
            "reference_month": {"type": "string", "description": "YYYY-MM"},
            "due_date": date,
            "issue_date": {"type": ["string", "null"], "description": "YYYY-MM-DD"},
            "total_cents": cents,
            "plan_name": {"type": ["string", "null"]},
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "description": {"type": "string"},
                        "amount_cents": cents,
                        "category": {"type": "string", "enum": [
                            "plano", "servico_adicional", "encargo_atraso", "imposto",
                            "energia", "bandeira", "iluminacao_publica", "outros"
                        ]},
                        "period": {
                            "type": ["object", "null"],
                            "properties": {"start": date, "end": date},
                            "required": ["start", "end"]
                        },
                        "original_text": {"type": "string"},
                        "page": {"type": "integer", "minimum": 1},
                        "plain_explanation": {"type": "string"}
                    },
                    "required": ["id", "description", "amount_cents", "category", "original_text", "page", "plain_explanation"]
                }
            },
            "late_fees": {
                "type": ["object", "null"],
                "properties": {
                    "base_cents": cents, "fine_cents": cents, "interest_cents": cents,
                    "days_late": {"type": "integer", "minimum": 0}
                },
                "required": ["base_cents", "fine_cents", "interest_cents", "days_late"]
            },
            "energy": {
                "type": ["object", "null"],
                "properties": {
                    "consumption_kwh": {"type": "integer", "minimum": 0},
                    "reading_type": {"type": "string", "enum": ["real", "estimada", "media"]},
                    "reading_prev_date": date,
                    "reading_curr_date": date,
                    "tariff_flag": {"type": ["string", "null"], "enum": ["verde", "amarela", "vermelha_1", "vermelha_2", "escassez", null]},
                    "history": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {"month": {"type": "string", "description": "YYYY-MM"}, "kwh": {"type": "integer", "minimum": 0}},
                            "required": ["month", "kwh"]
                        }
                    }
                },
                "required": ["consumption_kwh", "reading_type", "reading_prev_date", "reading_curr_date", "history"]
            },
            "extraction_confidence": {"type": "number", "minimum": 0, "maximum": 1}
        },
        "required": ["kind", "provider_name", "reference_month", "due_date", "total_cents", "items", "extraction_confidence"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RNF04: nenhum campo pessoal pode existir no schema enviado à IA.
    #[test]
    fn schema_sem_campos_pessoais() {
        let s = extracted_bill_schema().to_string().to_lowercase();
        for proibido in [
            "cpf", "cnpj", "address", "endereco", "holder", "titular", "instala", "phone", "email",
            "\"name\"",
        ] {
            assert!(
                !s.contains(proibido),
                "campo proibido no schema: {proibido}"
            );
        }
    }
}
