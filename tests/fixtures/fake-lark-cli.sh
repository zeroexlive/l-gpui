#!/bin/sh
# Fake lark-cli used by tests/fake_cli.rs. Behaviour is selected with
# FAKE_LARK_MODE: "ok" (default), "unconfigured", "signed_out", "api_error".
# Every invocation's argv is appended to $FAKE_LARK_LOG when set.
if [ -n "$FAKE_LARK_LOG" ]; then
  printf '%s\n' "$*" >> "$FAKE_LARK_LOG"
fi
mode="${FAKE_LARK_MODE:-ok}"
if [ "$mode" = "unconfigured" ]; then
  echo '{"ok":false,"error":{"type":"config","subtype":"not_configured","message":"not configured","hint":"run `lark-cli config init --new`"}}' >&2
  exit 3
fi
if [ "$mode" = "api_error" ] && [ "$1" != "auth" ]; then
  echo '{"ok":false,"error":{"type":"api","message":"permission denied: missing scope","hint":"run lark-cli auth login --recommend"}}' >&2
  exit 1
fi
case "$1 $2" in
  "auth status")
    if [ "$mode" = "signed_out" ]; then
      echo '{"identity":"none","app_id":"cli_test"}'
    else
      echo '{"identity":"user","user_name":"Test User","app_id":"cli_test"}'
    fi ;;
  "im +chat-list")
    echo 'warning: something harmless' >&2
    echo '{"ok":true,"data":{"chats":[{"chat_id":"oc_a","name":"Design Team","chat_mode":"group"},{"chat_id":"oc_b","name":"Ada Lovelace","chat_mode":"p2p"}],"has_more":false}}' ;;
  "im +chat-messages-list")
    echo '{"ok":true,"data":{"messages":[{"message_id":"om_2","content":"Looks good","sender":{"name":"Ada"},"create_time":"1700000060000"},{"message_id":"om_1","content":"Draft ready?","sender":{"name":"Test User"},"create_time":"1700000000000"}]}}' ;;
  "im +messages-send")
    echo '{"ok":true,"data":{"message_id":"om_new"}}' ;;
  "im +messages-search")
    echo '{"ok":true,"data":{"messages":[{"message_id":"om_9","chat_name":"Design Team","content":"draft v2","sender":{"name":"Ada"}}]}}' ;;
  "calendar +agenda")
    echo '{"ok":true,"data":[{"event_id":"ev1","summary":"Standup","start_time":{"datetime":"2030-01-01T09:00:00Z"},"end_time":{"datetime":"2030-01-01T09:15:00Z"}}]}' ;;
  "task +get-my-tasks")
    echo '{"ok":true,"data":{"items":[{"guid":"t1","summary":"Ship l-gpui","url":"https://applink.example/t1","due_at":"2030-01-02 18:00:00"}]}}' ;;
  "task +complete"|"task +create")
    echo '{"ok":true,"data":{"guid":"t1"}}' ;;
  "docs +search")
    echo '{"ok":true,"data":{"results":[{"title_highlighted":"<em>Roadmap</em> 2030","result_meta":{"url":"https://example.larksuite.com/docx/x"}}]}}' ;;
  "wiki +space-list")
    echo '{"ok":true,"data":{"spaces":[{"space_id":"s1","name":"Engineering"}]}}' ;;
  "mail +triage")
    echo '{"messages":[{"message_id":"m1","date":"Sat, 21 Mar 2026 11:40:00 +0800","from":"Alice <alice@example.com>","subject":"Weekly update","labels":"INBOX,UNREAD"}],"has_more":false}' ;;
  "mail +message")
    echo '{"ok":true,"data":{"message_id":"m1","subject":"Weekly update","from":"Alice <alice@example.com>","to":["me@example.com"],"body_plain_text":"All green."}}' ;;
  "mail +send")
    echo '{"ok":true,"data":{"message_id":"m2"}}' ;;
  "contact +search-user")
    echo '{"ok":true,"data":{"users":[{"open_id":"ou_1","name":"Ada Lovelace","email":"ada@example.com"}]}}' ;;
  "approval tasks")
    echo '{"code":0,"data":{"tasks":[{"task_id":"at1","instance_code":"ic1","title":"Expense report","initiator_names":["Bob"],"status":"PENDING"}]}}' ;;
  "minutes +search")
    echo '{"ok":true,"data":{"items":[{"minute_token":"obc1","title":"Weekly sync","url":"https://example.larksuite.com/minutes/obc1"}]}}' ;;
  *)
    echo "unknown command: $*" >&2
    exit 2 ;;
esac
