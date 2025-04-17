<template>
    <el-container>
        <el-main>
            <v-ace-editor ref="aceRef" v-model:value="content" lang="text" :options="options"
                :theme="isDark ? 'monokai' : 'chrome'" />
        </el-main>
        <el-footer>
            <el-space>
                <el-switch v-model="configStore.wechatConfig.front_msg_show" size="default" inline-prompt
                    style="--el-switch-on-color: #13ce66;" active-text="显示消息日志" inactive-text="关闭消息日志"
                    @change="handleMsgShowEvent" />

                <el-switch v-model="options.wrap" size="default" inline-prompt style="--el-switch-on-color: #13ce66;"
                    active-text="自动换行开启" inactive-text="自动换行关闭" @change="handleOptionsChange" />
                <el-button @click="clear">清空</el-button>
            </el-space>
        </el-footer>
    </el-container>
</template>

<script lang="ts" setup>
import { onMounted } from 'vue';
import { isDark } from "@/composables";
import { listen } from '@tauri-apps/api/event';
import { ref } from 'vue';
import { VAceEditor } from 'vue3-ace-editor';
import '@/components/ace/vace.config';
import 'ace-builds/src-noconflict/mode-text'; // Load the language definition file used below
import 'ace-builds/src-noconflict/theme-chrome'; // Load the theme definition file used below
import { useConfigStore } from "@/store/modules/config";

const configStore = useConfigStore();

const aceRef: any = ref(null);
const content = ref('');
const options: any = ref({
    useWorker: true, // 启用语法检查,必须为true
    enableBasicAutocompletion: false, // 自动补全
    enableLiveAutocompletion: false, // 智能补全
    enableSnippets: false, // 启用代码段
    showPrintMargin: false, // 去掉灰色的线，printMarginColumn
    highlightActiveLine: true, // 高亮行
    highlightSelectedWord: true, // 高亮选中的字符
    tabSize: 4, // tab锁进字符
    fontSize: 14, // 设置字号
    wrap: false, // 是否换行
    readonly: true, // 是否可编辑
});

const appendLogWithLimit = (message: any, maxLines = 9999) => {
    if (message.indexOf('NewEvents emitted without explicit RedrawEventsCleared') > -1) return;
    if (message.indexOf('RedrawEventsCleared emitted without explicit MainEventsCleared') > -1) return;
    content.value += message + "\n";
    let lines = content.value.split("\n");
    if (lines.length > maxLines) {
        lines = lines.slice(lines.length - maxLines);
        content.value = lines.join("\n");
    }
    if (!aceRef.value) return;
    aceRef.value.getAceInstance().renderer.scrollToLine(Number.POSITIVE_INFINITY)
}

const handleMsgShowEvent = async () => {
    await configStore.update();
}

const handleOptionsChange = () => {
    if (!aceRef.value) return;
    aceRef.value.getAceInstance().setOptions(options.value);
}

const clear = () => {
    content.value = '';
}

onMounted(async () => {
    await listen('log-message', (msg) => {
        appendLogWithLimit(msg.payload);
    });
})
</script>

<style lang="scss" scoped>
.cburl {
    width: 500px;
}

.el-container {
    padding: 0;
    height: calc(100vh - var(--header-height));

    >header {
        height: var(--header-height);
        padding: 8px;
    }

    >.el-main {
        padding: 0;

        >div {
            height: 100%;
        }
    }

    >.el-footer {
        height: calc(var(--header-height) - 1px);
        border-top: 1px solid var(--el-border-color);
        display: flex;
        justify-content: flex-end;
    }
}
</style>