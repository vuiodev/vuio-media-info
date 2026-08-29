#!/usr/bin/env python3
"""Reports differences against the reference MediaInfo CLI.

Advisory only: it always exits successfully. MediaInfo apportions container overhead
across streams so that its stream sizes sum to the file size, which makes StreamSize and
BitRate depend on its own parse bookkeeping rather than on the file. Two transport
streams with identical PSI structure get different splits, so those fields are not
reproducible by any implementation. tools/compare_ffprobe.py is the correctness gate.
"""
import json, subprocess, sys, os, glob
# Resolve the binary relative to the repository root when a bare relative path is used,
# so the tools work from any working directory.
_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("VUIO_BIN") or os.path.join(_ROOT, "target", "release", "vuio-media-info")
if not os.path.exists(BIN):
    raise SystemExit(f"binary not found: {BIN}\nBuild it with: cargo build --release")
MEDIA = os.environ.get("VUIO_CORPUS", "corpus")

# fields to compare per track type
FIELDS = {
 "General": ["Format","Format_Profile","FileSize"],
 "Video":   ["Format","Format_Profile","Format_Version","CodecID","Width","Height","BitDepth","ChromaSubsampling",
             "ColorSpace","FrameRate","BitRate","ScanType","ScanOrder","Standard","colour_range","colour_primaries","transfer_characteristics","matrix_coefficients"],
 "Audio":   ["Format","Format_Profile","CodecID","Channels","SamplingRate","BitDepth","BitRate"],
 "Text":    ["Format","CodecID"],
}

def run(cmd):
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
        return r.stdout
    except Exception as e:
        return ""

def norm(v):
    if v is None: return None
    s=str(v).strip()
    return s

def tracks(js):
    try:
        d=json.loads(js)
    except Exception:
        return None
    out=[]
    tl = d.get("media",{}).get("track",[])
    if isinstance(tl,dict): tl=[tl]
    for t in tl:
        out.append(t)
    return out

def key(t):
    return t.get("@type","?")

results=[]
files = sorted(glob.glob(os.path.join(MEDIA, "*")))
files = [f for f in files if os.path.isfile(f) and not f.endswith(('.sh','.txt'))]

for f in files:
    ours = run([BIN,"-O","json",f])
    ref  = run(["mediainfo","--Output=JSON",f])
    ot = tracks(ours); rt = tracks(ref)
    row={"file":os.path.basename(f),"issues":[]}
    if ot is None:
        row["issues"].append(("FATAL","our JSON unparseable/empty",ours[:200]))
        results.append(row); continue
    if rt is None:
        row["issues"].append(("SKIP","reference unparseable",""))
        results.append(row); continue
    # group by type
    def group(ts):
        g={}
        for t in ts: g.setdefault(key(t),[]).append(t)
        return g
    og,rg = group(ot),group(rt)
    for ty in ["General","Video","Audio","Text"]:
        ol,rl = og.get(ty,[]),rg.get(ty,[])
        if len(ol)!=len(rl):
            row["issues"].append(("COUNT",f"{ty} track count",f"ours={len(ol)} ref={len(rl)}"))
        for i in range(min(len(ol),len(rl))):
            for fld in FIELDS[ty]:
                ov,rv = norm(ol[i].get(fld)), norm(rl[i].get(fld))
                if rv is None: continue           # reference doesn't report it -> skip
                if ov is None:
                    row["issues"].append(("MISSING",f"{ty}[{i}].{fld}",f"ref={rv}"))
                else:
                    # numeric tolerance
                    try:
                        a,b=float(ov),float(rv)
                        if b!=0 and abs(a-b)/abs(b) < 0.02: continue
                        if a==b: continue
                    except ValueError:
                        if ov.lower()==rv.lower(): continue
                    row["issues"].append(("WRONG",f"{ty}[{i}].{fld}",f"ours={ov} ref={rv}"))
    results.append(row)

json.dump(results, open("cmp_results.json", "w"), indent=1)
# summary
clean=[r for r in results if not r["issues"]]
print(f"TOTAL {len(results)}  CLEAN {len(clean)}  WITH-ISSUES {len(results)-len(clean)}\n")
for r in results:
    if not r["issues"]: continue
    print(f"### {r['file']}")
    for k,w,d in r["issues"][:14]:
        print(f"   {k:8} {w:42} {d}")
    if len(r['issues'])>14: print(f"   ... +{len(r['issues'])-14} more")
    print()

print("\nAdvisory only - see tools/compare_ffprobe.py for the correctness gate.")
