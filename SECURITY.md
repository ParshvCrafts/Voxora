# Security policy

Voxora runs on your own computer, hears your microphone and pastes into other apps, so security
reports are taken seriously.

**Please report a vulnerability privately**: open the repository's **Security** tab and click
**Report a vulnerability** (GitHub private vulnerability reporting). Do not open a public issue.

Useful to include: the version (Settings > About), the operating system, what an attacker can
do, and the steps to reproduce. You should get a reply within a week.

In scope: anything that lets another program or website read what you dictate, trigger
dictation or pasting, tamper with the downloaded models or runtimes (they are pinned by
SHA-256 in `crates/voxora-core/src/engines/download.rs`), or reach the local inference servers
from outside `127.0.0.1`.

Only the latest release receives fixes.
