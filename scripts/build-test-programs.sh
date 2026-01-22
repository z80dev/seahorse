#!/usr/bin/env bash
# Build script for Seahorse test programs
# Compiles Seahorse examples to Rust, then builds them to .so files via Anchor

set -e

# Get script and root directories
SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="${ROOT_DIR}/.build-programs"
DEPLOY_DIR="${ROOT_DIR}/target/deploy"
SEAHORSE_BIN="${ROOT_DIR}/target/debug/seahorse"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Ensure seahorse is built
if [ ! -f "$SEAHORSE_BIN" ]; then
    info "Building seahorse compiler..."
    (cd "$ROOT_DIR" && cargo build --bin seahorse)
fi

# Ensure anchor is available
if ! command -v anchor &> /dev/null; then
    error "anchor CLI not found. Please install Anchor."
    exit 1
fi

# Create deploy directory
mkdir -p "$DEPLOY_DIR"

# Track build results
BUILT_PROGRAMS=()
FAILED_PROGRAMS=()

# Add a program to Anchor.toml scripts section once per run
add_program_to_anchor_toml() {
    local name="$1"
    local program_id="$2"
    local anchor_file="${BUILD_DIR}/Anchor.toml"
    local seen_file="${BUILD_DIR}/.anchor_scripts_seen"

    mkdir -p "${BUILD_DIR}"
    touch "$seen_file"

    # Avoid duplicates within a single run
    if grep -Fxq "$name" "$seen_file"; then
        return 0
    fi
    # Also guard against any pre-existing line in the file
    if grep -qF "^${name} = " "$anchor_file"; then
        echo "$name" >> "$seen_file"
        return 0
    fi

    echo "${name} = \"${program_id}\"" >> "$anchor_file"
    echo "$name" >> "$seen_file"
}

#
# Build a single program
# Usage: build_program <name> <rust_source_file>
#
build_program() {
    local name="$1"
    local rust_source="$2"
    local program_dir="${BUILD_DIR}/programs/${name}"

    info "Building program: $name"

    # Create program directory structure
    mkdir -p "${program_dir}/src"

    # The compiled Seahorse output contains multiple modules in one file
    # The format is:
    # // ===== dot/mod.rs =====
    # <mod content>
    # // ===== dot/program.rs =====
    # <program content>
    # // ===== lib.rs =====
    # <lib content with seahorse_util>

    # Check if this is a multi-file format (has "// ===== dot/" markers)
    if grep -q "// ===== dot/" "$rust_source"; then
        # Multi-file format - split into mod.rs, program.rs, and lib.rs

        # Create dot directory for program modules
        mkdir -p "${program_dir}/src/dot"

        # Use awk to split the file into the three sections
        awk '
        BEGIN { output="" }
        /^\/\/ ===== dot\/mod.rs =====/ { output="mod"; next }
        /^\/\/ ===== dot\/program.rs =====/ { output="program"; next }
        /^\/\/ ===== lib.rs =====/ { output="lib"; next }
        /^\/\/ ===== / { output=""; next }
        output=="mod" { print > "'"${program_dir}/src/dot/mod.rs"'" }
        output=="program" { print > "'"${program_dir}/src/dot/program.rs"'" }
        output=="lib" { print > "'"${program_dir}/src/lib.rs"'" }
        ' "$rust_source"

        # Verify that lib.rs was extracted (it contains seahorse_util module)
        if [ ! -s "${program_dir}/src/lib.rs" ]; then
            error "  lib.rs was not extracted from compiled file"
            return 1
        fi
    else
        # Simple single-file format - use as lib.rs directly
        cp "$rust_source" "${program_dir}/src/lib.rs"
    fi

    # Check if the source uses pyth-sdk-solana
    local needs_pyth=""
    if grep -q "pyth_sdk_solana" "$rust_source"; then
        needs_pyth="yes"
    fi

    # Enable init-if-needed feature when generated code uses it
    local init_if_needed_feature=""
    local default_features="default = []"
    if grep -q "init_if_needed" "$rust_source"; then
        init_if_needed_feature='init-if-needed = ["anchor-lang/init-if-needed"]'
        default_features='default = ["init-if-needed"]'
    fi

    # Create Cargo.toml for the program
    cat > "${program_dir}/Cargo.toml" << CARGOTML
[package]
name = "${name}"
version = "0.1.0"
description = "Created with Seahorse"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "${name//-/_}"

[features]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
${default_features}
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]
${init_if_needed_feature}

[dependencies]
anchor-lang = "0.32.0"
anchor-spl = "0.32.0"
blake3 = "=1.8.2"
paste = "1.0"
CARGOTML

    # Add pyth-sdk-solana if needed
    if [ -n "$needs_pyth" ]; then
        echo 'pyth-sdk-solana = "0.10.6"' >> "${program_dir}/Cargo.toml"
        info "  Added pyth-sdk-solana dependency"
    fi

    # Build with anchor
    info "  Running anchor build for $name..."
    if (cd "$BUILD_DIR" && anchor build -p "$name" 2>&1); then
        # Copy .so file to deploy directory
        local so_name="${name//-/_}.so"
        if [ -f "${BUILD_DIR}/target/deploy/${so_name}" ]; then
            cp "${BUILD_DIR}/target/deploy/${so_name}" "${DEPLOY_DIR}/"
            info "  Built: ${DEPLOY_DIR}/${so_name}"
            BUILT_PROGRAMS+=("$name")
            return 0
        else
            error "  .so file not found after build: ${so_name}"
            FAILED_PROGRAMS+=("$name")
            return 1
        fi
    else
        error "  Failed to build $name"
        FAILED_PROGRAMS+=("$name")
        return 1
    fi
}

#
# Initialize the build directory with Anchor project structure
#
init_build_dir() {
    info "Initializing build directory..."

    # Clean and recreate build directory
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"
    mkdir -p "${BUILD_DIR}/programs"

    # Create minimal Anchor.toml
    cat > "${BUILD_DIR}/Anchor.toml" << 'ANCHORTOML'
[features]
seeds = true
skip-lint = true

[programs.localnet]

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "localnet"
wallet = "~/.config/solana/id.json"

[scripts]
ANCHORTOML

    # Create minimal workspace Cargo.toml
    cat > "${BUILD_DIR}/Cargo.toml" << 'CARGOTOML'
[workspace]
members = [
    "programs/*"
]
resolver = "2"

[profile.release]
overflow-checks = true
lto = "fat"
codegen-units = 1

[profile.release.build-override]
opt-level = 3
incremental = false
codegen-units = 1
CARGOTOML
}

#
# Compile Seahorse examples
#
compile_seahorse_examples() {
    info "Compiling Seahorse examples..."

    local examples_dir="${ROOT_DIR}/examples"
    local compiled_dir="${ROOT_DIR}/tests/compiled-examples"

    mkdir -p "$compiled_dir"

    for py_file in "${examples_dir}"/*.py; do
        [ -f "$py_file" ] || continue

        local name
        name=$(basename "$py_file" .py)
        local rs_file="${compiled_dir}/${name}.rs"

        info "  Compiling $name.py -> $name.rs"
        if "$SEAHORSE_BIN" compile "$py_file" > "$rs_file" 2>&1; then
            # Verify output is valid (not empty and not an error message)
            if [ -s "$rs_file" ] && ! grep -q "^error" "$rs_file"; then
                info "    Compiled successfully"
            else
                warn "    Compilation produced empty or error output"
                rm -f "$rs_file"
            fi
        else
            warn "    Failed to compile $name.py"
            rm -f "$rs_file"
        fi
    done
}

#
# Build all Seahorse examples
#
build_seahorse_examples() {
    info "Building Seahorse examples to .so files..."

    local compiled_dir="${ROOT_DIR}/tests/compiled-examples"

    for rs_file in "${compiled_dir}"/*.rs; do
        [ -f "$rs_file" ] || continue

        local name
        name=$(basename "$rs_file" .rs)

        # Add program to Anchor.toml (dedupe to avoid duplicate keys)
        local program_id="Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
        add_program_to_anchor_toml "$name" "$program_id"

        build_program "$name" "$rs_file" || true
    done
}

#
# Build reference Anchor programs
#
build_reference_programs() {
    local ref_dir="${ROOT_DIR}/tests/anchor-reference"

    if [ ! -d "$ref_dir" ]; then
        info "No anchor-reference directory found - skipping reference programs"
        return 0
    fi

    info "Building reference Anchor programs..."

    for program_dir in "${ref_dir}"/*/; do
        [ -d "$program_dir" ] || continue

        local name
        name=$(basename "$program_dir")

        # Check for Cargo.toml (indicates a valid program)
        if [ ! -f "${program_dir}/Cargo.toml" ]; then
            warn "  Skipping $name - no Cargo.toml found"
            continue
        fi

        info "Building reference program: $name"

        # Copy program to build directory
        cp -r "$program_dir" "${BUILD_DIR}/programs/${name}"

        # Add to Anchor.toml (dedupe to avoid duplicate keys)
        local program_id="Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
        add_program_to_anchor_toml "$name" "$program_id"

        # Build
        if (cd "$BUILD_DIR" && anchor build -p "$name" 2>&1); then
            local so_name="${name//-/_}.so"
            if [ -f "${BUILD_DIR}/target/deploy/${so_name}" ]; then
                cp "${BUILD_DIR}/target/deploy/${so_name}" "${DEPLOY_DIR}/"
                info "  Built: ${DEPLOY_DIR}/${so_name}"
                BUILT_PROGRAMS+=("$name")
            else
                error "  .so file not found after build"
                FAILED_PROGRAMS+=("$name")
            fi
        else
            error "  Failed to build $name"
            FAILED_PROGRAMS+=("$name")
        fi
    done
}

#
# Main execution
#
main() {
    info "=== Seahorse Test Program Build Script ==="
    info "Root directory: $ROOT_DIR"
    info "Build directory: $BUILD_DIR"
    info "Deploy directory: $DEPLOY_DIR"
    echo

    # Step 1: Initialize build directory
    init_build_dir

    # Step 2: Compile Seahorse examples to Rust
    compile_seahorse_examples

    # Step 3: Build Seahorse examples to .so
    build_seahorse_examples

    # Step 4: Build reference Anchor programs
    build_reference_programs

    # Summary
    echo
    info "=== Build Summary ==="
    info "Built programs: ${#BUILT_PROGRAMS[@]}"
    for prog in "${BUILT_PROGRAMS[@]}"; do
        info "  - $prog"
    done

    if [ ${#FAILED_PROGRAMS[@]} -gt 0 ]; then
        warn "Failed programs: ${#FAILED_PROGRAMS[@]}"
        for prog in "${FAILED_PROGRAMS[@]}"; do
            warn "  - $prog"
        done
    fi

    info "Artifacts stored in: $DEPLOY_DIR"

    # List built .so files
    if [ -d "$DEPLOY_DIR" ] && [ "$(ls -A $DEPLOY_DIR 2>/dev/null)" ]; then
        info "Built artifacts:"
        ls -la "$DEPLOY_DIR"/*.so 2>/dev/null || true
    fi
}

main "$@"
