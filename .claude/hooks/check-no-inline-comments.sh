#!/bin/bash
## Checks that Claude is not writing inline or item-level doc comments.
## Inline comments (//) and item-level doc comments (///) are forbidden.
## Only file-level block comments (//!) and JSDoc (/** */) are allowed.
## HTML files use <!-- --> comments and are excluded from this check.
## Runs as PreToolUse hook on Write/Edit operations.

TOOL_INPUT=$(cat)

FILE_PATH=$(echo "$TOOL_INPUT" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    fp = data.get('file_path', '') or (data.get('tool_input') or {}).get('file_path', '')
    print(fp)
except:
    pass
" 2>/dev/null)

if [ -z "$FILE_PATH" ]; then
  exit 0
fi

case "$FILE_PATH" in
  *.rs|*.ts|*.tsx|*.js|*.jsx)
    ;;
  *)
    exit 0
    ;;
esac

CONTENT=$(echo "$TOOL_INPUT" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    inner = data.get('tool_input') or data
    content = inner.get('content', '') or inner.get('new_string', '')
    print(content)
except:
    pass
" 2>/dev/null)

if [ -z "$CONTENT" ]; then
  exit 0
fi

INLINE_VIOLATIONS=$(echo "$CONTENT" | grep -nE '^\s*//[^/!]' | head -5)

if [ -z "$INLINE_VIOLATIONS" ]; then
  INLINE_VIOLATIONS=$(echo "$CONTENT" | grep -nE ';\s*//[^/!]' | head -5)
fi

if [ -n "$INLINE_VIOLATIONS" ]; then
  echo "BLOCKED: Inline comments (//) detected in $FILE_PATH"
  echo "Violations:"
  echo "$INLINE_VIOLATIONS"
  echo ""
  echo "Rules: Use //! (Rust file-level) or /** */ (TypeScript) block comments only."
  exit 1
fi

case "$FILE_PATH" in
  *.rs)
    DOC_VIOLATIONS=$(echo "$CONTENT" | grep -nE '^\s*///' | head -5)
    if [ -n "$DOC_VIOLATIONS" ]; then
      echo "BLOCKED: Item-level doc comments (///) detected in $FILE_PATH"
      echo "Violations:"
      echo "$DOC_VIOLATIONS"
      echo ""
      echo "Rules: Move all documentation into the //! block comment at the top of the file."
      exit 1
    fi
    ;;
esac

exit 0
