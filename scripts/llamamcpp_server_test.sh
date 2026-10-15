
#!/usr/bin/env bash
set -euo pipefail

CONFIG="${CONFIG:-$HOME/.config/aifix/config.json}"

list_models() {
    jq -r '
        .providerlist[]
        | "  \(.name)\t\(if .active then "active" else "inactive" end)\t\(.comment // "")"
    ' "$CONFIG"
}

usage() {
    echo "Usage: $0 <model-name> [llama-server arguments...]"
    echo
    echo "Available models:"
    list_models
}

if [[ $# -eq 0 || "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    usage
    exit 0
fi

MODEL_NAME="$1"
shift

if ! jq -e --arg name "$MODEL_NAME" \
    '.providerlist[] | select(.name == $name)' \
    "$CONFIG" >/dev/null; then
    echo "Unknown model: $MODEL_NAME" >&2
    echo >&2
    usage >&2
    exit 1
fi

MODEL_DIR=$(jq -r --arg name "$MODEL_NAME" '
    .providerlist[]
    | select(.name == $name)
    | .llmmodeldir
' "$CONFIG")

MODEL_FILE=$(jq -r --arg name "$MODEL_NAME" '
    .providerlist[]
    | select(.name == $name)
    | .model
' "$CONFIG")

LLM_BIN=$(jq -r --arg name "$MODEL_NAME" '
    .providerlist[]
    | select(.name == $name)
    | .llmbin
' "$CONFIG")

mapfile -t LLM_PARAMS < <(
    jq -r --arg name "$MODEL_NAME" '
        .providerlist[]
        | select(.name == $name)
        | .llmparam[]
    ' "$CONFIG"
)

MODEL_PATH="${MODEL_DIR%/}/${MODEL_FILE}"

if [[ ! -f "$MODEL_PATH" ]]; then
    echo "Model file not found: $MODEL_PATH" >&2
    exit 1
fi

echo "Model:  $MODEL_NAME"
echo "File:   $MODEL_PATH"
echo "Server: $LLM_BIN"
echo

exec "$LLM_BIN" \
    -m "$MODEL_PATH" \
    "${LLM_PARAMS[@]}" \
    "$@"

