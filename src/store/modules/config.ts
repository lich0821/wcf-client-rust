import { invoke } from '@tauri-apps/api';
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { WechatConfig } from '~/types/config';

export const useConfigStore = defineStore('config', () => {
    // 整个配置文件
    const wechatConfig = ref<WechatConfig>({
      cburl: [],
      wsurl: '',
      file_dir: '',
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