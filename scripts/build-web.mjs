// 웹 앱 파일을 www/로 모은다 (Capacitor가 APK 안에 넣는 폴더).
// sw.js는 넣지 않는다: 앱 안에서는 파일이 이미 기기에 있어서 오프라인 캐시가 필요 없다.
import { rmSync, mkdirSync, cpSync } from 'node:fs';
const out = 'www';
rmSync(out, { recursive: true, force: true });
mkdirSync(out);
for (const f of ['index.html', 'config.js', 'manifest.webmanifest', 'icons']) cpSync(f, `${out}/${f}`, { recursive: true });
console.log('web files copied to www/');
