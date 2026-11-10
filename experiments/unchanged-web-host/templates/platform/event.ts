export type UnlistenFn = () => void
export interface Event<T> { event: string; id: number; payload: T }

export async function listen<T>(name: string, handler: (event: Event<T>) => void): Promise<UnlistenFn> {
  const source = new EventSource(`/_llmwiki/events?event=${encodeURIComponent(name)}`)
  let id = 0
  source.onmessage = (message) => handler({ event: name, id: ++id, payload: JSON.parse(message.data) as T })
  await new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => { source.close(); reject(new Error(`Event stream timed out: ${name}`)) }, 10000)
    source.onopen = () => { clearTimeout(timer); resolve() }
    source.onerror = () => { clearTimeout(timer); source.close(); reject(new Error(`Event stream unavailable: ${name}`)) }
  })
  // EventSource reconnects automatically after an established stream drops.
  source.onerror = null
  return () => source.close()
}
