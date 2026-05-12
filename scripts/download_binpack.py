"""Download a Stockfish binpack from HuggingFace.

Default: farseerT76.binpack from official-stockfish/master-binpacks.
See https://huggingface.co/datasets/official-stockfish/master-binpacks for
other available binpacks (farseerT74, farseerT75, training_data_pylon, etc).

Usage:
    # Default: farseerT76 to ./binpacks/
    python scripts/download_binpack.py

    # Custom file + dest
    python scripts/download_binpack.py --filename farseerT74.binpack --dest /data/binpacks
"""
import argparse
import os
import sys
import time

try:
    from huggingface_hub import hf_hub_download
except ImportError:
    print("ERROR: huggingface_hub not installed. Run: pip install huggingface_hub")
    sys.exit(1)


def main():
    p = argparse.ArgumentParser(description="Download Stockfish binpack from HuggingFace")
    p.add_argument("--filename", default="farseerT76.binpack",
                   help="Binpack filename (default: farseerT76.binpack)")
    p.add_argument("--repo", default="official-stockfish/master-binpacks",
                   help="HuggingFace dataset repo (default: official-stockfish/master-binpacks)")
    p.add_argument("--dest", default="./binpacks",
                   help="Destination directory (default: ./binpacks)")
    args = p.parse_args()

    os.makedirs(args.dest, exist_ok=True)
    target = os.path.join(args.dest, args.filename)

    if os.path.exists(target):
        sz = os.path.getsize(target) / 1e9
        print(f"OK: {target} already exists ({sz:.2f} GB), skipping")
        return

    print(f"Downloading {args.filename} from {args.repo} ...")
    print(f"Dest: {args.dest}")
    t0 = time.time()
    try:
        out = hf_hub_download(
            repo_id=args.repo,
            filename=args.filename,
            repo_type="dataset",
            local_dir=args.dest,
        )
        elapsed = time.time() - t0
        sz = os.path.getsize(out) / 1e9
        speed_mbps = sz * 1000 / elapsed if elapsed > 0 else 0
        print()
        print(f"OK:    {out}")
        print(f"Size:  {sz:.2f} GB")
        print(f"Time:  {elapsed/60:.1f} min")
        print(f"Speed: {speed_mbps:.0f} MB/s")
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
