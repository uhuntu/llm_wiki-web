import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
const host = path.dirname(fileURLToPath(import.meta.url))
const upstream = path.join(host, 'upstream')
const replacements = [['@tauri-apps/api/core','core'],['@tauri-apps/api/event','event'],['@tauri-apps/plugin-store','store'],['@tauri-apps/plugin-dialog','dialog'],['@tauri-apps/plugin-opener','opener'],['@tauri-apps/plugin-autostart','autostart'],['@tauri-apps/plugin-http','http']]
function relativeWikiLinks() {
  return {
    name: 'external-relative-wiki-links',
    enforce: 'pre',
    transform(code, id) {
      if (!id.replaceAll('\\', '/').endsWith('/src/components/editor/wiki-reader.tsx')) return
      const before = '    if (!href.startsWith("#")) return'
      const after = `    if (!href.startsWith("#")) {
      if (!currentFileDir || !wikiRoot || /^(?:[a-z][a-z0-9+.-]*:|\\/|\\?)/i.test(href)) return
      let target: string
      try {
        const url = new URL(href, "https://wiki.invalid" + currentFileDir + "/")
        target = decodeURIComponent(url.pathname)
      } catch { return }
      if (!target.toLowerCase().endsWith(".md")) return
      e.preventDefault()
      if (target.startsWith(wikiRoot + "/") && projectPathIndex.byPath.has(target)) {
        openPathInPreview(target)
      }
      return
    }`
      const click = 'onClick={(e) => isWikilink && handleAnchorClick(e, h)}'
      if (code.split(before).length !== 2 || code.split(click).length !== 2) {
        throw new Error('Pinned WikiReader link handler no longer matches adapter')
      }
      return code.replace(before, after).replace(click, 'onClick={(e) => handleAnchorClick(e, h)}')
    },
  }
}
function markdownGraphLinks() {
  return {
    name: 'external-markdown-graph-links',
    enforce: 'pre',
    transform(code, id) {
      if (!id.replaceAll('\\', '/').endsWith('/src/lib/wiki-graph.ts')) return
      const before = '...extractWikilinks(content), ...extractRelated(content)'
      if (code.split(before).length !== 2) throw new Error('Pinned graph link extraction no longer matches adapter')
      return code.replace(before, before + ', ...extractMarkdownGraphLinks(content, file.path, wikiRoot, mdFiles)') + `
function extractMarkdownGraphLinks(content: string, filePath: string, wikiRoot: string, files: FileNode[]): string[] {
  const paths = new Map(files.map(f => [normalizePath(f.path), f.name.replace(/\\.md$/i, "")]))
  const clean = content.replace(/^ {0,3}(\\x60{3,}|~{3,})[^\\n]*\\n[\\s\\S]*?^ {0,3}\\1[^\\n]*$/gm, "").replace(/\\x60+[^\\x60]*\\x60+/g, "")
  const links: string[] = []
  for (const match of clean.matchAll(/(?<!!)\\[[^\\]\\n]*\\]\\(<?([^\\s)>]+)>?(?:[ \\t]+["'][^"']*["'])?\\)/g)) {
    const href = match[1]
    if (/^(?:[a-z][a-z0-9+.-]*:|\\/|#|\\?)/i.test(href)) continue
    try {
      const target = decodeURIComponent(new URL(href, "https://wiki.invalid" + filePath).pathname)
      if (!target.startsWith(wikiRoot + "/")) continue
      const name = paths.get(target)
      if (name) links.push(name)
    } catch { /* Invalid links cannot create graph edges. */ }
  }
  return links
}
`
    },
  }
}
// Install before stylesheet tags so a failed first load can recover without a reload.
function retryStylesheet() {
  return {
    name: 'external-retry-stylesheet',
    transformIndexHtml: {
      order: 'pre',
      handler(html) {
        return html.replace('<head>', `<head><script>
document.addEventListener('error', (event) => {
  const link = event.target
  if (!(link instanceof HTMLLinkElement) || link.rel !== 'stylesheet') return
  const url = new URL(link.href)
  if (url.origin !== location.origin || !url.pathname.startsWith('/assets/') ||
      !url.pathname.endsWith('.css') || link.dataset.styleRetry === '1') return
  const retry = link.cloneNode()
  retry.dataset.styleRetry = '1'
  link.replaceWith(retry)
}, true)
</script>`)
      },
    },
  }
}
// This host has no desktop clip service to poll.
const disableDesktopClipWatcher = {
  name: 'external-disable-desktop-clip-watcher',
  enforce: 'pre',
  transform(_code, id) {
    if (!id.endsWith('/src/lib/clip-watcher.ts')) return
    return 'export function startClipWatcher() {}\nexport function stopClipWatcher() {}'
  },
}
export default defineConfig({
  root: upstream,
  plugins: [disableDesktopClipWatcher, retryStylesheet(), {name:'external-native-fetch-boundary',transformIndexHtml:{order:'pre',handler(html){return html.replace('<head>',`<head><script>
(function(){const native=window.fetch.bind(window);window.fetch=function(input,init){const url=new URL(input instanceof Request?input.url:String(input),location.href);if(url.protocol==='http:'&&['localhost','127.0.0.1'].includes(url.hostname)&&['19827','19828'].includes(url.port)){const target='/_llmwiki/unavailable/'+url.port+url.pathname+url.search;return native(input instanceof Request?new Request(target,input):target,init)}return native(input,init)}})();
</script>`)}}},relativeWikiLinks(),markdownGraphLinks(),react(),tailwindcss()],
  resolve: { alias: {...Object.fromEntries(replacements.map(([name,file])=>[name,path.join(host,'platform',file+'.ts')])), '@':path.join(upstream,'src')} },
  define: {__APP_VERSION__:JSON.stringify(JSON.parse(readFileSync(path.join(upstream,'package.json'),'utf8')).version)},
  build: {outDir:path.join(host,'dist'),emptyOutDir:true},
})
