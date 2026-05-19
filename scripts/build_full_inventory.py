import os, re, json, hashlib
from pathlib import Path
from collections import defaultdict

ROOTS = [
    "/tmp/trios-work",      # trios repo (contains docs/phd/theorems, trinity-clara/proofs, proofs, crates)
    "/tmp/t27-clone",       # t27 repo (contains proofs/, coq/)
]

EXCLUDE_PARTS = {".git", "node_modules", "target", "_build"}
# Verilog .v files we should NOT count as Coq:
VERILOG_HINTS = ("verilog", "fpga", "/src/phi-engine/", "ternary_ops.v")

# Theorem-like declarations (Coq + a few standard variants)
RX_THM = re.compile(
    r"^\s*(Theorem|Lemma|Proposition|Corollary|Fact|Remark|Example|Definition\s+\w+\s*:?=\s*)\s+(\w+)",
    re.MULTILINE,
)
RX_QED = re.compile(r"^\s*Qed\.", re.MULTILINE)
RX_ADM = re.compile(r"^\s*Admitted\.", re.MULTILINE)
RX_AX  = re.compile(r"^\s*(Axiom|Parameter)\s+(\w+)", re.MULTILINE)

inventory = []
seen_hashes = {}  # sha1(content) -> first_path

def is_verilog_file(p: str) -> bool:
    pl = p.lower()
    if any(h in pl for h in VERILOG_HINTS):
        return True
    # heuristic: open and see if it has `module` and no `Theorem`
    try:
        with open(p, "r", errors="replace") as f:
            head = f.read(4000)
        if "module " in head and "Theorem" not in head and "Lemma" not in head:
            return True
    except Exception:
        pass
    return False

for root in ROOTS:
    for dirpath, dirnames, fnames in os.walk(root):
        # prune
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_PARTS]
        for fn in fnames:
            if not fn.endswith(".v"):
                continue
            full = os.path.join(dirpath, fn)
            try:
                with open(full, "r", errors="replace") as f:
                    content = f.read()
            except Exception:
                continue
            is_verilog = is_verilog_file(full)
            kind = "verilog" if is_verilog else "coq"
            h = hashlib.sha1(content.encode("utf-8", errors="replace")).hexdigest()[:16]
            thms = [m.group(2) for m in RX_THM.finditer(content) if m.group(1).strip().startswith(("Theorem","Lemma","Proposition","Corollary","Fact"))]
            qeds = len(RX_QED.findall(content))
            adm  = len(RX_ADM.findall(content))
            ax   = len(RX_AX.findall(content))
            rel = full
            for r in ROOTS:
                if full.startswith(r):
                    rel = os.path.relpath(full, r)
                    break
            inventory.append({
                "abs_path": full,
                "rel_path": rel,
                "repo": "trios" if root == ROOTS[0] else "t27",
                "kind": kind,
                "sha1": h,
                "size": len(content),
                "theorems": thms,
                "theorems_count": len(thms),
                "qed_count": qeds,
                "admitted_count": adm,
                "axiom_count": ax,
            })

# Now dedup by content hash (same file in two roots = one logical file)
dedup = {}
for it in inventory:
    key = it["sha1"]
    if key not in dedup:
        dedup[key] = it
    else:
        # merge: keep all rel paths
        existing = dedup[key]
        existing.setdefault("also_at", []).append(f"{it['repo']}:{it['rel_path']}")

unique = list(dedup.values())

# Bucket assignment
def bucket(it):
    p = it["rel_path"].replace("\\","/").lower()
    if it["kind"] == "verilog":
        return "Verilog"
    if "/trinity/" in p or p.startswith("trinity/"):
        return "Trinity"
    if "/igla/" in p:
        return "IGLA"
    if "/kernel/" in p:
        return "Kernel"
    if "/theorems/" in p and "/phd/theorems/" not in p:
        return "Theorems"
    if "/sacred/" in p:
        return "Physics_Sacred"
    if "/gravity/" in p:
        return "Physics_Gravity"
    if "/bounds" in p or "_bounds" in p:
        return "Bounds"
    if "phi" in p.split("/")[-1].lower():
        return "Core_Phi"
    if "kat_vsa" in p.lower():
        return "Bridge"
    return "Other"

# Verilog .v files actually
for it in unique:
    it["bucket"] = bucket(it)

buckets = defaultdict(list)
for it in unique:
    buckets[it["bucket"]].append(it)

# Summary
total_coq = [it for it in unique if it["kind"]=="coq"]
total_ver = [it for it in unique if it["kind"]=="verilog"]
print(f"Unique .v files: {len(unique)} (Coq={len(total_coq)}, Verilog={len(total_ver)})")
print(f"Total raw files (before dedup): {len(inventory)}")
print()
print(f"{'Bucket':<22} {'Files':>5} {'Theorems':>10} {'Qed':>5} {'Adm':>5} {'Ax':>5}")
print("-"*60)
for b in sorted(buckets):
    files = buckets[b]
    thms = sum(it["theorems_count"] for it in files)
    qed  = sum(it["qed_count"]      for it in files)
    adm  = sum(it["admitted_count"] for it in files)
    ax   = sum(it["axiom_count"]    for it in files)
    print(f"{b:<22} {len(files):>5} {thms:>10} {qed:>5} {adm:>5} {ax:>5}")

print()
print("TOTAL Coq theorems:", sum(it["theorems_count"] for it in total_coq))
print("TOTAL Qed:         ", sum(it["qed_count"]      for it in total_coq))
print("TOTAL Admitted:    ", sum(it["admitted_count"] for it in total_coq))
print("TOTAL Axioms:      ", sum(it["axiom_count"]    for it in total_coq))

# Save
out = {
    "total_unique_v_files": len(unique),
    "coq_files": len(total_coq),
    "verilog_files": len(total_ver),
    "buckets": {b: [{"rel_path": it["rel_path"], "repo": it["repo"], "theorems_count": it["theorems_count"], "qed": it["qed_count"], "admitted": it["admitted_count"], "axiom": it["axiom_count"], "sha1": it["sha1"], "also_at": it.get("also_at", [])} for it in buckets[b]] for b in sorted(buckets)},
    "files": [{"abs_path": it["abs_path"], "rel_path": it["rel_path"], "repo": it["repo"], "kind": it["kind"], "bucket": it["bucket"], "sha1": it["sha1"], "theorems_count": it["theorems_count"], "theorems": it["theorems"], "qed_count": it["qed_count"], "admitted_count": it["admitted_count"], "axiom_count": it["axiom_count"], "also_at": it.get("also_at", [])} for it in unique],
}
with open("/home/user/workspace/phd_proofs_inventory_v2.json","w") as f:
    json.dump(out, f, indent=2)
print()
print("Saved -> /home/user/workspace/phd_proofs_inventory_v2.json")
