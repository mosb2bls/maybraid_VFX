#!/bin/zsh
set -e
M21_REVIEW_DIR="${0:A:h}"
exec "$M21_REVIEW_DIR/runtime/matched_dust" 'C' --data-root "$M21_REVIEW_DIR/data" --shader-root "$M21_REVIEW_DIR/runtime/shaders" "$@"
