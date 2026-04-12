# HANDOFF

更新日: `2026-04-10`  
対象 repository: `/run/media/flux/THPDOC/Develop/TUFF-RADICAL`

## 2026-04-10 追記

### 今回追加で完了した主要タスク

0. distro bootstrap 後段を実際に完走
   - 実行先:
     - `projects/tuff-linux-distro/build/pipeline/build-vm-stack.sh`
     - `projects/tuff-linux-distro/tests/boot/qemu-smoke-test.sh`
     - `projects/tuff-linux-distro/build/release/write-release-manifest.sh`
   - 実行結果:
     - root shell で統合パイプラインを実行
     - `raw VM image` 作成に成功
     - BIOS QEMU smoke test 成功
     - UEFI QEMU smoke test 成功
     - release manifest 生成成功
   - 出力:
     - `projects/tuff-linux-distro/out/images/vm/tuff-vm-stable-amd64-minbase.raw`
     - `projects/tuff-linux-distro/out/tests/boot/tuff-vm-stable-amd64-minbase.raw.bios.serial.log`
     - `projects/tuff-linux-distro/out/tests/boot/tuff-vm-stable-amd64-minbase.raw.uefi.serial.log`
     - `projects/tuff-linux-distro/out/release/20260410T024606Z/`

0.5. install/live image 側の最初の skeleton を追加
   - 追加先:
     - `projects/tuff-linux-distro/build/live-build/configure-live-build.sh`
     - `projects/tuff-linux-distro/build/live-build/build-live-image.sh`
     - `projects/tuff-linux-distro/build/live-build/render-package-list.sh`
     - `projects/tuff-linux-distro/tests/install/qemu-live-smoke-test.sh`
   - 内容:
     - `tuff-base` と `tuff-recovery` manifest から live-build 用 package list を自動生成
     - `overlay/` を `config/includes.chroot/` へ同期する入口を追加
     - ISO 生成後に BIOS / UEFI で QEMU smoke test する雛形を追加
     - `CHECK_LIVE_BUILD=1 ./build/preflight/check-build-host.sh` で
       `lb` / `xorriso` を確認できるようにした
     - `live-build` workspace の既定を `/var/tmp/tuff-live-build-work` に変更
       - 外部ドライブ配下は遅すぎた
       - `/tmp` は `nodev` で debootstrap の device node 検証に不向きだった
   - 現状の制約:
     - `lb` と `xorriso` は導入済み
     - workspace 生成は成功
     - `sudo LB_WORK_DIR=/var/tmp/tuff-live-build-work ./build/live-build/build-live-image.sh`
       で actual build 開始までは確認
     - ただし ISO 生成完了までは今回まだ未確認

1. distro bootstrap 導線を実運用寄りに強化
   - 追加先:
     - `projects/tuff-linux-distro/build/pipeline/build-vm-stack.sh`
     - `projects/tuff-linux-distro/build/release/write-release-manifest.sh`
     - `projects/tuff-linux-distro/build/preflight/check-build-host.sh`
   - 内容:
     - `mmdebstrap -> unpack -> VM image -> BIOS/UEFI smoke test -> manifest` の統合入口を追加
     - build host の依存コマンドと root 権限を事前確認する preflight を追加
     - artifact checksum / manifest を出力する release metadata 生成を追加

2. distro bootstrap overlay と package manifest を実体化
   - 追加先:
     - `projects/tuff-linux-distro/overlay/etc/issue`
     - `projects/tuff-linux-distro/overlay/etc/motd`
     - `projects/tuff-linux-distro/overlay/etc/default/tuff-release`
     - `projects/tuff-linux-distro/overlay/etc/systemd/journald.conf.d/10-tuff-base.conf`
     - `projects/tuff-linux-distro/overlay/etc/sysctl.d/90-tuff-base.conf`
     - `projects/tuff-linux-distro/overlay/usr/local/sbin/tuff-collect-support`
     - `projects/tuff-linux-distro/packages/tuff-base/manifest.txt`
     - `projects/tuff-linux-distro/packages/tuff-perf/manifest.txt`
     - `projects/tuff-linux-distro/packages/tuff-recovery/manifest.txt`
   - 内容:
     - base image に最低限入れる TUFF 固有設定を追加
     - support bundle 採取スクリプトを追加
     - package layering を README だけでなく manifest にも固定開始

3. distro release / channel 文書を追加
   - 追加先:
     - `projects/tuff-linux-distro/docs/release/artifact-contract.md`
     - `projects/tuff-linux-distro/docs/release/channel-policy.md`
     - `projects/tuff-linux-distro/docs/architecture/build-host-prerequisites.md`
   - 内容:
     - 「まだ正式 distro でなくても、それ相応に扱う」ための artifact contract を明文化
     - `bootstrap / nightly / testing / stable` の channel 名を先に固定
     - Debian 13 系 build host 依存を明文化

4. rootless 段階の bootstrap 実行確認
   - 実行結果:
     - `projects/tuff-linux-distro/build/mmdebstrap/build-rootfs.sh` は成功
     - 出力:
       - `projects/tuff-linux-distro/out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar`
       - サイズは約 `397 MiB`
     - `projects/tuff-linux-distro/build/mmdebstrap/unpack-rootfs.sh` も成功
   - 対応:
     - rootless 展開時に `dev` ノードで落ちる問題があったため、
       `unpack-rootfs.sh` で非 root 時は `dev/proc/sys/run` を tar 展開対象から除外する修正を追加

5. build host の PATH 問題を修正
   - 問題:
     - `flux` ユーザ環境の `PATH` に `/usr/sbin` と `/sbin` が入っておらず、
       `parted` / `losetup` / `grub-install` / `mkfs.*` を preflight が missing と誤認していた
   - 対応:
     - build / test / release 系シェルスクリプト先頭で
       `PATH=/usr/local/sbin:/usr/sbin:/sbin:${PATH}` を export するよう修正

6. Debian 13 実機の日本語化と Chrome + Mozc 導線の修正
   - 修正先:
     - `/home/flux/.xinputrc`
     - `/home/flux/.config/environment.d/fcitx5.conf`
     - `/home/flux/.profile`
     - `/home/flux/.config/plasma-localerc`
     - `/home/flux/.local/share/applications/google-chrome.desktop`
   - 内容:
     - `im-config` の実質設定を `none` から `fcitx5` 側へ戻した
     - `XMODIFIERS=@im=fcitx`
     - `GTK_IM_MODULE=fcitx`
     - `QT_IM_MODULE=fcitx`
     - `SDL_IM_MODULE=fcitx`
     - `LANG=ja_JP.UTF-8`
     - `LANGUAGE=ja_JP:ja`
     - `LC_ALL` は `~/.profile` 内で `unset`
     - Chrome の desktop entry を Wayland IME 有効で起動するよう変更
       - `--ozone-platform=wayland --enable-wayland-ime`

### 現在の到達点

- distro 側は「文書 skeleton」段階から、
  `rootfs tarball -> unpack -> raw VM image -> BIOS/UEFI smoke test -> release manifest`
  まで一通り通る bootstrap workspace へ進んだ
- いま残っているのは「起動確認の次」の段階:
  - install/live image の実 build
  - recovery 実装
  - release metadata の署名
  - package 化と apt repository 側

### 今回実行して分かった注意点

1. `grub-install` は Debian 13 の最小 rootfs では自動では入らない
   - `grub-common` / `grub-pc-bin` / `grub-efi-amd64-bin` だけでは不足
   - `build/mmdebstrap/package-lists/tuff-base.txt` に `grub2-common` が必要

2. Debian 13 の OVMF パスは従来名と違う
   - 固定値の `OVMF_CODE.fd` / `OVMF_VARS.fd` 前提だと UEFI smoke test が失敗
   - 実機では `/usr/share/OVMF/OVMF_CODE_4M.fd` と `/usr/share/OVMF/OVMF_VARS_4M.fd` が存在
   - `tests/boot/qemu-smoke-test.sh` はこの系統を自動検出するよう修正済み

3. `qemu-smoke-test.sh` には初期不具合があった
   - `timeout` の終了コード取得が誤っていた
   - `rg -Eq` という無効な呼び方をしていた
   - 現在は修正済みで BIOS/UEFI とも通過確認済み

### 再起動後の最優先確認

1. 実機日本語化確認
   - `locale`
   - 期待:
     - `LANG=ja_JP.UTF-8`
     - `LANGUAGE=ja_JP:ja`

2. fcitx5 / mozc 確認
   - `fcitx5-remote -n`
   - 期待:
     - `mozc`

3. Chrome での日本語入力確認
   - Chrome を既存プロセスごと完全終了
   - メニューから再起動
   - Mozc で日本語入力できるか確認

### 再起動後の distro 側確認コマンド

root shell (`su -`) で以下を実行:

```bash
cd /media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro
./build/preflight/check-build-host.sh
./build/pipeline/build-vm-stack.sh
```

現在は以下が通る状態:

1. preflight pass
2. rootfs tarball 再利用または再生成
3. raw VM image 作成
4. BIOS QEMU smoke test
5. UEFI QEMU smoke test
6. release manifest 生成

補助確認:

```bash
ls -lah out/images/vm
ls -lah out/tests/boot
ls -lah out/release
```

### distro 側の次の具体作業

1. host に `live-build` と `xorriso` を入れる
2. `build/live-build/configure-live-build.sh` を実行して workspace を生成する
3. `sudo build/live-build/build-live-image.sh` を実行して ISO を生成する
4. BIOS / UEFI の install/live QEMU smoke test を実成果物で通す
5. `tuff-base` / `tuff-perf` / `tuff-recovery` を metapackage 化できる形へ寄せる
6. release metadata 署名方針を決める
7. recovery boot / recovery media の導線を設計する

### もし Chrome + Mozc が再起動後も駄目なら

確認コマンド:

```bash
locale
fcitx5-remote -n
xargs -0 -L1 < /proc/$(pgrep -n chrome)/environ | rg '^(GTK_IM_MODULE|QT_IM_MODULE|XMODIFIERS|WAYLAND_DISPLAY|LANG|LANGUAGE)'
```

これで Chrome プロセスに IM / locale が乗っているか確認する。

## 今回完了した主要タスク

1. `TUFF-RADICAL` として repository 名称を統一
   - 旧 `AXON` 方向の変更は破棄し、トップディレクトリを `TUFF-RADICAL` に統一済み。
   - カーネル crate 名、ログ識別子、スキル名、補助スクリプト名も `TUFF-RADICAL` 系に揃えた。

2. 公開用 GitHub repository を作成
   - public remote: `https://github.com/elementary-particles-Man/TUFF-RADICAL`
   - branch: `main`
   - 初回コミットは push 済み。

3. 公開前提の最低限の整理を実施
   - `README.md` を追加。
   - `.gitignore` を追加。
   - `target/`、QEMU ログ、`logs/` が追跡対象に入らないようにした。

4. bare-metal カーネルのビルド確認
   - `TUFF-RADICAL-KERNEL/` で `cargo build` を実行。
   - build は成功。
   - warning は残っているが、名称変更で壊れてはいない。

5. Linux distro 企画の初期ロードマップを追加
   - 追加先: `projects/tuff-linux-distro/ROADMAP.md`
   - 方針:
     - `TUFF-RADICAL` は Pure Rust bare-metal OS のまま維持
     - Linux distro は別 repository として切り出す
     - ただしローカル配置はこの repo 配下の下位層に集約する
     - `TUFF-Xwin` は future distro の optional package layer として扱う

6. Linux distro 側の初期 skeleton を追加
   - 追加先: `projects/tuff-linux-distro/`
   - 追加内容:
     - `README.md`
     - `build/mmdebstrap/build-rootfs.sh`
     - `build/mmdebstrap/package-lists/tuff-base.txt`
     - `docs/support-matrix/v1.md`
     - `docs/architecture/package-layering.md`
     - `docs/architecture/tuff-xwin-integration.md`
   - 目的:
     - 将来の separate repo の骨格を先に固定
     - `mmdebstrap` による最小 rootfs 作成の起点を用意
     - v1 support matrix と package layering を文章として凍結開始

7. 既存ノートPC向け Debian 13 + TUFF-Xwin-oriented host playbook を追加
   - 追加先:
     - `projects/tuff-linux-distro/docs/architecture/hp-15s-eq1xxx-debian13-tuff-xwin-playbook.md`
     - `projects/tuff-linux-distro/build/host-setup/postinstall-hp-15s-eq1xxx.sh`
   - 追加内容:
     - Debian 13.4 installer 方針
     - Xfce/X11 主系 + Openbox fallback 方針
     - Chrome Memory Saver 最大化の導線
     - `zram-tools` を 4 GiB / `lz4` に抑えた設定
     - persistent journald / SSH / sysrq / Wi-Fi powersave 無効化
   - 意図:
     - 今の不安定なノートに対して、TUFF-Xwin の思想へ寄せた実用ベースを先に持つ

8. 実機再導入前の USB install checklist を追加
   - 追加先:
     - `projects/tuff-linux-distro/docs/architecture/hp-15s-eq1xxx-usb-install-checklist.md`
   - 内容:
     - USB 作成前確認
     - install 後に必要な host-setup 退避手順
     - partitioning の触る場所 / 触らない場所
     - installer での task 選択
     - 初回 boot 後の実行順

## 今回確定した重要方針

- bare-metal OS と Linux distro は混ぜない。
- future distro repo のローカル配置は、この repository 配下の下位ディレクトリに置く。
  - 現時点の予定パス: `projects/tuff-linux-distro/`
- `TUFF-Xwin` は base image ではなく、後載せ可能な package / session stack として扱う。
- distro v1 は現実的に絞る。
  - `x86_64`
  - Intel / AMD
  - BIOS + UEFI
  - Debian Stable ベースを第一候補

## 現行ロードマップの要点

`projects/tuff-linux-distro/ROADMAP.md` に書いた内容の要点だけ抜くと次です。

- `TUFF-RADICAL-KERNEL` の low-level 実装は distro の基盤コードとしては再利用しない。
- 代わりに、性能思想・圧縮思想・最小主義だけを方針として引き継ぐ。
- distro 側でまず必要なのは:
  - boot/install
  - package repo
  - update path
  - recovery path
  - support matrix
  - release engineering
- TUFF-Xwin は distro 完成後ではなく、distro が public installable になった後の optional integration phase で載せる。

## まだ未着手のもの

- install ISO / live image の実 build 検証
- recovery 実装
- `tuff-base` / `tuff-perf` / `tuff-recovery` の実 package 化
- signed repository / signed artifact 導線
- TUFF-Xwin の package build / integration test 実装
- host setup script の実機適用結果の検証

## 次にやるべき具体作業

1. `live-build` / `xorriso` を host へ導入して ISO build を実行する
2. ISO 向け BIOS / UEFI smoke test を実成果物で通す
3. `tuff-base` / `tuff-perf` / `tuff-recovery` を実 package 化へ寄せる
4. release metadata の署名導線を追加する
5. recovery boot / recovery media 方針を文書化する
6. TUFF-Xwin package 群の build 導線を base image 外に切ったまま設計する
7. Debian 13 実機へ `build/host-setup/postinstall-hp-15s-eq1xxx.sh` を適用して挙動確認する

## 注意点

- 「どの PC でも動く」は v1 の要求としては広すぎる。最初は support matrix を切ること。
- 「世界初」「最強」は public docs では裏取りなしに使わないこと。
- bare-metal 側の AVX / Vulkan / custom compression を distro v1 の blocker にしないこと。

## 参照先

- distro 企画ロードマップ:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/ROADMAP.md`
- distro workspace README:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/README.md`
- distro support matrix:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/docs/support-matrix/v1.md`
- distro package layering:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/docs/architecture/package-layering.md`
- distro TUFF-Xwin integration boundary:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/docs/architecture/tuff-xwin-integration.md`
- host install playbook:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/docs/architecture/hp-15s-eq1xxx-debian13-tuff-xwin-playbook.md`
- host setup script:
  - `/run/media/flux/THPDOC/Develop/TUFF-RADICAL/projects/tuff-linux-distro/build/host-setup/postinstall-hp-15s-eq1xxx.sh`
- TUFF-Xwin repository:
  - `/run/media/flux/THPDOC/Develop/TUFF-Xwin`
- TUFF-Xwin の最初に読むべき資料:
  - `/run/media/flux/THPDOC/Develop/TUFF-Xwin/docs/README.md`
  - `/run/media/flux/THPDOC/Develop/TUFF-Xwin/docs/architecture.md`
  - `/run/media/flux/THPDOC/Develop/TUFF-Xwin/docs/desktop-profiles.md`

## 2026-04-13 追記 (実体化完了)

### 実行結果
- `projects/tuff-linux-distro/` 配下の全パイプラインを実体化し、完走を確認した。
- `mmdebstrap -> build-vm-stack -> qemu-smoke-test -> write-release-manifest` の一連の流れが正常に動作する。
- BIOS / UEFI 両方で RAW イメージからのブート（シリアルコンソール経由）に成功。
- `live-build` のワークスペース設定 (`configure-live-build.sh`) を完了。

### 生成済み成果物
- `out/mmdebstrap/tuff-rootfs-stable-amd64-minbase.tar` (RootFS)
- `out/images/vm/tuff-vm-stable-amd64-minbase.raw` (Bootable RAW Image)
- `out/release/20260412T181551Z/` (Release Manifest & Artifacts)
- `out/tests/boot/*.log` (Boot validation logs)

### 次のステップ
1. `sudo build/live-build/build-live-image.sh` を実行して ISO 生成を開始する（長時間タスク）。
2. ISO 用の QEMU スモークテストを追加する。
3. リリース・メタデータの署名プロセスの検討。
