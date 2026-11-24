-- Friends sync: each account's people, blocked ranges and day setting.
--
-- Run once in the Supabase project's SQL Editor (or with `supabase db push`).
--
-- The app's key is public, so these rules are what keep each list private: every row belongs to
-- one account (`owner`), and a signed-in user can only read and write their own rows. Signed-out
-- requests see nothing.
--
-- How syncing uses the tables (the rules are in crates/friends-core/src/sync.rs):
-- - `id` is the row's id on every device; `modified_at` is when it last changed on a device (ms since
--   1970). When two devices change the same row, the later change wins: an upload with an older
--   `modified_at` than the stored one is ignored.
-- - Deleting keeps the row with `deleted = true` and its details wiped, so other devices learn of it.
-- - `updated_ms` is the server's own time of the last write, so a device can ask for what changed
--   since it last looked.

create table public.people (
  owner uuid not null default auth.uid() references auth.users (id) on delete cascade,
  id uuid not null,
  name text check (char_length(name) between 1 and 100),
  frequency_days integer check (frequency_days between 1 and 3653),
  last_connected date,
  deleted boolean not null default false,
  modified_at bigint not null,
  updated_ms bigint not null default 0,
  primary key (owner, id),
  check (deleted or (name is not null and frequency_days is not null and last_connected is not null))
);

create table public.blocked_dates (
  owner uuid not null default auth.uid() references auth.users (id) on delete cascade,
  id uuid not null,
  starts_on date,
  ends_on date,
  label text check (char_length(label) <= 100),
  deleted boolean not null default false,
  modified_at bigint not null,
  updated_ms bigint not null default 0,
  primary key (owner, id),
  check (deleted or (starts_on is not null and ends_on is not null and label is not null and starts_on <= ends_on))
);

create table public.settings (
  owner uuid primary key default auth.uid() references auth.users (id) on delete cascade,
  day_toggle text not null check (day_toggle in ('allDays', 'weekdaysOnly', 'weekendsOnly')),
  modified_at bigint not null,
  updated_ms bigint not null default 0
);

create index people_owner_updated on public.people (owner, updated_ms);
create index blocked_dates_owner_updated on public.blocked_dates (owner, updated_ms);

-- Every write: the row belongs to whoever is signed in, gets the server's time, and an older change
-- never replaces a newer one.
create function public.friends_stamp() returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if tg_op = 'UPDATE' and new.modified_at < old.modified_at then
    return null;
  end if;
  new.owner := auth.uid();
  new.updated_ms := (extract(epoch from clock_timestamp()) * 1000)::bigint;
  return new;
end;
$$;

create trigger people_stamp before insert or update on public.people
  for each row execute function public.friends_stamp();
create trigger blocked_dates_stamp before insert or update on public.blocked_dates
  for each row execute function public.friends_stamp();
create trigger settings_stamp before insert or update on public.settings
  for each row execute function public.friends_stamp();

alter table public.people enable row level security;
alter table public.blocked_dates enable row level security;
alter table public.settings enable row level security;

create policy "Own people" on public.people for all to authenticated
  using (owner = (select auth.uid())) with check (owner = (select auth.uid()));
create policy "Own blocked dates" on public.blocked_dates for all to authenticated
  using (owner = (select auth.uid())) with check (owner = (select auth.uid()));
create policy "Own settings" on public.settings for all to authenticated
  using (owner = (select auth.uid())) with check (owner = (select auth.uid()));

-- Signed-out requests get nothing; signed-in ones may read, add and change (not delete) rows.
revoke all on public.people, public.blocked_dates, public.settings from anon, authenticated;
grant select, insert, update on public.people, public.blocked_dates, public.settings to authenticated;
