# AgSnap

Windows x64 트레이 앱. 지정한 모디파이어를 누른 채 클릭하면 화면의 컨트롤이나 창을 PNG로 찍어 클립보드에 넣는다. 기본 조합은 `Ctrl+Shift+Alt`다.

- 좌클릭: 커서 아래 컨트롤
- 우클릭: 그 최상위 창
- 두 클릭 모두 대상 앱으로는 전달되지 않는다

클립보드에는 실제 PNG 파일(`CF_HDROP`), `PNG` 바이트, `CF_DIB`가 들어간다. 탐색기에서 붙여넣으면 파일이 생긴다. `FileGroupDescriptor`와 `DoDragDrop`은 쓰지 않는다.

동작은 SnapDrag 0.4.0과 같다. 설정 파일도 그 경로를 그대로 읽는다.

## 요구 사항

- Windows x64
- [Rust](https://rustup.rs/) 1.85 이상. 이 저장소는 1.99에서 확인했다

관리자로 실행하지 않는다. 매니페스트는 `asInvoker`이고, DPI는 Per-Monitor V2다.

## 빌드와 실행

```powershell
cargo run --bin snapdrag
cargo build --release --bin snapdrag
```

릴리스 바이너리는 `target\release\snapdrag.exe`다. 빌드가 `CHANGELOG.md`를 exe 옆으로 복사한다. 메인 창은 없고 트레이 아이콘만 생긴다. 툴팁은 `SnapDrag 0.4.0`이다.

## 조작

트레이에 적힌 조합을 누른 채 클릭한다.

| 입력 | 결과 |
| --- | --- |
| 좌클릭 | down만 삼킨다. 이동과 up은 통과한다. 커서 아래 가장 깊은 가시 컨트롤을 찍는다 |
| 우클릭 | down과 up을 삼킨다. 상황 메뉴는 열리지 않는다. 그림자 없는 창 프레임을 찍는다 |
| 더블클릭 | 시스템 더블클릭 시간과 거리 안에서 한 장만 찍고, 알림 없이 연결 프로그램으로 연다 |
| 한 번 클릭 | 더블클릭 시간이 지난 뒤 찍고 트레이 알림만 띄운다. 알림을 누르면 PNG가 열린다 |

좌클릭의 UI Automation 사각형은 창 안에 있고, 한 변이 8보다 크며, 창 면적의 85%(`17/20`) 미만일 때만 쓴다. 아니면 그 컨트롤의 창 사각형이다. 우클릭은 UI Automation으로 자르지 않는다. 소유 대화상자는 그 대화상자만 찍고 소유자 창까지 따라가지 않는다.

캡처 중 두 번째 down은 삼키지 않는다. 우클릭 up은 캡처 중에도 삼킨다. Alt down/up은 제스처에 Alt가 있고 `Ctrl` 또는 `Shift`도 같이 눌려 있을 때만 삼킨다.

| 메뉴 | 동작 |
| --- | --- |
| 단축키 표시 | 현재 조합 |
| 캡처 폴더 | `%TEMP%\shots`를 연다 |
| 보관 | 남길 일수. 기본 7일 |
| 단축키 | `Ctrl`, `Shift`, `Alt`. 하나도 없으면 저장하지 않는다 |
| 로그 열기 | 제스처 로그. 기록이 없으면 알림만 뜬다 |
| 버전 | exe 옆 `CHANGELOG.md`. `Esc`로 닫는다 |
| 언어 | 시스템, 한국어, English |
| 종료 | 훅을 풀고 프로세스를 끝낸다 |

## 파일

```text
%TEMP%\shots\yyyy-MM-dd\Screenshot-yyyyMMdd-HHmmss.png
%LOCALAPPDATA%\SnapDrag\modifiers.txt
%LOCALAPPDATA%\SnapDrag\retention.txt
%LOCALAPPDATA%\SnapDrag\culture.txt
%LOCALAPPDATA%\SnapDrag\gesture.log
```

같은 초에 다시 찍히면 `-2`부터 번호가 붙는다. 보관 일수가 지난 `Screenshot-*.png`만 실행 때와 1시간마다 지운다. 빈 날짜 폴더도 지운다. 클립보드에 넣은 직후에는 지우지 않는다. 좌표는 물리 픽셀이다.

전체 화면 독점 게임, DRM, UAC 보안 데스크톱은 검게 나올 수 있다. Chrome처럼 HWND가 하나인 본문은 접근성 트리가 없으면 창 전체가 찍힌다.

## 테스트

```powershell
cargo test
```

판정, 파일 이름, DIB, HDROP, WIC PNG를 고정한다. 마우스를 옮기는 전역 훅 테스트는 포함하지 않는다.

## 버전

`Cargo.toml`의 `version`과 `CHANGELOG.md` 첫 제목의 번호가 같다. 지금 버전은 0.4.0이다.
