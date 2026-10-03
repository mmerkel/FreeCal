# 28: .deb, .rpm and AppImage packaging

**What to build:** FreeCal can be installed from a `.deb`, an `.rpm` or run as an AppImage, all built by Tauri. All OS integration goes through portal-friendly APIs so a later Flathub release needs no code changes.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] The build produces `.deb`, `.rpm` and AppImage
- [ ] Each package installs or runs and launches FreeCal on a clean system
- [ ] Autostart, keyring, file dialogs and network status use portal-friendly APIs (checked as features land)
- [ ] A manual release checklist against a real Google test account is documented
