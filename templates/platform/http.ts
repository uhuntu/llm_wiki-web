export async function fetch(input: string | URL | Request, init?: RequestInit): Promise<Response> {
  const request = new Request(input, init)
  // A same-origin server relay preserves streaming and avoids model CORS
  // requirements. The relay validates configured destinations server-side.
  return globalThis.fetch('/_llmwiki/http', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/octet-stream',
      'X-LLM-Wiki-URL': request.url,
      'X-LLM-Wiki-Method': request.method,
      'X-LLM-Wiki-Headers': JSON.stringify(Object.fromEntries(request.headers.entries())),
    },
    body: ['GET', 'HEAD'].includes(request.method) ? undefined : await request.arrayBuffer(),
    signal: request.signal,
  })
}
