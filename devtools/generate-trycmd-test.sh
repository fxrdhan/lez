#!/usr/bin/env bash

# Generate test data for the program

if [ $# -le 1 ]; then
    echo "Usage: $0 <test name> <test argument>"
    exit 1
fi

test_name=$1
shift

# Clean up previous test data

if [ -f tests/cmd/"$test_name".toml ]; then
    rm tests/cmd/"$test_name".toml
fi

if [ -f tests/cmd/"$test_name".stdout ]; then
    rm tests/cmd/"$test_name".stdout
fi

if [ -f tests/cmd/"$test_name".stderr ]; then
    rm tests/cmd/"$test_name".stderr
fi

# Generate test data

touch tests/cmd/"$test_name".toml

{
    echo 'bin.name = "lez"'
    echo 'args = "'"$*"'"'
    echo 'env.inherit = false'
} >> tests/cmd/"$test_name".toml

# Generate expected output

binary="${CARGO_TARGET_DIR:-target}/debug/lez"

if [ -f "$binary" ]; then
    # The case runs with `env.inherit = false`, so record it the same way.
    env -i LEZ_CONFIG_DIR=/nonexistent-lez-config "$binary" "$@" > tests/cmd/"$test_name".stdout 2> tests/cmd/"$test_name".stderr
    returncode=$?
    if [ $returncode -ne 0 ]; then
        echo -e 'status.code = '$returncode'' >> tests/cmd/"$test_name".toml
        exit 0
    fi
else
    echo "Please build the program first: no binary at $binary"
    exit 1
fi
