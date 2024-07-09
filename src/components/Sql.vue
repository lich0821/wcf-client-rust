<template>
    <el-container>
        <el-main>
            <splitpanes class="default-theme">
                <pane min-size="20" max-size="60" size="20">
                    <div class="db-container">
                        <el-tree :data="dbTree"/>
                    </div>
                    
                </pane>
                <pane>
                    <v-ace-editor
                        ref="aceRef"
                        v-model:value="content"
                        lang="sql"
                        theme="chrome"
                        :options="options"
                    />
                </pane>
            </splitpanes>
        </el-main>
    </el-container>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue';
import { VAceEditor } from 'vue3-ace-editor';
import '@/components/ace/vace.config';
import 'ace-builds/src-noconflict/mode-sql'; // Load the language definition file used below
import 'ace-builds/src-noconflict/theme-chrome'; // Load the theme definition file used below
import wcf_api from '~/api/wcf_api';

const aceRef: any = ref(null);
const content = ref('select * from user');
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
    readonly: true, // 是否可编辑
});
const dbTree = ref<any[]>([]);

const getDb = async () => { 
    let dbs: any = await wcf_api.dbs();
    console.log(dbs);
    let dbNames = dbs.names;
    let tree: any[] = [];
    for (let i = 0; i < dbNames.length; i++) { 
        let name = dbNames[i];
        let tableData: any = await wcf_api.tables(name);
        let tables = tableData.tables;
        tree.push({
            label: name,
            children: tables.map((table: any) => {      
                return {
                    label: table.name,
                    sql: table.sql
                };
            })
        });
    }
    dbTree.value = tree;
}

onMounted(async () => { 
    await getDb();
})
</script>

<style lang="scss" scoped>
.el-container {
    padding: 0;
    height: calc(100vh - var(--header-height));

    >.el-main {
        padding: 0;

        >div {
            height: 100%;
        }

        .db-container {
            width: 100%;
            height: 100%;
            overflow: auto;

            .el-tree {
                width: 100%;
            }
        }
        

        .ace_editor {
            width: 100%;
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