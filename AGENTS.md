# AGENTS.md

Windows x64 트레이 앱. 동작 기준은 `D:\projects\SnapDrag`의 SnapDrag 0.4.0이다. 설정 파일 경로는 그 앱과 같다.

## 명령

관리자로 실행하지 않는다.

```powershell
cargo run --bin snapdrag
cargo test
```

버전은 `Cargo.toml`의 `version`이다. `CHANGELOG.md` 첫 `##` 번호와 같아야 한다.

## 맵

- `src/logic.rs` — 모디파이어, 클릭 시리즈, 경로, DIB, HDROP
- `src/strings.rs` — 영어 중립, 한국어
- `src/win.rs` — 훅, 캡처, WIC, 클립보드, 트레이
- `src/dialogs.rs` — 보관 일수, 버전 창
- `tests/behavior.rs` — 순수 동작

## 경계

- 훅 프로시저 안에서 캡처, UI Automation, 클립보드를 호출하지 않는다.
- 좌클릭은 down만 삼킨다. 우클릭은 down과 up을 삼킨다.
- `CF_HDROP` + `PNG` + `CF_DIB`만 넣는다. `FileGroupDescriptor`와 `DoDragDrop`은 넣지 않는다.
- 클립보드에 넣은 직후 PNG를 지우지 않는다.
- 가려진 창은 `Windows.Graphics.Capture`로 바꾸지 않는다. 화면 DC를 자른다.
