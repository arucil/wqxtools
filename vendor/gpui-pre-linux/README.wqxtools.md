# Local GPUI Linux patch

Source: crates.io `gpui-pre-linux` 0.3.6 (Zed snapshot `bcf6582ce3500df93a8a39366640173e6786cea6`), Apache-2.0; original license retained.

The unmodified X11 and Wayland backends drop every `is_modifier_key()` event,
including CapsLock. `ModifiersChangedEvent.capslock` reports the lock state but
cannot represent physical hold/release. This breaks the Qt-compatible mapping
of CapsLock to WQX input-method key 18 and prevents `CHECKKEY(18)` from detecting
the held key.

This patch lets **only CapsLock** pass the original down/up paths. Other
modifier handling, lock-state notifications, composition, and text input are
unchanged. Both backends share a regression-tested filter in `platform.rs`.
The GUI accepts the XKB `caps_lock` spelling as well as `capslock`.
The published manifest omitted upstream test support; the local manifest adds
`gpui-pre/test-support` as a dev dependency so its existing tests can run.
As a workspace member it uses the root Cargo resolver instead of its
published package-level resolver setting.
Strict validation also removes an upstream duplicated `allow(unused)`
attribute and the unused `x11-clipboard` dependency; no lint rules are disabled.

Use the normal workspace Cargo commands; the root `[patch.crates-io]` selects
this source without editing a machine's Cargo registry. Remove the patch when
an upstream release provides equivalent raw CapsLock events.

Checks: `cargo test -p gpui-pre-linux --lib`, GUI tests/Clippy/release build,
and an isolated X11 simulator `INKEY$` / `CHECKKEY(18)` hold-release test.
Wayland is compiled and shares the tested filter, but is not runtime-tested.
