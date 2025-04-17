import { invoke } from '@tauri-apps/api/core';
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { WechatConfig } from '~/types/config';

export const useConfigStore = defineStore('config', () => {
    // 整个配置文件
    const wechatConfig = ref<WechatConfig>({
      // http 回调地址
      cburl: [],
      wsurl: '',
      // http server配置
      http_server_port: 10010,
      // 显示消息日志
      front_msg_show: true,
      file_dir: '',
      msg_filter_regexp: '',
    });
     
    const update = async () => {
      console.log(wechatConfig.value);
      let res = await invoke("save_wechat_config", { config: wechatConfig.value });
      return res
    }

    const read = async ()=>{
      let res = await invoke("read_wechat_config");
      wechatConfig.value = res as WechatConfig;
    }

    return { wechatConfig, update, read}
  })