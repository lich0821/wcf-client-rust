import { defineStore } from 'pinia';
import wcf from '@/command/wcf';

export const useWechatStore = defineStore('wechat', {
    state: () => {
        return {
            selfInfo: getSlefInfo(),
            isServerRunning: false,
        }
    },
    actions: {
        setSelfInfo(selfInfo: any) {
            this.selfInfo = selfInfo;
            sessionStorage.setItem('selfInfo', JSON.stringify(selfInfo));
        },
        async start() { 
            await wcf.start_server('0.0.0.0', 10010, "");
            this.isServerRunning = await wcf.is_http_server_running();
        },
        async stop() { 
            await wcf.stop_server();
            this.isServerRunning = await wcf.is_http_server_running();
        },
    },
});

const getSlefInfo = () => { 
    let info = sessionStorage.getItem('selfInfo');
    if (!info) { 
        return {};
    }
    return JSON.parse(info);
}