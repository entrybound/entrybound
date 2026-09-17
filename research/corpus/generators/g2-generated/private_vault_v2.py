#!/usr/bin/env python3
"""Generated encrypted-private-archive workload for family F20 (corpus group g2-generated).
GENERATED DATA. Held-out only: a structurally distinct generator from private_vault.py (used for
tuning/validation) -- different document set, an RSA (not ed25519) SSH key used directly as an age
recipient instead of a dedicated throwaway OpenPGP keypair, a multi-volume encrypted 7z instead of
a single-file one, and a LUKS2 block-device-level encrypted container in addition to the
file/archive-level ones.

Corpus round-1 critic gap (F20 encrypted private archives, BLOCKER): see private_vault.py's
docstring for the full rationale. This is held-out's counterpart.

Produces:
  plaintext/                 a fake personal tree: a payroll-earnings-statement-style PDF
                              (reportlab), a personal budget workbook (XLSX, openpyxl, quarterly
                              layout), a free-text diary.txt whose prose embeds fake account
                              numbers/secrets (secrets hidden in natural-language content, not just
                              structured fields), a passwords_export.csv (the shape of an accidental
                              password-manager CSV export leak), an RSA-3072 OpenSSH keypair
                              (passphrase-protected), an age identity file, and a .env/config.yaml.
  plaintext.tar.gz            the same tree, tarred+gzipped.
  vault-zipcrypto.zip          `zip -P` -- legacy ZipCrypto.
  vault-7z-headers-mv.7z.001/.002 (+ more)  `7z -t7z -mhe=on -p -v<n> ` -- encrypted-header 7z
                              split into small volumes (multi-volume archive semantics).
  vault-gpg-symmetric.gpg      `gpg --batch --symmetric --cipher-algo AES256`.
  vault-age-passphrase.age     `age -p` (via `script -qec`, as in private_vault.py).
  vault-age-ssh-recipient.age  `age -r "ssh-rsa AAAA..."` -- age encrypting directly to the
                              plaintext tree's own SSH public key, so the SSH key is both a login
                              credential and (via age) a file-decryption credential -- a distinct
                              key-reuse pattern from private_vault.py's separate GPG keypair.
  vault-luks2.img              a small LUKS2 (cryptsetup) block image, ext4 inside, containing a
                              copy of plaintext/ -- a block-device-level encrypted container
                              alongside the file/archive-level ones, closing the "LUKS2 image if
                              feasible without Docker" request (loop device + cryptsetup, no
                              Docker involved).
  vault.kdbx                   a KeePass KDBX4 database (pykeepass) holding every password above.
  PROVENANCE.txt                every password/passphrase, tool versions and exact invocations,
                              written only into the materialized item (held-out descriptions stay
                              provenance-only in the committed source-of-truth; this file is never
                              read back by any agent).

Like private_vault.py, key material (SSH/age keypairs, LUKS2 header) is generated fresh by each
real tool's own CSPRNG at build time (not reproducible bit-for-bit); kind=build, output_pin
defaults to None (recorded, not pinned). Only the deterministic parts (documents' text/tables,
persona, passwords/passphrases, .env contents) are seeded from EB_SEED.

Params (EB_PARAMS_JSON): {"marker": <str>}
"""

import argparse
import csv
import json
import os
import random
import shlex
import shutil
import subprocess
import tarfile
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))

FIRST_NAMES = ["Selin", "Okonkwo", "Marisol", "Bartholomew", "Ingeborg", "Farouk", "Liesel", "Aditya"]
LAST_NAMES = ["Duarte", "Yilmaz", "Kastner", "Achebe", "Solberg", "Nasser", "Vukovic", "Ramaswamy"]
STREETS = ["Larkspur", "Thistlewood", "Quarryside", "Fenwick", "Brambleton", "Ashgrove"]
CITIES = [("Weldon Falls", "PA", "19xxx"), ("Cascade Mills", "WA", "985xx"), ("Grayport", "ME", "049xx")]
EMPLOYERS = ["Northgate Analytics LLC", "Cascade Freight Cooperative", "Weldon Municipal Utilities"]


def persona(rng):
    first, last = rng.choice(FIRST_NAMES), rng.choice(LAST_NAMES)
    street_no = rng.randint(100, 9999)
    street = rng.choice(STREETS)
    city, state, zipfmt = rng.choice(CITIES)
    zipc = zipfmt.replace("xx", f"{rng.randint(0, 99):02d}")
    return {
        "name": f"{first} {last}",
        "email": f"{first.lower()}.{last.lower()}{rng.randint(10, 99)}@example-mail.invalid",
        "address": f"{street_no} {street} Rd, {city}, {state} {zipc}",
        "employer": rng.choice(EMPLOYERS),
        "employee_id": f"EMP-{rng.randint(10000, 99999)}",
        "account_no": f"{rng.randint(10**9, 10**10 - 1)}",
    }


def fake_password(rng, label, n=16):
    alphabet = "abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789!#%*-_"
    body = "".join(rng.choice(alphabet) for _ in range(n))
    return f"eb-vault-heldout-{label}-{body}"


def run(cmd, **kw):
    return subprocess.run(cmd, check=True, capture_output=True, text=True, **kw)


# --------------------------------------------------------------------------------------
# documents


def build_pdf_earnings(path: Path, p: dict, rng: random.Random) -> None:
    from reportlab.lib.pagesizes import LETTER
    from reportlab.lib.units import inch
    from reportlab.pdfgen import canvas

    c = canvas.Canvas(str(path), pagesize=LETTER)
    c.setTitle("Earnings statement (fictitious test data)")
    c.setFont("Helvetica-Bold", 14)
    c.drawString(1 * inch, 10 * inch, p["employer"])
    c.setFont("Helvetica", 10)
    c.drawString(1 * inch, 9.7 * inch, "FICTITIOUS TEST DATA -- generated for Entrybound corpus research, not a real person")
    c.drawString(1 * inch, 9.4 * inch, f"Employee: {p['name']}  ({p['employee_id']})")
    c.drawString(1 * inch, 9.2 * inch, f"Address: {p['address']}")
    c.drawString(1 * inch, 9.0 * inch, "Pay period: 2026-Q1")
    y = 8.5 * inch
    c.setFont("Helvetica-Bold", 10)
    for label, x in [("Item", 1), ("Amount", 5)]:
        c.drawString(x * inch, y, label)
    c.setFont("Helvetica", 9)
    gross = round(rng.uniform(4200, 8800), 2)
    lines = [("Gross pay", gross), ("Federal tax", -round(gross * 0.14, 2)),
             ("State tax", -round(gross * 0.05, 2)), ("Retirement (401k)", -round(gross * 0.06, 2)),
             ("Health insurance", -round(rng.uniform(80, 220), 2))]
    net = sum(v for _, v in lines)
    y -= 0.25 * inch
    for label, amt in lines:
        c.drawString(1 * inch, y, label)
        c.drawRightString(6 * inch, y, f"{amt:,.2f}")
        y -= 0.22 * inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1 * inch, y - 0.2 * inch, f"Net pay: {net:,.2f}")
    c.save()


def build_xlsx_quarterly_budget(path: Path, p: dict, rng: random.Random) -> None:
    import openpyxl
    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "Quarterly Budget"
    ws.append(["FICTITIOUS TEST DATA", "generated for Entrybound corpus research"])
    ws.append(["Category", "Q1", "Q2", "Q3", "Q4", "Total"])
    cats = ["Housing", "Food", "Transportation", "Healthcare", "Entertainment", "Savings"]
    row = 3
    for name in cats:
        vals = [round(rng.uniform(200, 1600), 2) for _ in range(4)]
        ws.append([name, *vals, None])
        cols = "BCDE"
        ws.cell(row=row, column=6, value=f"=SUM({cols[0]}{row}:{cols[-1]}{row})")
        row += 1
    wb.save(str(path))


def build_diary(path: Path, p: dict, rng: random.Random, secrets: dict) -> None:
    entries = [
        f"2026-01-04: Finally set up autopay for the {p['employer']} direct deposit into acct "
        f"...{p['account_no'][-4:]}. Wrote the wifi password down here so I stop forgetting it: "
        f"{secrets['wifi_password']}.",
        "2026-01-19: Spent the afternoon reorganizing the garage. Found the old router box.",
        f"2026-02-02: New work laptop. IT set a temporary VPN passphrase, noting it here until I "
        f"memorize it: {secrets['vpn_passphrase']}.",
        "2026-02-20: Dentist appointment moved to next Thursday.",
        f"2026-03-11: Renewed the storage unit lease online, had to reset the account password to "
        f"{secrets['storage_site_password']} because the old one expired.",
    ]
    path.write_text("\n\n".join(entries) + "\n", encoding="utf-8")


def build_passwords_csv(path: Path, p: dict, rng: random.Random, secrets: dict) -> None:
    rows = [
        ("name", "url", "username", "password", "note"),
        ("Email", "https://mail.example-mail.invalid", p["email"], secrets["mail_password"], ""),
        ("Home Wi-Fi", "", "", secrets["wifi_password"], "router in hallway closet"),
        ("Work VPN", "https://vpn.example-corp.invalid", p["employee_id"], secrets["vpn_passphrase"], "temporary, IT-issued"),
        ("Storage unit portal", "https://storage.example-storage.invalid", p["email"], secrets["storage_site_password"], ""),
    ]
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerows(rows)


def build_dotfiles(root: Path, p: dict, rng: random.Random, secrets: dict) -> None:
    (root / ".env").write_text(
        "# FICTITIOUS TEST DATA -- fake credentials for Entrybound corpus research\n"
        f"SMTP_PASSWORD={secrets['mail_password']}\n"
        f"BACKUP_ENCRYPTION_KEY={secrets['backup_key']}\n", encoding="utf-8")
    (root / "config.yaml").write_text(
        f"# FICTITIOUS TEST DATA\nuser: {p['name']}\nemployee_id: {p['employee_id']}\n", encoding="utf-8")


# --------------------------------------------------------------------------------------
# keys


def build_ssh_keypair_rsa(root: Path, comment: str, passphrase: str) -> str:
    d = root / ".ssh"
    d.mkdir(parents=True, exist_ok=True)
    key_path = d / "id_rsa"
    run(["ssh-keygen", "-q", "-t", "rsa", "-b", "3072", "-N", passphrase, "-C", comment, "-f", str(key_path)])
    os.chmod(key_path, 0o600)
    return (d / "id_rsa.pub").read_text(encoding="utf-8").strip()


def build_age_identity(root: Path) -> str:
    d = root / ".config" / "age"
    d.mkdir(parents=True, exist_ok=True)
    key_path = d / "keys.txt"
    run(["age-keygen", "-o", str(key_path)])
    os.chmod(key_path, 0o600)
    return run(["age-keygen", "-y", str(key_path)]).stdout.strip()


# --------------------------------------------------------------------------------------
# encrypted containers


def zip_zipcrypto(src: Path, dst: Path, password: str) -> None:
    run(["zip", "-q", "-r", "-P", password, str(dst), src.name], cwd=str(src.parent))


def sevenz_encrypted_headers_multivolume(src: Path, dst: Path, password: str, volume_bytes: int) -> None:
    run(["7z", "a", "-t7z", "-mhe=on", f"-p{password}", "-mx=9", f"-v{volume_bytes}b", str(dst), src.name],
        cwd=str(src.parent))


def gpg_symmetric(src: Path, dst: Path, password: str, env: dict) -> None:
    run(["gpg", "--batch", "--yes", "--passphrase", password, "--pinentry-mode", "loopback",
         "--symmetric", "--cipher-algo", "AES256", "-o", str(dst), str(src)], env=env)


def age_passphrase(src: Path, dst: Path, password: str) -> None:
    inner = f"age -p -o {shlex.quote(str(dst))} {shlex.quote(str(src))}"
    r = subprocess.run(["script", "-qec", inner, "/dev/null"], input=f"{password}\n{password}\n", text=True,
                       capture_output=True)
    if r.returncode != 0 or not dst.exists():
        raise RuntimeError(f"age -p failed: rc={r.returncode} stdout={r.stdout!r} stderr={r.stderr!r}")


def age_ssh_recipient(src: Path, dst: Path, ssh_pubkey: str) -> None:
    run(["age", "-r", ssh_pubkey, "-o", str(dst), str(src)])


def build_luks2_image(plaintext: Path, dst_img: Path, password: str, scratch: Path, size_mib: int = 48) -> dict:
    """Block-device-level container: a loop-backed LUKS2 volume, ext4 inside, holding a copy of
    plaintext/. Uses only losetup/cryptsetup/mkfs.ext4/mount (already root inside WSL2 for all
    corpus provisioning; no Docker involved). Reduced luks2-metadata/keyslots area so a modest
    image size still leaves room for data (the default 16 MiB header would otherwise dominate a
    small-tier item)."""
    dst_img.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["dd", "if=/dev/zero", f"of={dst_img}", "bs=1M", f"count={size_mib}", "status=none"], check=True)
    loop = subprocess.run(["losetup", "-f"], check=True, capture_output=True, text=True).stdout.strip()
    mapper_name = f"eb-luks-{os.getpid()}"
    mnt = scratch / "luks-mnt"
    mnt.mkdir(parents=True, exist_ok=True)
    try:
        run(["losetup", loop, str(dst_img)])
        subprocess.run(["cryptsetup", "luksFormat", "--type", "luks2", "--batch-mode",
                        "--luks2-metadata-size", "2m", "--luks2-keyslots-size", "8m",
                        "--pbkdf", "argon2id", loop, "-"], input=password, text=True, check=True,
                       capture_output=True)
        subprocess.run(["cryptsetup", "open", "--type", "luks2", loop, mapper_name, "-"],
                       input=password, text=True, check=True, capture_output=True)
        run(["mkfs.ext4", "-q", "-F", f"/dev/mapper/{mapper_name}"])
        run(["mount", f"/dev/mapper/{mapper_name}", str(mnt)])
        run(["cp", "-a", f"{plaintext}/.", f"{mnt}/"])
        run(["sync"])
    finally:
        subprocess.run(["umount", str(mnt)], capture_output=True)
        subprocess.run(["cryptsetup", "close", mapper_name], capture_output=True)
        subprocess.run(["losetup", "-d", loop], capture_output=True)
    return {"loop_device_used": loop, "image_size_mib": size_mib, "filesystem": "ext4",
           "luks2_metadata_size": "2m", "luks2_keyslots_size": "8m", "pbkdf": "argon2id"}


def build_kdbx(path: Path, password: str, entries: list) -> None:
    from pykeepass import create_database
    kp = create_database(str(path), password=password)
    for title, user, pw, url, notes in entries:
        kp.add_entry(kp.root_group, title=title, username=user, password=pw, url=url, notes=notes)
    kp.save()


# --------------------------------------------------------------------------------------


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    out = Path(args.out)
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp")) / "private_vault_v2_scratch"
    scratch.mkdir(parents=True, exist_ok=True)
    home = Path(os.environ.get("HOME", scratch / "home"))
    home.mkdir(parents=True, exist_ok=True)
    gpg_env = dict(os.environ, GNUPGHOME=str(home / ".gnupg"), HOME=str(home))
    (home / ".gnupg").mkdir(mode=0o700, parents=True, exist_ok=True)

    rng = random.Random(args.seed)
    p = persona(rng)
    plaintext = out / "plaintext"
    plaintext.mkdir(parents=True, exist_ok=True)

    secrets = {
        "mail_password": fake_password(rng, "mail"),
        "wifi_password": fake_password(rng, "wifi", n=12),
        "vpn_passphrase": fake_password(rng, "vpn"),
        "storage_site_password": fake_password(rng, "storage"),
        "backup_key": fake_password(rng, "backup", n=32),
        "ssh_passphrase": fake_password(rng, "ssh"),
        "gpg_symmetric_passphrase": fake_password(rng, "gpgsym"),
        "age_passphrase": fake_password(rng, "age"),
        "zipcrypto_password": fake_password(rng, "zc"),
        "sevenz_password": fake_password(rng, "7z"),
        "luks2_password": fake_password(rng, "luks2"),
        "kdbx_password": fake_password(rng, "kdbx"),
    }

    build_pdf_earnings(plaintext / "earnings-statement-2026Q1.pdf", p, rng)
    build_xlsx_quarterly_budget(plaintext / "budget-quarterly.xlsx", p, rng)
    build_diary(plaintext / "diary.txt", p, rng, secrets)
    build_passwords_csv(plaintext / "passwords_export.csv", p, rng, secrets)
    build_dotfiles(plaintext, p, rng, secrets)
    ssh_pub = build_ssh_keypair_rsa(plaintext, p["email"], secrets["ssh_passphrase"])
    age_pub = build_age_identity(plaintext)

    for f in plaintext.rglob("*"):
        os.utime(f, (EPOCH, EPOCH))

    tar_path = out / "plaintext.tar.gz"
    with tarfile.open(tar_path, "w:gz") as tf:
        tf.add(plaintext, arcname="plaintext")

    zip_zipcrypto(plaintext, out / "vault-zipcrypto.zip", secrets["zipcrypto_password"])
    sevenz_encrypted_headers_multivolume(plaintext, out / "vault-7z-headers-mv.7z", secrets["sevenz_password"],
                                        volume_bytes=4 * 1024)
    gpg_symmetric(tar_path, out / "vault-gpg-symmetric.gpg", secrets["gpg_symmetric_passphrase"], gpg_env)
    age_passphrase(tar_path, out / "vault-age-passphrase.age", secrets["age_passphrase"])
    age_ssh_recipient(tar_path, out / "vault-age-ssh-recipient.age", ssh_pub)
    luks_info = build_luks2_image(plaintext, out / "vault-luks2.img", secrets["luks2_password"], scratch)

    build_kdbx(out / "vault.kdbx", secrets["kdbx_password"], [
        ("Email", p["email"], secrets["mail_password"], "https://mail.example-mail.invalid", "fake test credential"),
        ("Home Wi-Fi", "", secrets["wifi_password"], "", ""),
        ("Work VPN", p["employee_id"], secrets["vpn_passphrase"], "", ""),
        ("Storage unit", p["email"], secrets["storage_site_password"], "", ""),
        ("SSH key passphrase", p["email"], secrets["ssh_passphrase"], "", "protects plaintext/.ssh/id_rsa"),
        ("age passphrase", "", secrets["age_passphrase"], "", "protects vault-age-passphrase.age"),
        ("GPG symmetric passphrase", "", secrets["gpg_symmetric_passphrase"], "", "protects vault-gpg-symmetric.gpg"),
        ("LUKS2 volume password", "", secrets["luks2_password"], "", "protects vault-luks2.img"),
        ("ZipCrypto password", "", secrets["zipcrypto_password"], "", "protects vault-zipcrypto.zip"),
        ("7z multivolume password", "", secrets["sevenz_password"], "", "protects vault-7z-headers-mv.7z.*"),
    ])

    tool_versions = {}
    for name, cmd in {"zip": ["zip", "-v"], "7z": ["7z"], "gpg": ["gpg", "--version"], "age": ["age", "--version"],
                      "ssh-keygen": ["ssh-keygen", "-V"], "age-keygen": ["age-keygen", "--version"],
                      "cryptsetup": ["cryptsetup", "--version"]}.items():
        try:
            r = subprocess.run(cmd, capture_output=True, text=True)
            tool_versions[name] = (r.stdout or r.stderr).splitlines()[0].strip()
        except Exception as e:
            tool_versions[name] = f"<unavailable: {e}>"

    (out / "PROVENANCE.txt").write_text(
        "FICTITIOUS TEST DATA. Held-out item: this file exists only inside the materialized item "
        "(never committed to the corpus source-of-truth, never read back by any provisioning agent). "
        "Every password/passphrase below is a fixed, seeded test value chosen by this item's "
        "generator (private_vault_v2.py) from a documented formula (fake_password(rng, label)); "
        "none protects anything of real value.\n\n"
        f"persona: {json.dumps(p, indent=2)}\n\n"
        "passwords/passphrases:\n" + "\n".join(f"  {k}: {v}" for k, v in secrets.items()) + "\n\n"
        f"age identity public key: {age_pub}\n"
        f"ssh public key (also used as an age recipient): {ssh_pub}\n\n"
        "containers and exact invocations:\n"
        "  vault-zipcrypto.zip          zip -r -P <zipcrypto_password> vault-zipcrypto.zip plaintext/\n"
        "  vault-7z-headers-mv.7z.*     7z a -t7z -mhe=on -p<sevenz_password> -mx=9 -v4096b vault-7z-headers-mv.7z plaintext/\n"
        "  vault-gpg-symmetric.gpg      gpg --symmetric --cipher-algo AES256 --passphrase <gpg_symmetric_passphrase> plaintext.tar.gz\n"
        "  vault-age-passphrase.age     age -p (interactive passphrase via `script -qec`) plaintext.tar.gz\n"
        "  vault-age-ssh-recipient.age  age -r '<ssh public key above>' plaintext.tar.gz\n"
        "  vault-luks2.img              cryptsetup luksFormat --type luks2 --pbkdf argon2id (loop-backed, ext4 inside)\n"
        f"    LUKS2 build parameters: {json.dumps(luks_info, indent=2)}\n"
        "  vault.kdbx                   pykeepass.create_database(password=<kdbx_password>)\n\n"
        f"tool versions at build time: {json.dumps(tool_versions, indent=2)}\n",
        encoding="utf-8")

    print(f"private_vault_v2: persona={p['name']!r} containers=6+luks2")


if __name__ == "__main__":
    main()
