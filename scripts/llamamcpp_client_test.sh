curl -s http://localhost:8080/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{
    "messages": [
      {
        "role": "user",
        "content": "Return exactly: hello"
      }
    ],
    "max_tokens": 256,
    "temperature": 0,
    "chat_template_kwargs": {
      "enable_thinking": false
    }
  }' | jq '.choices[0]'
