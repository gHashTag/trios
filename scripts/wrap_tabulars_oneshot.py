#!/usr/bin/env python3
"""
ONE-SHOT bridge for `mechanical_latex_fixes::wrap_wide_tabulars`.

Implements the EXACT same algorithm as the Rust function in
`crates/trios-phd/src/main.rs::wrap_wide_tabulars`. Used only when
the operator cannot run `trios-phd compile-resilient` directly
(e.g. cargo unavailable in the current environment) but still
needs the canonical fix applied to docs/phd/ before tectonic.

Idempotent. Skips tables already wrapped, and `tabular*` / `tabularx`.
"""
import os, sys

ROOTS = ['docs/phd/chapters', 'docs/phd/appendix', 'docs/phd/frontmatter']
BEGIN = r'\begin{tabular}'
END = r'\end{tabular}'
OPEN = '\\resizebox{\\linewidth}{!}{%\n'
CLOSE = '\n}'

def wrap(s: str) -> str:
    out = []
    cursor = 0
    while True:
        rel = s.find(BEGIN, cursor)
        if rel < 0:
            out.append(s[cursor:])
            return ''.join(out)
        after = rel + len(BEGIN)
        # tabular* / tabularx → не наша таблица, идём дальше
        if after < len(s) and s[after] in ('*', 'x'):
            out.append(s[cursor:after])
            cursor = after
            continue
        end_rel = s.find(END, after)
        if end_rel < 0:
            out.append(s[cursor:])
            return ''.join(out)
        end_abs = end_rel + len(END)
        head = s[max(0, rel - 120):rel]
        already_resized = '\\resizebox{\\linewidth}{!}{' in head and head.count('{') > head.count('}')
        already_adjust = '\\adjustbox{' in head and head.count('{') > head.count('}')
        out.append(s[cursor:rel])
        if already_resized or already_adjust:
            out.append(s[rel:end_abs])
        else:
            out.append(OPEN)
            out.append(s[rel:end_abs])
            out.append(CLOSE)
        cursor = end_abs

def main():
    total = 0
    files_changed = 0
    for root in ROOTS:
        if not os.path.isdir(root):
            continue
        for dirpath, _, files in os.walk(root):
            for f in files:
                if not f.endswith('.tex'):
                    continue
                p = os.path.join(dirpath, f)
                with open(p, encoding='utf-8') as fh:
                    src = fh.read()
                new = wrap(src)
                if new != src:
                    with open(p, 'w', encoding='utf-8') as fh:
                        fh.write(new)
                    delta = new.count(OPEN) - src.count(OPEN)
                    total += delta
                    files_changed += 1
                    print(f"{p}: +{delta} wraps")
    print(f"TOTAL: {total} tabulars wrapped across {files_changed} files")

if __name__ == '__main__':
    main()
