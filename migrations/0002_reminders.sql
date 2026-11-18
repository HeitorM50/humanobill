-- Lembrete opcional por e-mail (RF18/RF19). Um por análise.
-- O e-mail é apagado ao enviar o aviso ou ao cancelar (a linha inteira some).
-- A linha também some junto com a análise (expira em 30 dias).
create table if not exists reminders (
  token       text primary key,
  analysis_id uuid not null unique references analyses (id) on delete cascade,
  email       text not null,
  due_at      timestamptz not null,
  created_at  timestamptz not null default now()
);

create index if not exists reminders_due_at_idx on reminders (due_at);

-- Só a API (role do servidor) acessa a tabela; nenhuma policy pública.
alter table reminders enable row level security;
