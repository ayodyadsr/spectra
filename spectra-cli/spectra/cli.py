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

    check = sub.add_parser(
        "check",
        help="Diff a baseline program source tree against a candidate upgrade",
    )
    check.add_argument(
        "--baseline",
        required=True,
        help="Path to the baseline (last released / on-chain) program source tree",
    )
    check.add_argument(
        "--candidate",
        required=True,
        help="Path to the candidate (upgrade under review) program source tree",
    )
    check.add_argument("--report", default=None, help="Optional path to write report")
    check.add_argument(
        "--format",
        default="json",
        choices=["json", "markdown", "md", "sarif"],
        help="Output format",
    )
    check.add_argument(
        "--quiet",
        action="store_true",
        help="Suppress stdout on clean runs; exit code still signals status",
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
        "--baseline", args.baseline,
        "--candidate", args.candidate,
        "--format", args.format,
    ]
    if args.report:
        cmd.extend(["--report", args.report])
    if args.quiet:
        cmd.append("--quiet")

    return subprocess.call(cmd)


if __name__ == "__main__":
    sys.exit(main())
