#!/usr/bin/env python3
"""Generated encrypted-private-archive workload for family F20 (corpus group g2-generated).
GENERATED DATA. Used for tuning and validation (different seed/persona per split); a
structurally distinct generator (private_vault_v2.py) is used for held-out.

Corpus round-1 critic gap (F20 encrypted private archives, BLOCKER): F20 previously had only
ciphertext-like high-entropy inputs (ChaCha20/AES-CTR keystreams). There was no secret-bearing
private-document tree for evaluating compress-then-encrypt leakage, and no *existing* encrypted
archives to import or refuse. This generator builds:

  plaintext/                a fake personal "home vault" tree: a bank/card-style statement (PDF,
                             reportlab), a resume/cover letter (DOCX, python-docx), a personal
                             budget workbook (XLSX, openpyxl, with real SUM formulas), an OpenSSH
                             ed25519 keypair (passphrase-protected private key), an age identity
                             file, a throwaway OpenPGP keypair's exported public+secret keys, a
                             .env file and a config.yaml with fake API tokens/DB passwords, and a
                             Chrome-"Login Data"-shaped SQLite database of fake saved logins.
  plaintext.tar.gz           the same tree, tarred+gzipped (the compress-then-encrypt precursor).
  vault-zipcrypto.zip        `zip -P` of plaintext/ -- legacy ZipCrypto (PKWARE stream cipher).
  vault-aes256.zip           `7z -tzip -mem=AES256 -p` of plaintext/ -- WinZip AE-2 AES-256.
  vault-7z-encrypted-headers.7z  `7z -t7z -mhe=on -p` of plaintext/ -- 7z AES-256 with encrypted
                             headers (filenames themselves are not visible without the password).
  vault-gpg-symmetric.gpg    `gpg --batch --symmetric --cipher-algo AES256` of plaintext.tar.gz.
  vault-gpg-pubkey.gpg       `gpg --encrypt -r <fpr>` of plaintext.tar.gz, to a fresh throwaway
                             OpenPGP keypair generated for this item (exported into plaintext/).
  vault-age-passphrase.age   `age -p` of plaintext.tar.gz (passphrase mode; age reads the
                             passphrase interactively from a pty, so this is driven through
                             `script -qec` to allocate one non-interactively).
  vault-age-recipient.age    `age -r <age1...>` of plaintext.tar.gz, to a fresh age identity
                             generated for this item (exported into plaintext/.config/age/).
  vault.kdbx                 a KeePass KDBX4 database (pykeepass) holding entries for every
                             password/passphrase used above, so the whole thing round-trips: a
                             password manager describing the very secrets that protect it.
  PROVENANCE.txt             every password/passphrase used, in the clear, plus tool versions and
                             exact invocations -- "known test passwords/keys recorded in the item
                             recipe" per the gap-closure requirement. Tuning/validation only (not
                             held-out; held-out descriptions stay provenance-only, so the held-out
                             counterpart writes this file only inside the materialized item, never
                             into the committed source-of-truth notes).

All key material (SSH/age/GPG keypairs) is generated fresh by the real tool's own CSPRNG at build
time -- gpg/ssh-keygen/age-keygen have no reproducible-seed mode, so this item's bytes are not the
same on every rebuild (kind=build, output_pin left at its build default of None: recorded but not
pinned, the same disposition already used for shrink_real_disk_image.py and real_kernel_headers.py
outputs in this file). Only the deterministic parts (documents' text/tables, fake persona,
passwords/passphrases themselves, .env/config contents, SQLite rows) are seeded from EB_SEED.

Params (EB_PARAMS_JSON): {"variant": "tuning" | "validation", "persona_seed": <int>}
"""

import argparse
import json
import os
import random
import shlex
import shutil
import sqlite3
import struct
import subprocess
import sys
import tarfile
import time
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))

FIRST_NAMES = ["Priya", "Marcus", "Elena", "Tomas", "Aisha", "Declan", "Naledi", "Yusuf", "Ingrid", "Kenji"]
LAST_NAMES = ["Okafor", "Nakamura", "Silva", "Kowalski", "Haddad", "Lindqvist", "Petrova", "Marchetti", "Oduya", "Reyes"]
STREETS = ["Maple", "Birch", "Cedar", "Elm", "Willow", "Harbor", "Meridian", "Foundry", "Orchard", "Summit"]
CITIES = [("Riverbend", "OR", "97xxx"), ("Millbrook", "NY", "125xx"), ("Fairhaven", "MI", "490xx"),
          ("Ashcombe", "TX", "787xx"), ("Portwick", "NC", "285xx")]
BANKS = ["Northgate Community Credit Union", "Ledgerview Savings Bank", "Cobalt Trust Bank"]


def persona(rng):
    first, last = rng.choice(FIRST_NAMES), rng.choice(LAST_NAMES)
    street_no = rng.randint(100, 9999)
    street = rng.choice(STREETS)
    city, state, zipfmt = rng.choice(CITIES)
    zipc = zipfmt.replace("xx", f"{rng.randint(0, 99):02d}")
    return {
        "name": f"{first} {last}",
        "email": f"{first.lower()}.{last.lower()}{rng.randint(10, 99)}@example-mail.invalid",
        "address": f"{street_no} {street} St, {city}, {state} {zipc}",
        "phone": f"({rng.randint(200, 999)}) 555-{rng.randint(1000, 9999)}",
        "account_no": f"{rng.randint(10**9, 10**10 - 1)}",
        "bank": rng.choice(BANKS),
    }


def fake_password(rng, label, n=16):
    alphabet = "abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789!#%*-_"
    body = "".join(rng.choice(alphabet) for _ in range(n))
    return f"eb-vault-{label}-{body}"


def run(cmd, **kw):
    r = subprocess.run(cmd, check=True, capture_output=True, text=True, **kw)
    return r


# --------------------------------------------------------------------------------------
# documents


def build_pdf_statement(path: Path, p: dict, rng: random.Random) -> None:
    from reportlab.lib.pagesizes import LETTER
    from reportlab.lib.units import inch
    from reportlab.pdfgen import canvas

    c = canvas.Canvas(str(path), pagesize=LETTER)
    c.setTitle(f"{p['bank']} statement (fictitious test data)")
    c.setFont("Helvetica-Bold", 14)
    c.drawString(1 * inch, 10 * inch, p["bank"])
    c.setFont("Helvetica", 10)
    c.drawString(1 * inch, 9.7 * inch, "FICTITIOUS TEST DATA -- generated for Entrybound corpus research, not a real person")
    c.drawString(1 * inch, 9.4 * inch, f"Account holder: {p['name']}")
    c.drawString(1 * inch, 9.2 * inch, f"Address: {p['address']}")
    c.drawString(1 * inch, 9.0 * inch, f"Account: ...{p['account_no'][-4:]}    Statement period: 2026-03")
    y = 8.5 * inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1 * inch, y, "Date")
    c.drawString(2 * inch, y, "Description")
    c.drawString(5.3 * inch, y, "Amount")
    c.setFont("Helvetica", 9)
    merchants = ["Riverside Grocers", "Northline Transit", "Cedar Utility Co-op", "Fernwood Pharmacy",
                 "Anchor Coffee House", "Millrace Hardware", "BluePeak Wireless", "Harborview Clinic"]
    balance = rng.uniform(800, 4000)
    y -= 0.25 * inch
    for day in range(1, 23):
        if rng.random() < 0.55:
            continue
        amt = round(rng.uniform(-220, -4) if rng.random() < 0.85 else rng.uniform(500, 2600), 2)
        balance += amt
        c.drawString(1 * inch, y, f"2026-03-{day:02d}")
        c.drawString(2 * inch, y, rng.choice(merchants))
        c.drawRightString(6 * inch, y, f"{amt:,.2f}")
        y -= 0.22 * inch
        if y < 1 * inch:
            c.showPage()
            c.setFont("Helvetica", 9)
            y = 10 * inch
    c.setFont("Helvetica-Bold", 10)
    c.drawString(1 * inch, y - 0.3 * inch, f"Ending balance: {balance:,.2f}")
    c.save()


def build_docx_resume(path: Path, p: dict, rng: random.Random) -> None:
    import docx
    d = docx.Document()
    d.add_heading(p["name"], level=1)
    d.add_paragraph("FICTITIOUS TEST DATA -- generated for Entrybound corpus research, not a real person.")
    d.add_paragraph(f"{p['address']}  |  {p['phone']}  |  {p['email']}")
    d.add_heading("Experience", level=2)
    roles = [("Senior Operations Analyst", "Cobalt Logistics Group", "2022-2026"),
             ("Operations Analyst", "Fernwood Data Systems", "2019-2022"),
             ("Junior Analyst", "Harborview Consulting", "2017-2019")]
    for title, org, span in roles:
        d.add_paragraph(f"{title}, {org} ({span})", style="List Bullet")
        d.add_paragraph("Coordinated cross-team reporting and quarterly reconciliation workflows.")
    d.add_heading("Education", level=2)
    d.add_paragraph("B.A., Applied Statistics, Millbrook State University")
    d.save(str(path))


def build_xlsx_budget(path: Path, p: dict, rng: random.Random) -> None:
    import openpyxl
    wb = openpyxl.Workbook()
    ws = wb.active
    ws.title = "Monthly Budget"
    ws.append(["FICTITIOUS TEST DATA", "generated for Entrybound corpus research", "", ""])
    ws.append(["Category", "Budgeted", "Actual", "Difference"])
    cats = [("Rent/Mortgage", 1450), ("Groceries", 420), ("Utilities", 180), ("Transit", 95),
            ("Insurance", 210), ("Subscriptions", 45), ("Savings", 500), ("Discretionary", 260)]
    row = 3
    for name, budget in cats:
        actual = round(budget * rng.uniform(0.7, 1.25), 2)
        ws.append([name, budget, actual, None])
        ws.cell(row=row, column=4, value=f"=B{row}-C{row}")
        row += 1
    ws.append(["Total", None, None, None])
    ws.cell(row=row, column=2, value=f"=SUM(B3:B{row - 1})")
    ws.cell(row=row, column=3, value=f"=SUM(C3:C{row - 1})")
    ws.cell(row=row, column=4, value=f"=SUM(D3:D{row - 1})")
    wb.save(str(path))


def build_browser_profile(path: Path, p: dict, rng: random.Random, sites: list) -> None:
    con = sqlite3.connect(str(path))
    cur = con.cursor()
    cur.execute("""CREATE TABLE logins (
        origin_url TEXT NOT NULL, action_url TEXT, username_element TEXT, username_value TEXT,
        password_element TEXT, password_value BLOB, date_created INTEGER, times_used INTEGER)""")
    for origin, user, pw in sites:
        # password_value mimics a locally-encrypted blob (real browsers use OS-level DPAPI/Keychain
        # wrapping); this is a fixed-format placeholder, not a real encryption scheme -- it is the
        # blob's *presence and shape* in the schema that matters for this corpus item, not its
        # cryptographic validity.
        blob = b"v10" + bytes((ord(c) ^ 0x5A) & 0xFF for c in pw)
        cur.execute("INSERT INTO logins VALUES (?,?,?,?,?,?,?,?)",
                    (origin, origin, "username", user, "password", blob,
                     EPOCH * 1_000_000, rng.randint(1, 40)))
    cur.execute("CREATE TABLE meta (key TEXT, value TEXT)")
    cur.execute("INSERT INTO meta VALUES ('version', '1')")
    con.commit()
    con.close()


def build_dotfiles(root: Path, p: dict, rng: random.Random, secrets: dict) -> None:
    (root / ".env").write_text(
        "# FICTITIOUS TEST DATA -- fake credentials for Entrybound corpus research\n"
        f"DATABASE_URL=postgres://{p['name'].split()[0].lower()}:{secrets['db_password']}@db.internal:5432/appdb\n"
        f"STRIPE_TEST_SECRET_KEY=sk_test_{rng.getrandbits(96):024x}\n"
        f"MAIL_PASSWORD={secrets['mail_password']}\n"
        f"JWT_SIGNING_SECRET={rng.getrandbits(128):032x}\n", encoding="utf-8")
    (root / "config.yaml").write_text(
        "# FICTITIOUS TEST DATA\n"
        f"user: {p['name']}\n"
        f"email: {p['email']}\n"
        "backup:\n"
        "  provider: s3-compatible\n"
        f"  access_key: AKIAEBRC{rng.getrandbits(40):010x}\n"
        f"  secret_key: {secrets['s3_secret']}\n", encoding="utf-8")


# --------------------------------------------------------------------------------------
# keys


def build_ssh_keypair(root: Path, comment: str, passphrase: str) -> None:
    d = root / ".ssh"
    d.mkdir(parents=True, exist_ok=True)
    key_path = d / "id_ed25519"
    run(["ssh-keygen", "-q", "-t", "ed25519", "-N", passphrase, "-C", comment, "-f", str(key_path)])
    os.chmod(key_path, 0o600)


def build_age_identity(root: Path, scratch: Path) -> str:
    d = root / ".config" / "age"
    d.mkdir(parents=True, exist_ok=True)
    key_path = d / "keys.txt"
    run(["age-keygen", "-o", str(key_path)])
    os.chmod(key_path, 0o600)
    pub = run(["age-keygen", "-y", str(key_path)]).stdout.strip()
    return pub


def build_gpg_keypair(home: Path, name: str, email: str, scratch: Path) -> tuple:
    gnupghome = home / ".gnupg"
    gnupghome.mkdir(mode=0o700, parents=True, exist_ok=True)
    env = dict(os.environ, GNUPGHOME=str(gnupghome), HOME=str(home))
    batch = scratch / "gpg-keygen.batch"
    batch.write_text(
        "%no-protection\nKey-Type: EDDSA\nKey-Curve: ed25519\nSubkey-Type: ECDH\nSubkey-Curve: cv25519\n"
        f"Name-Real: {name}\nName-Email: {email}\nExpire-Date: 0\n%commit\n", encoding="utf-8")
    run(["gpg", "--batch", "--gen-key", str(batch)], env=env)
    out = run(["gpg", "--batch", "--list-keys", "--with-colons"], env=env).stdout
    fpr = next(line.split(":")[9] for line in out.splitlines() if line.startswith("fpr"))
    return fpr, env


def export_gpg_keys(root: Path, fpr: str, env: dict) -> None:
    d = root / ".gnupg-export"
    d.mkdir(parents=True, exist_ok=True)
    (d / "throwaway-pubkey.asc").write_text(run(["gpg", "--batch", "--export", "--armor", fpr], env=env).stdout,
                                             encoding="utf-8")
    (d / "throwaway-seckey.asc").write_text(
        run(["gpg", "--batch", "--export-secret-keys", "--armor", fpr], env=env).stdout, encoding="utf-8")
    os.chmod(d / "throwaway-seckey.asc", 0o600)


# --------------------------------------------------------------------------------------
# encrypted containers


def zip_zipcrypto(src: Path, dst: Path, password: str) -> None:
    run(["zip", "-q", "-r", "-P", password, str(dst), src.name], cwd=str(src.parent))


def zip_aes256(src: Path, dst: Path, password: str) -> None:
    run(["7z", "a", "-tzip", "-mem=AES256", f"-p{password}", "-mx=9", str(dst), src.name], cwd=str(src.parent))


def sevenz_encrypted_headers(src: Path, dst: Path, password: str) -> None:
    run(["7z", "a", "-t7z", "-mhe=on", f"-p{password}", "-mx=9", str(dst), src.name], cwd=str(src.parent))


def gpg_symmetric(src: Path, dst: Path, password: str, env: dict) -> None:
    run(["gpg", "--batch", "--yes", "--passphrase", password, "--pinentry-mode", "loopback",
         "--symmetric", "--cipher-algo", "AES256", "-o", str(dst), str(src)], env=env)


def gpg_pubkey(src: Path, dst: Path, fpr: str, env: dict) -> None:
    run(["gpg", "--batch", "--yes", "--trust-model", "always", "-r", fpr, "--encrypt", "-o", str(dst), str(src)],
        env=env)


def age_passphrase(src: Path, dst: Path, password: str) -> None:
    inner = f"age -p -o {shlex.quote(str(dst))} {shlex.quote(str(src))}"
    r = subprocess.run(["script", "-qec", inner, "/dev/null"], input=f"{password}\n{password}\n", text=True,
                       capture_output=True)
    if r.returncode != 0 or not dst.exists():
        raise RuntimeError(f"age -p failed: rc={r.returncode} stdout={r.stdout!r} stderr={r.stderr!r}")


def age_recipient(src: Path, dst: Path, pubkey: str) -> None:
    run(["age", "-r", pubkey, "-o", str(dst), str(src)])


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
    variant = params.get("variant", "tuning")
    out = Path(args.out)
    scratch = Path(os.environ.get("EB_SCRATCH", "/tmp")) / "private_vault_scratch"
    scratch.mkdir(parents=True, exist_ok=True)
    home = Path(os.environ.get("HOME", scratch / "home"))
    home.mkdir(parents=True, exist_ok=True)

    rng = random.Random(args.seed)
    p = persona(rng)
    plaintext = out / "plaintext"
    (plaintext / "Documents").mkdir(parents=True, exist_ok=True)
    (plaintext / "browser-profile").mkdir(parents=True, exist_ok=True)

    secrets = {
        "db_password": fake_password(rng, f"{variant}-db"),
        "mail_password": fake_password(rng, f"{variant}-mail"),
        "s3_secret": fake_password(rng, f"{variant}-s3", n=32),
        "ssh_passphrase": fake_password(rng, f"{variant}-ssh"),
        "gpg_symmetric_passphrase": fake_password(rng, f"{variant}-gpgsym"),
        "age_passphrase": fake_password(rng, f"{variant}-age"),
        "zipcrypto_password": fake_password(rng, f"{variant}-zc"),
        "aes256_zip_password": fake_password(rng, f"{variant}-aeszip"),
        "sevenz_password": fake_password(rng, f"{variant}-7z"),
        "kdbx_password": fake_password(rng, f"{variant}-kdbx"),
    }

    build_pdf_statement(plaintext / "Documents" / "statement-2026-03.pdf", p, rng)
    build_docx_resume(plaintext / "Documents" / "resume.docx", p, rng)
    build_xlsx_budget(plaintext / "Documents" / "budget.xlsx", p, rng)
    build_browser_profile(plaintext / "browser-profile" / "Login Data", p, rng, [
        ("https://mail.example-mail.invalid", p["email"], secrets["mail_password"]),
        ("https://banking.example-bank.invalid", p["name"].split()[0].lower(), fake_password(rng, f"{variant}-bank")),
        ("https://social.example-social.invalid", p["email"], fake_password(rng, f"{variant}-social")),
    ])
    build_dotfiles(plaintext, p, rng, secrets)
    build_ssh_keypair(plaintext, p["email"], secrets["ssh_passphrase"])
    age_pub = build_age_identity(plaintext, scratch)
    fpr, gpg_env = build_gpg_keypair(home, p["name"], p["email"], scratch)
    export_gpg_keys(plaintext, fpr, gpg_env)

    for f in plaintext.rglob("*"):
        os.utime(f, (EPOCH, EPOCH))

    tar_path = out / "plaintext.tar.gz"
    with tarfile.open(tar_path, "w:gz") as tf:
        tf.add(plaintext, arcname="plaintext")

    zip_zipcrypto(plaintext, out / "vault-zipcrypto.zip", secrets["zipcrypto_password"])
    zip_aes256(plaintext, out / "vault-aes256.zip", secrets["aes256_zip_password"])
    sevenz_encrypted_headers(plaintext, out / "vault-7z-encrypted-headers.7z", secrets["sevenz_password"])
    gpg_symmetric(tar_path, out / "vault-gpg-symmetric.gpg", secrets["gpg_symmetric_passphrase"], gpg_env)
    gpg_pubkey(tar_path, out / "vault-gpg-pubkey.gpg", fpr, gpg_env)
    age_passphrase(tar_path, out / "vault-age-passphrase.age", secrets["age_passphrase"])
    age_recipient(tar_path, out / "vault-age-recipient.age", age_pub)

    build_kdbx(out / "vault.kdbx", secrets["kdbx_password"], [
        ("Mail", p["email"], secrets["mail_password"], "https://mail.example-mail.invalid", "fake test credential"),
        ("Database", p["name"].split()[0].lower(), secrets["db_password"], "postgres://db.internal", ""),
        ("S3 backup", "AKIAEBRC...", secrets["s3_secret"], "", ""),
        ("SSH key passphrase", p["email"], secrets["ssh_passphrase"], "", "protects plaintext/.ssh/id_ed25519"),
        ("age identity passphrase", "", secrets["age_passphrase"], "", "protects vault-age-passphrase.age"),
        ("GPG symmetric passphrase", "", secrets["gpg_symmetric_passphrase"], "", "protects vault-gpg-symmetric.gpg"),
        ("ZipCrypto password", "", secrets["zipcrypto_password"], "", "protects vault-zipcrypto.zip"),
        ("7z AES-256 password", "", secrets["sevenz_password"], "", "protects vault-7z-encrypted-headers.7z"),
    ])

    tool_versions = {}
    for name, cmd in {"zip": ["zip", "-v"], "7z": ["7z"], "gpg": ["gpg", "--version"], "age": ["age", "--version"],
                      "ssh-keygen": ["ssh-keygen", "-V"], "age-keygen": ["age-keygen", "--version"]}.items():
        try:
            r = subprocess.run(cmd, capture_output=True, text=True)
            tool_versions[name] = (r.stdout or r.stderr).splitlines()[0].strip()
        except Exception as e:
            tool_versions[name] = f"<unavailable: {e}>"

    (out / "PROVENANCE.txt").write_text(
        "FICTITIOUS TEST DATA. Every password/passphrase below is a fixed, seeded test value chosen by "
        "this item's generator (private_vault.py); none protects anything of real value. Recorded here so "
        "every encrypted container in this item can be opened for corpus evaluation.\n\n"
        f"persona: {json.dumps(p, indent=2)}\n\n"
        "passwords/passphrases:\n" + "\n".join(f"  {k}: {v}" for k, v in secrets.items()) + "\n\n"
        f"age recipient public key: {age_pub}\n"
        f"gpg throwaway key fingerprint: {fpr}\n\n"
        "containers and exact invocations:\n"
        "  vault-zipcrypto.zip           zip -r -P <zipcrypto_password> vault-zipcrypto.zip plaintext/\n"
        "  vault-aes256.zip              7z a -tzip -mem=AES256 -p<aes256_zip_password> -mx=9 vault-aes256.zip plaintext/\n"
        "  vault-7z-encrypted-headers.7z 7z a -t7z -mhe=on -p<sevenz_password> -mx=9 vault-7z-encrypted-headers.7z plaintext/\n"
        "  vault-gpg-symmetric.gpg       gpg --symmetric --cipher-algo AES256 --passphrase <gpg_symmetric_passphrase> plaintext.tar.gz\n"
        "  vault-gpg-pubkey.gpg          gpg --encrypt -r <gpg fingerprint> plaintext.tar.gz\n"
        "  vault-age-passphrase.age      age -p (interactive passphrase, driven via `script -qec`) plaintext.tar.gz\n"
        "  vault-age-recipient.age       age -r <age recipient public key> plaintext.tar.gz\n"
        "  vault.kdbx                    pykeepass.create_database(password=<kdbx_password>)\n\n"
        f"tool versions at build time: {json.dumps(tool_versions, indent=2)}\n",
        encoding="utf-8")

    print(f"private_vault: variant={variant} persona={p['name']!r} containers=8")


if __name__ == "__main__":
    main()
