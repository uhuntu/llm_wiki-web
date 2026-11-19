export type UnlistenFn = () => void
export interface Event<T> { event: string; id: number; payload: T }

type Handler = (event: Event<unknown>) => void
const handlers = new Map<string, Set<Handler>>()
let source: EventSource | undefined
let ready: Promise<void> | undefined
let nextId = 0

function connect(): Promise<void> {
  if (ready) return ready
  const stream = new EventSource('/_llmwiki/events?event=_web_all')
  source = stream
  stream.onmessage = (message) => {
    const frame = JSON.parse(message.data) as { event: string; payload: unknown }
    const event = { event: frame.event, payload: frame.payload, id: ++nextId }
    for (const handler of handlers.get(frame.event) ?? []) handler(event)
  }
  ready = new Promise<void>((resolve, reject) => {
    const fail = () => {
      clearTimeout(timer)
      stream.close()
      if (source === stream) { source = undefined; ready = undefined }
      reject(new Error('Web event stream unavailable'))
    }
    const timer = setTimeout(fail, 10000)
    stream.onopen = () => {
      clearTimeout(timer)
      stream.onerror = null // EventSource reconnects after an established connection drops.
      resolve()
    }
    stream.onerror = fail
  })
  return ready
}

export async function listen<T>(name: string, handler: (event: Event<T>) => void): Promise<UnlistenFn> {
  // Use a distinct wrapper for each subscription, including identical callbacks.
  const callback: Handler = (event) => handler(event as Event<T>)
  let list = handlers.get(name)
  if (!list) { list = new Set(); handlers.set(name, list) }
  list.add(callback)
  const remove = () => {
    list.delete(callback)
    if (!list.size && handlers.get(name) === list) handlers.delete(name)
    if (!handlers.size) {
      source?.close()
      source = undefined
      ready = undefined
    }
  }
  try { await connect() } catch (error) { remove(); throw error }
  return remove
}
