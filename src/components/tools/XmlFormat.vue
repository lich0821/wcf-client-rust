
import { log } from 'console';
<template>
    <el-container>
        <el-header>
            <el-space>
                <el-button type="success" @click="pretty">格式化</el-button>
            </el-space>
        </el-header>
        <el-main>
            <v-ace-editor h="full" ref="aceRef" v-model:value="content" lang="xml"
                :theme="isDark ? 'monokai' : 'chrome'" :options="options" />
        </el-main>
    </el-container>
</template>

<script lang="ts" setup>
import { ref } from 'vue';
import { isDark } from "@/composables";
import { VAceEditor } from 'vue3-ace-editor';
import '@/components/ace/vace.config';
import 'ace-builds/src-noconflict/mode-xml'; // Load the language definition file used below
import 'ace-builds/src-noconflict/theme-chrome'; // Load the theme definition file used below
import 'ace-builds/src-noconflict/ext-language_tools';
import xmlFormat from 'xml-formatter';

const aceRef: any = ref(null);
const content: any = ref('');
const options: any = ref({
    useWorker: true, // 启用语法检查,必须为true
    //代码提示及自动补全
    enableBasicAutocompletion: true, // 自动补全
    enableLiveAutocompletion: true, // 智能补全
    enableSnippets: true, // 启用代码段
    showPrintMargin: false, // 去掉灰色的线，printMarginColumn
    highlightActiveLine: true, // 高亮行
    highlightSelectedWord: true, // 高亮选中的字符
    tabSize: 4, // tab锁进字符
    fontSize: 14, // 设置字号
    wrap: false, // 是否换行
    readonly: false, // 是否可编辑
});

const pretty = async () => {
    if (!aceRef.value) return;
    let instance = aceRef.value.getAceInstance();
    let selected = instance.getSelectedText();
    if (selected) {
        instance.session.replace(instance.selection.getRange(), xmlFormat(selected.replace(/[\\n\\r\\t]/g, '')));
    } else {
        // debugger
        content.value = xmlFormat(content.value.replace(/[\\n\\r\\t]/g, ''));
    }
}

</script>

<style lang="scss" scoped>
.el-container {
    padding: 0 10px 10px 0;
    height: calc(100vh - var(--header-height));

    >header {
        height: var(--header-height);
        display: flex;
        justify-content: flex-end;
    }

    >.el-main {
        padding: 0;
        border: 1px solid var(--el-border-color);

        >.ace-editor {
            height: 100%;
        }
    }
}
</style>