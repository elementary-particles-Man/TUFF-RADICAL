# TUFF-RADICAL Roadmap

更新日: `2026-04-13`

## 前提

- `TUFF-RADICAL` は今後も `Pure Rust bare-metal OS` の系統として維持する
- カーネル自体は `CUI` 止まりを原則とする
- GUI / display session / desktop profile の責務は sibling repository `TUFF-Xwin` に寄せる
- `AVX/SIMD` と `Vulkan` は「名前だけ有効」ではなく、`OS が状態管理できること` と `実デバイス/実ドライバ前提の基盤` までを到達条件にする
- 圧縮の既定は `Google 系` を優先し、まずは `Snappy default` を前提に進める
  - `LZ4` は互換・比較用
  - `Brotli` は高圧縮だがホットパス既定にはしない

## 0. 境界の固定

- [ ] `README.md` と今後の文書で「kernel は CUI、GUI は TUFF-Xwin」の境界を明記し続ける
- [ ] `TUFF-RADICAL` 側では login manager / desktop shell を抱え込まない
- [ ] `TUFF-Xwin` 連携は「将来の Linux distro / host session 側の受け口」として設計する

## 1. ベースラインのリファイン

- [ ] `main.rs` 集中の初期化を `boot`, `arch/x86_64`, `mm`, `drivers`, `sched` に分割する
- [ ] `unsafe` 境界を整理する
  - [ ] `CR3/MSR`
  - [ ] `PCI config I/O`
  - [ ] `MMIO`
  - [ ] `framebuffer write`
- [ ] `serial` ログを subsystem / event / error code 付きへ寄せる
- [ ] `cargo build` warning をゼロへ近づける

## 2. ブートとメモリ管理

- [ ] `UEFI ExitBootServices` を正式導入する
- [ ] UEFI memory map の保持形式を固定する
- [ ] 単純な「最大領域から切るだけ」の PMM を卒業する
  - [ ] frame allocator
  - [ ] reserved region 管理
  - [ ] DMA 向け低位物理メモリ確保
- [ ] heap 初期化を「最大 1 領域依存」から外す
- [ ] panic / page fault 時の診断情報を増やす

## 3. ページングと保護

- [ ] 4GB 全域 identity map をやめる
- [ ] `text=RX`, `rodata=R`, `data/bss=RW`, `heap=RW/NX`, `MMIO=UC/NX` を分離する
- [ ] `NX` を実際に policy として使う
- [ ] 将来の user space 導入を見据え、kernel 空間と user 空間の分離準備を始める

## 4. 割り込み・タイマ・実行器

- [ ] PIC 前提から `APIC/x2APIC` を視野に入れた構造へ移行する
- [ ] timer を `tick 1本` から deadline / sleep queue へ拡張する
- [ ] 現行 executor の「キュー満杯時に黙って捨てる」挙動をやめる
- [ ] `task` / `timer` / `interrupt wakeup` の責務を分離する
- [ ] watchdog 用の heartbeat と kernel 内 telemetry を導入する

## 5. CPU 機能検出と SIMD/AVX

- [ ] `CPUID` だけでなく `OSXSAVE` / `XGETBV` を確認する
- [ ] `CR4.OSXSAVE` を適切に設定する
- [ ] `XSAVE/XSAVEOPT` ベースの SIMD 状態保存領域を定義する
- [ ] 例外・割り込み・タスク切替で `x87/SSE/AVX` 状態を壊さない
- [ ] `AVX`, `AVX2`, `AVX-512` の runtime dispatch を導入する
- [ ] `AVX-512` は実験扱いで gate する
- [ ] SIMD self-test を追加する
  - [ ] register corruption test
  - [ ] preemption / interrupt 跨ぎ test
  - [ ] scalar fallback 差分検証

## 6. 圧縮基盤

- [ ] `zram.rs` を backend 抽象化する
- [ ] 既定 backend を `Snappy` にする
- [ ] `LZ4` backend を比較用に残す
- [ ] page-sized buffer と larger slab の両方で benchmark を持つ
- [ ] 圧縮率だけでなく、`latency` / `cycles` / `allocation pressure` を測る
- [ ] 将来の page cache / swap-like 圧縮へつなげる API を設計する

## 7. PCIe / ストレージ / デバイス基盤

- [ ] PCIe 列挙をまともに作る
  - [ ] class/subclass/progif 解釈
  - [ ] BAR probe
  - [ ] capability list
  - [ ] MSI/MSI-X
- [ ] `VirtioBlk` を「ログだけ」から脱却させる
- [ ] DMA buffer と bounce buffer の基盤を作る
- [ ] device discovery / driver bind / error path を分離する

## 8. GPU 基盤

- [ ] 対象 GPU を最初は絞る
  - [ ] 開発初期は `virtio-gpu` など QEMU で再現しやすいものを優先
  - [ ] 実 GPU 同時多対応は後回し
- [ ] framebuffer 直書き PoC を driver 構造へ置き換える
- [ ] command submission, fence, completion, reset path を持つ
- [ ] GPU 用メモリ管理を入れる
  - [ ] staging buffer
  - [ ] coherent / non-coherent
  - [ ] MMIO / doorbell の厳密化

## 9. Vulkan 対応

- [ ] `Vulkan対応` の定義を「Khronos API 全面実装」ではなく段階化する
- [ ] 第1段階:
  `compute queue 相当`, `buffer bind`, `fence`, `dispatch` の kernel substrate を作る
- [ ] 第2段階:
  `SPIR-V loader` と shader artifact 受け口を作る
- [ ] 第3段階:
  将来の user-space runtime から叩ける syscall / IPC 境界を設計する
- [ ] 第4段階:
  実 GPU 上での queue submit / completion / recovery を通す
- [ ] `Vulkan` は kernel の中で完結させず、最終的には user-space runtime へ責務を渡す

## 10. テストと検証

- [ ] `cargo build` 以外の検証導線を作る
- [ ] QEMU boot smoke test を追加する
- [ ] page fault / GPF / double fault 回帰テストを作る
- [ ] SIMD/AVX self-test を自動化する
- [ ] 圧縮 round-trip / corruption / stress test を入れる
- [ ] GPU submit smoke test を追加する

## 11. TUFF-Xwin 連携

- [ ] `TUFF-RADICAL` 本体へ GUI を入れない
- [ ] GUI login / profile chooser / session recovery は `TUFF-Xwin` 側で持つ
- [ ] 将来の `TUFF Linux Distro` では次の構成を基本線にする
  - [ ] boot/login/kernel: distro + TUFF-RADICAL 方針
  - [ ] GUI session: TUFF-Xwin
  - [ ] optional desktop profile: `Wayland native` or `LeyerX11`
- [ ] `TUFF-Xwin` と共有すべきものを整理する
  - [ ] GPU capability contract
  - [ ] compression / asset cache policy
  - [ ] crash / recovery telemetry contract

## 優先順

1. ベースライン整理
2. メモリ管理 / ページング
3. 割り込み / executor
4. SIMD/AVX の OS 対応
5. Snappy default 圧縮基盤
6. PCIe / VirtIO
7. GPU driver substrate
8. Vulkan user-space handoff
9. TUFF-Xwin と distro 側 integration hardening
