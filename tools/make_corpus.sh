#!/bin/bash
# Builds the sample corpus that tools/compare_ffprobe.py and tools/compare_mediainfo.py
# run against. Requires ffmpeg.
#
#   VUIO_CORPUS=corpus ./tools/make_corpus.sh
#
# Codecs missing from the local ffmpeg build are reported and skipped, so the corpus
# covers whatever that build can produce.
set -u

M="${VUIO_CORPUS:-corpus}"
mkdir -p "$M"
cd "$M"

Q="-y -v error"
V="-f lavfi -i testsrc2=s=320x240:r=25:d=2"
A="-f lavfi -i sine=frequency=440:sample_rate=48000:duration=2"
A2="-f lavfi -i sine=frequency=440:sample_rate=48000:duration=2 -ac 2"
S="-f lavfi -i sine=frequency=440:sample_rate=48000:duration=2 -ac 2"
VA="$V $A"

ok=0
skipped=0

# Runs one ffmpeg invocation. A failed encode can still leave a zero-length file behind,
# which would otherwise look like a corpus entry, so those are removed.
g() {
  desc=$1; shift
  out=""
  for a in "$@"; do out="$a"; done
  if ffmpeg $Q "$@" 2>/tmp/vuio_corpus_err.txt; then
    echo "OK    $desc"
    ok=$((ok + 1))
  else
    echo "SKIP  $desc :: $(grep -v '^ *$' /tmp/vuio_corpus_err.txt | tail -1)"
    skipped=$((skipped + 1))
  fi
  if [ -f "$out" ] && [ ! -s "$out" ]; then rm -f "$out"; fi
  return 0
}

### --- ISOBMFF family ---
g "mp4 h264+aac"      $VA -c:v libx264 -c:a aac -pix_fmt yuv420p iso_h264_aac.mp4
g "mov h264"          $VA -c:v libx264 -c:a aac -pix_fmt yuv420p iso_h264.mov
g "m4v h264"          $V  -c:v libx264 -pix_fmt yuv420p iso.m4v
g "m4a aac"           $A  -c:a aac iso_aac.m4a
g "m4b aac"           $A  -c:a aac -f mp4 iso_aac.m4b
g "qt h264"           $V  -c:v libx264 -pix_fmt yuv420p -f mov iso.qt
g "mp4 hevc"          $V  -c:v libx265 -pix_fmt yuv420p -tag:v hvc1 iso_hevc.mp4
g "mp4 hevc 10bit"    $V  -c:v libx265 -pix_fmt yuv420p10le -tag:v hvc1 iso_hevc10.mp4
g "mp4 av1"           $V  -c:v libaom-av1 -cpu-used 8 -pix_fmt yuv420p iso_av1.mp4
g "mp4 vp9"           $V  -c:v libvpx-vp9 -pix_fmt yuv420p iso_vp9.mp4
g "mp4 mpeg4visual"   $V  -c:v mpeg4 -pix_fmt yuv420p iso_mpeg4v.mp4
g "mp4 alac"          $A  -c:a alac iso_alac.m4a
g "mp4 ac3"           $A  -c:a ac3 iso_ac3.mp4
g "mp4 opus"          $A  -c:a libopus -f mp4 iso_opus.mp4
### --- Matroska ---
g "mkv h264+aac"      $VA -c:v libx264 -c:a aac -pix_fmt yuv420p mkv_h264_aac.mkv
g "mka flac"          $A  -c:a flac -f matroska mka_flac.mka
g "webm vp9+opus"     $VA -c:v libvpx-vp9 -c:a libopus -pix_fmt yuv420p wm_vp9_opus.webm
g "webm vp8+vorbis"   $VA -c:v libvpx -c:a libvorbis -pix_fmt yuv420p wm_vp8_vorbis.webm
g "mk3d h264"         $V  -c:v libx264 -pix_fmt yuv420p -f matroska mkv3d.mk3d
g "mkv av1"           $V  -c:v libaom-av1 -cpu-used 8 -pix_fmt yuv420p mkv_av1.mkv
g "mkv hevc"          $V  -c:v libx265 -pix_fmt yuv420p mkv_hevc.mkv
### --- RIFF ---
g "avi mpeg4+mp3"     $VA -c:v mpeg4 -c:a libmp3lame -pix_fmt yuv420p riff.avi
g "wav pcm16"         $A  -c:a pcm_s16le riff_pcm16.wav
g "wav pcm24"         $A  -c:a pcm_s24le riff_pcm24.wav
g "wav pcmf32"        $A  -c:a pcm_f32le riff_f32.wav
g "wave pcm16"        $A  -c:a pcm_s16le -f wav riff.wave
g "bwf pcm16"         $A  -c:a pcm_s16le -write_bext 1 -f wav riff.bwf
g "rf64 pcm16"        $A  -c:a pcm_s16le -rf64 always -f wav riff.rf64
### --- MPEG-TS / PS ---
g "ts h264+aac"       $VA -c:v libx264 -c:a aac -pix_fmt yuv420p ts_h264.ts
g "m2ts h264+ac3"     $VA -c:v libx264 -c:a ac3 -pix_fmt yuv420p -f mpegts ts_h264.m2ts
g "mts h264"          $V  -c:v libx264 -pix_fmt yuv420p -f mpegts ts.mts
g "m2t h264"          $V  -c:v libx264 -pix_fmt yuv420p -f mpegts ts.m2t
g "ts hevc"           $V  -c:v libx265 -pix_fmt yuv420p -f mpegts ts_hevc.ts
g "mpg mpeg1"         $VA -c:v mpeg1video -c:a mp2 -pix_fmt yuv420p ps_mpeg1.mpg
g "mpeg mpeg2"        $VA -c:v mpeg2video -c:a mp2 -pix_fmt yuv420p ps_mpeg2.mpeg
g "vob mpeg2"         $VA -c:v mpeg2video -c:a ac3 -pix_fmt yuv420p -f vob ps.vob
g "evob mpeg2"        $VA -c:v mpeg2video -c:a ac3 -pix_fmt yuv420p -f vob ps.evob
### --- Ogg ---
g "ogg vorbis"        $A  -c:a libvorbis ogg_vorbis.ogg
g "oga flac"          $A  -c:a flac -f ogg ogg_flac.oga
g "ogv theora"        $VA -c:v libtheora -c:a libvorbis -pix_fmt yuv420p ogg_theora.ogv
g "opus"              $A  -c:a libopus ogg.opus
g "ogx vorbis"        $A  -c:a libvorbis -f ogg ogg.ogx
g "spx speex"         $A  -c:a libspeex -f ogg ogg.spx
### --- MXF / ASF / FLV ---
g "mxf mpeg2"         $VA -c:v mpeg2video -c:a pcm_s16le -pix_fmt yuv420p mxf_mpeg2.mxf
g "mxf dnxhd"         -f lavfi -i testsrc2=s=1920x1080:r=25:d=1 -c:v dnxhd -b:v 36M -pix_fmt yuv422p mxf_dnxhd.mxf
g "asf wmv2+wma"      $VA -c:v wmv2 -c:a wmav2 -pix_fmt yuv420p asf.asf
g "wmv wmv2"          $VA -c:v wmv2 -c:a wmav2 -pix_fmt yuv420p asf.wmv
g "wma wmav2"         $A  -c:a wmav2 asf.wma
g "flv h264+aac"      $VA -c:v libx264 -c:a aac -pix_fmt yuv420p flv.flv
g "f4v h264+aac"      $VA -c:v libx264 -c:a aac -pix_fmt yuv420p -f mp4 flv.f4v
### --- Audiophile containers ---
g "caf pcm"           $A  -c:a pcm_s16be caf.caf
g "caf alac"          $A  -c:a alac -f caf caf_alac.caf
g "aiff pcm"          $A  -c:a pcm_s16be aiff.aiff
g "aif pcm"           $A  -c:a pcm_s16be -f aiff aiff.aif
g "aifc pcm"          $A  -c:a pcm_s16be -f aiff aiff.aifc
g "wv wavpack"        $A  -c:a wavpack wavpack.wv
g "tta"               $A  -c:a tta -f tta ta.tta
g "amr nb"            $A  -ar 8000 -ac 1 -c:a libopencore_amrnb -b:a 12.2k amr.amr
g "awb wb"            $A  -ar 16000 -ac 1 -c:a libvo_amrwbenc -b:a 23.85k amr.awb
g "dsf"               $A  -c:a dsd_lsbf_planar -f dsf dsd.dsf
g "ivf av1"           $V  -c:v libaom-av1 -cpu-used 8 -pix_fmt yuv420p -f ivf ivf_av1.ivf
g "ivf vp9"           $V  -c:v libvpx-vp9 -pix_fmt yuv420p -f ivf ivf_vp9.ivf
g "y4m"               $V  -pix_fmt yuv420p y4m.y4m
### --- Elementary audio streams ---
g "mp3"               $A  -c:a libmp3lame es.mp3
g "mp2"               $A  -c:a mp2 -f mp2 es.mp2
g "mp1"               $A  -c:a mp2 -f mp2 es.mp1
g "aac adts"          $A  -c:a aac -f adts es.aac
g "he-aac v1"         $A  -c:a libfdk_aac -profile:a aac_he -f adts es_heaac.aac
g "ac3"               $A  -c:a ac3 es.ac3
g "eac3"              $A  -c:a eac3 -f eac3 es.eac3
g "ec3"               $A  -c:a eac3 -f eac3 es.ec3
g "dts"               $A  -c:a dca -strict -2 -f dts es.dts
g "flac"              $A  -c:a flac es.flac
g "truehd"            $A  -c:a truehd -f truehd es.thd
g "mp4 av1"     $V -c:v libsvtav1 -pix_fmt yuv420p iso_av1.mp4
g "mkv av1"     $V -c:v libsvtav1 -pix_fmt yuv420p mkv_av1.mkv
g "ivf av1"     $V -c:v libsvtav1 -pix_fmt yuv420p -f ivf ivf_av1.ivf
g "webm vp8+vorbis" $V $A -c:v libvpx -c:a vorbis -strict -2 -pix_fmt yuv420p wm_vp8_vorbis.webm
g "ogg vorbis"  $A -c:a vorbis -strict -2 ogg_vorbis.ogg
g "ogx vorbis"  $A -c:a vorbis -strict -2 -f ogg ogg.ogx
g "truehd"      $A2 -c:a truehd -f truehd es.thd
g "mlp"         $A2 -c:a mlp -f mlp es.mlp
g "mka truehd"  $A2 -c:a truehd -f matroska mkv_truehd.mka
g "mov cineform" $V -c:v cfhd -pix_fmt yuv422p10le cineform.mov
g "mxf dv"      $V -c:v dvvideo -pix_fmt yuv411p -s 720x480 -r 30000/1001 dv.mxf
g "avi dv"      $V -c:v dvvideo -pix_fmt yuv411p -s 720x480 -r 30000/1001 -f avi dv.avi
g "mkv ffv1"    $V -c:v ffv1 -pix_fmt yuv420p ffv1.mkv
g "mkv dnxhd"   -f lavfi -i testsrc2=s=1920x1080:r=25:d=1 -c:v dnxhd -b:v 36M -pix_fmt yuv422p dnxhd.mkv
g "dsf"         $A2 -c:a dsd_lsbf_planar -ar 352800 -f dsf dsd.dsf
g "mp4 prores"  $V -c:v prores_ks -profile:v 3 -f mp4 prores_in.mp4
echo "--- subtitles ---"
printf '1\n00:00:00,000 --> 00:00:02,000\nHello world\n\n2\n00:00:02,000 --> 00:00:04,000\nSecond line\n' > sub.srt && echo "OK   srt"
ffmpeg $Q -i sub.srt sub.ass && echo "OK   ass"
ffmpeg $Q -i sub.srt -f webvtt sub.vtt && echo "OK   vtt"
ffmpeg $Q -i sub.srt -f ass sub.ssa && echo "OK   ssa"
g "mkv +srt"  -i mkv_h264_aac.mkv -i sub.srt -c copy -c:s srt mkv_subs.mkv
g "mkv +ass"  -i mkv_h264_aac.mkv -i sub.ass -c copy -c:s ass mkv_ass.mkv
g "mp4 +tx3g" -i iso_h264_aac.mp4 -i sub.srt -c copy -c:s mov_text mp4_subs.mp4
echo "--- tags ---"
g "mp3+id3v2.4" $A -c:a libmp3lame -id3v2_version 4 -metadata title=TestTitle -metadata artist=TestArtist -metadata album=TestAlbum tag_id3v24.mp3
g "mp3+id3v2.3" $A -c:a libmp3lame -id3v2_version 3 -metadata title=TestTitle -metadata artist=TestArtist tag_id3v23.mp3
g "flac+vorbis" $A -c:a flac -metadata title=TestTitle -metadata artist=TestArtist tag_vorbis.flac
g "m4a+ilst"    $A -c:a aac -metadata title=TestTitle -metadata artist=TestArtist tag_ilst.m4a
g "ogg+vorbis"  $A -c:a vorbis -strict -2 -metadata title=TestTitle tag_vorbis.ogg
g "wv+ape"      $A -c:a wavpack -metadata title=TestTitle tag_ape.wv
g "mkv+tags"    $V -c:v libx264 -pix_fmt yuv420p -metadata title=TestTitle mkv_tags.mkv
g "ogg vorbis"  $S -c:a vorbis -strict -2 -sample_fmt fltp ogg_vorbis.ogg
g "ogx vorbis"  $S -c:a vorbis -strict -2 -sample_fmt fltp -f ogg ogg.ogx
g "webm vp8+vorbis" $V $S -c:v libvpx -c:a vorbis -strict -2 -sample_fmt fltp -pix_fmt yuv420p wm_vp8_vorbis.webm
g "truehd"      $S -c:a truehd -sample_fmt s16 -strict -2 -f truehd es.thd
g "mlp"         $S -c:a mlp -sample_fmt s16 -strict -2 -f mlp es.mlp
g "mka truehd"  $S -c:a truehd -sample_fmt s16 -strict -2 -f matroska mkv_truehd.mka
g "mp4 prores"  $V -c:v prores_ks -profile:v 3 -tag:v apch -f mp4 -strict -2 prores_in.mp4
g "dsf"         $S -c:a dsd_lsbf -ar 2822400 -f dsf dsd.dsf
g "truehd" -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=2" -af "aformat=sample_fmts=s16p:channel_layouts=stereo" -c:a truehd -f truehd es.thd
g "mlp"    -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=2" -af "aformat=sample_fmts=s16p:channel_layouts=stereo" -c:a mlp -f mlp es.mlp
g "mka truehd" -f lavfi -i "sine=frequency=440:sample_rate=48000:duration=2" -af "aformat=sample_fmts=s16p:channel_layouts=stereo" -c:a truehd -f matroska mkv_truehd.mka
g "mp4 prores" -i prores_p3.mov -c copy -f mp4 prores_in.mp4
g "mkv prores" -i prores_p3.mov -c copy -f matroska prores_in.mkv
g "prores 4444 alpha" -f lavfi -i "testsrc2=s=320x240:r=25:d=1" -vf "format=yuva444p10le" -c:v prores_ks -profile:v 4444 prores_alpha.mov
g "prores tagged" -f lavfi -i "testsrc2=s=320x240:r=25:d=1" -c:v prores_ks -profile:v 3 -color_primaries bt709 -color_trc bt709 -colorspace bt709 prores_tagged.mov
g "dsf" -f lavfi -i "sine=frequency=440:sample_rate=44100:duration=1" -af "aformat=channel_layouts=stereo" -c:a dsd_lsbf_planar -ar 2822400 dsd.dsf
g "pr 23.976" -f lavfi -i testsrc2=s=320x240:r=24000/1001:d=1 -c:v prores_ks -profile:v 3 pr_fps23976.mov
g "pr 24"     -f lavfi -i testsrc2=s=320x240:r=24:d=1        -c:v prores_ks -profile:v 3 pr_fps24.mov
g "pr 29.97"  -f lavfi -i testsrc2=s=320x240:r=30000/1001:d=1 -c:v prores_ks -profile:v 3 pr_fps2997.mov
g "pr 50"     -f lavfi -i testsrc2=s=320x240:r=50:d=1        -c:v prores_ks -profile:v 3 pr_fps50.mov
g "pr 59.94"  -f lavfi -i testsrc2=s=320x240:r=60000/1001:d=1 -c:v prores_ks -profile:v 3 pr_fps5994.mov
g "pr 60"     -f lavfi -i testsrc2=s=320x240:r=60:d=1        -c:v prores_ks -profile:v 3 pr_fps60.mov
g "pr tff"    -f lavfi -i testsrc2=s=720x576:r=25:d=1 -vf "setfield=tff,format=yuv422p10le" -flags +ilme+ildct -c:v prores_ks -profile:v 3 -field_order tt pr_tff.mov
g "pr bff"    -f lavfi -i testsrc2=s=720x576:r=25:d=1 -vf "setfield=bff,format=yuv422p10le" -flags +ilme+ildct -c:v prores_ks -profile:v 3 -field_order bb pr_bff.mov
g "pr 16:9"   -f lavfi -i testsrc2=s=1280x720:r=25:d=1 -c:v prores_ks -profile:v 3 pr_169.mov
g "pr anam"   -f lavfi -i testsrc2=s=720x480:r=25:d=1 -c:v prores_ks -profile:v 3 -aspect 16:9 pr_anamorphic.mov
g "pr bt2020" -f lavfi -i testsrc2=s=320x240:r=25:d=1 -c:v prores_ks -profile:v 3 -color_primaries bt2020 -color_trc smpte2084 -colorspace bt2020nc pr_hdr.mov
g "pr a16"    -f lavfi -i testsrc2=s=320x240:r=25:d=1 -vf format=yuva444p10le -c:v prores_ks -profile:v 4444 -alpha_bits 16 pr_alpha16.mov
g "pr a8"     -f lavfi -i testsrc2=s=320x240:r=25:d=1 -vf format=yuva444p10le -c:v prores_ks -profile:v 4444 -alpha_bits 8 pr_alpha8.mov
g "pr a0"     -f lavfi -i testsrc2=s=320x240:r=25:d=1 -vf format=yuva444p10le -c:v prores_ks -profile:v 4444 -alpha_bits 0 pr_alpha0.mov
### --- Apple ProRes: the full profile ladder plus spec variants ---
for prof in 0 1 2 3 4 5; do
  g "prores profile $prof" -f lavfi -i testsrc2=s=320x240:r=25:d=1 \
    -c:v prores_ks -profile:v $prof "prores_p${prof}.mov"
done
g "prores in matroska" -i prores_p3.mov -c copy -f matroska prores_in.mkv
g "prores alpha"       -f lavfi -i testsrc2=s=320x240:r=25:d=1 -vf format=yuva444p10le \
  -c:v prores_ks -profile:v 4444 prores_alpha.mov
g "prores tagged"      -f lavfi -i testsrc2=s=320x240:r=25:d=1 -c:v prores_ks -profile:v 3 \
  -color_primaries bt709 -color_trc bt709 -colorspace bt709 prores_tagged.mov

### --- DV in both line systems, whose bit rate is structural ---
g "dv pal"   -f lavfi -i testsrc=s=720x576:r=25:d=2 -c:v dvvideo -pix_fmt yuv420p dv_pal.avi

### --- Vorbis comments in Ogg ---
g "ogg + vorbis comments" $S -c:a vorbis -strict -2 -sample_fmt fltp \
  -metadata title=TestTitle -metadata artist=TestArtist tag_vorbis.ogg


echo
echo "corpus: $ok generated, $skipped skipped, $(ls -1 | wc -l | tr -d ' ') files in $M"
