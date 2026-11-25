-- "Custom day": the days of the week picked to aim for. "Any day" became custom with all seven days,
-- so 'allDays' stays allowed for rows written before this (apps read it as custom).

alter table public.settings drop constraint settings_day_toggle_check;
alter table public.settings add constraint settings_day_toggle_check
  check (day_toggle in ('allDays', 'weekdaysOnly', 'weekendsOnly', 'custom'));

alter table public.settings
  add column custom_days text[] not null
    default array['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun']
    check (
      cardinality(custom_days) between 1 and 7
      and custom_days <@ array['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun']
    );
