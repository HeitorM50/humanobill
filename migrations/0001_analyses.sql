-- Análises anonimizadas (sem arquivo, sem nome/CPF/endereço). Expiram em 30 dias (RF14).
create table if not exists analyses (
  id                uuid primary key,
  created_at        timestamptz not null default now(),
  expires_at        timestamptz not null default now() + interval '30 days',
  source            text not null check (source in ('upload', 'demo')),
  kind              text not null check (kind in ('telecom', 'energy')),
  bill              jsonb not null,
  answers           jsonb not null default '{}'::jsonb,
  paid              boolean,
  findings          jsonb not null default '[]'::jsonb,
  questions         jsonb not null default '[]'::jsonb,
  recoverable_cents bigint not null default 0,
  rules_version     text not null
);

create index if not exists analyses_expires_at_idx on analyses (expires_at);

-- Só a API (role do servidor) acessa a tabela; nenhuma policy pública.
alter table analyses enable row level security;
