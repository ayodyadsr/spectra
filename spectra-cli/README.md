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
spectra-py check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown
```

Identical flags to `spectra check`; see the main repo README.
