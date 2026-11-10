import { convertFileSrc } from './core'

export async function openUrl(url: string): Promise<void> {
  const parsed = new URL(url, location.origin)
  if (!['http:', 'https:', 'mailto:'].includes(parsed.protocol)) throw new Error('Unsupported URL protocol')
  window.open(parsed.href, '_blank', 'noopener,noreferrer')
}

export async function openPath(path: string): Promise<void> {
  window.open(convertFileSrc(path), '_blank', 'noopener,noreferrer')
}
