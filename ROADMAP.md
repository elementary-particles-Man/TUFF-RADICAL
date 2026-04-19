# TUFF-RADICAL Roadmap

更新日: `2026-04-19`

## 前提

- `TUFF-RADICAL` は今後も `Pure Rust bare-metal OS` の系統として維持する
- カーネル自体は `CUI` 止まりを原則とする
- GUI / display session / desktop profile の責務は sibling repository `TUFF-Xwin` に寄せる
- `AVX/SIMD` と `Vulkan` は「名前だけ有効」ではなく、`OS が状態管理できること` と `実デバイス/実ドライバ前提の基盤` までを到達条件にする
- 圧縮の既定は `Google 系` を優先し、まずは `Snappy default` を前提に進める
  - `LZ4` は互換・比較用
  - `Brotli` は高圧縮だがホットパス既定にはしない

## 0. 境界の固定

- [x] `README.md` と今後の文書で「kernel は CUI、GUI は TUFF-Xwin」の境界を明記し続ける
- [x] `TUFF-RADICAL` 側では login manager / desktop shell を抱え込まない
- [x] `TUFF-Xwin` 連携は「将来の Linux distro / host session 側の受け口」として設計する

## 1. ベースラインのリファイン

- [x] `main.rs` 集中の初期化を `boot`, `arch/x86_64`, `mm`, `drivers`, `sched` に分割する
- [x] `unsafe` 境界を整理する
  - [x] `CR3/MSR` (registers.rs)
  - [x] `PCI config I/O` (pci.rs / io.rs)
  - [x] `MMIO` (paging.rs / gpu.rs)
  - [x] `framebuffer write` (gpu.rs)
- [x] `serial` ログを subsystem / event / error code 付きへ寄せる
- [x] `cargo build` warning をゼロにする

## 2. ブートとメモリ管理

- [x] `UEFI ExitBootServices` を正式導入する
- [x] UEFI memory map の保持形式を固定する
- [x] 単純な「最大領域から切るだけ」の PMM を卒業する
  - [x] frame allocator
  - [x] reserved region 管理
  - [x] DMA 向け低位物理メモリ確保
- [x] heap 初期化を「最大 1 領域依存」から外す (allocator.rs)
- [x] panic / page fault 時の診断情報を増やす

## 3. ページングと保護

- [x] 4GB 全域 identity map をやめる
- [x] `text=RX`, `rodata=R`, `data/bss=RW`, `heap=RW/NX`, `MMIO=UC/NX` を分離する
- [x] `NX` を実際に policy として使う
- [ ] 将来の user space 導入を見据え、kernel 空間と user 空間の分離準備を始める

## 4. 割り込み・タイマ・実行器

- [x] PIC 前提から `APIC/x2APIC` を視野に入れた構造へ移行する
- [x] timer を `tick 1本` から deadline / sleep queue へ拡張する
- [x] 現行 executor の「キュー満杯時に黙って捨てる」挙動をやめる
- [x] `task` / `timer` / `interrupt wakeup` の責務を分離する
- [x] watchdog 用の heartbeat と kernel 内 telemetry を導入する

## 5. CPU 機能検出と SIMD/AVX

- [x] `CPUID` だけでなく `OSXSAVE` / `XGETBV` を確認する
- [x] `CR4.OSXSAVE` を適切に設定する
- [x] `XSAVE/XSAVEOPT` ベースの SIMD 状態保存領域を定義する
- [x] 例外・割り込み・タスク切替で `x87/SSE/AVX` 状態を壊さない (Executor Guard)
- [x] `AVX`, `AVX2`, `AVX-512` の runtime dispatch を導入する
- [ ] SIMD self-test を追加する
  - [ ] register corruption test
  - [ ] preemption / interrupt 跨ぎ test
  - [ ] scalar fallback 差分検証

## 6. 圧縮基盤

- [x] `zram.rs` を backend 抽象化する (Compressor trait)
- [x] 既定 backend を `Snappy` にする
- [x] `LZ4` backend を比較用に残す
- [ ] page-sized buffer と larger slab の両方で benchmark を持つ
- [ ] 圧縮率だけでなく、`latency` / `cycles` / `allocation pressure` を測る
- [ ] 将来の page cache / swap-like 圧縮へつなげる API を設計する

## 7. PCIe / ストレージ / デバイス基盤

- [x] PCIe 列挙をまともに作る
  - [x] class/subclass/progif 解釈
  - [x] BAR probe (Size detection)
  - [x] capability list
- [x] `VirtioBlk` を「ログだけ」から脱却させる (BlockDevice implementation)
- [x] DMA buffer と bounce buffer の基盤を作る (allocate_dma_pages)
- [ ] device discovery / driver bind / error path を分離する

## 8. GPU 基盤

- [x] 対象 GPU を最初は絞る (VirtIO-GPU)
- [x] framebuffer 直書き PoC を driver 構造へ置き換える (GpuDriver)
- [x] command submission, fence, completion, reset path を持つ (GpuCommandRing)
- [ ] GPU 用メモリ管理を入れる
  - [ ] staging buffer
  - [ ] coherent / non-coherent

## 9. Vulkan 対応

- [x] 第1段階: `compute queue 相当`, `buffer bind`, `fence`, `dispatch` の kernel substrate を作る
- [ ] 第2段階: `SPIR-V loader` と shader artifact 受け口を作る
- [ ] 第3段階: 将来の user-space runtime から叩ける syscall / IPC 境界を設計する

## 優先順 (更新後)

1. システム保護の完遂 (User-space separation)
2. SPIR-V loader 基盤
3. Driver bind architecture
4. TUFF-Xwin telemetry integration
