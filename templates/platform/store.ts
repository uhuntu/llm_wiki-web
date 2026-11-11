import { invoke } from './core'

class WebStore {
  constructor(private readonly name: string) {}
  get<T>(key: string): Promise<T | undefined> {
    return invoke<T | undefined>('_web_store', { store: this.name, action: 'get', key })
  }
  async set(key: string, value: unknown): Promise<void> {
    await invoke('_web_store', { store: this.name, action: 'set', key, value })
  }
  delete(key: string): Promise<boolean> {
    return invoke<boolean>('_web_store', { store: this.name, action: 'delete', key })
  }
  async save(): Promise<void> {
    // Each mutation is committed by the server before its response returns.
    await invoke('_web_store', { store: this.name, action: 'flush' })
  }
}

export async function load(name: string, _options?: unknown): Promise<WebStore> {
  if (name !== 'app-state.json') throw new Error(`Unsupported settings store: ${name}`)
  return new WebStore(name)
}
