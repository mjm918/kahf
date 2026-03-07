#!/bin/bash
## Checks that Angular component files use Syncfusion components
## instead of custom UI implementations.
## Runs as PostToolUse hook on Write/Edit operations for .ts and .html files.

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
  *.ts|*.html)
    ;;
  *)
    exit 0
    ;;
esac

case "$FILE_PATH" in
  *test*|*spec*|*.config.*|*.d.ts|*index.ts|*main.ts|*environments/*)
    exit 0
    ;;
esac

if [ ! -f "$FILE_PATH" ]; then
  exit 0
fi

CONTENT=$(cat "$FILE_PATH")
WARNINGS=""

if echo "$CONTENT" | grep -qE '<table[>\s]|<Table[^A-Z]' && ! echo "$CONTENT" | grep -qi 'syncfusion\|ejs-grid\|GridModule'; then
  WARNINGS="$WARNINGS\n  - <table> found. Use Syncfusion ejs-grid (GridComponent)."
fi

if echo "$CONTENT" | grep -qE 'class=".*modal|class=".*dialog' && ! echo "$CONTENT" | grep -qi 'syncfusion\|ejs-dialog\|DialogModule\|DialogUtility'; then
  WARNINGS="$WARNINGS\n  - Custom modal/dialog found. Use Syncfusion ejs-dialog (DialogComponent)."
fi

if echo "$CONTENT" | grep -qE 'class=".*calendar' && ! echo "$CONTENT" | grep -qi 'syncfusion\|ejs-schedule\|ScheduleModule'; then
  WARNINGS="$WARNINGS\n  - Custom calendar found. Use Syncfusion ejs-schedule (ScheduleComponent)."
fi

if echo "$CONTENT" | grep -qE 'contentEditable|contenteditable' && ! echo "$CONTENT" | grep -qi 'syncfusion\|ejs-richtexteditor\|RichTextEditorModule'; then
  WARNINGS="$WARNINGS\n  - contentEditable found. Use Syncfusion ejs-richtexteditor."
fi

if [ -n "$WARNINGS" ]; then
  echo "WARNING: $FILE_PATH uses custom UI where Syncfusion components are required."
  echo -e "Issues:$WARNINGS"
  echo "All UI MUST use Syncfusion EJ2 Angular components."
  exit 1
fi

exit 0
