-- Novos tipos de conta: extrato bancário (#54) e fatura do cartão de crédito (#55).
-- Troca o check do tipo criado na 0001 (nome automático do Postgres: analyses_kind_check).
alter table analyses drop constraint if exists analyses_kind_check;
alter table analyses add constraint analyses_kind_check
  check (kind in ('telecom', 'energy', 'bank_statement', 'card_bill'));
