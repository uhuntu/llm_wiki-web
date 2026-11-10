import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { readFileSync } from 'node:fs'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
const host = path.dirname(fileURLToPath(import.meta.url))
const upstream = path.join(host, 'upstream')
const replacements = [['@tauri-apps/api/core','core'],['@tauri-apps/api/event','event'],['@tauri-apps/plugin-store','store'],['@tauri-apps/plugin-dialog','dialog'],['@tauri-apps/plugin-opener','opener'],['@tauri-apps/plugin-autostart','autostart'],['@tauri-apps/plugin-http','http']]
export default defineConfig({
  root: upstream,
  plugins: [{name:'external-native-fetch-boundary',transformIndexHtml:{order:'pre',handler(html){return html.replace('<head>',`<head><script>
(function(){const native=window.fetch.bind(window);window.fetch=function(input,init){const url=new URL(input instanceof Request?input.url:String(input),location.href);if(url.protocol==='http:'&&['localhost','127.0.0.1'].includes(url.hostname)&&['19827','19828'].includes(url.port)){const target='/_llmwiki/unavailable/'+url.port+url.pathname+url.search;return native(input instanceof Request?new Request(target,input):target,init)}return native(input,init)}})();
</script>`)}}},react(),tailwindcss()],
  resolve: { alias: {...Object.fromEntries(replacements.map(([name,file])=>[name,path.join(host,'platform',file+'.ts')])), '@':path.join(upstream,'src')} },
  define: {__APP_VERSION__:JSON.stringify(JSON.parse(readFileSync(path.join(upstream,'package.json'),'utf8')).version)},
  build: {outDir:path.join(host,'dist'),emptyOutDir:true},
})
