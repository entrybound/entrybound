import pytest

from ebr.timev import TimeVParseError, exit_code_from, parse_clock, parse_time_v

SAMPLE = """\tCommand being timed: "bash -o pipefail -c gzip -9 -c -n 'in: file' > out.gz"
\tUser time (seconds): 0.12
\tSystem time (seconds): 0.03
\tPercent of CPU this job got: 98%
\tElapsed (wall clock) time (h:mm:ss or m:ss): 0:00.15
\tAverage shared text size (kbytes): 0
\tAverage unshared data size (kbytes): 0
\tAverage stack size (kbytes): 0
\tAverage total size (kbytes): 0
\tMaximum resident set size (kbytes): 3348
\tAverage resident set size (kbytes): 0
\tMajor (requiring I/O) page faults: 1
\tMinor (reclaiming a frame) page faults: 158
\tVoluntary context switches: 4
\tInvoluntary context switches: 2
\tSwaps: 0
\tFile system inputs: 16
\tFile system outputs: 824
\tSocket messages sent: 0
\tSocket messages received: 0
\tSignals delivered: 0
\tPage size (bytes): 4096
\tExit status: 0
"""


def test_parse_normal():
    p = parse_time_v(SAMPLE)
    assert p["user_s"] == 0.12 and p["sys_s"] == 0.03
    assert p["elapsed_s"] == pytest.approx(0.15)
    assert p["max_rss_kib"] == 3348
    assert p["fs_inputs"] == 16 and p["fs_outputs"] == 824
    assert p["cpu_percent"] == 98.0
    assert p["command"].startswith("bash -o pipefail -c gzip")
    assert exit_code_from(p) == 0


def test_parse_nonzero_and_signal():
    p = parse_time_v("Command exited with non-zero status 3\n" + SAMPLE.replace("Exit status: 0", "Exit status: 3"))
    assert p["nonzero_status"] == 3 and exit_code_from(p) == 3
    s = parse_time_v("Command terminated by signal 9\n" + SAMPLE.replace("98%", "?%"))
    assert exit_code_from(s) == -9
    assert s["cpu_percent"] is None


def test_clock_formats():
    assert parse_clock("1:02:03") == 3723
    assert parse_clock("0:01.50") == 1.5
    assert parse_clock("12:00.25") == pytest.approx(720.25)
    with pytest.raises(ValueError):
        parse_clock("12")


def test_missing_fields():
    with pytest.raises(TimeVParseError):
        parse_time_v("\tUser time (seconds): 0.1\n")
