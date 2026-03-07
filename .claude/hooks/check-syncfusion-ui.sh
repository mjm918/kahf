#!/bin/bash
## Checks that React component files use Syncfusion components
## instead of custom UI implementations.
## Runs as PostToolUse hook on Write/Edit operations for .tsx files.

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
  *.tsx)
    ;;
  *)
    exit 0
    ;;
esac

case "$FILE_PATH" in
  *test*|*spec*|*__tests__*)
    exit 0
    ;;
esac

if [ ! -f "$FILE_PATH" ]; then
  exit 0
fi

CONTENT=$(cat "$FILE_PATH")
WARNINGS=""

if echo "$CONTENT" | grep -qE '<table|<Table[^A-Z]' && ! echo "$CONTENT" | grep -q '@syncfusion'; then
  WARNINGS="$WARNINGS\n  - <table> found. Use Syncfusion GridComponent."
fi

if echo "$CONTENT" | grep -qE 'className=".*modal|className=".*dialog' && ! echo "$CONTENT" | grep -q '@syncfusion'; then
  WARNINGS="$WARNINGS\n  - Custom modal/dialog found. Use Syncfusion DialogComponent."
fi

if echo "$CONTENT" | grep -qE 'className=".*calendar' && ! echo "$CONTENT" | grep -q '@syncfusion'; then
  WARNINGS="$WARNINGS\n  - Custom calendar found. Use Syncfusion ScheduleComponent."
fi

if echo "$CONTENT" | grep -qE 'className=".*sidebar|className=".*nav-' && ! echo "$CONTENT" | grep -q '@syncfusion'; then
  WARNINGS="$WARNINGS\n  - Custom sidebar/nav found. Use Syncfusion SidebarComponent."
fi

if echo "$CONTENT" | grep -qE 'contentEditable|contenteditable' && ! echo "$CONTENT" | grep -q '@syncfusion'; then
  WARNINGS="$WARNINGS\n  - contentEditable found. Use Syncfusion RichTextEditorComponent."
fi

if [ -n "$WARNINGS" ]; then
  echo "WARNING: $FILE_PATH uses custom UI where Syncfusion components are required."
  echo -e "Issues:$WARNINGS"
  echo "All UI MUST use Syncfusion EJ2 React components."
  exit 1
fi

exit 0
