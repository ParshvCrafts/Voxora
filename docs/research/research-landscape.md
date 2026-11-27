# Dictation landscape (checked 2026-11-26)

Method: GitHub numbers come from api.github.com/repos/<owner>/<repo> (stars, SPDX license, pushed_at) and /releases/latest, fetched 2026-11-26. Vendor facts come from vendor pages fetched the same day. "unverified" means I could not confirm it from a primary source. Page summaries were produced by a small model, so exact prices on pages that looked inconsistent are flagged. Voxora's own properties (MIT, local whisper.cpp large-v3-turbo, Qwen3.5-4B cleanup, context modes, code/math/LaTeX) come from the owner's brief and are not independently checked here.

## Open source (GitHub API, 2026-11-26)

| Tool | License | Stars | Last release / last push | Platforms | Offline | AI cleanup | Per-app context | Dictionary | Notes / URL |
|---|---|---|---|---|---|---|---|---|---|
| Handy (cjpais/Handy) | MIT | 40,372 | v0.10.2, 2026-11-24 (push 2026-11-26) | Win, macOS, Linux | Yes: Whisper (Small..Large), Parakeet V3, custom GGML; Silero VAD | README mentions "post-processing" hook only; details unverified | not documented in README | not documented (Raycast mention only) | Push-to-talk. Windows cannot paste into admin apps unless run as admin; Wayland limited. https://github.com/cjpais/Handy , https://raw.githubusercontent.com/cjpais/Handy/main/README.md |
| OpenWhispr (OpenWhispr/openwhispr) | MIT | 12,277 | v1.12.2, 2026-11-25 | macOS, Win, Linux | Optional: local Parakeet/Whisper/Cohere; cloud via BYOK; hosted paid cloud tier | local LLM via llama.cpp credited; agent supports cloud LLMs (BYOK) | unverified | Yes (free feature) | Free: local or BYOK; hosted free 2,000 words/wk; paid cloud adds sync/mobile. https://github.com/OpenWhispr/openwhispr |
| VoiceInk (Beingpax/VoiceInk) | GPL-3.0 per README; GitHub API reports "Other/NOASSERTION" (inconsistent metadata) | 7,222 | v2.27, 2026-11-11 | macOS 15+ only | Yes (local models; cloud optional) | Yes (AI enhancement, Smart Modes) | Yes ("Modes" with app/URL detection) | Yes (words + replacements) | Source-available/OSS but sold: $25 / $39 / $49 one-time (1/2/3 Macs); free trial; Homebrew cask. README claims "99% accuracy" with no data (vendor claim). https://tryvoiceink.com , https://raw.githubusercontent.com/Beingpax/VoiceInk/main/README.md |
| Amical (amicalhq/amical) | MIT | 1,660 | v1.13.2, 2026-11-12 | macOS, Windows | Yes (Whisper, Parakeet) | unverified | unverified | unverified | "Free, open source voice typing for macOS and Windows." Closest like-for-like OSS competitor on platforms. https://github.com/amicalhq/amical |
| Whispering (EpicenterHQ/epicenter, apps/whispering) | AGPL-3.0-or-later (repo LICENSE index; MIT tier dissolved 2026-08; API shows "Other") | 4,844 (whole monorepo) | v7.11.0, 2025-12-27 (push 2026-11-26) | README: desktop built for macOS; "Windows and Linux are untested" | Optional: local models in desktop app; Ollama/LM Studio; OpenAI-compatible endpoints | Transformations deferred/removed in current docs | no | no | No push-to-talk (single toggle shortcut). Repo is mid-restructure; verify before citing. https://raw.githubusercontent.com/EpicenterHQ/epicenter/main/apps/whispering/README.md |
| Vibe (thewh1teagle/vibe) | MIT | 8,245 | v3.3.8, 2026-11-21 | macOS, Win, Linux | Yes | Ollama/Claude summaries | n/a | n/a | Primarily file transcription, not dictation. https://github.com/thewh1teagle/vibe |
| Buzz (chidiwilliams/buzz) | MIT | 23,116 | v1.4.5, 2026-08-23 | Win, macOS, Linux | Yes | n/a | n/a | n/a | Transcription/translation app, NOT dictation. https://github.com/chidiwilliams/buzz |
| whisper-writer (savbell) | GPL-3.0 | 1,107 | last push 2024-08-24 | Python, cross-platform | Optional | n/a | n/a | n/a | Effectively dormant (no push in 27 months). https://github.com/savbell/whisper-writer |
| nerd-dictation (ideasman42) | GPL-3.0 | 1,950 | last push 2025-10-10 | Linux (VOSK) | Yes | n/a | n/a | n/a | Linux-only, VOSK engine (engine detail unverified here). https://github.com/ideasman42/nerd-dictation |
| drajb/whisper-local | MIT | 206 | last push 2026-11-17 | Win, macOS | Yes | voice commands | unverified | unverified | Small but same pitch as Voxora. https://github.com/drajb/whisper-local |
| Other 2026 small OSS entrants | MIT mostly | 3-97 | various | various | various | some with LLM cleanup | | | Search listing (api.github.com/search/repositories, 2026-11-26): oliv, TuxTalk, Golosok, Plume, VoiceTyper, xhisper, windictate, etc. Crowded long tail of tiny projects; also OpenTypeless, Yap, WhisprTypr, Local-Dictate surfaced in search, not individually verified. |

WhisperTyping: closed-source, Windows-only, cloud (FAQ: "requires an internet connection"), free tier plus about $5/mo unlimited per vendor comparison page (Capterra says contact vendor; sources disagree). https://whispertyping.com/support/faq/

## Commercial / built-in

| Tool | License | Price (verified source) | Platforms | Offline | Cloud for AI cleanup | Context/modes | Dictionary | Published accuracy/latency |
|---|---|---|---|---|---|---|---|---|
| Wispr Flow | Closed | Free (weekly word cap); Pro $15/mo or $12 annual; Growth $23/$18 (https://wisprflow.ai/pricing). TechCrunch: free 2,000 words/wk | Mac, Win, iPhone, Android | No: privacy page says "Transcription always happens in the cloud" (https://wisprflow.ai/privacy) | Yes, all cloud | Context-aware per vendor (page vague) | Yes (+snippets) | Wispr's own blog: 96-98% (no method; vendor claim) |
| Superwhisper | Closed | Free (local Whisper, 2 modes, no AI); Pro $8.49/mo, $84.99/yr, $249.99 lifetime (https://superwhisper.com/docs/billing/plans.md). Pricing page showed odd "$849"; docs used. | Mac, Win, iOS, Android, Linux (site) | Yes, local models (best on Apple Silicon) | Optional (cloud LLMs, BYO keys) | Yes: modes + custom modes | Yes | "~80 ms" is a vendor claim repeated by TechCrunch; unverified |
| MacWhisper | Closed | Free; Pro EUR 64 one-time (https://www.macwhisper.com) | Mac (+ iOS/iPad mentioned) | Yes, local | Optional (Ollama/LM Studio local, others) | dictation + auto cleanup; modes unverified | not mentioned | none found |
| Aqua Voice | Closed | Free 2,000 words; Pro $10 annual / $12 monthly; Max $19/$24 (https://aquavoice.com/pricing) | Mac, Win, iOS, Android | Not stated (unverified) | Cloud (Avalon model) | Deep Context, custom instructions | Yes (Pro+) | Own "AISpeak" benchmark, 965 ms end-to-end, self-reported (https://aquavoice.com/blog/best-dictation-apps-compared-2026 via search summary; page 404 on direct fetch) |
| Willow Voice | Closed | Free 2,000 words/mo; Pro $15/mo or $12 (https://willowvoice.com/pricing) | Mac, Win, iPhone | Page lists "Offline mode" | Cloud default | style/tone per app | Yes | Vendor: text in "as little as 200 ms", "3x more accurate than built-in" (unverified) |
| Monologue (Every) | Closed | $15/mo or $144/yr; trial 1,000 words (https://www.monologue.to) | Mac, iPhone, iPad, Watch (no Win) | Local models on Mac; formatting/sync need internet | Yes for smart formatting | Modes (Chat, Email, Notes, Coding) | Yes | none |
| Typeless | Closed | Free 4,000 words/wk; $12/mo annual (TechCrunch) | Win, Mac | unverified | unverified | unverified | unverified | none |
| VoiceTypr | Closed? | $35 lifetime 1 device (TechCrunch) | Mac, Win | Offline-first | unverified | unverified | unverified | none |
| Dragon Professional v16 (Nuance/Microsoft) | Closed | About $699 one-time, Windows only; Mac ended 2018; cloud Anywhere by quote (reseller/competitor blogs, not Microsoft; Nuance URL redirects to microsoft.com/health-solutions). Possible end-of-sale 2026-12-31 reported by one reseller, unconfirmed | Windows | Local desktop edition | n/a | custom commands | Yes | none verified |
| Windows Voice Typing (Win+H) | OS feature | Free | Windows | No: needs internet, Azure Speech. Copilot+ PCs add on-device "fluid dictation" SLM cleanup (English) | Cloud | none | none stated | none (https://support.microsoft.com/en-us/windows/use-voice-typing-to-talk-instead-of-type-on-your-pc-fec94565-c4bd-329d-e59a-af033fa5689f) |
| macOS Dictation | OS feature | Free | macOS | On-device option in settings; macOS 27 Apple Intelligence cleanup on M3+/12 GB | partial | none | none | none (https://support.apple.com/guide/mac-help/use-dictation-on-your-mac-mh40584/mac) |
| Talon Voice | Closed (EULA; license not stated on site) | "Free to use", funded by Patreon donations/beta access (https://talonvoice.com , https://talon.wiki/) | macOS, Windows, Linux X11 | Not stated on pages (unverified; commonly local) | n/a (command-based, not LLM) | scripting via .talon files | scripts | none found. It is the incumbent for spoken code via commands, a different paradigm |

## Whisper accuracy sources

- Open ASR Leaderboard paper (arXiv 2510.06961 v4, https://arxiv.org/html/2510.06961v4), checked 2026-11-26. English short-form average WER over the leaderboard datasets: large-v3 7.44 (RTFx 146), large-v3-turbo 7.83 (RTFx 200). Long-form: large-v3 11.2 (RTFx 68.6), turbo 11.0 (148). Per-dataset (LibriSpeech/Common Voice) values were not in the fetched text.
- HF model card (https://huggingface.co/openai/whisper-large-v3-turbo): turbo is large-v3 with decoder layers cut 32 to 4, 809M vs 1,550M params, "minor quality degradation"; card sidebar lists mean WER 7.83 / RTFx 200 from the open-asr-leaderboard.
- Secondary, via search summary only (not primary-verified): one paper reports large-v3 LibriSpeech 2.03 clean / 3.90 other; another lists large-v3 2.2 vs turbo 2.4 on LibriSpeech clean; WhisperKit CoreML results show 2.04 vs 2.03. Treat as "turbo costs roughly 0.1-0.4 WER points, about 5% relative, for about 1.4x faster". Source for the per-dataset numbers: unverified.
- Leaderboard Space itself did not render for the fetcher; the live table was not read.

## Independent reviews 2025-2026

- TechCrunch, "The best AI-powered dictation apps of 2025", Ivan Mehta, 2026-05-02 (https://techcrunch.com/2026/05/02/the-best-ai-powered-dictation-apps-of-2025). Editorial roundup of pricing and features, not a WER test. Lists Handy as the only free open-source entry; VoiceInk listed as Mac-only $25 lifetime.
- mezha.net "We test and rank the best AI dictation apps" (403 on fetch; contents unverified).
- AssemblyAI "best Wispr Flow alternatives" (Sept 2026, vendor of a speech API, via search summary): notes Windows options are few and OSS Mac apps drop out on Windows.
- Aqua, Willow, Wispr and getvoibe/usevoicy/spokenly blogs are competitor-authored. I found NO independent, methodologically documented WER/latency head-to-head of these apps. That absence is itself a finding.

## Assessment

Genuinely distinctive on verifiable dimensions (relative to what I verified):
1. Free + MIT + fully local + Windows-first, together. Wispr, Aqua, Willow, Superwhisper, MacWhisper, VoiceInk are closed or paid (VoiceInk is GPL and $25 on Mac only). Free MIT local peers exist though: Handy (40k stars), OpenWhispr, Amical. So "free open-source local dictation" alone is NOT unique.
2. Where Voxora can differ from Handy/OpenWhispr/Amical (from their READMEs; none document it): a local LLM cleanup pass with a safety guard, per-app context modes, spoken code/math/LaTeX. Handy's README documents no per-app modes or custom-word handling; OpenWhispr's none for per-app. These are "not documented", not "absent"; re-check their settings before claiming absence.
3. Math/LaTeX dictation: no competitor I checked advertises it. Monologue has a Coding mode; Talon does code via commands. Unverified that nobody has it; phrase as "built-in" not "only".

Competitors clearly ahead: user base and polish (Handy 40k stars, OpenWhispr 12k, Superwhisper "hundreds of thousands" claimed), mobile (Wispr, Superwhisper, Aqua, Willow, Monologue have iOS/Android), Linux (Handy, OpenWhispr, Superwhisper), cloud-model accuracy/latency and hosted team features (SOC 2, HIPAA), mature voice commands/editing (Talon, Dragon, Aqua Realtime), accessibility/hands-free control (Talon, Dragon), existing signed macOS builds (VoiceInk, Superwhisper; Voxora macOS is beta).

Defensible one-liners:
- "Free, MIT-licensed dictation for Windows and macOS that runs entirely on your machine, including optional AI cleanup." (True if Voxora's defaults are local; Handy also fits the first half, so keep the AI cleanup clause.)
- "Hold a hotkey, speak, and get cleaned-up text pasted into any app, with no account, no cloud and no word limits." (Contrasts with Wispr/Aqua/Willow caps and cloud; Superwhisper free also has no cap for local Whisper, so do not claim uniqueness over it.)
- "Dictate code, math and LaTeX by voice with per-app modes, locally." Say "built in" rather than "the only".

Avoid:
- "The only open-source/offline dictation app" (Handy, OpenWhispr, Amical, Whispering exist).
- "More accurate than Wispr/Superwhisper/Aqua" or any WER/latency superiority without your own bench; no independent third-party comparison exists and vendors' numbers are self-reported.
- "Best Windows dictation" (Win+H is built in; Dragon exists; Wispr/Aqua are polished on Windows).
- "Large-v3-turbo is as accurate as large-v3": leaderboard average is 7.83 vs 7.44 WER on short-form English.
- "Private by default" for the LLM step unless the model is bundled and verified offline; "no data leaves your machine" must hold for the update check and model download too.
- Any statement about competitors' missing features based on READMEs; say "not documented".
