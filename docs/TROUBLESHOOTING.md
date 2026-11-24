# Troubleshooting

Logs: `%LOCALAPPDATA%\Voxora\logs\` (`voxora.*.log`, `whisper-server.log`, `llama-server.log`).
Settings → General → **Logs** opens the folder. Set `VOXORA_LOG=debug` for more detail.

| Symptom | Cause | Fix |
|---|---|---|
| Pill says "Admin window: press Ctrl+V to paste" | The target app runs as Administrator. Windows (UIPI) blocks synthetic keystrokes from a normal app into an elevated one. | Press Ctrl+V yourself; the text is on the clipboard. Or run Voxora as admin too. |
| An app pastes your *old* clipboard | Some web editors read the clipboard late. | Settings → Pasting & privacy → raise **Restore delay** (e.g. 350 ms). |
| "No sound from the mic. Is it muted?" | The device delivers digital silence (hardware mute switch, privacy setting). | Windows Settings → Privacy → Microphone → allow desktop apps; check the mute key. |
| "No speech detected" | Voice activity detection heard less than 0.2 s of speech. | Speak after the pill appears; check the mic in Settings → Microphone. |
| Speech / Rewrite status shows `error` | A server failed to start (often GPU memory used by a game or training job). | Voxora retries on CPU automatically. Free VRAM, then tray → **Restart engines**. |
| First dictation after boot is slow | Models load into VRAM (~5–15 s) after login. | Wait for "ready" in the tray menu's status line or on the Home page. |
| Right Alt types special characters (AltGr layouts) | On some keyboard layouts Right Alt is AltGr. | Settings → Hotkeys → choose another key (e.g. Right Ctrl, F13). |
| Ctrl+Alt+V conflicts with Word/Excel Paste Special | Same shortcut. | Settings → Hotkeys → change the re-paste key. |
| Windows Defender / SmartScreen warning | Unsigned app that uses a keyboard hook and `SendInput`. | "More info → Run anyway", or build from source. |
| Text lands in the wrong window | Voxora pastes into whatever has focus at the moment the text is ready. | Keep focus in the target until the pill disappears (typically < 1 s). |

## Recovering text
Every dictation is saved to `%LOCALAPPDATA%\Voxora\voxora.db` *before* it is pasted. If a paste
was missed: **Ctrl+Alt+V** pastes the last dictation, or open **History** and click Copy.
If transcription itself failed, tray → **Retry failed recording** re-runs the audio (kept in RAM only).
