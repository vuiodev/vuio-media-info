#!/usr/bin/env python3
"""Cross-checks our output against ffprobe, an implementation independent of MediaInfo.

Compares only quantities ffprobe measures directly: frame geometry, chroma format,
bit depth, channel count, sample rate, and the coded size of each elementary stream.
"""
import json, subprocess, glob, os, sys

# Resolve the binary relative to the repository root when a bare relative path is used,
# so the tools work from any working directory.
_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("VUIO_BIN") or os.path.join(_ROOT, "target", "release", "vuio-media-info")
if not os.path.exists(BIN):
    raise SystemExit(f"binary not found: {BIN}\nBuild it with: cargo build --release")
MEDIA = os.environ.get("VUIO_CORPUS", "corpus")

# pix_fmt -> (chroma subsampling, bits per component)
PIXFMT = {
    "yuv420p": ("4:2:0", 8), "yuvj420p": ("4:2:0", 8), "yuv420p10le": ("4:2:0", 10),
    "yuv420p12le": ("4:2:0", 12), "yuv422p": ("4:2:2", 8), "yuvj422p": ("4:2:2", 8),
    "yuv422p10le": ("4:2:2", 10), "yuv422p12le": ("4:2:2", 12),
    "yuv444p": ("4:4:4", 8), "yuv444p10le": ("4:4:4", 10), "yuv444p12le": ("4:4:4", 12),
    "yuva444p10le": ("4:4:4", 10), "yuva444p12le": ("4:4:4", 12),
    "yuv411p": ("4:1:1", 8), "gray": ("Monochrome", 8),
}

def run(cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=90).stdout
    except Exception:
        return ""

def probe(path):
    out = run(["ffprobe", "-v", "error", "-show_streams", "-of", "json", path])
    try:
        return json.load(__import__("io").StringIO(out)).get("streams", [])
    except Exception:
        return []

def stream_bytes(path, index):
    out = run(["ffprobe", "-v", "error", "-select_streams", str(index),
               "-show_packets", "-show_entries", "packet=size", "-of", "csv=p=0", path])
    total = 0
    count = 0
    for line in out.splitlines():
        parts = line.strip().split(",")
        if parts and parts[0].strip().isdigit():
            total += int(parts[0].strip())
            count += 1
    return total, count

def ours(path):
    out = run([BIN, "-O", "json", path])
    try:
        d = json.loads(out)
    except Exception:
        return None
    tracks = d.get("media", {}).get("track", [])
    return tracks if isinstance(tracks, list) else [tracks]

results = []
files = sorted(f for f in glob.glob(os.path.join(MEDIA, "*")) if os.path.isfile(f) and not f.endswith(".sh"))

for path in files:
    streams = probe(path)
    mine = ours(path)
    row = {"file": os.path.basename(path), "issues": []}
    if mine is None:
        row["issues"].append(("FATAL", "our JSON unparseable", ""))
        results.append(row); continue

    our_v = [t for t in mine if t.get("@type") == "Video"]
    our_a = [t for t in mine if t.get("@type") == "Audio"]
    ff_v = [s for s in streams if s.get("codec_type") == "video"]
    ff_a = [s for s in streams if s.get("codec_type") == "audio"]

    def check(kind, label, got, want):
        if want is None or got is None:
            return
        if str(got) != str(want):
            row["issues"].append((kind, label, f"ours={got} ffprobe={want}"))

    for i, s in enumerate(ff_v):
        if i >= len(our_v):
            row["issues"].append(("COUNT", "video track missing", f"ffprobe has {len(ff_v)}"))
            break
        t = our_v[i]
        check("WRONG", f"Video[{i}].Width", t.get("Width"), s.get("width"))
        check("WRONG", f"Video[{i}].Height", t.get("Height"), s.get("height"))

        # Frame rate, as a rational, so 30000/1001 compares exactly. avg_frame_rate is
        # preferred because r_frame_rate is a guessed base that doubles on field-coded
        # MPEG streams.
        rate = s.get("avg_frame_rate") or s.get("r_frame_rate", "0/0")
        try:
            num, den = (int(x) for x in rate.split("/"))
        except ValueError:
            num = den = 0
        if den and num:
            want = num / den
            got = t.get("FrameRate")
            if got is not None and abs(float(got) - want) > 0.002:
                row["issues"].append(
                    ("WRONG", f"Video[{i}].FrameRate", f"ours={got} ffprobe={want:.3f}")
                )

        # Colour range: ffprobe reports tv (limited) or pc (full).
        rng = {"tv": "Limited", "pc": "Full"}.get(s.get("color_range"))
        check("WRONG", f"Video[{i}].colour_range", t.get("colour_range"), rng)

        # Codec identity, via the families both tools name the same way.
        codec_map = {
            "h264": "AVC", "hevc": "HEVC", "av1": "AV1", "vp9": "VP9", "vp8": "VP8",
            "prores": "ProRes", "dnxhd": "VC-3", "ffv1": "FFV1", "dvvideo": "DV",
            "cfhd": "CineForm", "mpeg2video": "MPEG Video", "mpeg1video": "MPEG Video",
            "mpeg4": "MPEG-4 Visual", "theora": "Theora",
        }
        want_codec = codec_map.get(s.get("codec_name"))
        check("WRONG", f"Video[{i}].Format", t.get("Format"), want_codec)
        chroma_depth = PIXFMT.get(s.get("pix_fmt", ""))
        if chroma_depth:
            chroma, depth = chroma_depth
            check("WRONG", f"Video[{i}].ChromaSubsampling", t.get("ChromaSubsampling"), chroma)
            check("WRONG", f"Video[{i}].BitDepth", t.get("BitDepth"), depth)
        size, packets = stream_bytes(path, s.get("index", i))
        if size:
            # Matroska stores ProRes frames without their 4-byte size and `icpf`
            # prefix; ffmpeg re-adds those 8 bytes per frame when demuxing, so its
            # packet sum exceeds what is actually on disk.
            if s.get("codec_name") == "prores" and path.endswith((".mkv", ".webm")):
                size -= 8 * packets
            check("WRONG", f"Video[{i}].StreamSize", t.get("StreamSize"), size)

    for i, s in enumerate(ff_a):
        if i >= len(our_a):
            row["issues"].append(("COUNT", "audio track missing", f"ffprobe has {len(ff_a)}"))
            break
        t = our_a[i]
        check("WRONG", f"Audio[{i}].Channels", t.get("Channels"), s.get("channels"))
        check("WRONG", f"Audio[{i}].SamplingRate", t.get("SamplingRate"), s.get("sample_rate"))
        acodec = {
            "aac": "AAC", "ac3": "AC-3", "eac3": "E-AC-3", "flac": "FLAC", "opus": "Opus",
            "vorbis": "Vorbis", "alac": "ALAC", "mp3": "MPEG Audio", "mp2": "MPEG Audio",
            "truehd": "TrueHD", "dts": "DTS", "wavpack": "WavPack", "tta": "TTA",
        }.get(s.get("codec_name"))
        check("WRONG", f"Audio[{i}].Format", t.get("Format"), acodec)
        bits = s.get("bits_per_raw_sample") or s.get("bits_per_sample")
        if bits and str(bits) != "0":
            check("WRONG", f"Audio[{i}].BitDepth", t.get("BitDepth"), bits)

    results.append(row)

json.dump(results, open("cmp_ffprobe_results.json", "w"), indent=1)
clean = [r for r in results if not r["issues"]]
failed = len(results) - len(clean)
print(f"TOTAL {len(results)}  CLEAN {len(clean)}  WITH-ISSUES {failed}\n")
for r in results:
    if not r["issues"]:
        continue
    print(f"### {r['file']}")
    for kind, label, detail in r["issues"][:10]:
        print(f"   {kind:7} {label:34} {detail}")
    print()

if not results:
    print("No sample files found; set VUIO_CORPUS or run tools/make_corpus.sh first.")
    sys.exit(2)
sys.exit(1 if failed else 0)
