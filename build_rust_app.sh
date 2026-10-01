#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust/Cargo was not found. Install it from https://rustup.rs/ first." >&2
  exit 1
fi

cargo build --workspace --release
mkdir -p dist
cp target/release/tf2-demo-toolkit dist/TF2_Demo_Toolkit
cp target/release/tf2-demo-director dist/TF2_Demo_Director
cp target/release/export_all dist/export_all
mkdir -p dist/recording_resources_archive
cp recording_resources_archive/resources.part* dist/recording_resources_archive/
echo "Built TF2 Demo Toolkit, TF2 Demo Director, the parser, and recording resources in dist/"
