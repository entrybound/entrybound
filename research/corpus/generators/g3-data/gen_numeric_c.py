#!/usr/bin/env python3
"""Synthetic simulation outputs from time-stepped models, written as NetCDF-3 and raw
arrays (F07, generated).  g3-data generator "numeric C" -- a different model family and
container from gen_numeric_ab.py (spectral random fields), used for held-out generated items.

ebrc generator contract:
    python gen_numeric_c.py --out <empty dir> --seed <int> --params <canonical JSON>
Pure function of (script bytes, seed, params) for the venv's pinned numpy/scipy builds.

params:
  runs: list of {"name": dir name, "model": name, ...model options}
models:
  gray-scott   2-D reaction-diffusion (options: "n" grid size, "steps", "every" snapshot
               interval, "F", "k").  Writes <name>/gray_scott.nc (NetCDF-3 64-bit offset,
               float32 variables u, v with dims time,y,x) plus a float64 time coordinate.
  advection    2-D advection-diffusion of a tracer on a periodic domain with a rotating
               velocity field (options "n", "steps", "every").  Writes <name>/tracer.f64
               (raw little-endian float64 [time,y,x]) and <name>/tracer.json.
  lorenz96     Lorenz-96 ensemble (options "k" variables, "members", "steps", "dt").
               Writes <name>/lorenz96.nc with float64 state [time, member, k].
  ar-audio     autoregressive "acoustic" channels quantized to int16 PCM plus int32
               sample counters (options "channels", "seconds", "rate").  Writes
               <name>/pcm_int16.raw, <name>/counters_int32.raw and a JSON sidecar.
"""

import argparse
import json
import os

import numpy as np
from scipy.io import netcdf_file


def lap(a):
    return (np.roll(a, 1, 0) + np.roll(a, -1, 0) + np.roll(a, 1, 1) + np.roll(a, -1, 1) - 4 * a)


def m_gray_scott(rng, d, spec):
    n, steps, every = int(spec.get("n", 256)), int(spec.get("steps", 4000)), int(spec.get("every", 100))
    F, k = float(spec.get("F", 0.037)), float(spec.get("k", 0.06))
    u = np.ones((n, n), dtype=np.float64)
    v = np.zeros((n, n), dtype=np.float64)
    for _ in range(int(spec.get("seeds", 12))):
        cy, cx, r = rng.integers(0, n), rng.integers(0, n), rng.integers(3, max(4, n // 20))
        u[max(0, cy - r):cy + r, max(0, cx - r):cx + r] = 0.5
        v[max(0, cy - r):cy + r, max(0, cx - r):cx + r] = 0.25
    u += rng.normal(0, 0.01, (n, n))
    snaps = steps // every
    f = netcdf_file(os.path.join(d, "gray_scott.nc"), "w", version=2)
    f.title = b"ebrc generated Gray-Scott reaction-diffusion"
    f.createDimension("time", snaps)
    f.createDimension("y", n)
    f.createDimension("x", n)
    tv = f.createVariable("time", "d", ("time",))
    tv.units = b"model steps"
    uv = f.createVariable("u", "f", ("time", "y", "x"))
    vv = f.createVariable("v", "f", ("time", "y", "x"))
    uv.long_name = b"activator concentration"
    vv.long_name = b"inhibitor concentration"
    s = 0
    for step in range(steps):
        uvv = u * v * v
        u += 0.16 * lap(u) - uvv + F * (1 - u)
        v += 0.08 * lap(v) + uvv - (F + k) * v
        if (step + 1) % every == 0 and s < snaps:
            tv[s] = step + 1
            uv[s] = u.astype(np.float32)
            vv[s] = v.astype(np.float32)
            s += 1
    f.close()


def m_advection(rng, d, spec):
    n, steps, every = int(spec.get("n", 256)), int(spec.get("steps", 2000)), int(spec.get("every", 50))
    y, x = np.meshgrid(np.linspace(0, 2 * np.pi, n, endpoint=False), np.linspace(0, 2 * np.pi, n, endpoint=False), indexing="ij")
    c = np.zeros((n, n))
    for _ in range(6):
        cy, cx, w = rng.uniform(0, 2 * np.pi), rng.uniform(0, 2 * np.pi), rng.uniform(0.1, 0.5)
        c += np.exp(-(((y - cy + np.pi) % (2 * np.pi) - np.pi) ** 2 + ((x - cx + np.pi) % (2 * np.pi) - np.pi) ** 2) / w ** 2)
    uvel = np.sin(y) * np.cos(x)
    vvel = -np.cos(y) * np.sin(x)
    dt, dx, kappa = 0.2, 2 * np.pi / n, 1e-4
    snaps = []
    for step in range(steps):
        # first-order upwind advection + diffusion
        cxm, cxp = np.roll(c, 1, 1), np.roll(c, -1, 1)
        cym, cyp = np.roll(c, 1, 0), np.roll(c, -1, 0)
        adv_x = np.where(uvel > 0, c - cxm, cxp - c) * uvel / dx
        adv_y = np.where(vvel > 0, c - cym, cyp - c) * vvel / dx
        c = c - dt * dx * (adv_x + adv_y) * 0.5 + kappa * dt / dx ** 2 * lap(c) * 1e-3
        if (step + 1) % every == 0:
            snaps.append(c.copy())
    arr = np.ascontiguousarray(np.stack(snaps).astype("<f8"))
    with open(os.path.join(d, "tracer.f64"), "wb") as fh:
        fh.write(arr.tobytes())
    with open(os.path.join(d, "tracer.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump({"shape": list(arr.shape), "dtype": "<f8", "dims": ["time", "y", "x"], "model": "advection-diffusion"}, fh, sort_keys=True)
        fh.write("\n")


def m_lorenz96(rng, d, spec):
    k, members, steps, dt = int(spec.get("k", 40)), int(spec.get("members", 32)), int(spec.get("steps", 20000)), float(spec.get("dt", 0.01))
    every = int(spec.get("every", 5))
    forcing = 8.0
    x = forcing + rng.normal(0, 1.0, (members, k))

    def rhs(s):
        return (np.roll(s, -1, 1) - np.roll(s, 2, 1)) * np.roll(s, 1, 1) - s + forcing

    snaps = steps // every
    f = netcdf_file(os.path.join(d, "lorenz96.nc"), "w", version=2)
    f.title = b"ebrc generated Lorenz-96 ensemble"
    f.createDimension("time", snaps)
    f.createDimension("member", members)
    f.createDimension("k", k)
    tv = f.createVariable("time", "d", ("time",))
    sv = f.createVariable("state", "d", ("time", "member", "k"))
    s = 0
    for step in range(steps):
        k1 = rhs(x)
        k2 = rhs(x + 0.5 * dt * k1)
        k3 = rhs(x + 0.5 * dt * k2)
        k4 = rhs(x + dt * k3)
        x = x + dt / 6 * (k1 + 2 * k2 + 2 * k3 + k4)
        if (step + 1) % every == 0 and s < snaps:
            tv[s] = (step + 1) * dt
            sv[s] = x
            s += 1
    f.close()


def m_ar_audio(rng, d, spec):
    ch, secs, rate = int(spec.get("channels", 4)), float(spec.get("seconds", 60)), int(spec.get("rate", 48000))
    n = int(secs * rate)
    from scipy.signal import lfilter
    pcm = np.empty((n, ch), dtype="<i2")
    for c in range(ch):
        # stable AR(4) with slowly varying excitation envelope
        a = [1.0, -rng.uniform(1.2, 1.9), rng.uniform(0.5, 0.95), -rng.uniform(0.0, 0.2), rng.uniform(0.0, 0.05)]
        roots = np.roots(a)
        if np.any(np.abs(roots) >= 0.999):
            a = [1.0, -1.5, 0.7, 0.0, 0.0]
        env = np.repeat(np.abs(rng.standard_normal(int(secs * 10) + 1)), rate // 10)[:n]
        sig = lfilter([1.0], a, rng.standard_normal(n) * env)
        sig = sig / (np.abs(sig).max() + 1e-12) * rng.uniform(4000, 30000)
        pcm[:, c] = np.clip(np.rint(sig), -32768, 32767).astype("<i2")
    counters = (np.arange(n, dtype=np.int64) * 3 + rng.integers(0, 2, n).cumsum()).astype("<i4")
    with open(os.path.join(d, "pcm_int16.raw"), "wb") as fh:
        fh.write(pcm.tobytes())
    with open(os.path.join(d, "counters_int32.raw"), "wb") as fh:
        fh.write(counters.tobytes())
    with open(os.path.join(d, "audio.json"), "w", encoding="utf-8", newline="\n") as fh:
        json.dump({"pcm": {"dtype": "<i2", "shape": [n, ch], "rate": rate, "interleaved": True},
                   "counters": {"dtype": "<i4", "shape": [n]}}, fh, sort_keys=True)
        fh.write("\n")


MODELS = {"gray-scott": m_gray_scott, "advection": m_advection, "lorenz96": m_lorenz96, "ar-audio": m_ar_audio}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    p = json.loads(args.params)
    runs = p["runs"]
    children = np.random.SeedSequence([args.seed, 0xC0FFEE]).spawn(len(runs))
    for spec, child in zip(runs, children):
        rng = np.random.default_rng(child)
        d = os.path.join(args.out, spec["name"])
        os.makedirs(d, exist_ok=True)
        MODELS[spec["model"]](rng, d, spec)


if __name__ == "__main__":
    main()
