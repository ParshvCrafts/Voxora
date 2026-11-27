# How Voxora compares

Checked 2026-11-26 against each project's own site, docs, pricing page or GitHub repository.
Prices, limits and star counts change; if something here is out of date, please open an issue.
"Not documented" means the project's README or site does not mention it, not that it is absent.

## The short version

- **Voxora is not the only free, open-source, offline dictation app.** [Handy](https://github.com/cjpais/Handy)
  (MIT, about 40k stars, Windows, macOS and Linux), [OpenWhispr](https://github.com/OpenWhispr/openwhispr)
  (MIT, about 12k stars) and [Amical](https://github.com/amicalhq/amical) (MIT) also run Whisper
  locally. Handy is the most popular and supports Linux, which Voxora does not.
- **What Voxora adds** is the layer after speech recognition, aimed at people who dictate into
  code editors, AI chats and LaTeX: rules for spoken code, paths, URLs and math; modes that
  follow the app you are typing in; and an optional local AI rewrite checked by a guard that
  rejects any rewrite that adds or drops a number or identifier. None of the open-source
  projects above documents a guard like this, per-app modes, or spoken math and LaTeX.
- **Voxora publishes reproducible numbers.** The [benchmarks](BENCHMARKS.md) (cases, audio
  script, per-case output) are in the repo. No independent, methodical accuracy or latency
  comparison of dictation apps exists that we could find; vendor figures are self-reported.
  We have not benchmarked other apps, so we make no claim that Voxora is more accurate than
  any of them.

## Feature table

| | Voxora | Handy | OpenWhispr | Wispr Flow | Superwhisper | VoiceInk | Windows Win+H |
|---|---|---|---|---|---|---|---|
| License | MIT | MIT | MIT | Closed | Closed | GPL-3.0 (per README) | Built in |
| Price | Free | Free | Free (local or own keys); paid hosted tier | Free with weekly word cap; Pro $15/mo | Free local tier; Pro from $8.49/mo | $25 one-time | Free |
| Windows / macOS / Linux | Yes / beta / no | Yes / yes / yes | Yes / yes / yes | Yes / yes / no | Yes / yes / yes | No / yes / no | Yes / no / no |
| Works fully offline | Yes | Yes | Optional | No ("transcription always happens in the cloud") | Yes, local models | Yes | No (Azure Speech) |
| AI cleanup | Local LLM, optional, with a guard | Post-processing hook | Local llama.cpp or cloud | Cloud | Optional, cloud or own keys | Yes | Copilot+ PCs only |
| Modes per app | Yes | Not documented | Not documented | Context-aware (vendor) | Yes | Yes | No |
| Spoken code, math and LaTeX rules | Built in | Not documented | Not documented | Not documented | Not documented | Not documented | No |
| Custom vocabulary | Yes | Not documented | Yes | Yes | Yes | Yes | No |
| Published, reproducible benchmark | Yes | Not found | Not found | Vendor claim only | Vendor claim only | Vendor claim only | No |

Sources: [Handy README](https://github.com/cjpais/Handy), [OpenWhispr](https://github.com/OpenWhispr/openwhispr),
[Wispr Flow pricing](https://wisprflow.ai/pricing) and [privacy](https://wisprflow.ai/privacy),
[Superwhisper plans](https://superwhisper.com/docs/billing/plans.md), [VoiceInk](https://tryvoiceink.com),
[Windows voice typing](https://support.microsoft.com/en-us/windows/use-voice-typing-to-talk-instead-of-type-on-your-pc-fec94565-c4bd-329d-e59a-af033fa5689f).

## Where others are ahead

- **Reach and polish:** Handy and OpenWhispr have far larger user bases and more releases.
- **Platforms:** Linux (Handy, OpenWhispr, Superwhisper) and phones (Wispr Flow, Superwhisper,
  Aqua, Willow). Voxora's macOS build is a beta that has not been used on a real Mac yet.
- **Signed builds:** Voxora's installers are unsigned, so Windows SmartScreen and macOS
  Gatekeeper warn on first run.
- **Voice control:** Talon and Dragon drive the whole computer by voice; Voxora only dictates.
- **Cloud accuracy:** cloud services can run larger models than a laptop GPU.

## Speech recognition model

Voxora uses Whisper large-v3-turbo. On the Hugging Face Open ASR Leaderboard
([paper](https://arxiv.org/html/2510.06961v4)), its average English short-form word error rate
is 7.83%, against 7.44% for the larger large-v3, while running about 1.4 times faster
(RTFx 200 vs 146). Voxora trades that 0.4 points of accuracy for speed, then relies on the
dictionary prompt and the rules to fix the words that matter most to its users.

## Claims we make, and claims we do not

We say: free, MIT-licensed dictation for Windows and macOS that runs entirely on your computer,
including the optional AI rewrite; no account, no cloud, no word limits; spoken code, math and
LaTeX built in.

We do not say: the only offline or open-source dictation app; the most accurate; the fastest;
better than any named product.
