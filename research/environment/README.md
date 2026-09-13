# Research environment fingerprints

This directory records the execution environments used for Entrybound research runs.
Each run result should carry the `env_id` of the environment it ran in.

| File | Purpose |
| --- | --- |
| `<env_id>.json` | One fingerprint per distinct environment state (content-addressed, never edited) |
| `index.json` | Current `name -> env_id` mapping plus per-name `history` of every id ever recorded |
| `provenance/docker-ubuntu-24.04.downloads.json` | URL, size and SHA-256 of every byte downloaded to build the container environment |

Tooling lives in `research/tools/env/`:

| Script | Runs on | Does |
| --- | --- | --- |
| `capture_env.py` | Windows CPython 3.13, Linux CPython 3.12 (stdlib only) | `capture`, `store`, `verify`, `lookup` |
| `capture_wsl.sh` | WSL Ubuntu (via `wsl.exe -d Ubuntu -- bash ...`) | captures `wsl-ubuntu` |
| `capture_docker.py` / `capture_docker.sh` | WSL Ubuntu, driving Docker Desktop | pulls/verifies the pinned image, builds the probe image, captures `docker-ubuntu-24.04` |
| `docker/ubuntu-24.04-env.Dockerfile` | BuildKit | pinned base + `python3` from a pinned apt snapshot |
| `capture_all.ps1` | PowerShell 7 on Windows | runs all three captures in order, then `verify` |

## Environments captured on 2026-09-12 (baseline SHA 9e44608)

`index.json` is authoritative; this table is a snapshot.

| Name | env_id | platform_id | What it is |
| --- | --- | --- | --- |
| `windows-host` | `71b86070b993…` | `0ec6f785147b…` | Windows 11 Pro 25H2 (build 26200.9445) on the laptop itself |
| `wsl-ubuntu` | `8576a4a4947e…` | `0232788ff3a6…` | WSL2 distro `Ubuntu` 24.04.1, kernel 5.15.167.4-microsoft-standard-WSL2 |
| `docker-ubuntu-24.04` | `e4774cb5fe0c…` | `53cb43274671…` | `docker.io/library/ubuntu:24.04@sha256:224a1869…` (linux/amd64 manifest `sha256:a61567bd…`) + `python3` from snapshot `20260912T000000Z`, on Docker Desktop 4.90.0 / Engine 29.7.2 |

## Rerunning

```powershell
pwsh -File D:/Projects/entrybound/entrybound/research/tools/env/capture_all.ps1
```

Every step is idempotent. When the recomputed stable section matches an existing file, that file is kept
unchanged, including its original `captured_at_utc`. A changed environment gets a new `<env_id>.json`; the
index moves the name to the new id and appends it to `history`, and the old file stays so old results still
resolve. Integrity failures abort with a non-zero exit: a fingerprint whose stored ids don't match its
content, a file name that differs from its `env_id`, a registry manifest that doesn't hash to its pinned
digest, or a `.deb` whose SHA-256 differs from the provenance file.

`python capture_env.py verify --out-dir research/environment` re-hashes everything. Run it after any
checkout. The ids are computed over parsed JSON, not file bytes, so `core.autocrlf` line-ending changes to
these files don't affect verification.

## Identity model

Each fingerprint has two sections:

* `stable` is hashed. It holds everything that defines the environment.
* `capture` is not hashed. It holds the capture time, uptime, load average, available memory, AC/battery
  state, worktree dirtiness, mount source devices, free disk space, Docker driver details and
  human-readable `observations`.

Three ids are derived from `stable` with
`canonical_json = json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)` in UTF-8:

| Id | Hash of | Use |
| --- | --- | --- |
| `env_id` | all of `stable` | Exact environment, including the repo commit and `Cargo.lock` |
| `env_id_without_repo` | `stable` minus `repo` | Group results from the same machine and toolchain across commits |
| `platform_id` | `stable.platform` only | OS, kernel, CPU, memory, virtualization and power configuration. Tools, toolchains and repo are excluded |

Floats are rejected inside `stable`, so the encoding is identical on Python 3.12 and 3.13. A verify run from
WSL Python 3.12 accepts the files written by Windows Python 3.13.

**Parent link.** `wsl-ubuntu` and `docker-ubuntu-24.04` store `stable.platform.parent = {relation: "hosted-by",
platform_id: <windows-host platform_id>}`. Neither guest can see the host's P/E-core layout, power scheme,
Defender state or Windows build. Linking to the host's `platform_id` means any change to those host facts
also changes the guests' ids. Host tool updates (for example Git for Windows' `xz.exe`) don't propagate.

**Privacy.** Every recorded string passes through a redactor that replaces Windows profile names
(`C:\Users\<user>`, `/mnt/c/Users/<user>`), `/home/<user>` and the current account name. The hostname, volume
serials, BIOS/memory serial numbers, engine IDs and `PATH` are never recorded. `.wslconfig` lives in the
Windows user profile and is deliberately not read. WSL VM limits are recorded as observed from inside the VM.

## Field reference (`stable`)

| Field | Content |
| --- | --- |
| `schema`, `kind` | `entrybound.research.env-fingerprint/1`; `windows-host`, `wsl`, `container` or `linux` (auto-detected) |
| `platform.os` | Windows: registry `CurrentVersion` values plus a derived `marketing_name`. The registry `ProductName` still says "Windows 10 Pro" on Windows 11; use `build >= 22000`. Linux: `/etc/os-release` and glibc version. Containers also record the count and SHA-256 of the full `dpkg` package list |
| `platform.kernel` | NT version `10.0.build.UBR`; Linux release, `/proc/version`, clocksource and THP mode |
| `platform.virtualization` | Windows: `HypervisorPresent`, VBS status, `wsl --version`. Linux: CPUID `hypervisor` flag, WSL2 kernel marker, `systemd-detect-virt`, WSL distro/interop/WSLg, `/etc/wsl.conf`, container markers (`/.dockerenv`, `/run/.containerenv`) |
| `platform.cpu` | Windows: brand string, CPUID identifier, registry microcode revision, `GetLogicalProcessorInformationEx` topology (cores and logical processors per `EfficiencyClass`, SMT, LP id ranges, per-class L2 size, cache inventory, groups, NUMA) plus a `Win32_Processor` cross-check. Linux: `/proc/cpuinfo` model, microcode, flags hash plus key flags, the topology as presented in sysfs, hybrid PMU lists (`/sys/devices/cpu_core`, `cpu_atom`), cpufreq availability, cpu0 caches, vulnerability mitigations |
| `platform.memory` | Windows: `GlobalMemoryStatusEx` total, installed memory, module capacity and speed. Linux: `MemTotal`, `SwapTotal`, huge page size, cgroup limits |
| `platform.power` (Windows) | Active scheme GUID and name (`PowerGetActiveScheme` and `powercfg /getactivescheme`), Windows power-mode overlay for AC/DC, and AC/DC values of the processor power settings, including hidden ones such as boost mode, EPP (with the class-1 variant), heterogeneous policies, core parking and cooling policy |
| `platform.hardware`, `platform.security` (Windows) | Chassis type (Mobile), manufacturer/model, BIOS version and date, battery count; Defender real-time protection state |
| `platform.parent` | See parent link above |
| `filesystems[]` | For each `--work-dir`: Windows volume root, file system, flags and max component length. Linux mount point, fstype and options (9p fds and overlay snapshot directories are stripped because they change per boot or container), block size and `name_max`. Paths that don't exist yet resolve through their nearest existing ancestor |
| `process` | Python implementation, version, build, compiler, executable SHA-256; encodings; allow-listed tool-affecting env vars (`XZ_OPT`, `ZSTD_*`, `TAR_OPTIONS`, `RUSTFLAGS`, `RUSTUP_TOOLCHAIN`, `SOURCE_DATE_EPOCH`, `LANG`/`LC_*`, …); POSIX umask, rlimits and euid |
| `rust` | rustup path and version, `rustup show active-toolchain` run in the repo, installed toolchains, `rustc -Vv` and `cargo -V` via the rustup proxy in the repo dir (honours `rust-toolchain.toml`) and for every installed toolchain, and the `rustc` that plain `PATH` resolves to |
| `tools` | For tar, bsdtar, 7z, 7zz, 7za, 7zr, zstd, xz, gzip, pigz, lz4, lzip, brotli, pixz, par2, zip, unzip, bzip2, cpio, mksquashfs, hyperfine, sqlite3, jq, python, python3, java, docker, git, rustc, cargo and rustup: resolved path, symlink target, size and SHA-256 of the executable, parsed version and version line, exit code, and on Debian/Ubuntu the owning package and version. Windows app-execution aliases (for example `WindowsApps\python3.exe`) are recorded but never executed |
| `docker` | Client and server versions, Docker Desktop platform name, component versions (containerd, runc, docker-init), engine info subset (OS, kernel, NCPU, MemTotal, storage driver, cgroup driver and version, runtimes, security options, CLI plugins) |
| `repo` | HEAD ref and SHA read directly from `.git` (cross-checked against `git rev-parse HEAD` when git exists), and `Cargo.lock` / `rust-toolchain.toml` byte count, SHA-256, LF-normalized SHA-256 and CRLF flag |
| `container` | Only in the container fingerprint: pinned base image reference, platform manifest digest, apt snapshot, added packages, recipe SHA-256 (Dockerfile with LF line endings, base, platform, snapshot, packages) and run flags |

## CPU topology caveats

**The host CPU is hybrid.** The Intel Core i9-14900HX has 8 P-cores with Hyper-Threading and 16 E-cores
without it: 24 physical cores and 32 logical processors in one package, one NUMA node and one processor group.
Windows reports it exactly (`windows-host` → `platform.cpu.topology.efficiency_classes`):

| Class | Label | Cores | Logical processors | Windows LP ids | L2 |
| --- | --- | --- | --- | --- | --- |
| EfficiencyClass 1 | P-core | 8 (SMT) | 16 | 0-15 | 2 MiB per core |
| EfficiencyClass 0 | E-core | 16 | 16 | 16-31 | 4 MiB shared per 4-core cluster |

L3 is 36 MiB, shared by all 32 logical processors. The registry microcode revision is `0x123`.

**WSL2 and Docker Desktop show a synthetic, homogeneous topology.** Both guests run in the same Hyper-V
utility VM, so their NCPU and MemTotal are identical. That VM exposes 32 vCPUs as 1 package × 16 cores ×
2 threads, with L2 of "2048K" on every vCPU. There are no P/E labels (`/sys/devices/cpu_core` and `cpu_atom`
are absent, and `hybrid_cpu` is not in the flags), no cpufreq, and microcode is masked (`0xffffffff`).
Consequences:

* A Linux vCPU number does not correspond to a Windows logical processor. The Windows hypervisor schedules
  vCPUs onto any P- or E-core and migrates them. `taskset` inside WSL or a container cannot select P-cores.
* `nproc`, `lscpu`, `7z` "Threads:32" and zstd/xz/pigz auto-thread counts all see 32 equal CPUs. On the host,
  16 of those threads are E-core threads, and 8 of the "cores" are SMT siblings. Multi-threaded throughput
  does not scale like a 16-core or 32-core homogeneous machine.
* Single-thread results can land on either core class, so run-to-run variance is larger than on a
  homogeneous CPU. Report medians plus dispersion, never single runs.
* On Windows, P-cores can be targeted explicitly with processor affinity. LP mask `0x0000FFFF` is the
  P-core threads and `0xFFFF0000` is the E-cores.

**The WSL VM is a subset of the host.** The guest sees 31.2 GiB `MemTotal` and 8 GiB swap, against 63.7 GiB
usable (64 GiB installed) on the host. Page cache inside the VM is separate from Windows' cache, so cold/warm
cache state has to be managed per side.

**This is a laptop.** Chassis type is Mobile (model X370SNx1, BIOS 1.07.01aNS1), with a battery. The HX part
runs far above its base power when load starts, then pulls back to sustained limits set by the firmware and
the cooling. Long benchmarks therefore drift downward as the machine heats. Temperatures and throttling
counters are not captured: they need admin rights on Windows, and the WSL VM exposes no thermal zones. For
the later timing phase:

* Stay on AC power. The AC/battery state is in `capture.platform.power_status`; it was online, battery 80%,
  at this capture.
* Warm up before measuring, interleave compared tools (ABAB, not AAAA BBBB), and record the timestamps.
* Treat results across different `platform_id`s, or across AC/DC, as not comparable.

**Windows power scheme at capture was "Balanced"** (`381b4222-f694-41f0-9685-ff5bb260df2e`). The AC power-mode
overlay was `00000000-…` (Balanced, no overlay); the DC overlay was `961cc777-…` (Best power efficiency).
Processor settings on AC:

| Setting | AC value | Meaning |
| --- | --- | --- |
| Minimum processor state | 5% | |
| Maximum processor state | 100% | |
| `PERFBOOSTMODE` | 2 | Aggressive |
| `PERFBOOSTPOL` | 60 | |
| `PERFEPP` / `PERFEPP1` | 45 | |
| `CPMINCORES` | 4% | Core parking allowed |
| `HETEROPOLICY` | 0 | |
| `SCHEDPOLICY` | 5 | |
| `SYSCOOLPOL` | 1 | Active |

Under Balanced, Windows parks cores and steers background and low-priority threads to E-cores. That affects
WSL vCPU placement too. Switching to High performance or Best performance changes `windows-host`'s
`platform_id` and, through the parent link, the ids of both guests.

## Other findings recorded at capture

These are in `capture.observations`; decisions should rely on the stable fields.

1. **In WSL, `rustc` on PATH is not the repo toolchain.** Research scripts run as
   `wsl.exe -d Ubuntu -- bash script.sh`, a non-login shell that doesn't source `~/.cargo/env`. A bare `rustc`
   there resolves to Ubuntu's `/usr/bin/rustc` 1.75.0. The repo toolchain (rustup `stable`, 1.98.1) is only
   reachable as `/root/.cargo/bin/cargo` or `rustc`. rustup's own default toolchain is also 1.75.0, and
   `nightly-2026-08-01` (1.99.0-nightly) is installed.
2. **Windows and WSL build with different stable compilers.** `rust-toolchain.toml` pins
   `channel = "stable"`, which floats. Windows `stable-x86_64-pc-windows-msvc` is 1.97.1 (LLVM 22.1.6); WSL
   `stable-x86_64-unknown-linux-gnu` is 1.98.1 (LLVM 22.1.8). Windows rustup isn't on `PATH` either.
3. **`7z` is 7-Zip 23.01, not p7zip 16.02, and there is no `7zz`.** In WSL, `7z`, `7za` and `7zr` all come
   from Ubuntu package `7zip 23.01+dfsg-11`. `p7zip-full` is only a transitional package. The upstream-style
   `7zz` binary is not on PATH. Label baselines by the recorded path and package, not by the command name.
4. **WSL tool versions.** GNU tar 1.35, bsdtar 3.7.2, zstd 1.5.5, xz 5.4.5 (package
   `5.6.1+really5.4.5`), gzip 1.12, pigz 2.8, lz4 1.9.4, lzip 1.24.1, brotli 1.1.0, pixz 1.0.7, par2 0.8.1,
   Zip 3.0, UnZip 6.00, OpenJDK 21.0.12, Python 3.12.3. On Windows, `tar` is `System32\tar.exe`, which is
   bsdtar 3.8.8. `xz` 5.6.3, `brotli` and `bzip2` come from Git for Windows' `mingw64\bin` only because that
   directory is on the capturing shell's PATH. The other baseline tools are absent on Windows.
5. **The repo on `/mnt/d` is drvfs (9p) without `metadata`.** chmod, chown and POSIX modes are not
   persisted there, so metadata-fidelity experiments must use ext4 (`/root/eb-research`, on the WSL VHD).
   The Docker bind mount of the repo is also 9p, but with `metadata`.
6. **Windows host I/O caveats.** Microsoft Defender real-time protection is on and tamper-protected. The
   Hyper-V hypervisor is present (VBS running), so the Windows host itself runs as the root partition.
7. **Docker Desktop caches `docker info`.** `SystemTime` stays frozen across calls, so container and image
   counts from it are indicative only. The real container clock was measured separately
   (`capture.docker_driver.container_clock_minus_wsl_seconds`, about 0 s).
8. **The container is newer than the WSL distro.** The pinned image reports Ubuntu 24.04.4 while WSL is
   24.04.1, and the stock image contains only `tar` and `gzip` among the baseline tools. The container's
   `python3` binary is byte-identical to WSL's (`3.12.3-1ubuntu0.17`).

## Container provenance

The base image is pulled by index digest. `capture_docker.py` refetches the raw OCI index and the linux/amd64
manifest and checks that they hash to the pinned digests. `provenance/docker-ubuntu-24.04.downloads.json`
records the registry URL, size and SHA-256 of the index, manifest, config and the single layer (29,763,253
bytes). It also records all 14 `.deb` files installed from
`https://snapshot.ubuntu.com/ubuntu/20260912T000000Z/` (retrieval date 2026-09-12).

The snapshot service is HTTPS-only and the base image has no CA store. WSL's
`/etc/ssl/certs/ca-certificates.crt` is therefore bind-mounted for the build step only. Its SHA-256 is recorded,
and it is never written into a layer. Package integrity comes from apt's InRelease signature check plus the
recorded SHA-256 values. A second build on 2026-09-12, triggered by a Dockerfile edit, reproduced every
`.deb` hash exactly.
