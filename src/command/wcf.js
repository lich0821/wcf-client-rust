import { tauri, event } from '@tauri-apps/api';

/** 获取内网IP */
async function ip() { 
  return await tauri.invoke('ip');
}

/** 开启服务 */
async function start_server(host, port, cburl) { 
  return await tauri.invoke('start_server', { "host": host, "port": port, "cburl": cburl });
}

/** 关闭服务 */
async function stop_server() { 
  return await tauri.invoke('stop_server', {});
}

/** 服务是否在运行 */
async function is_http_server_running() { 
    return await tauri.invoke('is_http_server_running', {});
}

/** 退出 */
async function exit() { 
  await tauri.invoke('confirm_exit');
}

export default {
    ip,
    start_server,
    stop_server,
    is_http_server_running,
    exit
}