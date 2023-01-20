#!/usr/bin/env python3

# Description: Run tests for a Rust project with different feature sets

import argparse
import subprocess
import sys

def main():
    parser = argparse.ArgumentParser(description="Run tests for a Rust project.")
    parser.add_argument("--feature-sets", help="Feature set to test", default="default")
    parser.add_argument("--channels", help="Channel to test (stable, beta, nightly)", default="default")
    args, cargo_args = parser.parse_known_args()

    for channel in args.channels.split(","):
        for features in args.feature_sets.split(":"):
            process = None
            if features == "default":
                process = subprocess.run(["cargo", *([f"+{channel}"] if channel != "default" else []), "test", *cargo_args])
            else:
                process = subprocess.run(["cargo", *([f"+{channel}"] if channel != "default" else []), "test", "--no-default-features", "--features", features, *cargo_args])
            if process.returncode != 0:
                return process.returncode

    return 0


if __name__ == "__main__":
    sys.exit(main())
