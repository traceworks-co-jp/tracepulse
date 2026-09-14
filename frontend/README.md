# TracePulse Web Frontend

This directory contains TypeScript sources for browser-side WebGUI code.

## Build

Run from the repository root:

```bash
npm install
npm run typecheck
npm run build:web
```

`npm run build:web` writes compiled browser JavaScript to `frontend/dist/`.
The generated `frontend/dist/*.js` files are intentionally kept as repository
artifacts during the initial migration so Rust builds can serve stable assets
without requiring Node.js during every `cargo build`.

## Rust build integration policy

The initial migration does not call npm from `build.rs`. This keeps ordinary
Rust builds independent from Node.js tooling. CI or release jobs should run:

```bash
npm run build:web
git diff --exit-code -- frontend/dist
cargo build
```

A future release workflow may opt in to a `build.rs`-driven frontend build if
fully automated asset generation becomes more important than keeping `cargo
build` independent from Node.js.

## Browser integration tests

Install the browser once and run the Playwright tests from the repository root:

```bash
npx playwright install chromium
npm run test:browser
```

The test server uses `127.0.0.1:8080`; stop any existing TracePulse WebGUI
process on that port before running the suite so Playwright does not reuse an
older binary.

## Runtime globals

Values injected by `src/web/server.rs` are typed in
`frontend/src/types/globals.d.ts`. Page scripts should access those values via
`window.*` so TypeScript can validate optional page-specific globals.
