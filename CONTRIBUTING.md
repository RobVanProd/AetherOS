# Contributing to AetherOS

AetherOS is the canonical repo for the GenAI‑native operating system effort.

## Repo rules (non-negotiable)
- **Source-only**: no large/binary artifacts in git.
  - Put images/ISOs/zips/binaries in GitHub Releases.
- Prefer **small PRs** with a clear “Definition of Done”.
- Every PR should either:
  - improve build/run reproducibility, or
  - improve docs/roadmap clarity, or
  - ship a tested module move from `legacy/` into canonical paths.

## PR hygiene
- Include a short description + screenshots/logs if relevant.
- If you touch build/run:
  - include exact commands to reproduce
  - include expected output

## Suggested PR lanes
1) **Docs-first**: README, roadmap, V0 definition, architecture notes
2) **Build harness**: QEMU boot scripts, toolchain setup, CI smoke boot
3) **Module adoption**: move one subsystem from `legacy/MyOS` into canonical tree

## Commit style
- Use prefixes when helpful: `docs:`, `ci:`, `tools:`, `kernel:`

Thanks for building.
