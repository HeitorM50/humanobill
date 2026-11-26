-- Liga o RLS na tabela de controle do sqlx (alerta crítico do Supabase Advisor).
-- Sem policy, os papéis anon e authenticated ficam sem acesso nenhum. A API não
-- é afetada: conecta pelo Postgres direto, com o papel dono da tabela.
alter table _sqlx_migrations enable row level security;
