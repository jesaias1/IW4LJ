"""Stand-in C/C++ compiler and archiver for `cargo check` of iOS targets on Windows.

`cargo check` runs every dependency's build script, and the build scripts of the
`-sys` crates compile C with the Apple SDK, which only exists on a Mac. A type
check never links those objects, so this tool just creates the files the
build scripts ask for. It is for type checking only and produces nothing that
can run; real iOS builds happen on macOS (see ../build-ios.sh and the CI
workflow).
"""
import os
import sys


def touch(path):
    directory = os.path.dirname(path)
    if directory:
        os.makedirs(directory, exist_ok=True)
    with open(path, "ab"):
        pass


def main(argv):
    name = os.path.basename(argv[0]).lower() if argv else ""
    args = argv[1:]
    if "--version" in args or "-v" in args or "-dumpmachine" in args:
        print("clang version 17.0.0 (fake)")
        return 0
    if "-print-search-dirs" in args or "-print-prog-name=ar" in args:
        print("ar")
        return 0
    if name.startswith("fakear"):
        # ar rcs lib.a obj...  /  ar cq lib.a obj...
        archives = [a for a in args if a.endswith(".a")]
        if archives:
            touch(archives[0])
        return 0
    if "-E" in args or "-M" in args:
        return 0
    out = None
    for index, arg in enumerate(args):
        if arg == "-o" and index + 1 < len(args):
            out = args[index + 1]
        elif arg.startswith("-o") and len(arg) > 2:
            out = arg[2:]
    if out:
        touch(out)
    for index, arg in enumerate(args):
        if arg in ("-MF",) and index + 1 < len(args):
            touch(args[index + 1])
    return 0


sys.exit(main(sys.argv))
