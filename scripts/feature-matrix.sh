#!/usr/bin/env bash
set -euo pipefail

# The base states are the distinct std/alloc/descriptive-errors combinations
# after accounting for std and descriptive-errors both implying alloc.
bases=("" alloc std descriptive-errors std,descriptive-errors)
modifiers=("" bits logging bits,logging)

for base in "${bases[@]}"; do
    for modifier in "${modifiers[@]}"; do
        # `just test` already runs no-default-features, default features, and
        # all-features, so omit those three configurations from this matrix.
        if [[ -z "$base" && -z "$modifier" ]]; then
            continue
        fi
        if [[ "$base" == "std,descriptive-errors" && ( "$modifier" == "bits" || "$modifier" == "bits,logging" ) ]]; then
            continue
        fi

        if [[ -n "$base" && -n "$modifier" ]]; then
            printf '%s,%s\n' "$base" "$modifier"
        elif [[ -n "$base" ]]; then
            printf '%s\n' "$base"
        else
            printf '%s\n' "$modifier"
        fi
    done
done
