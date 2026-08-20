# React rendering and edge-tab Windows acceptance

Automated checks completed on 2026-08-20: TypeScript build, Vitest suite, Rust tests, and Rust build.

The following require a live packaged Windows desktop and remain unverified in this environment:

| Matrix | Status |
|---|---|
| 100/125/150/200% scaling, light/dark backgrounds | Unverified manually |
| All four corners and top/left/right/bottom taskbars | Unverified manually |
| Tray-only, overlay-only, and both surfaces | Unverified manually |
| Mouse, keyboard, reduced motion, and rapid reversal | Unverified manually |
| Monitor disconnect/reconnect and fallback placement | Unverified manually |
| CSS curves without stair-stepping; disjoint gaps click through | Unverified manually |
| Hidden HWND does not block desktop; settings frame repair | Unverified manually |
