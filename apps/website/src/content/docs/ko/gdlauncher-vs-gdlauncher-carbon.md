---
title: "GDLauncher vs GDLauncher Carbon"
---

# GDLauncher vs GDLauncher Carbon

GDLauncher Carbon은 새로운 디자인과 기능을 갖추어 처음부터 다시 만든 GDLauncher의 새 버전입니다.

## 왜 새 버전을 만들고 있나요?

새 버전을 만드는 이유는 여러 가지가 있습니다. 대부분의 내용은 [여기](/blog/curseforge-partnership-announcement)에서 확인할 수 있습니다.

## 무엇이 새로워졌나요?

GDLauncher Carbon은 더 효율적이고 안정적인 기술 스택으로 처음부터 다시 작성되었습니다. 프런트엔드는 Electron과 SolidJS를 사용하고, 무거운 처리는 Rust로 작성된 별도 바이너리(Core Module)가 담당합니다.

리라이트와 함께 UI/UX도 전면 재설계하여, 기존의 분위기를 유지하면서 더 사용자 친화적이고 모던하게 만들었습니다.

이미 구버전과의 기능 동등성을 달성하고 그 이상을 제공하고 있으며, 현재는 사용 경험을 다듬고 새로운 기능을 추가하는 데 집중하고 있습니다.

새로운 기능 일부:

- **새로운 Java 매니저**: GDLauncher가 Java를 자동으로 관리하도록 하거나, 고급 Java 관리 기능으로 직접 버전을 관리할 수 있습니다.
- **매끄러운 모드 및 모드 로더 설치**: 어떤 Minecraft 버전이든, Forge, Fabric, Quilt, Neoforge 같은 모드 로더 설치가 그 어느 때보다 쉬워졌습니다.
- **광범위한 애드온 및 모드팩 지원**: CurseForge와 Modrinth에서 애드온과 모드팩을 직접 설치할 수 있습니다.
- **모드팩 업데이터**: 언제든 모드팩 버전을 변경하고 적용된 변경 사항의 정확한 변경 로그를 생성할 수 있습니다.
- **인스턴스 가져오기/내보내기**: (작업 중) GDLauncher Carbon과 CurseForge, MultiMC, ATLauncher, Technic, Prism, Modrinth, FTB 등 다른 런처 간에 게임 인스턴스를 옮길 수 있도록 작업하고 있습니다.

## 구버전은 어떻게 되나요?

구버전은 계속 다운로드할 수 있지만 더 이상 유지 관리되지 않습니다.

최근 보고된 심각한 버그가 매우 많아, 가능한 한 빨리 GDLauncher Carbon으로 전환할 것을 권장합니다.
