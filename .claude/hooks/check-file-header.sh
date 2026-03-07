#!/bin/bash
## Checks that source files have a block comment header at the top.
## Rust: must start with //!
## TypeScript/React: must start with /** */
## Runs as PostToolUse hook on Write/Edit operations.

TOOL_INPUT=$(cat)

FILE_PATH=$(echo "$TOOL_INPUT" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('file_path', ''))
except:
    pass
" 2>/dev/null)

if [ -z "$FILE_PATH" ]; then
  exit 0
fi

case "$FILE_PATH" in
  *.rs|*.ts|*.tsx)
    ;;
  *)
    exit 0
    ;;
esac

case "$FILE_PATH" in
  *test*|*spec*|*.config.*|*.d.ts|*mod.rs|*index.ts)
    exit 0
    ;;
esac

if [ ! -f "$FILE_PATH" ]; then
  exit 0
fi

FIRST_CONTENT_LINE=$(head -10 "$FILE_PATH" | grep -v '^$' | head -1)

case "$FILE_PATH" in
  *.rs)
    if ! echo "$FIRST_CONTENT_LINE" | grep -qE '^\s*(//!)'; then
      echo "WARNING: $FILE_PATH missing //! block comment header at top of file."
      exit 1
    fi
    ;;
  *.ts|*.tsx)
    if ! echo "$FIRST_CONTENT_LINE" | grep -qE '^\s*/\*\*'; then
      echo "WARNING: $FILE_PATH missing /** */ block comment header at top of file."
      exit 1
    fi
    ;;
esac

exit 0
