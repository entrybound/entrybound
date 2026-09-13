#!/usr/bin/env python3
"""Synthetic numeric arrays: smooth random fields and sensor time series (F07, generated).
g3-data generator "numeric A/B".

ebrc generator contract:
    python gen_numeric_ab.py --out <empty dir> --seed <int> --params <canonical JSON>
Pure function of (script bytes, seed, params) for a fixed numpy build (the venv pins
numpy via research/tools/setup_venv.sh; FFT round-off could differ in another numpy).

params:
  arrays: list of {"path": rel path, "kind": name, "shape": [..], "dtype": "float32"|"float64"|"int16"|"int32"|"uint16",
                   "container": "raw"|"npy", ...kind options}
kinds:
  grf           Gaussian random field by spectral synthesis, power spectrum ~ k^-beta
                (option "beta", default 3.0), scaled by "scale"/"offset"; integer dtypes
                are rounded (quantized) with clipping.  Optional "fill_fraction" replaces
                a smooth blob-shaped region by "fill_value" (land/sea-mask style).
  timeseries    multichannel sensor series: shape [channels, samples]; random walk drift +
                seasonal sines + white noise (options "noise", "drift"); integer dtypes emulate ADCs.
  timestamps    monotonically increasing float64/int64 sample times with jitter and gaps
  particles     float32 positions/velocities of clustered particles, shape [n, 6]
A "raw" container writes little-endian bytes with no header plus a sidecar .json describing
shape/dtype; "npy" writes NumPy .npy v1.0.
"""

import argparse
import json
import os

import numpy as np


def spectral_field(rng, shape, beta):
    white = rng.standard_normal(shape, dtype=np.float64)
    f = np.fft.rfftn(white)
    grids = np.meshgrid(*[np.fft.fftfreq(n) if i < len(shape) - 1 else np.fft.rfftfreq(n)
                          for i, n in enumerate(shape)], indexing="ij", sparse=True)
    k2 = sum(g.astype(np.float64) ** 2 for g in grids)
    k2.flat[0] = np.inf
    f *= k2 ** (-beta / 4.0)
    field = np.fft.irfftn(f, s=shape)
    field -= field.mean()
    std = field.std()
    return field / (std if std > 0 else 1.0)


def to_dtype(x, dtype):
    dt = np.dtype(dtype)
    if dt.kind in "iu":
        info = np.iinfo(dt)
        return np.clip(np.rint(x), info.min, info.max).astype(dt)
    return x.astype(dt)


def k_grf(rng, spec):
    shape = tuple(spec["shape"])
    x = spectral_field(rng, shape, float(spec.get("beta", 3.0)))
    x = x * float(spec.get("scale", 1.0)) + float(spec.get("offset", 0.0))
    fill = float(spec.get("fill_fraction", 0.0))
    arr = to_dtype(x, spec["dtype"])
    if fill > 0:
        mask_field = spectral_field(rng, shape, 4.0)
        thr = np.quantile(mask_field, fill)
        arr[mask_field < thr] = np.array(spec.get("fill_value", -9999), dtype=arr.dtype)
    return arr


def k_timeseries(rng, spec):
    ch, n = spec["shape"]
    t = np.arange(n, dtype=np.float64)
    out = np.empty((ch, n), dtype=np.float64)
    for c in range(ch):
        drift = np.cumsum(rng.standard_normal(n)) * float(spec.get("drift", 0.02))
        season = sum(rng.uniform(0.2, 2.0) * np.sin(2 * np.pi * t / rng.uniform(50, n / 2) + rng.uniform(0, 6.3))
                     for _ in range(3))
        noise = rng.standard_normal(n) * float(spec.get("noise", 0.1))
        out[c] = drift + season + noise
    out = out * float(spec.get("scale", 1.0)) + float(spec.get("offset", 0.0))
    return to_dtype(out, spec["dtype"])


def k_timestamps(rng, spec):
    (n,) = spec["shape"]
    step = float(spec.get("step", 0.001))
    d = np.full(n, step) + rng.normal(0, step * 0.01, n)
    gaps = rng.random(n) < 1e-4
    d[gaps] += rng.exponential(step * 5000, gaps.sum())
    ts = float(spec.get("t0", 1.7e9)) + np.cumsum(d)
    return ts.astype(np.float64) if np.dtype(spec["dtype"]).kind == "f" else (ts * 1e9).astype(np.int64)


def k_particles(rng, spec):
    n = spec["shape"][0]
    centers = rng.uniform(0, 1000, (32, 3))
    idx = rng.integers(0, 32, n)
    pos = centers[idx] + rng.standard_normal((n, 3)) * rng.uniform(1, 30, 32)[idx, None]
    vel = rng.standard_normal((n, 3)) * 50
    order = np.lexsort((pos[:, 2], pos[:, 1], (pos[:, 0] // 50)))
    return np.hstack([pos, vel])[order].astype(np.dtype(spec.get("dtype", "float32")))


KINDS = {"grf": k_grf, "timeseries": k_timeseries, "timestamps": k_timestamps, "particles": k_particles}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    ss = np.random.SeedSequence(args.seed)
    children = ss.spawn(len(p["arrays"]))
    for spec, child in zip(p["arrays"], children):
        rng = np.random.Generator(np.random.PCG64(child))
        arr = KINDS[spec["kind"]](rng, spec)
        arr = arr.astype(arr.dtype.newbyteorder("<"), copy=False)
        path = os.path.join(args.out, spec["path"])
        os.makedirs(os.path.dirname(path) or args.out, exist_ok=True)
        if spec.get("container", "raw") == "npy":
            np.save(path, arr, allow_pickle=False)
        else:
            with open(path, "wb") as f:
                f.write(np.ascontiguousarray(arr).tobytes())
            meta = {"shape": list(arr.shape), "dtype": arr.dtype.str, "order": "C", "kind": spec["kind"],
                    "generator": "research/corpus/generators/g3-data/gen_numeric_ab.py"}
            with open(path + ".json", "w", encoding="utf-8", newline="\n") as f:
                json.dump(meta, f, sort_keys=True, indent=1)
                f.write("\n")


if __name__ == "__main__":
    main()
