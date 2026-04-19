# TUFF-RADICAL Roadmap

更新日: `2026-04-19`

## 前提

- `TUFF-RADICAL` は今後も `Pure Rust bare-metal OS` の系統として維持する
- カーネル自体は `CUI` 止まりを原則とする
- `Ring 0 / Ring 3` 分離を完了し、真のマイクロカーネル基盤へ移行
- `AVX/SIMD`, `Vulkan`, `APIC/MSI-X` をハードウェアレベルで完全管理

## 0. 特権分離とユーザー空間 (未踏領域)

- [x] GDT / TSS 拡張 (User segments, Ring 0 Stack)
- [x] SYSCALL / SYSRET 高速通信路の確立
- [x] Page Table Hierarchy における `USER_ACCESSIBLE` ビットの伝播
- [x] Ring 3 への `IRETQ` コンテキストスイッチ実装
- [x] ユーザー空間プログラムの試験的ロードと実行

## 1. 割り込みの現代化 (APIC/MSI-X)

- [x] 8259 legacy PIC の完全無効化
- [x] I/O APIC Discovery と Redirection Table 操作
- [x] IRQ 1 (Keyboard) の I/O APIC 経由でのルーティング
- [x] PCIe MSI-X Capability の検出とパース

## 2. ベースラインのリファイン (完了)

- [x] モジュール分割 (arch, drivers, mm, compression)
- [x] `unsafe` 境界の整理 (registers.rs, io.rs)
- [x] `cargo build` warning ゼロ化

## 3. ページングと保護 (完了)

- [x] identity map 脱却とセクション分離 (RX/RW/NX)
- [x] Huge-page baseline による安定化

## 4. CPU 機能と SIMD (完了)

- [x] XSAVE / XRSTOR による SIMD 状態の保存・復元
- [x] Executor 内での per-task SIMD context guard

## 5. 次なる未踏領域

- [ ] ユーザー空間メモリ管理 (Separate Address Spaces)
- [ ] SPIR-V shader loader と Vulkan command submission
- [ ] MSI-X 割り込みの実際のドライバ側受信
- [ ] マルチプロセッサ (SMP) 起動とスケジューリング

## 優先順 (更新後)

1. マルチプロセッサ (SMP) 基盤
2. User-space memory separation (VMM)
3. SPIR-V / Vulkan runtime
