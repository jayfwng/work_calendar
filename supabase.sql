-- 업무 캘린더 동기화용 Supabase 설정
-- Supabase 대시보드 → SQL Editor → 이 파일 내용을 붙여 넣고 Run.
--
-- 서버에는 [저장소 ID(키에서 유도한 해시), 암호문, 버전]만 저장된다.
-- 테이블은 직접 읽거나 목록을 볼 수 없게 막고, 아래 두 함수로만 접근한다.
-- 저장소 ID를 모르면 남의 데이터를 읽거나 덮어쓸 수 없다.

create table if not exists public.vaults (
  vid        text primary key check (vid ~ '^[0-9a-f]{64}$'),
  data       text not null check (length(data) <= 5000000),
  rev        bigint not null default 1,
  updated_at timestamptz not null default now()
);

alter table public.vaults enable row level security;   -- 정책을 만들지 않으므로 직접 접근은 전부 거부
revoke all on public.vaults from anon, authenticated;

create or replace function public.get_vault(p_vid text)
returns json
language sql
security definer
set search_path = public
as $$
  select json_build_object('data', data, 'rev', rev) from public.vaults where vid = p_vid;
$$;

-- p_rev: 마지막으로 받은 버전(없으면 0). 그 사이 다른 기기가 저장했으면 ok=false를 돌려주고, 앱이 다시 받아 병합한다.
create or replace function public.put_vault(p_vid text, p_data text, p_rev bigint)
returns json
language plpgsql
security definer
set search_path = public
as $$
declare
  cur bigint;
begin
  select rev into cur from public.vaults where vid = p_vid for update;
  if cur is null then
    if p_rev <> 0 then
      return json_build_object('ok', false, 'rev', 0);
    end if;
    insert into public.vaults (vid, data) values (p_vid, p_data)
      on conflict (vid) do nothing;
    if not found then
      return json_build_object('ok', false, 'rev', 1);
    end if;
    return json_build_object('ok', true, 'rev', 1);
  end if;
  if cur <> p_rev then
    return json_build_object('ok', false, 'rev', cur);
  end if;
  update public.vaults set data = p_data, rev = rev + 1, updated_at = now() where vid = p_vid;
  return json_build_object('ok', true, 'rev', cur + 1);
end;
$$;

revoke all on function public.get_vault(text) from public;
revoke all on function public.put_vault(text, text, bigint) from public;
grant execute on function public.get_vault(text) to anon;
grant execute on function public.put_vault(text, text, bigint) to anon;
