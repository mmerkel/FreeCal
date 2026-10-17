import glob, re, subprocess, html, sys, os, shutil
from datetime import date

# Usage: tickets_pdf.py [feature-dir] [out.pdf]
FEATURE = sys.argv[1] if len(sys.argv) > 1 else ".scratch/freecal-v1"
OUT = os.path.abspath(sys.argv[2] if len(sys.argv) > 2 else "tickets.pdf")
ISSUES = os.path.join(FEATURE, "issues")
FEATURE_NAME = os.path.basename(os.path.normpath(FEATURE))

DAYS = {"S": 0.5, "M": 1.5, "L": 4, "XL": 7.5, "-": 0}

tickets = {}
for f in sorted(glob.glob(os.path.join(ISSUES, "*.md"))):
    t = open(f).read()
    num = os.path.basename(f)[:2]
    title = re.search(r"^# \d+: (.*)", t, re.M).group(1)
    b = re.search(r"\*\*Blocked by:\*\*\s*(.*)", t)
    s = re.search(r"\*\*Status:\*\*\s*(\S+)", t)
    sp = re.search(r"\*\*Split into:\*\*\s*(.*)", t)
    e = re.search(r"\*\*Effort:\*\*\s*(XL|S|M|L)\b", t)
    blocked = re.findall(r"\b\d{2}\b", b.group(1)) if b else []
    split = re.findall(r"\d{2}", sp.group(1)) if sp else []
    tickets[num] = dict(
        num=num, title=title, blocked=blocked, status=s.group(1) if s else "?",
        split=split, effort=e.group(1) if e else "-", file=f,
    )

missing = [t["file"] for t in tickets.values() if t["effort"] == "-" and not t["split"] and t["status"] != "wontfix"]
if missing:
    sys.exit("No **Effort:** line in:\n" + "\n".join(missing))

# A blocker that was split stands for all of its parts.
def resolve(n):
    t = tickets[n]
    return [x for p in t["split"] for x in resolve(p)] if t["split"] else [n]

for t in tickets.values():
    t["deps"] = sorted({x for b in t["blocked"] for x in resolve(b)})
active = {n: t for n, t in tickets.items() if not t["split"]}
blocks = {n: [] for n in active}
for n, t in active.items():
    for d in t["deps"]:
        blocks[d].append(n)

open_ = {n: t for n, t in active.items() if t["status"] not in ("done", "wontfix")}

def ready(t):
    return all(active[d]["status"] == "done" for d in t["deps"])

# Longest chain of remaining work (by days) ending at each open ticket.
memo = {}
def chain(n):
    if n not in memo:
        prev = [chain(d) for d in active[n]["deps"] if d in open_]
        best = max(prev, key=lambda c: c[0], default=(0, []))
        memo[n] = (best[0] + DAYS[active[n]["effort"]], best[1] + [n])
    return memo[n]

crit = max((chain(n) for n in open_), key=lambda c: c[0])
crit_set = set(crit[1])

# Transitive dependents, to show how much each open ticket unblocks.
def downstream(n, seen=None):
    seen = set() if seen is None else seen
    for c in blocks[n]:
        if c not in seen:
            seen.add(c)
            downstream(c, seen)
    return seen

# Graph
COL = {
    "done": ("#e7e7e7", "#9a9a9a", "#8a8a8a"),
    "ready-for-agent": ("#dbeafe", "#2563eb", "#111"),
    "ready-for-human": ("#fde7c7", "#d97706", "#111"),
    "needs-triage": ("#f3e8ff", "#9333ea", "#111"),
}
def short(s, n=30):
    import textwrap
    ls = textwrap.wrap(s, n)
    if len(ls) > 2:
        ls = [ls[0], textwrap.shorten(" ".join(ls[1:]), n, placeholder="…")]
    return "<br/>".join(html.escape(l) for l in ls)

linked = {n for n, t in active.items() if t["deps"] or blocks[n]}
isolated = [n for n in active if n not in linked]

lines = ['digraph G {', 'rankdir=LR; ranksep=0.45; nodesep=0.12; size="16,9.2"; ratio="fill"; bgcolor="transparent";',
         'node [shape=box style="rounded,filled" fontname="Helvetica" fontsize=10 margin="0.08,0.03" penwidth=1.2];',
         'edge [color="#9aa3af" arrowsize=0.5 penwidth=0.8];']
for n, t in active.items():
    if n not in linked:
        continue
    fill, line, font = COL.get(t["status"], ("#fff", "#000", "#000"))
    ready_now = n in open_ and ready(t)
    pen = 2.6 if n in crit_set else 1.2
    if n in crit_set:
        line = "#dc2626"
    label = f"<<b>{n}</b>  <font point-size='8'>[{t['effort']}]</font><br/>{short(t['title'])}>"
    style = "rounded,filled" + (",dashed" if n in open_ and not ready_now else "")
    lines.append(f'"{n}" [label={label} fillcolor="{fill}" color="{line}" fontcolor="{font}" penwidth={pen} style="{style}"];')
for n, t in active.items():
    for d in t["deps"]:
        attr = ' [color="#dc2626" penwidth=2]' if d in crit_set and n in crit_set and crit[1].index(n) == crit[1].index(d) + 1 else ""
        lines.append(f'"{d}" -> "{n}"{attr};')
lines.append("}")
svg = subprocess.run(["dot", "-Tsvg"], input="\n".join(lines), capture_output=True, text=True, check=True).stdout
svg = svg[svg.index("<svg"):]
svg = re.sub(r'width="[\d.]+pt" height="[\d.]+pt"', 'width="100%" height="100%" preserveAspectRatio="xMidYMid meet"', svg, count=1)

# Tables
STATUS_LABEL = {"done": "done", "ready-for-agent": "agent", "ready-for-human": "human", "needs-triage": "triage", "wontfix": "split"}
def badge(s):
    return f'<span class="b {s}">{STATUS_LABEL.get(s, s)}</span>'
def eff(e):
    return f'<span class="e e{e}">{e}</span>' if e != "-" else "–"

rows = []
for n, t in tickets.items():
    if t["split"]:
        rows.append(f'<tr class="dim"><td>{n}</td><td>{html.escape(t["title"])}<div class="note">Split into {", ".join(t["split"])}</div></td><td>{badge("wontfix")}</td><td>–</td><td>{", ".join(t["blocked"]) or "–"}</td><td>–</td></tr>')
        continue
    blocked_txt = ", ".join(
        (f'<b>{d}</b>' if active[d]["status"] != "done" else d) for d in t["deps"]) or "–"
    unblocks = ", ".join(sorted(blocks[n])) or "–"
    cls = "dim" if t["status"] == "done" else ("crit" if n in crit_set else "")
    rows.append(f'<tr class="{cls}"><td>{n}</td><td>{html.escape(t["title"])}</td><td>{badge(t["status"])}</td><td>{eff(t["effort"])}</td><td>{blocked_txt}</td><td>{unblocks}</td></tr>')

frontier = sorted((n for n, t in open_.items() if ready(t)), key=lambda n: (-len(downstream(n)), n))
front_rows = "".join(
    f'<tr class="{"crit" if n in crit_set else ""}"><td>{n}</td><td>{html.escape(open_[n]["title"])}</td><td>{badge(open_[n]["status"])}</td><td>{eff(open_[n]["effort"])}</td><td>{len(downstream(n))}</td></tr>'
    for n in frontier)

cnt = lambda st: sum(1 for t in active.values() if t["status"] == st)
open_days = sum(DAYS[t["effort"]] for t in open_.values())
done_days = sum(DAYS[t["effort"]] for t in active.values() if t["status"] == "done")
eff_counts = {e: sum(1 for t in open_.values() if t["effort"] == e) for e in ("S", "M", "L", "XL")}
crit_txt = " → ".join(f"{n} [{active[n]['effort']}]" for n in crit[1])

iso_html = " ".join(
    f'<span class="chip {active[n]["status"]}"><b>{n}</b> [{active[n]["effort"]}] {html.escape(active[n]["title"])}</span>' for n in isolated)

split_nums = [n for n, t in tickets.items() if t["split"]]
split_txt = f"Split tickets ({', '.join(split_nums)}) are left out; a ticket blocked by one of them counts as blocked by its parts." if split_nums else ""

page = f"""<!doctype html><html><head><meta charset="utf-8"><title>{FEATURE_NAME} tickets</title><style>
@page {{ size: A4; margin: 14mm 12mm; }}
@page graph {{ size: A3 landscape; margin: 8mm; }}
body {{ font: 9.5pt/1.4 Helvetica, Arial, sans-serif; color: #1f2328; margin: 0; }}
h1 {{ font-size: 20pt; margin: 0 0 2pt; }} h2 {{ font-size: 13pt; margin: 16pt 0 6pt; }}
.sub {{ color: #666; margin-bottom: 12pt; }}
.stats {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 8pt; margin: 10pt 0; }}
.stat {{ border: 1px solid #ddd; border-radius: 6px; padding: 7pt 9pt; }}
.stat .v {{ font-size: 17pt; font-weight: 700; }} .stat .l {{ color: #666; font-size: 8.5pt; }}
table {{ width: 100%; border-collapse: collapse; }}
th {{ text-align: left; font-size: 8pt; color: #666; border-bottom: 1.5px solid #333; padding: 3pt 4pt; }}
td {{ border-bottom: 1px solid #e5e5e5; padding: 3pt 4pt; vertical-align: top; }}
td:first-child {{ font-weight: 700; width: 18pt; }}
tr {{ page-break-inside: avoid; }}
tr.dim td {{ color: #8a8a8a; }} tr.crit td:first-child {{ color: #dc2626; }}
.note {{ font-size: 8pt; }}
.b {{ font-size: 7.5pt; padding: 1pt 5pt; border-radius: 8pt; white-space: nowrap; }}
.b.done {{ background: #eee; color: #777; }} .b.ready-for-agent {{ background: #dbeafe; color: #1d4ed8; }}
.b.ready-for-human {{ background: #fde7c7; color: #b45309; }} .b.needs-triage {{ background: #f3e8ff; color: #7e22ce; }}
.b.wontfix {{ background: #f5f5f5; color: #999; }}
.e {{ display: inline-block; min-width: 16pt; text-align: center; font-weight: 700; font-size: 8pt; border-radius: 3pt; padding: 0 3pt; }}
.eS {{ background: #dcfce7; color: #15803d; }} .eM {{ background: #fef9c3; color: #a16207; }}
.eL {{ background: #ffedd5; color: #c2410c; }} .eXL {{ background: #fee2e2; color: #b91c1c; }}
.graph {{ page: graph; break-before: page; break-after: page; height: 270mm; display: flex; flex-direction: column; }}
.graph .svg {{ flex: 1; min-height: 0; }}
.legend {{ font-size: 9pt; color: #444; margin-bottom: 4pt; }}
.legend i {{ display: inline-block; width: 14pt; height: 9pt; border-radius: 3pt; vertical-align: middle; margin: 0 3pt 0 10pt; border: 1.2px solid; }}
.iso {{ font-size: 8pt; line-height: 1.9; margin-top: 4pt; }}
.chip {{ display: inline-block; border: 1px solid #ccc; border-radius: 4pt; padding: 0 4pt; margin-right: 3pt; white-space: nowrap; line-height: 1.5; }}
.chip.done {{ color: #888; background: #f0f0f0; }} .chip.ready-for-agent {{ background: #dbeafe; border-color: #2563eb; }}
.chip.ready-for-human {{ background: #fde7c7; border-color: #d97706; }} .chip.needs-triage {{ background: #f3e8ff; border-color: #9333ea; }}
.crittxt {{ background: #fef2f2; border-left: 3px solid #dc2626; padding: 6pt 9pt; font-size: 9pt; }}
</style></head><body>
<h1>{FEATURE_NAME} — ticket overview</h1>
<div class="sub">Generated {date.today().isoformat()} from <code>{html.escape(ISSUES)}/</code> · {len(tickets)} ticket files</div>
<div class="stats">
<div class="stat"><div class="v">{cnt("done")}</div><div class="l">done (≈{done_days:.0f} days)</div></div>
<div class="stat"><div class="v">{len(open_)}</div><div class="l">open: {cnt("ready-for-agent")} agent · {cnt("ready-for-human")} human · {cnt("needs-triage")} triage</div></div>
<div class="stat"><div class="v">≈{open_days:.0f} d</div><div class="l">remaining effort ({eff_counts["S"]} S · {eff_counts["M"]} M · {eff_counts["L"]} L · {eff_counts["XL"]} XL)</div></div>
<div class="stat"><div class="v">≈{crit[0]:.0f} d</div><div class="l">longest remaining chain ({len(crit[1])} tickets)</div></div>
</div>
<p><b>Effort</b> comes from each ticket's Effort line: {eff("S")} ≈ ½ day · {eff("M")} ≈ 1–2 days · {eff("L")} ≈ 3–5 days · {eff("XL")} ≈ 1–2 weeks. Totals use 0.5 / 1.5 / 4 / 7.5 days. {split_txt}</p>
<div class="crittxt"><b>Longest remaining chain</b> (the critical path through open work): {crit_txt}</div>

<h2>Ready to start now ({len(frontier)})</h2>
<p style="margin-top:0">Open tickets whose blockers are all done, sorted by how many open tickets they eventually unblock.</p>
<table><tr><th>#</th><th>Ticket</th><th>Status</th><th>Effort</th><th>Unblocks (transitively)</th></tr>{front_rows}</table>

<div class="graph"><h2 style="margin-top:0">Dependency graph</h2>
<div class="legend">Arrow = "must come before".<i style="background:#e7e7e7;border-color:#9a9a9a"></i>done<i style="background:#dbeafe;border-color:#2563eb"></i>ready for agent<i style="background:#fde7c7;border-color:#d97706"></i>ready for human<i style="background:#f3e8ff;border-color:#9333ea"></i>needs triage<i style="background:#fff;border-color:#555;border-style:dashed"></i>open but blocked<i style="background:#fff;border-color:#dc2626;border-width:2.5px"></i>critical path · [S/M/L/XL] effort</div>
<div class="svg">{svg}</div>
<div class="iso"><b>No dependencies either way:</b> {iso_html}</div></div>

<h2>All tickets</h2>
<p style="margin-top:0">Blockers in <b>bold</b> are not done yet. Red numbers are on the critical path.</p>
<table><tr><th>#</th><th>Ticket</th><th>Status</th><th>Effort</th><th>Blocked by</th><th>Blocks</th></tr>{"".join(rows)}</table>
</body></html>"""
html_path = OUT[:-4] + ".html"
open(html_path, "w").write(page)
chrome = next((c for c in ("google-chrome", "chromium", "chromium-browser") if shutil.which(c)), None)
if not chrome:
    sys.exit(f"No Chrome or Chromium found; the HTML is at {html_path}")
subprocess.run([chrome, "--headless", "--disable-gpu", "--no-pdf-header-footer", f"--print-to-pdf={OUT}", html_path],
               check=True, capture_output=True)
os.remove(html_path)
print(f"Wrote {OUT}: {len(active)} tickets, {len(open_)} open, ~{open_days:.0f} days remaining, longest chain {crit_txt}")
for n in frontier[:3]:
    print(f"  ready: {n} [{open_[n]['effort']}] {open_[n]['title']} (unblocks {len(downstream(n))})")
