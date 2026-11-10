let askOnClose = false
let closeHandlerInstalled = false

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const response = await fetch('/_llmwiki/invoke', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ command, args }),
  })
  const result = await response.json()
  if (!response.ok || result.ok !== true) {
    throw new Error(result.error || `Command ${command} failed (${response.status})`)
  }
  if (command === 'open_project_folder' || command === 'open_path_in_project') {
    const target = result.value as { path: string; isDir: boolean }
    window.open(target.isDir ? `/_llmwiki/files?path=${encodeURIComponent(target.path)}` : convertFileSrc(target.path), '_blank', 'noopener,noreferrer')
  }
  if (command === 'set_close_behavior') {
    askOnClose = args.value === 'ask'
    if (!closeHandlerInstalled) {
      window.addEventListener('beforeunload', (event) => { if (askOnClose) event.preventDefault() })
      closeHandlerInstalled = true
    }
  }
  if (command === 'export_project_archive' && typeof args.destination === 'string') {
    const download = document.createElement('a')
    download.href = convertFileSrc(args.destination)
    download.download = args.destination.split('/').pop() || 'project.llmwiki.zip'
    download.click()
  }
  return result.value as T
}

export function convertFileSrc(path: string): string {
  return `/_llmwiki/asset?path=${encodeURIComponent(path)}`
}
