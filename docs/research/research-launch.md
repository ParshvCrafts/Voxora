# Voxora launch research (2026-11-26)

Evidence grade key: [OFFICIAL] primary rule/doc; [DATA] measured dataset or peer-reviewed; [VENDOR] marketing-tool analysis; [ANECDOTE] single case; [MINE] I queried it live today (HN Algolia API, GitHub API, repo files).
Not reachable: reddit.com blocks automated fetch (WebFetch refused, curl got an HTML wall), so subreddit rules below come from secondary sources and MUST be re-read by hand before posting.

## 1. Show HN

### Official rules [OFFICIAL] https://news.ycombinator.com/showhn.html
- Qualifies: something people can run on their own computer; non-trivial, you personally worked on it and can discuss it; others can try it; early-stage OK.
- Does not qualify: blog posts, sign-up pages, landing pages, fundraisers, quickly generated one-offs, work not ready to try.
- Make it easy to try without signup/email. Do NOT ask friends to upvote or comment (not allowed).
- Title must begin "Show HN". Shows on /shownew first, moves to /show after a small points threshold.
- Voting-ring detection exists; asking for upvotes can keep a post off the front page (secondary sources: https://github.com/minimaxir/hacker-news-undocumented). Moderator tip (dang, via search summary): put your explanatory text at top of the submission or as first comment. https://news.ycombinator.com/newsguidelines.html
- Gap: no first-source data found on which first-comment styles work. Everything on that is opinion.

### Base rates and star conversion
- [DATA, single-author blog, 188,085 Show HN posts, 14 yrs] median score 2, mean 13.5; 50+ points = top 6%, 250+ = top 1%. ~1.4 GitHub stars per HN point within 48h (median); conversion sublinear (about 1.77 stars/pt at 258-350 pts, 0.79 at 700+). Median successful project (258+) gains ~509 stars on day 1. Impact half-life ~24h, ~92% over by 48h. HN score explains only ~8% of star variance (r=0.29). ~28,302 Show HN posts in 2025 (~200/day). https://danfking.github.io/blog/2026/04/23/show-hn-by-the-numbers/
- [DATA, arXiv preprint, 138 AI-tool repos 2024-25, observational] mean +121 stars at 24h, +189 at 48h, +289 at 1 week (means; medians much lower). Show HN tag itself: no significant advantage after controls (-119 stars, p=0.39). https://arxiv.org/html/2511.04453v1
- [DATA, arXiv] 2,195 HN AI-repo stories: stars, forks, contributors all significantly higher in the month after posting vs month before (163 repos; p=0.001/0.004/0.021), no effect sizes, no control group. https://arxiv.org/html/2506.12643v1
- [DATA, conference paper cited in search results] difference-in-differences over 18,253 repos: HN appearance raises stars/forks; effect mainly for smaller projects (Voxora fits). I did not open the paper itself.
- [ANECDOTE] Nebula Graph: ~18h on front page, 2,248 repo visitors, 300+ stars https://www.hackernoon.com/how-hacker-news-home-page-got-us-300-stars-on-github-in-24-hours-mp153w7f ; Encore 182 pts -> "How we got 1500 stars" https://encore.dev/blog/how-we-got-1500-stars ; PostHog post-HN https://posthog.com/blog/after-the-hn-launch ; Skyvern 10 upvotes, never left /new https://www.skyvern.com/blog/we-open-sourced-and-ended-up-1-on-hackernews/

### Same-niche evidence [MINE, HN Algolia API, Show HN posts since 2025-01-01, top relevance hits for "dictation"/"speech-to-text"/"whisper"]
Direct competitors on Show HN (points / comments / UTC time posted):
- Whispering (open-source local-first dictation): 591 / 152, Mon 16:52 UTC (2025-08-18). Repo braden-w/whispering shows 290 stars (monorepo now EpicenterHQ/epicenter 4,844 stars; so points did not equal stars for the app repo).
- Ghost Pepper (local hold-to-talk macOS): 467 / 200, Mon 19:50 UTC. Repo created 2026-03-20, now 3,241 stars.
- BetterWispr (free OSS dictation for Mac): 220 / 128, Sun 12:01 UTC (2026-10-11). Repo opennookorg/betterwispr created 2026-10-07, 547 stars now.
- Yap 104 / 44 (Mon 18:36); Qwen Scribe 97 / 21 (Wed 14:47).
- Many others scored 5-36: FnScribe 36, Chirp (Windows, Parakeet) 34, Sayso (Mac/Win/Linux OSS) 25, Ito AI 16, Wordmate 5, Open WhisperScribe 5, TongueType 5.
- Reading: the category is crowded (a dozen+ dictation Show HNs in ~2 years), median outcome is under 40 points, but the top performers hit 200-590 and each had a distinct hook (trust/open-source local-first; hold-to-talk simplicity; free). Windows-first local dictation (Chirp) got only 34. Note sample is relevance-ranked hits, not a census.
- Context repos: Handy (Tauri/Rust/whisper dictation, created 2025-02, 40,373 stars), VoiceInk 7,222, TypeWhisper-mac 1,941 [MINE via GitHub API]. Voxora's differentiator must be the code/math/LaTeX angle, not "local dictation" in general.

### Best day/time (evidence conflicts; no analysis of Show HN front-page-by-hour that is rigorous)
- [DATA, 188k Show HN scrape] best P(50+ pts) slots: Mon 00:00 UTC (Sun 7pm ET) 10.8%; Sun 02:00 UTC 9.8%; Sat 19:00 UTC 9.2%; worst Thu 06:00 UTC 2.6%. Mean score highest Fri 12-15 UTC and Sun 16-19 UTC. https://danfking.github.io/blog/2026/04/23/show-hn-by-the-numbers/
- [DATA, 2019, all HN not Show HN, 50+ upvotes proxy] Sunday 06:00 UTC ~2.5x likelier to reach front page than Wed 09:00 UTC; low competition wins, but fewer eyeballs. https://chanind.github.io/2019/05/07/best-time-to-submit-to-hacker-news.html
- [DATA, arXiv, AI tools] 12-17 UTC best for stars gained (US morning + EU afternoon); weekend vs weekday negligible (+10 stars, p=0.81). https://arxiv.org/html/2511.04453v1
- Note both competitors' hits: BetterWispr Sunday 12:00 UTC, Whispering Monday 16:52 UTC. Takeaway: opinion-level: Sunday-evening-ET to Monday-morning-ET window or Sun ~12-17 UTC is consistent with the data; nothing proves it.
- Base rate: ~2.3% of all submissions reached front page in Q1 2026 (search-summary of a secondary source; unverified).

## 2. Reddit (rules NOT verified at source; re-read each sidebar/wiki before posting)
- r/opensource: HIGH RISK. [per Linux Magazine, 2026-10-30, describing mod post of 2026-10-24] new rules: promoted projects must be ~1 year old, actively maintained, with multiple contributors (subjectively 3+), existing community, not a common capstone, not "intended for AI"; "If it's obvious you're using AI in your project, we don't want it." Permanent bans for unwanted posts incl. LLM output. https://www.linux-magazine.com/Online/News/r-opensource-Bans-AI-Assisted-and-AI-Related-Projects ; also https://www.theregister.com/a/5305392 . Voxora (days-old, 1 contributor, local LLM cleanup, student project) probably fails several. Recommendation (opinion): skip r/opensource. Older flair rule (2022): promotional posts must use the promotional flair.
- r/selfhosted: [per search summary of mod post, Q2 2026] standalone new-project posts removed; use the weekly "New Projects" megathread (open daily, run Fridays); include project name and an AI-involvement line. Also desktop dictation is off-topic for a self-hosting sub (it is for server apps). Skip or megathread only. (A "Vibe Code Friday" article contradicting this is low quality.)
- r/LocalLLaMA: [per wiki mirror https://r.datuan.dev/r/LocalLLaMA/wiki/index, not live] "1/10th rule": self-promotion should be at most ~10% of your content, disclose affiliation. Mods reported self-promotion (Rule 4) was the top violation after rules tightened: https://ai-radar.it/article/qualita-e-controllo-le-nuove-regole-di-r-localllama-migliorano-il-dibattito?lang=en . Best topical fit (local whisper.cpp + local LLM) but the account needs prior non-promo participation (opinion).
- r/rust: [mirror https://axebps-redlib.hf.space/r/rust/wiki/rules] self-promotion allowed within limits, commercial gets extra scrutiny, ask mods if unsure; follows Rust CoC. Show flair/weekly thread rules not verified.
- r/MacApps: [secondary] self-promotion welcome but posts must be tagged. Voxora macOS is beta; say so.
- r/software, r/windows, r/productivity, r/speechtech, r/commandline: nothing reliable found. Unverified. (Opinion: r/commandline is a poor fit, Voxora is a GUI app.)
- Generic: Reddit's "<10% self-promo" guideline is the widely-quoted norm.

## 3. Product Hunt
- [DATA, PH community post by Databox, all 2026 featured launches] Saturday has ~3x fewer launches than Tuesday (Tue avg 30.4/day, Sat 10.7); median points to reach top 5: Saturday 266, Monday 322, Wednesday 360 (hardest). Measures rank difficulty only, not signups or stars; author warns the advantage may erode now it's published. https://www.producthunt.com/p/databox/i-ve-analyzed-all-2026-ph-launches-to-find-the-best-day-to-launch-saturday-wins-and-it-s-not-close-2
- [DATA, small] 2023 Golden Kitty dev-tool nominees: 89% (16/18) launched on weekdays; OSS nominees 79% (11/14). https://dev.to/fmerian/i-analyzed-the-18-best-dev-tools-launched-on-product-hunt-in-2023-heres-what-i-found-4i73 (weekly-ranking caveat: Fri-Sun launches rarely win Product of the Week, per community posts.)
- [ANECDOTE] OSS tool 0 -> 168 stars over 3 weeks, ~80 on launch day, mostly pre-launch traction: https://dev.to/rahil_pirani_c48446facc8c/i-launched-an-open-source-tool-on-product-hunt-with-no-budget-no-team-and-no-audience-heres-57c3
- No study links Product Hunt to GitHub stars. PH audience skews to SaaS/consumer, not OSS. Claims of 2026 algorithm changes (account-age weighting, comment-quality) are from unsourced blogs; do not rely.

## 4. LinkedIn
- [VENDOR, Socialinsider, ~1.3M posts, 16,645 pages, 2024-25] native PDF/document carousels highest engagement rate (~7.0% 2025 vs video ~6.0%); multi-image highest likes. Company pages, not individuals. https://www.socialmediatoday.com/news/report-shows-document-posts-on-linkedin-see-more-engagement/816551/
- [VENDOR/DATA, MagicPost, 865,980 posts, 15,719 creators, Jul 2024-Jul 2026, within-creator comparison] posts with a link: -17% impressions vs same creator's non-link posts; -11% vs equally promotional posts. The penalty is mostly the link PREVIEW card (-35%, shrinking to -27% in 2026); a plain URL in text is ~-3%; link in first comment is no better than URL in post (-2%). Clicks per 1,000 impressions: image+URL 2.6, text+URL 1.6, preview alone 1.3. Not causal. https://magicpost.in/studies/linkedin-link-penalty
- Conflicting: Metricool 2025 found link posts +4.9% impressions; other vendors claim -30% to -60%. LinkedIn confirms none. Opinion: "link in comments" has no support in the best dataset; put the repo URL in text without the preview card, attach image/GIF/short native video.
- Students/recruiters: no rigorous survey exists. Consistent practitioner advice: pin 3-6 substantial repos with READMEs that run, avoid empty forks and padded graphs. https://underdog.io/blog/what-hiring-managers-look-for-on-github ; the "92% check contribution graph in 10 seconds" stat is unsourced, ignore. A real project with releases, CI, tests and a decisions doc (Voxora has these) is the evidence-light-but-consistent signal.

## 5. X / Bluesky
- [DATA, ICSE 2022 Distinguished Paper, Fang et al., 44,544 tweets, 2,370 repos, causal inference] tweets about a repo -> ~+7% new stars on average, only ~+2% commit authors ("more people know about a project, but don't necessarily commit"). Tweets from fans (not the developer) worked best; posts asking for help with a specific task got better response. https://conf.researchr.org/details/icse-2022/icse-2022-papers/84/-This-Is-Damn-Slick-Estimating-the-Impact-of-Tweets-on-Open-Source-Project-Populari ; https://www.cmu.edu/news/stories/archives/2022/june/open-source-software.html . Pre-2023 X data.
- 2018 IEEE Computer, top-100 projects: Twitter, meetups, blogs, HN most-used channels https://ar5iv.labs.arxiv.org/html/1908.04219
- Bluesky: no launch-level data for indie OSS. Mixed signals: mobile DAU -25.6% YoY (Jul), X ~302M MAU (TechCrunch via search). Opinion: low cost, low expected reach; post once on each, do not invest.

## 6. Awesome lists (verified via GitHub API / raw files 2026-11-26)
| List | Exists | Fit / rule |
|---|---|---|
| tauri-apps/awesome-tauri (8.2k stars, active) | yes | CONTRIBUTING says "Application submissions are not accepted anymore!" Only plugins/templates/guides/articles; plugins need Tauri 2, 30+ days old, signed commits. NOT a target. |
| rust-unofficial/awesome-rust (60.8k) | yes | Accept if stars > 50 OR crates.io downloads > 2000; format `[ACCOUNT/REPO](url) - DESC`, alphabetical. Has "Applications" section; already lists Tauri dictation apps Glimpse and LokalBot. Eligible after 50 stars. |
| sindresorhus/awesome-whisper (2.4k) | yes | "Apps" section lists TypeWhisper, VoiceInk, Ito, OpenSuperWhisper. Rule: open-source GitHub projects need >= 100 stars; PR title `Add Project Name`; no marketing taglines; one PR per item. Eligible at 100 stars. |
| serhii-londar/open-source-mac-os-apps (51.6k) | yes | Edit applications.json (not README); no star minimum stated; already lists dictation apps (Amical, Glimpse, Dictamelo...). Best first target for the macOS beta. |
| iCHAIT/awesome-macOS (19.5k) | yes | No Electron; no AI-prompt wrappers (local specialised ML models fine); free apps get Freeware symbol; OSS symbol; link to website. Tauri is not Electron but check maintainers' view. Needs a website. |
| awesome-selfhosted/awesome-selfhosted (340k) | yes | List is "network services and web applications hosted on your own server"; PRs go to awesome-selfhosted-data. Desktop dictation does NOT qualify. |
| pluja/awesome-privacy (20k) / Lissy93/awesome-privacy (10k) | yes | Privacy-respecting alternatives to services; plausible "dictation" alternative category but I did not verify contribution rules (misc/Contributing.md not read). |
| janhq/awesome-local-ai (2k) | yes but last push 2024-11 | stale; others (msb-msb 337, etc.) are tiny. Low value. |
| awesome-speech-recognition | only a papers list (3.1k, stale 2023) and tiny lists | skip |
| 0PandaDEV/awesome-windows (3.2k) | yes | not read; unverified rules. |
Note: another dictation app list candidates not checked: danielrosehill/Awesome-Whisper-Apps (17 stars).

## 7. GitHub repo signals
- No peer-reviewed study found for social preview image or topics effect on stars. Practitioner posts only. https://dev.to/iris1031/github-readme-best-practices-how-to-write-a-readme-that-gets-stars-2gb2 [VENDOR/ANECDOTE]
- [DATA, 2022 arXiv, 1,950 READMEs] popular projects' READMEs are well organized, use lists and images and link out (correlation). https://arxiv.org/abs/2206.10772
- [DATA, CSCW 2019, Qiu et al.; 15 interviews + 9,977 projects] contributors judge projects by README structure and recent activity; impolite language lowers contributor attraction. https://par.nsf.gov/biblio/10189104
- Good first issue: EASE 2026 longitudinal study (arXiv 2604.27532) and earlier work: many GFIs go unresolved by newcomers; I could not read numbers (PDF text unreadable). Treat GFI labels as necessary hygiene, not a proven growth lever.
- Tweets -> stars big, contributors tiny (ICSE 2022): attention does not convert to contributors; contributors need specific, scoped asks.
- Voxora repo today [MINE]: 1 star, topics already set (llama-cpp, local-first, macos, privacy, rust, speech-to-text, tauri, voice-dictation, whisper, windows).

## 8. Sequencing and timing
- Calendar (2026): Thu Nov 26 Thanksgiving; Fri Nov 27 Black Friday; Sat-Sun Nov 28-29; Mon Nov 30 Cyber Monday; Tue Dec 1 Giving Tuesday.
- [DATA, Cloudflare] US internet traffic ~10% lower on Thanksgiving than the prior week, mobile share higher (54.5%); no HN-specific holiday data found. https://blog.cloudflare.com/do-hackers-eat-turkey-and-other-thanksgiving-internet-trends/ . Lower HN competition could help (the 2019 study says low-activity slots help front-page odds) but the audience is also away; this is opinion, no evidence either way for Thanksgiving weekend.
- Evidence-backed principles: (1) HN/stars effect is front-loaded, 92% in 48h, so be ready (installer, README, issues triaged, Windows SmartScreen note) before posting; (2) most single launches fail (median 2 pts), so plan for several shots rather than one; HN allows reposting a project after a significant change. (3) Reddit needs account history (1/10th rule); (4) awesome lists need star thresholds (50 / 100), so they come AFTER launch stars, not before.
- Suggested order (opinion): finish launch assets -> r/MacApps/r/LocalLLaMA/r/rust soft launch for bugs and first stars -> open-source-mac-os-apps PR -> Show HN on a Sun evening ET / Mon morning ET after the holiday weekend with a distinct hook (code/math/LaTeX dictation, measured benchmark) -> awesome-rust at 50 stars, awesome-whisper at 100 -> LinkedIn post with GIF (text URL, no preview card) -> optional X/Bluesky/PH.
