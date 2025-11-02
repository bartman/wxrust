#!/usr/bin/env python3
import argparse
import os
import sys
import subprocess
import tempfile
import shutil

import difflib
from pathlib import Path

def load_flags(flags_file):
    flags = {}
    if flags_file.exists():
        with open(flags_file, 'r') as f:
            for line in f:
                line = line.strip()
                if '=' in line:
                    key, value = line.split('=', 1)
                    if value.lower() == 'true':
                        flags[key] = True
                    elif value.lower() == 'false':
                        flags[key] = False
                    else:
                        flags[key] = value
    return flags

def substitute_variables(text, variables):
    for var, value in variables.items():
        text = text.replace(f"{{{{{var}}}}}", value)
    # Check for unsubstituted variables
    import re
    matches = re.findall(r'\{\{([^}]+)\}\}', text)
    for match in matches:
        if match not in variables:
            raise ValueError(f"Unknown variable: {match}")
    return text

def normalize(text, ignore_case, ignore_blank_lines, ignore_white_space):
    if ignore_case:
        text = text.lower()
    lines = text.splitlines()
    if ignore_blank_lines:
        lines = [line for line in lines if line.strip()]
    if ignore_white_space:
        lines = [' '.join(line.split()) for line in lines]
    return '\n'.join(lines)

def run_test(test_dir, variables, output_file, verbose):
    command_file = test_dir / "command"
    if not command_file.exists():
        return False, "No command file"

    with open(command_file, 'r') as f:
        command = f.read().strip()

    flags_file = test_dir / "flags"
    flags = load_flags(flags_file)

    ignore_case = flags.get('ignore-case', False)
    ignore_blank_lines = flags.get('ignore-blank-lines', False)
    ignore_white_space = flags.get('ignore-white-space', False)

    if verbose:
        print(f"Command (before substitution): {command}", file=output_file)

    try:
        command = substitute_variables(command, variables)
    except ValueError as e:
        return False, str(e)

    if verbose:
        print(f"Command (after substitution): {command}", file=output_file)

    # Execute command
    try:
        result = subprocess.run(command, shell=True, capture_output=True, text=True, cwd=os.getcwd())
        stdout = result.stdout
        stderr = result.stderr
        returncode = result.returncode
    except Exception as e:
        return False, f"Execution failed: {e}"

    # Write outputs to work_dir
    output_dir = Path(variables['WORK_DIR']) / test_dir.name
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "output.stdout").write_text(stdout)
    (output_dir / "output.stderr").write_text(stderr)
    (output_dir / "output.code").write_text(str(returncode))

    # Compare stdout
    expected_stdout_file = test_dir / "expected.stdout"
    if expected_stdout_file.exists():
        with open(expected_stdout_file, 'r') as f:
            expected_stdout = f.read()
        expected_stdout = normalize(expected_stdout, ignore_case, ignore_blank_lines, ignore_white_space)
        actual_stdout = normalize(stdout, ignore_case, ignore_blank_lines, ignore_white_space)
    if actual_stdout != expected_stdout:
        diff = list(difflib.unified_diff(expected_stdout.splitlines(keepends=True), actual_stdout.splitlines(keepends=True), fromfile='expected', tofile='actual'))
        print(f"Diff for {test_dir.name} stdout:", file=output_file)
        print(''.join(diff), file=output_file)
        return False, f"stdout mismatch\nExpected: {expected_stdout_file}\nActual: {output_dir / 'output.stdout'}"

    # Compare stderr
    expected_stderr_file = test_dir / "expected.stderr"
    if expected_stderr_file.exists():
        with open(expected_stderr_file, 'r') as f:
            expected_stderr = f.read()
        expected_stderr = normalize(expected_stderr, ignore_case, ignore_blank_lines, ignore_white_space)
        actual_stderr = normalize(stderr, ignore_case, ignore_blank_lines, ignore_white_space)
        if actual_stderr != expected_stderr:
            diff = list(difflib.unified_diff(expected_stderr.splitlines(keepends=True), actual_stderr.splitlines(keepends=True), fromfile='expected', tofile='actual'))
            print(f"Diff for {test_dir.name} stderr:", file=output_file)
            print(''.join(diff), file=output_file)
            return False, f"stderr mismatch\nExpected: {expected_stderr_file}\nActual: {output_dir / 'output.stderr'}"

    # Compare return code
    expected_code_file = test_dir / "expected.code"
    if expected_code_file.exists():
        with open(expected_code_file, 'r') as f:
            expected_code = int(f.read().strip())
        if returncode != expected_code:
            return False, f"return code mismatch: expected {expected_code}, got {returncode}"

    return True, ""

def main():
    parser = argparse.ArgumentParser(description="Run smoke tests")
    parser.add_argument('--target-dir', default='target', help='Build directory')
    parser.add_argument('--smoke-dir', default='smoke', help='Smoke tests directory')
    parser.add_argument('--work-dir', help='Temporary work directory')
    parser.add_argument('--output', help='Output file for verbose logs')
    parser.add_argument('--keep-work-dir', action='store_true', help='Keep the work directory after tests')
    parser.add_argument('--list', action='store_true', help='List all available tests')
    parser.add_argument('--test', help='Run only the specified test')
    parser.add_argument('--variable', action='append', help='Set variable VAR=VAL')
    parser.add_argument('--variables', action='store_true', help='List all variables')

    args = parser.parse_args()

    # Set up variables
    project_name = os.path.basename(os.getcwd())
    pid = os.getpid()
    variables = {
        'PROJECT_NAME': project_name,
        'PID': str(pid),
        'TARGET_DIR': args.target_dir,
        'TARGET': 'debug',
        'PROGRAM': 'wxrust',
        'SMOKE_DIR': args.smoke_dir,
        'CREDENTIALS': 'credentials.txt',
        'WORK_DIR': args.work_dir or f'/tmp/{project_name}-{pid}',
        'PROGRAM_PATH': f'{args.target_dir}/debug/wxrust'
    }

    # Override with --variable
    if args.variable:
        for var_val in args.variable:
            if '=' not in var_val:
                print(f"Invalid variable format: {var_val}", file=sys.stderr)
                sys.exit(1)
            var, val = var_val.split('=', 1)
            variables[var] = val

    if args.variables:
        for var, val in variables.items():
            print(f"{var}={val}")
        sys.exit(0)

    # Set up work dir
    work_dir = args.work_dir or f'/tmp/{project_name}-{pid}'
    variables['WORK_DIR'] = work_dir
    os.makedirs(work_dir, exist_ok=True)

    # Set up output
    output_file = None
    if args.output:
        output_path = Path(args.output)
        if output_path.exists():
            backup_path = Path(f"{args.output}.bak")
            shutil.move(str(output_path), str(backup_path))
        output_file = open(output_path, 'w')
    else:
        output_file = open(os.devnull, 'w')

    # Find test dirs
    smoke_dir = Path(args.smoke_dir)
    if not smoke_dir.exists():
        print(f"Smoke dir {smoke_dir} does not exist", file=sys.stderr)
        sys.exit(1)

    test_dirs = [d for d in smoke_dir.iterdir() if d.is_dir() and (d / "command").exists()]
    test_dirs.sort()

    if args.list:
        for test_dir in test_dirs:
            print(test_dir.name)
        sys.exit(0)

    if args.test:
        test_dirs = [d for d in test_dirs if d.name == args.test]
        if not test_dirs:
            print(f"Test {args.test} not found", file=sys.stderr)
            sys.exit(1)

    # Run tests
    passed = 0
    failed = 0
    for test_dir in test_dirs:
        print(f"Running test: {test_dir.name}")
        success, reason = run_test(test_dir, variables, output_file, args.output is not None)
        if success:
            print("\033[32mPASS\033[0m")
            passed += 1
        else:
            print(f"\033[31mFAIL: {reason}\033[0m")
            failed += 1

    print(f"\nSummary: {passed} passed, {failed} failed")

    if not args.keep_work_dir and args.work_dir is None:
        shutil.rmtree(work_dir)

    if output_file:
        output_file.close()

if __name__ == "__main__":
    main()