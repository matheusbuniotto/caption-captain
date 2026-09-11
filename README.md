<p align="center">
  <img src="assets/gemini-banner.jpg" alt="Captain Caption" width="100%">
</p>

# ⚓ capcap — Captain Caption

Ahoy. Your video's got no subtitles and ye keep shippin' it off to some cloud server to get 'em? That's a landlubber's move — slow, costly, and ye don't know who's readin' yer footage on the way.

**`capcap` does the job right here, on yer own machine. No cloud, no upload, no accounts. Just point it at a video and it hands ye back a captioned one.**

## What she does

- Transcribes yer video's audio locally with a bundled Whisper model — no network needed.
- Writes a clean `.srt` sidecar every time.
- Embeds the captions straight into the video where the container allows it, with zero re-encoding (stream copy, not a re-render):
  - `.mp4` / `.mov` → native `mov_text` track, ready for QuickTime.
  - `.mkv` → native SRT subtitle stream.
  - Anything else (`.avi`, etc.) → sidecar `.srt` only, plus a warning — embedding isn't reliable there, so we don't fake it.
- Never touches yer original file.

## Setting sail

```bash
capcap run "movie.mp4"
```

That's the whole voyage. Find `movie.srt` and `movie.captioned.mp4` next to yer original when she's done.

Skip the embed, sidecar only:

```bash
capcap run "movie.mp4" --no-embed
```

Force the spoken language instead of auto-detect:

```bash
capcap run "movie.mp4" --lang en
```

## Why local-first

- **Privacy** — yer footage never leaves the ship.
- **Speed** — no upload queue, no waitin' on someone else's server.
- **QuickTime compatibility** — Apple's players won't pick up a loose `.srt`; they want it embedded. `capcap` handles that muxing for ye.

## Status

Core CLI (transcribe → sidecar → container-aware mux) is seaworthy. A drag-and-drop desktop app for non-technical crew is charted on the roadmap — see the open issues for what's next.

---

*No cloud APIs were harmed in the making of this README.*
