import os
import re

TARGET_DIR = os.environ.get(
    "TUFF_RADICAL_DOC_TARGET_DIR",
    os.path.dirname(os.path.abspath(__file__)),
)

REPLACEMENTS = {
    r"Linuxカーネル": "TUFF-RADICAL-KERNEL (Pure Rust ベアメタル)",
    r"bzImage": "tuff-radical-kernel.efi (T-RAD Core)",
    r"Linuxベース": "Pure Rust ベアメタルベース",
    r"LinuxOS": "TUFF-RADICAL-KERNEL",
    r"ハイパーバイザ": "T-RAD Executive (非同期エグゼキュータ)",
    r"Linuxのドライバ": "TUFF-RADICAL独自のRustドライバ",
    r"Linux": "TUFF-RADICAL-KERNEL", # Be careful with this, might match too much. Let's stick to specific terms.
}

SPECIFIC_REPLACEMENTS = [
    (r"上位OS（Windows / Linux / macOS等）の下位レイヤーで稼働するセキュリティ基盤OSです。", 
     "UEFIから主権を奪取し、完全なPure Rustベアメタル環境で稼働するセキュリティ基盤OS（T-RAD Core）です。"),
    (r"OS自体はLinuxカーネル（bzImage）を基盤としている",
     "OS自体は一切のレガシーカーネル（Linux等）を排除し、独自の非同期エグゼキュータとハードウェア直結のZRAM圧縮プールを備えたPure Rustカーネルを基盤としている"),
]

ZRAM_PARAGRAPH_JP = """
### 🚀 徹底的なZRAM圧縮アーキテクチャ (T-RAD ZRAM)
TUFF-RADICALは、GoogleのSnappy/LZ4アルゴリズムの思想をベアメタルレベルで統合しています。
システムメモリ（T-RAD Heap）の大部分を「インメモリ圧縮プール」として確保し、
Unique Queue (UQ)のデータやファイルシステムのキャッシュを透過的かつ超高速に圧縮・伸張します。
これにより、極端にメモリが少ない環境でも、実質的なメモリ容量を数倍に拡張し、
Vulkan GPUオフロードやPQC（耐量子暗号）の並列処理をリソース枯渇なしに実行し続けます。
"""

ZRAM_PARAGRAPH_EN = """
### 🚀 Absolute ZRAM Compression Architecture (T-RAD ZRAM)
TUFF-RADICAL integrates the extreme memory-efficiency concepts inspired by Google's Snappy/LZ4 at the bare-metal level.
A massive portion of the system memory (T-RAD Heap) is designated as an "in-memory compression pool",
transparently and ultra-fastly compressing/decompressing Unique Queue (UQ) data and filesystem caches.
This effectively multiplies available RAM, allowing Vulkan GPU offloading and PQC parallel processing 
to run continuously without resource exhaustion, even on severely memory-constrained nodes.
"""

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    original_content = content

    for old, new in REPLACEMENTS.items():
        content = re.sub(old, new, content)

    for old, new in SPECIFIC_REPLACEMENTS:
        content = re.sub(old, new, content)
        
    # Inject ZRAM paragraph after "## 1. アーキテクチャ概要" or similar
    if "## 1. アーキテクチャ概要" in content and "T-RAD ZRAM" not in content:
        content = content.replace("## 1. アーキテクチャ概要", "## 1. アーキテクチャ概要\n" + ZRAM_PARAGRAPH_JP)
        
    if "## 1. Architecture Overview" in content and "T-RAD ZRAM" not in content:
        content = content.replace("## 1. Architecture Overview", "## 1. Architecture Overview\n" + ZRAM_PARAGRAPH_EN)

    # Inject into READMEs
    if os.path.basename(filepath) == "README_JP.md" and "T-RAD ZRAM" not in content:
        content = content.replace("## TUFF-RADICAL について", "## TUFF-RADICAL について\n" + ZRAM_PARAGRAPH_JP)
        
    if os.path.basename(filepath) == "README.md" and "T-RAD ZRAM" not in content:
        content = content.replace("## About TUFF-RADICAL", "## About TUFF-RADICAL\n" + ZRAM_PARAGRAPH_EN)

    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"Updated: {filepath}")

for root, _, files in os.walk(TARGET_DIR):
    for file in files:
        if file.endswith(".md"):
            process_file(os.path.join(root, file))

print("Documentation overhaul complete.")
