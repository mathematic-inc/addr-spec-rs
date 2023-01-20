#!/usr/bin/env python3

import glob
import json
import subprocess
import argparse
import os
import tempfile

RUSTFLAGS = '-C instrument-coverage'
RUSTDOCFLAGS = '' # -C instrument-coverage -Z unstable-options --persist-doctests target/debug/doctestbins
        
def main():
    parser = argparse.ArgumentParser(description='Generate coverage report for a Rust project.')
    _, cargo_args = parser.parse_known_args()

    # Make temporary directory for profile data
    tempdir = tempfile.TemporaryDirectory(prefix='coverage-')

    # Set the appropriate flags
    os.environ['RUSTFLAGS'] = ' '.join([os.getenv('RUSTFLAGS', ''), RUSTFLAGS])
    os.environ['RUSTDOCFLAGS'] = ' '.join([os.getenv('RUSTDOCFLAGS', ''), RUSTDOCFLAGS])
    os.environ['LLVM_PROFILE_FILE'] = f'{tempdir.name}/default_%p.profraw'

    # Run cargo test with the appropriate flags
    cmd = ['cargo', 'test']
    cmd.extend(cargo_args)
    process = subprocess.run(cmd)
    if process.returncode != 0:
        return process.returncode

    # Merge the raw profiles
    cmd = ['llvm-profdata', 'merge', '-sparse', *glob.glob(f'{tempdir.name}/default_*.profraw'), '-o', f'{tempdir.name}/coverage.profdata']
    process = subprocess.run(cmd)
    if process.returncode != 0:
        return process.returncode

    # Run cargo test with the appropriate flags
    cmd = ['cargo', 'test']
    cmd.extend(cargo_args)
    cmd.extend(['--no-run', '--message-format=json'])

    # Get the path to the test binaries
    output = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL).stdout.decode('utf-8')
    binaries = []
    binaries.extend(glob.glob('target/debug/doctestbins/*/rust_out'))
    for lines in output.splitlines():
        message = json.loads(lines)
        if 'profile' in message and 'test' in message['profile'] and message['profile']['test']:
            binaries.extend(message['filenames'])
            break
    
    # Generate the coverage report
    cmd = ['llvm-cov', 'export', "--format", "lcov", "--ignore-filename-regex", "/.cargo/registry",
            '-instr-profile', f'{tempdir.name}/coverage.profdata']
    [cmd.extend(["-object", binary]) for binary in binaries]
    process = subprocess.run(cmd, check=True, capture_output=True)
    if process.returncode != 0:
        return process.returncode
    with open('target/coverage.info', 'wb') as f:
        f.write(process.stdout)
    
    return 0

if __name__ == '__main__' :
    main()