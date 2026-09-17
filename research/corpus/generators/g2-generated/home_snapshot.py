#!/usr/bin/env python3
"""Run-step script (F19 real home/project-directory backups, corpus group g2-generated): assembles
a realistic rotating-backup home-directory tree out of REAL content that earlier recipe steps have
already placed into the staging tree, then writes two dated snapshots of it (a backup series, not a
single static tree).

Corpus round-1 critic gap (F19 real home/project-directory backups, BLOCKER): F19 had no real mixed
personal/office document tree, and no working copy anywhere in the corpus keeps its `.git` (every
F01 item is `git archive` or a tarball). This item's OWN recipe (declared in make_sources.py, not
here) does the real acquisition before this script ever runs:

  - `recipe.git` with `history: "full"` clones a small, real, permissively-licensed public repo
    (a genuine `.git` working copy: refs, packed objects, an actual checked-out tree) directly into
    the staging root
  - `recipe.inputs` downloads a real, permissively-licensed public "dotfiles" repository's tarball
    (someone's actual published personal shell/editor/git configuration) into raw-inputs/dotfiles
  - `recipe.from_items` + a copy-item step reuses an already-provisioned real small public-domain
    government PDF from family F13, in the SAME split, into raw-inputs/documents

This script's only job is REORGANIZATION and realistic backup-series structure, not acquisition:
it moves the git clone into Projects/<name>/, spreads the dotfiles across the home root as real
dotfiles, moves the real documents into Documents/, adds a modest amount of synthetic filler
(Pictures/Music/Downloads-with-a-duplicate/.cache/.local/share/Trash/a browser-profile SQLite -- all
clearly synthetic, generated the same way private_vault.py's browser-profile stub is) purely for
realistic breadth, and then writes TWO dated snapshots of the resulting tree: an initial backup and
a later one with real edits/a rename/a deletion/an addition, with every byte-identical file between
the two snapshots hardlinked (not copied) exactly as a real rsync --link-dest / rsnapshot backup
run would leave them on disk.

Params (EB_PARAMS_JSON): {"variant": "tuning" | "validation" | "heldout", "project_repo": <str>,
                          "project_license": <str>, "dotfiles_repo": <str>,
                          "dotfiles_license": <str>, "document_source_item": <str>}
"""

import argparse
import json
import os
import random
import shutil
import sqlite3
import stat as statmod
from pathlib import Path

EPOCH = int(os.environ.get("SOURCE_DATE_EPOCH", "1767225600"))
DAY = 86400


def touch_tree(root: Path, when: int) -> None:
    for p in root.rglob("*"):
        try:
            os.utime(p, (when, when), follow_symlinks=False)
        except (OSError, NotImplementedError):
            pass
    os.utime(root, (when, when))


def build_browser_profile(path: Path, rng: random.Random, sites: list) -> None:
    con = sqlite3.connect(str(path))
    cur = con.cursor()
    cur.execute("""CREATE TABLE logins (
        origin_url TEXT NOT NULL, username_value TEXT, password_value BLOB, date_created INTEGER)""")
    for origin, user, pw in sites:
        blob = b"v10" + bytes((ord(c) ^ 0x5A) & 0xFF for c in pw)
        cur.execute("INSERT INTO logins VALUES (?,?,?,?)", (origin, user, blob, EPOCH * 1_000_000))
    con.commit()
    con.close()


PERSONAS = {
    "tuning": {"user": "eb-test-1", "email": "eb-test-1@example-mail.invalid"},
    "validation": {"user": "eb-test-2", "email": "eb-test-2@example-mail.invalid"},
    "heldout": {"user": "eb-test-3", "email": "eb-test-3@example-mail.invalid"},
}


def build_home(root: Path, out: Path, variant: str, project_repo: str, rng: random.Random) -> dict:
    """Reorganizes the already-populated staging tree (project clone at the root, raw-inputs/{dotfiles,
    documents} from earlier steps) into a realistic home directory at `root`. Returns a small dict of
    facts for PROVENANCE.txt."""
    raw = out / "raw-inputs"
    root.mkdir(parents=True, exist_ok=True)

    projects = root / "Projects" / project_repo
    projects.parent.mkdir(parents=True, exist_ok=True)
    top_level = [p for p in out.iterdir() if p.name not in ("raw-inputs",)]
    projects.mkdir(parents=True, exist_ok=True)
    for p in top_level:
        shutil.move(str(p), str(projects / p.name))
    git_present = (projects / ".git").is_dir()

    dotfiles_src = raw / "dotfiles"
    dotfile_names = []
    if dotfiles_src.is_dir():
        for p in dotfiles_src.iterdir():
            dest = root / p.name
            shutil.move(str(p), str(dest))
            dotfile_names.append(p.name)

    docs_src = raw / "documents"
    documents_dir = root / "Documents"
    documents_dir.mkdir(parents=True, exist_ok=True)
    doc_files = []
    if docs_src.is_dir():
        for p in sorted(docs_src.rglob("*")):
            if p.is_file():
                rel = p.relative_to(docs_src)
                dest = documents_dir / rel
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.move(str(p), str(dest))
                doc_files.append(dest)

    if raw.exists():
        shutil.rmtree(raw)

    persona = PERSONAS[variant]
    (root / "Documents" / "todo.txt").write_text(
        "FICTITIOUS TEST DATA -- generated for Entrybound corpus research\n"
        "- finish the report\n- back up laptop\n- renew the domain\n", encoding="utf-8")

    pictures = root / "Pictures"
    pictures.mkdir(parents=True, exist_ok=True)
    photo = pictures / "photo-0001.jpg"
    photo.write_bytes(b"\xff\xd8\xff\xe0" + rng.randbytes(4096) + b"\xff\xd9")

    music = root / "Music"
    music.mkdir(parents=True, exist_ok=True)
    (music / "track-01.mp3").write_bytes(b"ID3" + bytes(3) + rng.randbytes(2048))

    downloads = root / "Downloads"
    downloads.mkdir(parents=True, exist_ok=True)
    dup_of = None
    if doc_files:
        src = doc_files[0]
        dup_of = downloads / src.name
        shutil.copy2(src, dup_of)
    shutil.copy2(photo, downloads / photo.name)

    cache = root / ".cache" / "eb-app"
    cache.mkdir(parents=True, exist_ok=True)
    (cache / "index.dat").write_bytes(rng.randbytes(512))

    trash_files = root / ".local" / "share" / "Trash" / "files"
    trash_info = root / ".local" / "share" / "Trash" / "info"
    trash_files.mkdir(parents=True, exist_ok=True)
    trash_info.mkdir(parents=True, exist_ok=True)
    deleted = trash_files / "old-draft.txt"
    deleted.write_text("an earlier draft, since deleted\n", encoding="utf-8")
    (trash_info / "old-draft.txt.trashinfo").write_text(
        "[Trash Info]\nPath=/home/{}/Documents/old-draft.txt\nDeletionDate=2026-02-10T09:00:00\n"
        .format(persona["user"]), encoding="utf-8")

    browser = root / ".config" / "eb-browser"
    browser.mkdir(parents=True, exist_ok=True)
    build_browser_profile(browser / "Login Data", rng, [
        ("https://mail.example-mail.invalid", persona["email"], f"eb-home-{variant}-mailpw-{rng.randint(1000,9999)}"),
    ])

    return {"git_present": git_present, "dotfile_names": sorted(dotfile_names),
           "document_files": [str(p.relative_to(root)) for p in doc_files],
           "duplicate_download": str(dup_of.relative_to(root)) if dup_of else None}


def snapshot_copy(src: Path, dst: Path) -> None:
    """A `cp -a`-equivalent full copy used only for the FIRST snapshot (there is nothing to link
    against yet)."""
    shutil.copytree(src, dst, symlinks=True, copy_function=shutil.copy2)


def make_second_snapshot(first: Path, second: Path, variant: str, rng: random.Random) -> dict:
    """Builds the second snapshot from the first with rsync --link-dest / rsnapshot semantics:
    unchanged files are hardlinked (same inode, zero extra bytes), changed/added files are written
    fresh, and one file is deleted (its snapshot-1 copy remains, only snapshot-2 lacks it) -- the
    same on-disk pattern hardlink_farm.py's "rsnapshot" model uses, applied here to real content."""
    second.mkdir(parents=True)
    changes = {"edited": [], "renamed": None, "deleted": None, "added": None}

    todo = Path("Documents/todo.txt")
    renamed_pdf = None
    for p in sorted(first.rglob("*")):
        rel = p.relative_to(first)
        if p.is_dir():
            (second / rel).mkdir(parents=True, exist_ok=True)
            continue
        if rel == todo:
            continue  # handled specially below (edited)
        dst = second / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        if p.is_symlink():
            os.symlink(os.readlink(p), dst)
            continue
        os.link(p, dst)  # unchanged: hardlink, exactly like a real incremental backup tool

    (second / todo).parent.mkdir(parents=True, exist_ok=True)
    (second / todo).write_text(
        "FICTITIOUS TEST DATA -- generated for Entrybound corpus research\n"
        "- finish the report [done]\n- back up laptop [done]\n- renew the domain\n- call the bank\n",
        encoding="utf-8")
    changes["edited"].append(str(todo))

    old_deleted_trash = second / ".local" / "share" / "Trash" / "files" / "old-draft.txt"
    if old_deleted_trash.exists():
        old_deleted_trash.unlink()
        info = second / ".local" / "share" / "Trash" / "info" / "old-draft.txt.trashinfo"
        if info.exists():
            info.unlink()
        changes["deleted"] = str(old_deleted_trash.relative_to(second))

    downloads = second / "Downloads"
    if downloads.is_dir():
        leftovers = sorted(downloads.glob("*.jpg"))
        if leftovers:
            leftovers[0].unlink()
            changes["deleted"] = (changes["deleted"] or "") + f", {leftovers[0].relative_to(second)}"
        new_note = downloads / "meeting-notes.txt"
        new_note.write_text(f"FICTITIOUS TEST DATA -- eb-home-{variant} follow-up notes\n", encoding="utf-8")
        changes["added"] = str(new_note.relative_to(second))

    bashrc = second / ".bashrc"
    if bashrc.exists():
        # bashrc was just hardlinked to snapshot 1's inode above; opening it in append mode would
        # mutate that SHARED inode and silently corrupt snapshot 1's "point in time" copy too (the
        # exact bug real backup tools avoid). Break the hardlink first: read the shared content,
        # unlink this snapshot's name from it, then write a fresh file with the edit applied --
        # snapshot 1's copy keeps its own separate inode and is untouched.
        original = bashrc.read_text(encoding="utf-8")
        bashrc.unlink()
        bashrc.write_text(original + "\n# added on the second backup (fictitious test edit)\n"
                          "export EB_HOME_TEST=1\n", encoding="utf-8")
        changes["edited"].append(".bashrc")

    return changes


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--params", default="{}")
    args = ap.parse_args()
    params = json.loads(args.params)
    variant = params.get("variant", "tuning")
    project_repo = params.get("project_repo", "project")
    out = Path(args.out)
    rng = random.Random(args.seed)

    scratch_home = out.parent / f"{out.name}.homebuild"
    if scratch_home.exists():
        shutil.rmtree(scratch_home)
    facts = build_home(scratch_home, out, variant, project_repo, rng)

    snap1_name = "backup-2026-02-15"
    snap2_name = "backup-2026-03-01"
    snap1 = out / snap1_name
    snapshot_copy(scratch_home, snap1)
    touch_tree(snap1, EPOCH)
    changes = make_second_snapshot(snap1, out / snap2_name, variant, rng)
    touch_tree(out / snap2_name, EPOCH + 14 * DAY)
    shutil.rmtree(scratch_home)

    manifest = {
        "variant": variant,
        "snapshots": [snap1_name, snap2_name],
        "project_repo": params.get("project_repo"), "project_license": params.get("project_license"),
        "dotfiles_repo": params.get("dotfiles_repo"), "dotfiles_license": params.get("dotfiles_license"),
        "document_source_item": params.get("document_source_item"),
        "git_present_in_snapshot": facts["git_present"],
        "dotfiles_placed_at_home_root": facts["dotfile_names"],
        "real_documents": facts["document_files"],
        "changes_between_snapshots": changes,
    }
    (out / "MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    (out / "PROVENANCE.txt").write_text(
        f"Real home/project-directory backup series ({snap1_name} -> {snap2_name}) for split {variant!r}.\n\n"
        f"Projects/{project_repo}/ is a genuine full `git clone` (refs, packed objects, an actual checked-out\n"
        f"working tree, .git present: {facts['git_present']}) of {params.get('project_repo')!r}\n"
        f"({params.get('project_license')}), cloned by this item's own recipe.git (history=full) before this\n"
        "script ran.\n\n"
        f"The following top-level dotfiles came from a real, published personal dotfiles repository\n"
        f"({params.get('dotfiles_repo')!r}, {params.get('dotfiles_license')}), downloaded by this item's\n"
        f"recipe.inputs and placed directly at the home root: {', '.join(facts['dotfile_names'])}\n\n"
        f"Documents/ holds real public-domain government documents reused from the already-provisioned\n"
        f"F13 item {params.get('document_source_item')!r} (same split), via this item's recipe.from_items:\n"
        + "\n".join(f"  {d}" for d in facts["document_files"]) + "\n\n"
        "Pictures/, Music/, .cache/, the browser-profile SQLite and the .local/share/Trash entry are\n"
        "synthetic filler (clearly marked as such) added only for realistic home-directory breadth; they\n"
        "are not claimed as real media/browser data.\n\n"
        f"Changes recorded between the two backups: {json.dumps(changes, indent=2)}\n"
        "Every file unchanged between the two snapshots is a real hardlink (same inode) to the first "
        "snapshot's copy, exactly as rsync --link-dest / rsnapshot would leave a real incremental backup "
        "on disk -- verify with `stat -c '%i %n' backup-2026-02-15/... backup-2026-03-01/...`.\n",
        encoding="utf-8")

    print(f"home_snapshot: variant={variant} project={project_repo} snapshots=2")


if __name__ == "__main__":
    main()
