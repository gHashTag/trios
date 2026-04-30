import psycopg2, subprocess, os, time, re

URI = "postgresql://neondb_owner:npg_NHBC5hdbM0Kx@ep-curly-math-ao51pquy-pooler.c-2.ap-southeast-1.aws.neon.tech/neondb?sslmode=require&channel_binding=require"
ROOT = "/home/user/workspace/phd_book"
BUILD = f"{ROOT}/build"
TEMPLATE = f"{ROOT}/template"
TECTONIC = "/home/user/.local/bin/tectonic"

os.makedirs(BUILD, exist_ok=True)

# Strip 4-byte unicode (emoji + symbols outside BMP) 
EMOJI_RE = re.compile(r'[\U00010000-\U0010ffff]')

def latex_escape_title(s):
    """Escape special chars in chapter titles for use in \chapter*{...}."""
    s = EMOJI_RE.sub('', s)
    repl = {
        '\\': r'\textbackslash{}',
        '_': r'\_',
        '#': r'\#',
        '&': r'\&',
        '$': r'\$',
        '%': r'\%',
        '{': r'\{',
        '}': r'\}',
        '~': r'\textasciitilde{}',
        '^': r'\textasciicircum{}',
    }
    out = []
    for ch in s:
        out.append(repl.get(ch, ch))
    return ''.join(out)

def md_to_tex(md_text):
    md_text = EMOJI_RE.sub('', md_text)
    r = subprocess.run(
        ['pandoc','-f','gfm','-t','latex','--no-highlight','--wrap=preserve'],
        input=md_text, capture_output=True, text=True, timeout=60
    )
    if r.returncode != 0:
        raise RuntimeError(f"pandoc failed: {r.stderr[:500]}")
    return r.stdout

def slug(ch_num):
    return ch_num.replace('.','_').lower()

def compile_one(cur, ch_num):
    cur.execute("SELECT id, title, body_md FROM ssot.chapters WHERE ch_num=%s", (ch_num,))
    row = cur.fetchone()
    if not row: return None
    ch_id, title, body = row
    if not body: return None
    safe_title = latex_escape_title(title)
    tex = md_to_tex(body)
    chapter_tex = f"\n\n\\chapter*{{{safe_title}}}\n\\addcontentsline{{toc}}{{chapter}}{{{safe_title}}}\n\n{tex}\n"
    fp = f"{BUILD}/ch_{slug(ch_num)}.tex"
    open(fp,'w').write(chapter_tex)
    cur.execute("UPDATE ssot.chapters SET body_tex=%s WHERE id=%s", (tex, ch_id))
    return fp

def compile_all():
    conn = psycopg2.connect(URI); conn.autocommit = True
    cur = conn.cursor()
    cur.execute("SELECT ch_num FROM ssot.chapters ORDER BY ch_num")
    chs = [r[0] for r in cur.fetchall()]
    files = []
    for c in chs:
        try:
            fp = compile_one(cur, c)
            if fp: files.append((c, fp))
        except Exception as e:
            print(f"  ERR {c}: {e}")
    print(f"Generated {len(files)} chapter .tex")
    inputs = '\n'.join([f'\\input{{ch_{slug(c)}.tex}}' for c, _ in files])
    main_tpl = open(f"{TEMPLATE}/main.tex.tpl").read()
    main_tex = main_tpl.replace('%%CHAPTERS%%', inputs)
    open(f"{BUILD}/main.tex",'w').write(main_tex)
    import shutil
    shutil.copy(f"{TEMPLATE}/preamble.tex", f"{BUILD}/preamble.tex")
    print(f"\nRunning tectonic on {BUILD}/main.tex")
    t0 = time.time()
    r = subprocess.run(
        [TECTONIC, '-X', 'compile', '--keep-intermediates', '--outdir', BUILD, f"{BUILD}/main.tex"],
        capture_output=True, text=True, timeout=600
    )
    elapsed = time.time() - t0
    print(f"Elapsed: {elapsed:.1f}s · returncode: {r.returncode}")
    if r.returncode != 0:
        for ln in r.stderr.strip().split('\n')[-20:]:
            print(f"  {ln}")
    pdf = f"{BUILD}/main.pdf"
    pdf_ok = os.path.exists(pdf) and os.path.getsize(pdf) > 1000
    sz = os.path.getsize(pdf) if os.path.exists(pdf) else 0
    print(f"PDF size: {sz} bytes · OK={pdf_ok}")
    cur.execute("UPDATE ssot.chapters SET last_compiled=now(), compile_ok=%s, compile_log=%s",
                (pdf_ok, r.stderr[-2000:]))
    conn.close()
    return pdf if pdf_ok else None

if __name__ == "__main__":
    print("RESULT:", compile_all())
