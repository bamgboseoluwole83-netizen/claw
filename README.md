# CRS — Code Research System

Static analysis + formal verification pipeline for C/C++ vulnerability research.

## Pipeline

```
Target source → Weggli (AST patterns) → Semgrep (taint) → CodeQL (DB queries)
               ↓
         Joern (CPG) → CBMC/ESBMC (SMT proof) → Frama-C (abstract interpretation)
               ↓
         Finding confirmed + variant analysis
```

## Tools

| Tool | Purpose | Install |
|------|---------|---------|
| Weggli | Google P0 — semantic AST pattern matching (5s across 30k pkgs) | `curl -L weggli-rs/weggli` |
| Semgrep | Inter-procedural taint tracking without build (YAML rules) | `pip3 install --user semgrep` |
| CodeQL | Deep relational DB queries (30min DB build) | `codeql CLI from GitHub` |
| Joern | Code Property Graph — Scala-based graph traversals | Needs Java |
| CBMC | Bounded model checking — SMT solver for array bounds | `cbmc from GitHub` |
| ESBMC | SMT-based verification (fallback for CBMC) | `esbmc from GitHub` |
| Frama-C | Abstract interpretation — mathematical value ranges | Needs OCaml |

## Structure

```
rules/          — Analysis rules and queries
  weggli/       —   AST pattern files
  semgrep/      —   YAML taint rules
  codeql/       —   QL queries
  joern/        —   Scala CPG scripts
harnesses/      — Formal verification harnesses
  cbmc/         —   CBMC proof harnesses
  frama-c/      —   Frama-C ACSL annotations
targets/        — Downloaded target source trees
findings/       — Confirmed and suspected bugs
scripts/        — Pipeline orchestration
  pipeline.sh   —   Full pipeline runner
  variant_analysis.sh — Cross-target variant scanner
  cbmc_verify.sh —   CBMC solver runner
  setup.sh      —   Tool installer
```

## Usage

```bash
# Install tools (needs network)
./scripts/setup.sh

# Download target source
./scripts/setup.sh targets

# Run full pipeline
./scripts/pipeline.sh libxml2
```

## Current findings

- **libxml2 DTD content model heap corruption** — confirmed SIGSEGV in
  `xmlRegEpxFromParse` when DTD OR-group exceeds ~400 alternatives.
  See `findings/libxml2_dtd_crash.md`
