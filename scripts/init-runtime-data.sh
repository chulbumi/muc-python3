#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
data_root=${MUC_DATA_DIR:-"$project_root/data"}
runtime_root=${MUC_RUNTIME_DIR:-"$project_root/runtime"}

if [[ ! -d "$data_root/defaults/state" ]]; then
    echo "private game data is missing: $data_root" >&2
    echo "run: git submodule update --init --recursive" >&2
    exit 1
fi

mkdir -p \
    "$runtime_root/user" \
    "$runtime_root/log/group" \
    "$runtime_root/soul" \
    "$runtime_root/box" \
    "$runtime_root/state"

for name in book guild rank oneitem oneitem_index; do
    source_file="$data_root/defaults/state/$name.json"
    destination="$runtime_root/state/$name.json"
    if [[ ! -e "$destination" ]]; then
        cp "$source_file" "$destination"
    fi
done

echo "runtime data initialized at $runtime_root"
