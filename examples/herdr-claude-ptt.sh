#!/usr/bin/env bash
# examples/herdr-claude-ptt.sh
# Headless Cross-Desktop Push-to-Talk into Claude Code (via holdkey + superwhisper + herdr)
#
# Hold your PTT key (e.g. F10) anywhere across any workspace.
# On release, transcribed voice is injected straight into your Claude Code pane in Herdr
# without moving focus, changing windows, or interrupting your active work.

set -u

on_press() {
  echo "[PTT] Key pressed: starting voice recording..."
  superwhisper start >/dev/null 2>&1 || true
}

on_release() {
  echo "[PTT] Key released: stopping recording and transcribing..."
  superwhisper stop >/dev/null 2>&1 || true

  # Poll until transcription is finalized
  TEXT=""
  for i in {1..40}; do
    sleep 0.1
    STATUS=$(superwhisper status 2>/dev/null || true)
    PHASE=$(echo "$STATUS" | jq -r '.phase // "idle"' 2>/dev/null || echo "idle")

    if [ "$PHASE" = "idle" ]; then
      TEXT=$(echo "$STATUS" | jq -r '.last.text // empty' 2>/dev/null || true)
      [ -n "$TEXT" ] && break
    fi
  done

  if [ -z "$TEXT" ]; then
    TEXT=$(superwhisper read 2>/dev/null | jq -r '.text // empty' 2>/dev/null || true)
  fi

  if [ -n "$TEXT" ]; then
    echo "[PTT] Transcribed: \"$TEXT\""

    # Query Herdr for an active Claude Code pane
    CLAUDE_PANE=$(herdr pane list 2>/dev/null | jq -r '(.result.panes[] | select(.agent == "claude") | .pane_id) // empty' 2>/dev/null | head -n 1)

    if [ -n "$CLAUDE_PANE" ]; then
      echo "[PTT] Injected into Herdr Claude pane ($CLAUDE_PANE)"
      herdr pane send-text "$CLAUDE_PANE" "$TEXT" 2>/dev/null || true
      notify-send -a "Claude Code PTT" "→ Claude Code ($CLAUDE_PANE)" "$TEXT" -i terminal 2>/dev/null || true
    else
      echo "[PTT] No Herdr Claude pane found; pasting at focused cursor..."
      wtype "$TEXT" 2>/dev/null || true
      notify-send -a "Voice PTT" "Pasted at cursor" "$TEXT" -i terminal 2>/dev/null || true
    fi
  else
    echo "[PTT] No speech transcribed."
  fi
}

case "${1:-watch}" in
  press)
    on_press
    ;;
  release)
    on_release
    ;;
  watch)
    echo "holdkey PTT daemon listener active. Monitoring events..."
    holdkey watch | while IFS= read -r line; do
      [ -z "$line" ] && continue
      event=$(echo "$line" | jq -r '(.event // "")' 2>/dev/null || true)
      action=$(echo "$line" | jq -r '(.action_id // .action // "")' 2>/dev/null || true)

      if [ "$action" = "ptt" ]; then
        if [ "$event" = "pressed" ]; then
          on_press
        elif [ "$event" = "released" ]; then
          on_release
        fi
      fi
    done
    ;;
  *)
    echo "Usage: $0 [press|release|watch]"
    exit 1
    ;;
esac
