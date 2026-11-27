// Records the demo's dictations into real websites in a throwaway Chrome profile (no sign-ins,
// no history): ChatGPT's logged-out composer and KaTeX's live demo. This script prepares the page
// over CDP (focus the box, empty it, scroll), then capture-chrome.ps1 does the guarded key press
// and the screen recording, then this script reads what Voxora pasted.
//
//   node video/capture-chrome.cjs p1 [m1 ...]     (Voxora running, mic = CABLE Output)
//
// Playwright: set PLAYWRIGHT to a playwright(-core) module path if it is not resolvable.
const { spawn, spawnSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const { chromium } = require(process.env.PLAYWRIGHT || "playwright-core");

const CHROME = "C:/Program Files/Google/Chrome/Application/chrome.exe";
const PORT = 9333;
const here = __dirname;
const profile = path.join(here, "clips", "chrome-profile");

const chatgpt = {
  url: "https://chatgpt.com/",
  // Logged out, the composer is a plain textarea (signed in, a ProseMirror #prompt-textarea).
  box: (p) => p.locator('textarea[name="prompt"]:visible, #prompt-textarea:visible').first(),
  async prep(p) {
    const box = this.box(p);
    await box.waitFor({ timeout: 20000 });
    await box.fill("");
    return clickPoint(box);
  },
  read(p) {
    return this.box(p).evaluate((e) => ("value" in e ? e.value : e.innerText));
  },
};
const katex = {
  url: "https://katex.org/",
  async prep(p) {
    const box = p.locator("#demo-input");
    await box.waitFor({ timeout: 20000 });
    await box.fill("");
    await p.evaluate(() => window.scrollTo(0, document.querySelector("#demo-input").getBoundingClientRect().top + scrollY - 150));
    await p.waitForTimeout(500);
    return clickPoint(box);
  },
  read: (p) => p.locator("#demo-input").inputValue(),
};
// Click point for capture-chrome.ps1: the box's left part, relative to the window's top-left.
const clickPoint = (box) => box.evaluate((e) => {
  const r = e.getBoundingClientRect();
  return { x: (outerWidth - innerWidth) / 2 + r.left + Math.min(60, r.width / 2), y: outerHeight - innerHeight + r.top + Math.min(r.height / 2, 30) };
});
const CLIPS = { p1: chatgpt, p2: chatgpt, p3: chatgpt, m1: katex, m2: katex, m4: katex };

(async () => {
  const want = process.argv.slice(2);
  if (!want.length || want.some((c) => !CLIPS[c])) throw new Error(`clips: ${Object.keys(CLIPS).join(" ")}`);
  fs.rmSync(profile, { recursive: true, force: true }); // fresh: no history, no "Restore pages?"
  const chrome = spawn(CHROME, [
    `--remote-debugging-port=${PORT}`, `--user-data-dir=${profile}`, "--no-first-run", "--no-default-browser-check", "--hide-crash-restore-bubble",
    "--force-device-scale-factor=1.5", "--window-size=1000,700", "--disable-features=CalculateNativeWinOcclusion,Translate",
    "about:blank"]);
  await new Promise((r) => setTimeout(r, 2500));
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`);
  const page = browser.contexts()[0].pages()[0];
  try {
    for (const id of want) {
      const c = CLIPS[id];
      await page.goto(c.url, { waitUntil: "domcontentloaded" });
      await page.waitForTimeout(4000);
      const pt = await c.prep.call(c, page);
      const wav = path.join(here, "speech2", "out16", `${id}.padded.wav`);
      const r = spawnSync("powershell", ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", path.join(here, "capture-chrome.ps1"),
        "-ChromePid", String(chrome.pid), "-Clip", id, "-Wav", wav, "-ClickX", String(pt.x), "-ClickY", String(pt.y), ...(process.env.CABLE_OUT ? ["-Out", process.env.CABLE_OUT] : [])],
        { stdio: "inherit" });
      if (r.status !== 0) throw new Error(`capture of ${id} failed (exit ${r.status}); stopping`);
      const result = await c.read.call(c, page);
      fs.writeFileSync(path.join(here, "clips", `${id}.result.txt`), result);
      console.log(`${id} result: ${result}`);
    }
  } finally {
    // Close Chrome properly (a killed Chrome offers to restore pages next time).
    const cdp = await browser.newBrowserCDPSession().catch(() => null);
    await cdp?.send("Browser.close").catch(() => {});
    await new Promise((r) => setTimeout(r, 1500));
    chrome.kill();
  }
})().catch((e) => { console.error(e.message); process.exit(1); });
