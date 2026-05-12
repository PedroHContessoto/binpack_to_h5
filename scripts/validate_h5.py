"""Audit an HDF5 file produced by binpack_to_h5.

Verifies data integrity:
- attrs (eval_scale, n_train, n_val, pipeline_version, etc.)
- sign agreement: |eval| and result agree on sign for decisive positions
  (target >99% with Stockfish-style filtering, ~97% without)
- eval distribution: range, mean, percentiles
- WDL distribution: W/D/L percentages, white skew
- STM balance: should be ~50% white
- piece_count distribution across 8 buckets (clamp((pc-2)//4, 0, 7))
- HalfP feature index range (must be 0-767)

Usage:
    python scripts/validate_h5.py <path/to/output.h5>
"""
import argparse
import sys

import h5py
import numpy as np


def main():
    p = argparse.ArgumentParser(description="Audit HDF5 produced by binpack_to_h5")
    p.add_argument("h5_path", help="Path to .h5 file")
    p.add_argument("--sample-size", type=int, default=200_000,
                   help="Number of positions to sample (default 200k)")
    p.add_argument("--regions", type=int, default=10,
                   help="Spread sampling across N contiguous regions (default 10)")
    args = p.parse_args()

    with h5py.File(args.h5_path, "r") as f:
        print("=" * 75)
        print(f"AUDIT: {args.h5_path}")
        print("=" * 75)

        # --- Attrs ---
        print("\n--- METADATA ---")
        for k, v in sorted(f.attrs.items()):
            print(f"  {k}: {v}")

        # --- Sample positions across N regions for distribution stats ---
        if "train" not in f:
            print("\nERROR: no 'train' group found")
            sys.exit(1)
        g = f["train"]
        total = g["evals"].shape[0]
        if total == 0:
            print("\nWARN: train group is empty (n_train=0); nothing to sample")
            return
        # For tiny datasets, fall back to reading everything contiguously.
        if total < args.regions * 10:
            print(f"\n--- Reading all {total:,} positions (too small for sampling) ---")
            evs = g["evals"][:]
            wdls = g["wdl"][:]
            stms = g["stm"][:]
            pcs = g["piece_count"][:]
        else:
            N = min(args.sample_size, total)
            print(f"\n--- Sampling {N:,} of {total:,} positions ({args.regions} regions) ---")
            per_region = max(1, N // args.regions)
            evs_l, wdls_l, stms_l, pcs_l = [], [], [], []
            for r in range(args.regions):
                start = (total // args.regions) * r
                end = min(start + per_region, total)
                if end <= start:
                    continue
                evs_l.append(g["evals"][start:end])
                wdls_l.append(g["wdl"][start:end])
                stms_l.append(g["stm"][start:end])
                pcs_l.append(g["piece_count"][start:end])
            evs = np.concatenate(evs_l)
            wdls = np.concatenate(wdls_l)
            stms = np.concatenate(stms_l)
            pcs = np.concatenate(pcs_l)

        if len(evs) == 0:
            print("\nWARN: no samples collected; aborting distribution stats")
            return

        # --- Sign agreement ---
        mask = (np.abs(wdls) > 0.5) & (np.abs(evs) > 1.0)
        if mask.sum() > 100:
            agree = np.sum(np.sign(evs[mask]) == np.sign(wdls[mask]))
            sign_pct = agree / mask.sum() * 100
            mark = "OK" if sign_pct >= 99.0 else "WARN" if sign_pct >= 95.0 else "BAD"
            print(f"\n--- SIGN AGREEMENT ---")
            print(f"  {agree:,}/{mask.sum():,} = {sign_pct:.2f}%  [{mark}]")
            print(f"  Target: >=99% (with Stockfish-style filtering)")
        else:
            print(f"\n--- SIGN AGREEMENT: insufficient decisive samples ({mask.sum()}) ---")

        # --- Eval distribution ---
        print(f"\n--- EVAL DISTRIBUTION ---")
        print(f"  range:      [{evs.min():.3f}, {evs.max():.3f}]")
        print(f"  mean:       {evs.mean():+.4f}  (positive = white-favored)")
        print(f"  median |.|: {np.median(np.abs(evs)):.4f}")
        print(f"  p10/p50/p90: {np.percentile(evs,10):.3f} / {np.percentile(evs,50):.3f} / {np.percentile(evs,90):.3f}")

        # --- WDL ---
        print(f"\n--- WDL DISTRIBUTION ---")
        w = (wdls > 0.5).sum() / len(wdls) * 100
        d = (np.abs(wdls) <= 0.5).sum() / len(wdls) * 100
        l = (wdls < -0.5).sum() / len(wdls) * 100
        print(f"  W={w:.1f}%  D={d:.1f}%  L={l:.1f}%  (skew W-L = {w - l:+.1f}pp)")

        # --- STM balance ---
        white_pct = (stms == 0).sum() / len(stms) * 100
        mark = "OK" if 45 <= white_pct <= 55 else "WARN"
        print(f"\n--- STM BALANCE ---")
        print(f"  White: {white_pct:.1f}%  (expected ~50%)  [{mark}]")

        # --- Piece count buckets ---
        print(f"\n--- PIECE COUNT DISTRIBUTION (8 buckets) ---")
        buckets = np.clip((pcs.astype(int) - 2) // 4, 0, 7)
        labels = [
            "pc 2-5    deep endgame",
            "pc 6-9    endgame",
            "pc 10-13  late middle",
            "pc 14-17  middlegame",
            "pc 18-21  middle core",
            "pc 22-25  middle core",
            "pc 26-29  transition",
            "pc 30+    opening full",
        ]
        for b in range(8):
            count = (buckets == b).sum()
            pct = count / len(pcs) * 100
            bar = "#" * int(pct * 0.8)
            print(f"  bucket {b}: {labels[b]:<25} {pct:5.2f}%  {bar}")

        # --- Feature index range ---
        print(f"\n--- HALFP FEATURE INDEX (1M sample) ---")
        sample = g["w_flat"][: min(1_000_000, g["w_flat"].shape[0])]
        print(f"  range:  [{sample.min()}, {sample.max()}]  (must be 0-767)")
        print(f"  unique: {len(np.unique(sample))}")

        print(f"\n=== AUDIT END ===")


if __name__ == "__main__":
    main()
