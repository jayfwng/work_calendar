// 배포 전에 본인 Supabase 프로젝트 값을 넣어 두면 각 기기에서 URL/키를 따로 입력하지 않아도 된다.
// (Supabase → Project Settings → API). 공개 키(anon / publishable)는 원래 공개용이라 넣어도 된다.
// 절대 service_role / secret 키는 넣지 말 것.
window.WORKCAL_CONFIG = {
  supabaseUrl: '',   // 예: 'https://abcdefgh.supabase.co'
  supabaseKey: '',   // 예: 'sb_publishable_...' 또는 'eyJ...'(anon)
};
