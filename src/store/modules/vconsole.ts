import VConsole from 'vconsole';
import { defineStore } from 'pinia';
import { useDark } from "@vueuse/core";

export const useVConsoleStore = defineStore('vconsole', {
    state: () => {
        return {
            vConsole: {} as any
        }
    },
    actions: {
        async init(target: any) { 
            this.vConsole = new VConsole({
                theme: 'dark',
                defaultPlugins: [],
                target: target
            });             
        },
        async show() { 
            this.vConsole.show();
        },
    },
});
