"""Emit one line whenever a drive's free space falls below its limit.

Used as a long-running monitor alongside research workflows so storage
pressure is noticed immediately. Re-arms after free space recovers to 120% of
the limit.

Usage: python research/orchestration/disk_alarm.py [--c-min-gb 50] [--d-min-gb 100] [--interval 60]
"""

import argparse
import shutil
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--c-min-gb", type=float, default=50.0)
    parser.add_argument("--d-min-gb", type=float, default=100.0)
    parser.add_argument("--interval", type=float, default=60.0)
    args = parser.parse_args()
    limits = {"C:\\": args.c_min_gb, "D:\\": args.d_min_gb}
    warned = set()
    while True:
        for drive, limit in limits.items():
            free_gb = shutil.disk_usage(drive).free / 1024**3
            if free_gb < limit and drive not in warned:
                print(f"DISK LOW {drive} free={free_gb:.1f}GB below {limit}GB", flush=True)
                warned.add(drive)
            elif free_gb >= limit * 1.2 and drive in warned:
                warned.discard(drive)
        time.sleep(args.interval)


if __name__ == "__main__":
    main()
