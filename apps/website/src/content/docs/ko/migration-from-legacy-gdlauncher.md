---
title: "구 GDLauncher에서 마이그레이션"
---

# 구 GDLauncher에서 마이그레이션

아직 GDLauncher Legacy를 사용하고 있다면, GDLauncher Carbon으로의 마이그레이션은 간단합니다. 진행 방법은 다음과 같습니다.

## GDLauncher Carbon 다운로드

GDLauncher Carbon은 [공식 웹사이트](https://gdlauncher.com)에서 다운로드할 수 있습니다.

## GDLauncher Carbon 설치 / 교체

새 버전을 다운로드한 후 운영 체제에 맞는 설치 안내를 [여기](/docs/installation)에서 확인하여 설치하세요.
GDLauncher Carbon을 설치하면 두 버전이 동일한 앱 ID를 공유하기 때문에 구버전이 교체됩니다. 구 인스턴스는 다른 폴더에 저장되어 있으므로 손실되지 않습니다.

## 구 인스턴스 자동 가져오기

GDLauncher Carbon을 실행하고 로그인하면, 구 인스턴스를 가져올지 묻는 온보딩 모달이 표시됩니다.

## 구 인스턴스 수동 가져오기

어떤 이유로든 자동 가져오기가 작동하지 않는 경우, 먼저 [여기](https://discord.gdlauncher.com)에서 문제를 보고해 주세요.
다음 단계로 구 인스턴스를 수동으로 가져올 수 있습니다.

- GDLauncher Carbon에서 동일한 구성으로 새 인스턴스를 생성합니다. 모드팩이라면 정확히 같은 버전의 모드팩을 설치하고, 바닐라 인스턴스라면 동일한 Minecraft 버전과 모드 로더를 설치하세요.
- 구 GDLauncher 데이터 경로를 엽니다.
- 새 Carbon 인스턴스를 오른쪽 클릭 → **More Options** → **Open Folder**로 열고 하위 폴더 `instance/`로 이동하세요. 구 게임 디렉터리의 내용을 여기에 복사하여 월드는 `instance/saves/`, 모드는 `instance/mods/`에 들어가게 하세요. 데이터 경로는 [문제 해결](/docs/troubleshooting)을 참고하세요.

## 구 인스턴스 삭제 (선택)

가져오기가 끝난 뒤에는 필요에 따라 구 인스턴스를 삭제할 수 있습니다. 자동으로 삭제되지는 않습니다.

원본을 삭제하기 전에 새 Carbon 인스턴스를 실행하여 월드와 모드가 모두 있고 정상적으로 작동하는지 확인하세요.
