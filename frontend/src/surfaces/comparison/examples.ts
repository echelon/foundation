export const quoteExample = `curl http://localhost:4000/v1/quotes \\
  -H 'Content-Type: application/json' \\
  -d '{
    "model": "studio_image_demo",
    "workload": {
      "kind": "image",
      "count": 1,
      "aspect_ratio": "7:5"
    }
  }'`;

export const chatExample = `curl -i http://localhost:4000/v1/chat/completions \\
  -H "Authorization: Bearer $TOKENCRAFT_API_KEY" \\
  -H "Idempotency-Key: $TOKENCRAFT_REQUEST_KEY" \\
  -H 'Content-Type: application/json' \\
  -d '{
    "model": "studio_chat_wire_demo",
    "messages": [{"role": "user", "content": "Hello, TokenCraft"}],
    "max_completion_tokens": 128
  }'`;
