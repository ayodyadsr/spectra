# spectra-cli (Python wrapper)

Thin Python entry point that shells out to the Rust `spectra` binary. Useful for users with Python toolchains who don't want to install Rust just to run Spectra in CI.

## Install

```bash
cd spectra-cli
pip install -e .
```

You also need the Rust binary on PATH:

```bash
cargo install --path ../spectra-core
```

## Usage

```bash
spectra-py check --baseline examples/vault_baseline --candidate examples/vault_candidate --format markdown
```

`--baseline` and `--candidate` are Anchor **Rust source trees** (the last
released / on-chain version, and the upgrade under review), not IDL JSON.

Identical flags to `spectra check`; see the main repo README.
