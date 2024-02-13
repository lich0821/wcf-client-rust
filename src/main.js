// import { confirm } from '@tauri-apps/api/dialog';
const { invoke } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;

let urlInputEl;
let logTextarea;

let btnStart;
var flag = false;

async function _start(cburl) {
    invoke('start_server', { "host": "0.0.0.0", "port": 10010, "cburl": cburl });
    btnStart.textContent = "停止";
}

async function _stop() {
    invoke('stop_server');
    btnStart.textContent = "启动";
}

async function start(cburl) {
    if (flag) {
        await _stop();
        flag = false;
    } else {
        await _start(cburl);
        flag = true;
    }
}

function appendLogWithLimit(message, maxLines = 9999) {
    logTextarea.value += message + "\n";
    let lines = logTextarea.value.split("\n");

    if (lines.length > maxLines) {
        lines = lines.slice(lines.length - maxLines);
        logTextarea.value = lines.join("\n");
    }

    logTextarea.scrollTop = logTextarea.scrollHeight;
}

async function startSerialEventListener() {
    await listen('log-message', (logMessage) => {
        appendLogWithLimit(logMessage.payload);
    });
}

async function confirmExit() {
    const shouldExit = await confirm("退出将无法使用服务，确定要退出吗？");
    if (shouldExit) {
        await invoke('confirm_exit');
    }
}

async function startExitEventListener() {
    await listen('request-exit', () => {
        confirmExit();
    });
}

window.addEventListener("DOMContentLoaded", () => {
    urlInputEl = document.querySelector("#cburl");
    logTextarea = document.querySelector("#log");
    btnStart = document.getElementById("btn-start");

    logTextarea.textContent = "填写回调地址（不填写也可以，消息会显示在此处），然后点击【启动】\n";
    document.querySelector("#log-form").addEventListener("submit", (e) => {
        e.preventDefault();
        start(urlInputEl.value);
    });
    startSerialEventListener();
    startExitEventListener();
});
