#!/usr/bin/env python3
# =============================================================================
# EH3D-LASER-CAM - tools/ci_validate.py
# Copyright (C) 2026 JuanenRac (Electro Hobby 3D) <electrohobby3d@gmail.com>
# GPL-3.0-or-later - see LICENSE
# =============================================================================
# canonical: EH3D-DOCS tools 1
"""The repository checks that CI runs besides cargo: manifest, versions, licence, changelog, the eight README files
and the words that must never appear in public files. Prints CI_VALIDATION=PASS or FAIL with the reasons."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLS_VERSION = "1"  # raise it with every change of this file; the header of every copy says which one it holds
DEFAULT_SKIP_DIRS = {"target", ".git"}  # folders the scan for forbidden words never enters; a repository adds its own in tools/ci_config.json
REQUIRED = ["schema_version", "ecosystem", "name", "version", "role", "stack", "technologies", "deployment_target",
            "maturity", "family", "parent", "build", "notes", "native_version"]
LANGUAGES = {"README.md": "en", "README_spa.md": "es", "README_deu.md": "de", "README_fra.md": "fr",
             "README_ita.md": "it", "README_jpn.md": "ja", "README_zho.md": "zh", "README_por.md": "pt"}
FORBIDDEN = re.compile(r"\b(claude|anthropic|sonnet)\b", re.IGNORECASE)
TEXT_SUFFIXES = {".md", ".rs", ".toml", ".json", ".yml", ".yaml", ".py", ".txt", ".svg"}


def headings(text: str) -> list[int]:
    """The level of every heading outside code blocks, in order: the structure all eight READMEs share."""
    levels, fenced = [], False
    for line in text.splitlines():
        if line.lstrip().startswith("```"):
            fenced = not fenced
        elif not fenced and re.match(r"#{1,6} ", line):
            levels.append(len(line) - len(line.lstrip("#")))
    return levels



BUILD_FOLDERS = {".compile", "build", "dist", "target", "node_modules", ".venv", "venv", "__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache",
                 "managed_components", ".gradle", ".cxx", ".idea", ".vscode"}
PRIVATE_FOLDERS = {"local-data", "re", "secrets"}
PRIVATE_FILES = re.compile(r"(^|/)(\.env(\.(?!example$).*)?|id_[a-z0-9]+|\.coverage|sdkconfig(\.old)?|Thumbs\.db|\.DS_Store)$|\.(pem|key|p12|pfx|jks|keystore|log|tmp|pyc)$", re.IGNORECASE)
MAX_TRACKED_BYTES = 1_500_000


def hygiene_problems() -> list[str]:
    """Build folders, private data, keys, logs and big files must never be tracked by git."""
    import subprocess

    try:
        out = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout
    except (OSError, subprocess.CalledProcessError):
        return []
    found: list[str] = []
    folders: dict[str, int] = {}
    for raw in out.split(b"\0"):
        if not raw:
            continue
        name = raw.decode("utf-8", "replace")
        parts = name.split("/")
        bad = next((p for p in parts[:-1] if p in BUILD_FOLDERS), None) or (parts[0] if len(parts) > 1 and parts[0] in PRIVATE_FOLDERS else None)
        if bad:
            folders[bad] = folders.get(bad, 0) + 1
            continue
        if PRIVATE_FILES.search(name):
            found.append(f"{name} is a private or local file and must not be tracked")
        path = ROOT / name
        if path.is_file() and path.stat().st_size > MAX_TRACKED_BYTES and path.suffix.lower() not in {".png", ".jpg", ".svg", ".pdf", ".stl", ".step"}:
            found.append(f"{name} is bigger than {MAX_TRACKED_BYTES // 1000} KB: build output does not belong in the repository")
    for folder, count in sorted(folders.items()):
        found.append(f"{folder}/ is a build or private folder and {count} of its files are tracked")
    return found

STATUS_LIGHTS = ("🟢", "🟡", "🟠", "🔴")


def status_lights(text: str) -> list[str] | None:
    """The lights of the "Project status" section of a README, in order, or None when the section is missing or malformed.

    The section is the second `## ` heading (right after "At a glance"), has the legend with the four lights and a
    table whose State column holds only a light."""
    lines, fenced, heads = text.splitlines(), False, []
    for index, line in enumerate(lines):
        if line.lstrip().startswith("```"):
            fenced = not fenced
        elif not fenced and line.startswith("## "):
            heads.append(index)
    if len(heads) < 3 or not lines[heads[1]].startswith("## 🚦 "):
        return None
    block = lines[heads[1]:heads[2]]
    if not all(light in " ".join(block) for light in STATUS_LIGHTS):
        return None
    rows = [line for line in block if line.startswith("|")][2:]
    lights = []
    for row in rows:
        cells = [part.strip() for part in re.split(r"(?<!\\)\|", row.strip().strip("|"))]
        if len(cells) != 3 or cells[1] not in STATUS_LIGHTS or not cells[0] or not cells[2]:
            return None
        lights.append(cells[1])
    return lights if len(lights) >= 3 else None


TREE_EXEMPT = re.compile(
    r"^(README(_[a-z]{3})?\.md|LICENSE(\.md)?|CHANGELOG\.md|CODE_OF_CONDUCT\.md|CONTRIBUTING\.md|SECURITY\.md|SUPPORT\.md|VERSION"
    r"|eh3d\.project\.json|images|\..*|build.*|run.*|test[.](bat|sh)|dev.*|bump.*|.*\.lock|Cargo\.toml|package(-lock)?\.json|pyproject\.toml|.*\.toml)$"
)
TREE_ROW = re.compile(r"^[├└]── ([^\s,]+(?:(?:, | / | \+ )[^\s,]+)*)")


def tree_names(text: str) -> set[str] | None:
    """The first-level names that the file tree of a README lists (the code block with the most tree rows), or None without a tree."""
    best, best_rows = "", 0
    for block in re.finditer(r"```[a-z]*\r?\n(.*?)```", text, re.S):
        rows = len(re.findall(r"^[├└]── ", block.group(1), re.M))
        if rows > best_rows:
            best, best_rows = block.group(1), rows
    if not best_rows:
        return None
    names: set[str] = set()
    for line in best.splitlines():
        found = TREE_ROW.match(line)
        if not found:
            continue
        previous = ""
        for token in re.split(r", | / | \+ ", found.group(1)):
            for part in re.split(r"/(?=\.)", token):
                part = part.rstrip("/")
                if part.startswith(".") and "." in previous.lstrip("."):
                    part = previous.rsplit(".", 1)[0] + part
                if part.split("/")[0]:
                    names.add(part.split("/")[0])
                previous = part
    return names


def tree_problems(tracked: list[str] | None = None) -> list[str]:
    """Every versioned first-level entry must appear in the file tree of each README (the structure that the READMEs describe must not go stale)."""
    import fnmatch
    import subprocess

    if tracked is None:
        try:
            out = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT, capture_output=True, check=True).stdout
        except (OSError, subprocess.CalledProcessError):
            return []
        tracked = sorted({item.split("/")[0] for item in out.decode("utf-8", "replace").split(chr(0)) if item})
    problems = []
    for name in LANGUAGES:
        path = ROOT / name
        if not path.exists():
            continue
        listed = tree_names(path.read_text(encoding="utf-8"))
        if listed is None:
            continue
        missing = [entry for entry in tracked if not TREE_EXEMPT.match(entry) and not any(fnmatch.fnmatchcase(entry, pattern) for pattern in listed)]
        if missing:
            problems.append(f"{name} file tree does not list: {', '.join(missing)}")
    return problems


def _universal_readme_ci(root: Path) -> list[str]:
    """The layout of the README (22 sections in a fixed order, in the eight languages): a warning until the family has migrated; see tools/universal_readme_check.py."""
    try:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import universal_readme_check
    except ImportError:
        return []
    return universal_readme_check.run_in_ci(root)


def _status_lights_ci(root: Path) -> list[str]:
    """The lights of the Project status table stay inside what the manifest (maturity, evidence) allows; see tools/status_lights.py."""
    try:
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import status_lights as lights_tool
    except ImportError:
        return []
    return lights_tool.run_in_ci(root)



def _depends_on_problems(root: Path, manifest: dict) -> list[str]:
    """The sibling repositories that a Cargo.toml reaches with a path are the ones the manifest names in depends_on (schema 1.1)."""
    found: set[str] = set()
    for cargo in root.rglob("Cargo.toml"):
        if any(part in ("target", ".git", "node_modules") for part in cargo.parts):
            continue
        found.update(re.findall(r'path\s*=\s*"(?:\.\./)+(EH3D-[A-Za-z0-9-]+)', cargo.read_text(encoding="utf-8")))
    found.discard(manifest.get("name"))
    declared = set(manifest.get("depends_on") or [])
    out = []
    if found != declared:
        out.append(f"depends_on {sorted(declared)} does not match the sibling repositories that Cargo.toml reaches by path {sorted(found)}")
    if declared and manifest.get("schema_version") != "1.1":
        out.append("depends_on needs schema_version 1.1")
    return out


def load_config(root: Path) -> tuple[dict, list[str]]:
    """The data of this repository (tools/ci_config.json, optional): {"skip_dirs": [...]} adds folders the scan for forbidden words skips."""
    path = root / "tools" / "ci_config.json"
    if not path.is_file():
        return {}, []
    try:
        config = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as error:
        return {}, [f"tools/ci_config.json is not valid JSON: {error}"]
    if not isinstance(config, dict):
        return {}, ["tools/ci_config.json must hold an object"]
    skip = config.get("skip_dirs", [])
    if not isinstance(skip, list) or not all(isinstance(item, str) for item in skip):
        return {}, ["tools/ci_config.json: skip_dirs must be a list of folder names"]
    return config, []


def skip_dirs(config: dict) -> set[str]:
    return DEFAULT_SKIP_DIRS | set(config.get("skip_dirs", []))


def header_version(text: str) -> str | None:
    """The version in the `# canonical: EH3D-DOCS tools N` line of the header of a copy, or None when it has none."""
    for line in text.replace("\r\n", "\n").split("\n"):
        if not line.startswith("#"):
            break
        found = re.match(r"# canonical: EH3D-DOCS tools (\S+)\s*$", line)
        if found:
            return found.group(1)
    return None


def header_problems(path: Path | None = None) -> list[str]:
    """The header of this file has to carry the same tools version as the code."""
    path = path or Path(__file__).resolve()
    stated = header_version(path.read_text(encoding="utf-8"))
    if stated != TOOLS_VERSION:
        return [f"{path.name} header says canonical tools {stated}, the code is {TOOLS_VERSION}: run tools/sync_shared_tools.py --write in EH3D-DOCS"]
    return []


def native_version(root: Path, native: dict) -> tuple[str, str]:
    """The version the code of the repository declares, read where the manifest says (native_version.file and .pattern), and the name of that file.

    The pattern can be one regex (its groups are joined with dots, empty ones left out) or an object with major/minor/patch(/build) regexes."""
    name = native.get("file", "pyproject.toml")
    try:
        source = (root / name).read_text(encoding="utf-8", errors="replace")
        pattern = native.get("pattern", r'(?m)^version\s*=\s*"([^"]+)"')
        if isinstance(pattern, dict):
            parts = []
            for key in ("major", "minor", "patch", "build"):
                if key in pattern and (found := re.search(pattern[key], source, re.MULTILINE)):
                    parts.append(found.group(1))
            return ".".join(parts) if len(parts) >= 3 else "?", name
        match = re.search(pattern, source, re.MULTILINE)
        return (".".join(group for group in match.groups() if group) if match else "?"), name
    except (OSError, re.error):
        return "?", name


def project_yml_problems(root: Path, version: str) -> list[str]:
    """An iOS project (project.yml): the marketing version is the one of the manifest and the bundle shows it, not a literal."""
    path = root / "project.yml"
    if not path.is_file():
        return []
    text = path.read_text(encoding="utf-8")
    out = []
    marketing = re.search(r'MARKETING_VERSION:\s*"([^"]+)"', text)
    if not marketing or marketing.group(1) != version:
        out.append(f"project.yml MARKETING_VERSION ({marketing.group(1) if marketing else 'none'}) is not the version {version}")
    if 'CFBundleShortVersionString: "$(MARKETING_VERSION)"' not in text:
        out.append("project.yml must give CFBundleShortVersionString as $(MARKETING_VERSION), not a literal")
    return out


def main() -> int:
    problems: list[str] = header_problems()
    config, config_problems = load_config(ROOT)
    problems.extend(config_problems)
    skipped = skip_dirs(config)
    manifest = json.loads((ROOT / "eh3d.project.json").read_text(encoding="utf-8"))
    for key in REQUIRED:
        if key not in manifest:
            problems.append(f"eh3d.project.json lacks '{key}'")
    version = manifest.get("version", "?")
    native = manifest.get("native_version", {})
    found, native_file = native_version(ROOT, native if isinstance(native, dict) else {})
    if found != version:
        problems.append(f"{native_file} version {found} differs from the manifest {version}")
    problems.extend(project_yml_problems(ROOT, version))
    if manifest.get("name") != ROOT.name and ROOT.name.startswith("EH3D"):
        problems.append(f"the manifest name {manifest.get('name')} differs from the folder {ROOT.name}")
    if "GNU GENERAL PUBLIC LICENSE" not in (ROOT / "LICENSE").read_text(encoding="utf-8", errors="replace")[:200]:
        problems.append("LICENSE is not the GPL text")
    changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
    top = re.search(r"(?m)^## \[(\d[^\]]*)\]", changelog)
    if not top or top.group(1) != version:
        problems.append(f"the newest CHANGELOG entry ({top.group(1) if top else 'none'}) is not the version {version}")
    english = headings((ROOT / "README.md").read_text(encoding="utf-8"))
    english_lights = status_lights((ROOT / "README.md").read_text(encoding="utf-8"))
    for name in LANGUAGES:
        path = ROOT / name
        if not path.exists():
            problems.append(f"{name} is missing")
            continue
        text = path.read_text(encoding="utf-8")
        if headings(text) != english:
            problems.append(f"{name} does not have the heading structure of README.md")
        lights = status_lights(text)
        if lights is None:
            problems.append(f"{name} has no valid 'Project status' section right after 'At a glance' (a table of lights, one row per part)")
        elif lights != english_lights:
            problems.append(f"{name} does not show the same lights as README.md in its 'Project status' table")
        if f"`{version}`" not in text:
            problems.append(f"{name} does not show the version {version}")
        for other in LANGUAGES:
            if other != name and other not in text:
                problems.append(f"{name} does not link to {other} in its language bar")
    for path in ROOT.rglob("*"):
        if not path.is_file() or skipped & set(path.relative_to(ROOT).parts) or path.suffix not in TEXT_SUFFIXES:
            continue
        if path.name == "ci_validate.py":
            continue
        if FORBIDDEN.search(path.read_text(encoding="utf-8", errors="replace")):
            problems.append(f"{path.relative_to(ROOT)} mentions a word that must not appear in public files")
    problems.extend(hygiene_problems())
    problems.extend(tree_problems())
    problems.extend(_universal_readme_ci(ROOT))
    problems.extend(_status_lights_ci(ROOT))
    problems.extend(_depends_on_problems(ROOT, manifest))
    if problems:
        for problem in problems:
            print("  FAIL", problem)
        print(f"CI_VALIDATION=FAIL project={manifest.get('name')} version={version} tools={TOOLS_VERSION}")
        return 1
    print(f"CI_VALIDATION=PASS project={manifest.get('name')} version={version} tools={TOOLS_VERSION}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
