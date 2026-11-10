export async function isEnabled(): Promise<boolean> { return false }
export async function enable(): Promise<void> {
  throw new Error('Web 服务的开机启动由服务器部署配置管理。')
}
export async function disable(): Promise<void> {
  throw new Error('Web 服务的开机启动由服务器部署配置管理。')
}
