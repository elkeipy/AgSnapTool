# TODO List

## 지침

- 지침 부분은 삭제하지마라.
- `## 계획`을 확인하고 `## 답변`에 답변만 작성해라. (구현은 하지 않는다.)
- `## 작업`을 확인하여 구현을 진행한다.
  - 완료되면 체크박스를 체크한다.
  - `## 답변` 영역에 발생 원인 및 간략한 구현 내용을 작성해라.
- CHANGELOG.md 와 README.md 파일을 늘 최신으로 유지해라
- `깃에 올려` 라고 하면 적당한 메시지를 추가해서 git commit/push 해라
- `배포해` 라고 하면 master 에 병합하고, 태그 추가하고 배포한다.

## 계획

## 작업

- [x] 스크린샷 저장시 기본 태그를 설정
  - [x] 윈도우 텍스트를 태그명으로 설정한다. 

```
2026-11-02 15:14:47.753 gesture hwnd=0x90CDC class=Qt5QWindowIcon text=정보창 rect=-1528,190,1504,849 point=-380,617 scope=control action=clipboard file=C:\Users\kplee\AppData\Local\AgSnapTool\shots\2026-11-02\Screenshot-20261102-151447.png
```

- 현재 로그 샘플인데 여기서 text 의 값으로 태그명을 추가한다.

## 답변

저장 직후 최상위 창 제목을 `notes.json` 태그로 넣는다. `text=정보창`이면 `#정보창`. 빈 제목은 건너뛴다. 영역 캡처는 원점 아래 창 제목을 쓴다.
