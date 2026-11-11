# nsh-device-info

Device model, OS version and app version for Native Shell apps.

Part of the Native Shell (NSH) capability set. Same layout as the other SuperNSH repos: a Rust crate, docs in `docs/`, CI on GitHub Actions, and a web status page on GitHub Pages at https://supernsh.github.io/device-info/.

## Status

**Implemented, not yet verified.** The Rust crate and the TypeScript package are in the repo and the CI gates are wired; no run has passed yet, so nothing here claims CI green. [docs/parity.md](docs/parity.md) tracks what is done.

## Approach

Native APIs for the model on every platform, os_info 3 for the OS version on the three desktop targets, app version from the bundle, package manager or exe version resource. What was chosen and why is in [docs/decisions.md](docs/decisions.md).

## Layout

- `crates/nsh-device-info/`: the Rust crate — facade, errors, mock backend and the platform bridges.
- `packages/nsh-device-info/`: the TypeScript package over the same Rust facade.
- `docs/brief.md`: options, recommended path and edge cases from the research.
- `docs/goals.md`: goals, acceptance criteria and the canonical test table.
- `docs/decisions.md`: what was chosen and why.
- `docs/parity.md`: feature status.
- `site/`: static status page, published by `.github/workflows/pages.yml`.
- `.zcode/workflow.ts`: the phases this repo runs through.

## Checks

Every compile and test runs in GitHub Actions. Nothing is built on a developer machine.

## Licence

MIT. See [LICENSE](LICENSE).

Repository: https://github.com/SuperNSH/device-info
