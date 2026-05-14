import argparse
import shutil
import subprocess
import sys


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="spectra-py",
        description="Python wrapper for the spectra Rust binary.",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    check = sub.add_parser("check", help="Diff two Anchor IDL JSON files")
    check.add_argument("--old", required=True, help="Path to baseline (v_n) IDL JSON")
    check.add_argument("--new", required=True, help="Path to new (v_{n+1}) IDL JSON")
    check.add_argument("--report", default=None, help="Optional path to write report")
    check.add_argument(
        "--format",
        default="json",
        choices=["json", "markdown", "md"],
        help="Output format",
    )

    args = parser.parse_args()

    binary = shutil.which("spectra")
    if binary is None:
        print(
            "error: 'spectra' binary not found on PATH. "
            "Install via: cargo install --path spectra-core",
            file=sys.stderr,
        )
        return 2

    cmd = [
        binary,
        "check",
        "--old", args.old,
        "--new", args.new,
        "--format", args.format,
    ]
    if args.report:
        cmd.extend(["--report", args.report])

    return subprocess.call(cmd)


if __name__ == "__main__":
    sys.exit(main())
