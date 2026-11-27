## Install

**Windows 10/11:** download `Voxora_*_x64-setup.exe` and run it. The installer is unsigned, so
SmartScreen may warn: click **More info**, then **Run anyway**. Updating: run it over your
current install; models, history and settings are kept. The `.msi` is the same app as a Windows
Installer package, for managed installs.

**macOS 13.3+ on Apple silicon (beta):** download `Voxora_*_aarch64.dmg` and drag Voxora to
Applications. The app is not notarized: open it once, then go to **System Settings > Privacy &
Security** and click **Open Anyway**. After an update, turn Voxora off and on again under
**Accessibility**.

**Check a download:** compare `certutil -hashfile <file> SHA256` (Windows) or
`shasum -a 256 <file>` (macOS) with `SHA256SUMS.txt`.

First launch downloads the speech and rewrite models (about 4.7 GB with an NVIDIA GPU, 2.2 GB
without, 3.6 GB on a Mac). Full guide: [README](https://github.com/ParshvCrafts/Voxora#install).
