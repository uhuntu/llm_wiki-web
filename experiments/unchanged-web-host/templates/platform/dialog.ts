import { invoke } from './core'

interface Options {
  directory?: boolean
  multiple?: boolean
  defaultPath?: string
  title?: string
  createDirectories?: boolean
  filters?: { name: string; extensions: string[] }[]
}
interface Entry { name: string; path: string; is_dir: boolean }

export async function open(options: Options = {}): Promise<string | string[] | null> {
  const info = await invoke<{ root: string }>('_web_info')
  return new Promise((resolve) => {
    const dialog = document.createElement('dialog')
    dialog.style.cssText = 'width:min(720px,90vw);height:70vh;padding:24px;border:1px solid #888;border-radius:12px;background:var(--background,#fff);color:var(--foreground,#111)'
    const title = document.createElement('h2')
    title.textContent = options.title || (options.directory ? '选择服务器目录' : '选择服务器文件')
    const current = document.createElement('p')
    const list = document.createElement('div')
    list.style.cssText = 'height:calc(100% - 140px);overflow:auto;margin:16px 0'
    const up = document.createElement('button'); up.textContent = '上一级'
    const cancel = document.createElement('button'); cancel.textContent = '取消'
    const select = document.createElement('button'); select.textContent = '选择'
    const upload = document.createElement('button'); upload.textContent = '从电脑上传文件'
    for (const b of [up, cancel, select]) b.style.cssText = 'padding:8px 16px;margin:4px;border:1px solid #888;border-radius:6px'
    let path = info.root
    const selected = new Set<string>()
    const finish = (value: string | string[] | null) => { dialog.close(); dialog.remove(); resolve(value) }
    const browse = async (next: string) => {
      path = next; current.textContent = next; list.replaceChildren(); selected.clear()
      try {
        const entries = await invoke<Entry[]>('list_directory', { path, maxDepth: 1 })
        for (const entry of entries) {
          const extensions = options.filters?.flatMap((f) => f.extensions)
          if (!entry.is_dir && (options.directory || (extensions?.length && !extensions.some((ext) => entry.name.toLowerCase().endsWith(`.${ext.toLowerCase()}`))))) continue
          const row = document.createElement('button')
          row.style.cssText = 'display:block;width:100%;padding:9px;text-align:left;border-bottom:1px solid #aaa4'
          row.textContent = `${entry.is_dir ? '📁' : '📄'} ${entry.name}`
          row.onclick = () => {
            if (entry.is_dir) { void browse(entry.path); return }
            if (!options.multiple) { finish(entry.path); return }
            if (selected.has(entry.path)) selected.delete(entry.path); else selected.add(entry.path)
            row.style.background = selected.has(entry.path) ? '#2563eb33' : ''
          }
          list.append(row)
        }
      } catch (error) { list.textContent = String(error) }
    }
    up.onclick = () => { if (path !== info.root) void browse(path.slice(0, path.lastIndexOf('/')) || info.root) }
    cancel.onclick = () => finish(null)
    upload.onclick = () => {
      const input = document.createElement('input')
      input.type = 'file'; input.multiple = options.multiple === true
      input.accept = options.filters?.flatMap((f) => f.extensions.map((ext) => `.${ext}`)).join(',') || ''
      input.onchange = async () => {
        upload.disabled = true
        const paths: string[] = []
        try {
          for (const file of Array.from(input.files || [])) {
            if (file.size > 100 * 1024 * 1024) throw new Error('单文件上传上限为 100 MB')
            upload.textContent = `正在上传 ${file.name}…`
            const response = await fetch('/_llmwiki/upload', { method: 'POST',
              headers: { 'Content-Type': 'application/octet-stream', 'X-LLM-Wiki-Filename': encodeURIComponent(file.name) }, body: file })
            const result = await response.json()
            if (!response.ok) throw new Error(result.error || 'Upload failed')
            paths.push(result.path)
          }
          if (paths.length) finish(options.multiple ? paths : paths[0])
        } catch (error) { list.textContent = String(error) }
        finally { upload.disabled = false; upload.textContent = '从电脑上传文件' }
      }
      input.click()
    }
    select.onclick = () => {
      if (options.directory) finish(options.multiple ? [path] : path)
      else if (selected.size) finish([...selected])
    }
    dialog.oncancel = (event) => { event.preventDefault(); finish(null) }
    dialog.append(title, current, up, list, cancel, select)
    if (!options.directory) dialog.append(upload)
    document.body.append(dialog); dialog.showModal(); void browse(path)
  })
}

export async function save(options: Options = {}): Promise<string | null> {
  const info = await invoke<{ root: string }>('_web_info')
  const name = options.defaultPath?.split('/').pop() || 'project.llmwiki.zip'
  const directory = `${info.root}/.downloads/${crypto.randomUUID()}`
  await invoke('create_directory', { path: directory })
  return `${directory}/${name}`
}
export async function message(text: string, _options?: unknown): Promise<void> { window.alert(text) }
