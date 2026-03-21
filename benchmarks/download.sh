#!/bin/bash
# Download HWMCC'24 benchmarks from Zenodo
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

ZENODO_URL="https://zenodo.org/records/14156844/files"

echo "Downloading HWMCC'24 benchmarks..."

# Create directories
mkdir -p bv array

# Download BV track
if [ ! -f "hwmcc24-bv.tar.gz" ]; then
    echo "Downloading BV track..."
    curl -L -o hwmcc24-bv.tar.gz "${ZENODO_URL}/bv.tar.gz?download=1" || {
        echo "Warning: Could not download BV track. Check URL at https://zenodo.org/records/14156844"
        echo "You may need to download manually."
    }
fi

# Download Array track
if [ ! -f "hwmcc24-array.tar.gz" ]; then
    echo "Downloading Array track..."
    curl -L -o hwmcc24-array.tar.gz "${ZENODO_URL}/array.tar.gz?download=1" || {
        echo "Warning: Could not download Array track. Check URL at https://zenodo.org/records/14156844"
        echo "You may need to download manually."
    }
fi

# Extract
if [ -f "hwmcc24-bv.tar.gz" ]; then
    echo "Extracting BV track..."
    tar xzf hwmcc24-bv.tar.gz -C bv/ 2>/dev/null || tar xzf hwmcc24-bv.tar.gz -C bv/
    echo "BV track: $(find bv/ -name '*.btor2' | wc -l) files"
fi

if [ -f "hwmcc24-array.tar.gz" ]; then
    echo "Extracting Array track..."
    tar xzf hwmcc24-array.tar.gz -C array/ 2>/dev/null || tar xzf hwmcc24-array.tar.gz -C array/
    echo "Array track: $(find array/ -name '*.btor2' | wc -l) files"
fi

echo "Done."
