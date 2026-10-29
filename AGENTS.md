# AGENTS.md

Windows x64 트레이 앱. 캡처 동작은 SnapDrag 0.4.0과 같고, 창은 Tauri다.

## 명령

관리자로 실행하지 않는다.

```powershell
cargo run --bin agsnaptool
cargo test
```

버전은 `Cargo.toml`의 `version`이다. `CHANGELOG.md` 첫 `##` 번호와 같아야 한다.

## 맵

- `src/logic.rs` — 모디파이어, 클릭 시리즈, 경로, 갤러리 목록, DIB, HDROP
- `src/strings.rs` — 영어 중립, 한국어
- `src/win.rs` — 훅, 캡처, WIC, 클립보드
- `src/main.rs` — Tauri 창, 트레이, 알림
- `ui/` — 모아보기
- `tests/behavior.rs` — 순수 동작

## 경계

- 훅 프로시저 안에서 캡처, UI Automation, 클립보드를 호출하지 않는다.
- 좌클릭은 down만 삼킨다. 우클릭은 down과 up을 삼킨다.
- 모디파이어+좌 드래그로 영역을 고르는 동안은 move와 up도 삼킨다.
- `CF_HDROP` + `PNG` + `CF_DIB`만 넣는다.
- 클립보드에 넣은 직후 PNG를 지우지 않는다.
- 창을 닫아도 캡처는 남는다. 종료는 트레이 메뉴다.
