<template>
    <el-container>
        <el-header>
            <el-space>
                <el-select v-model="selectedDb" style="width: 240px">
                    <el-option
                        v-for="item in dbOptions"
                        :key="item.value"
                        :label="item.label"
                        :value="item.value"
                    />
                </el-select>
                <el-button @click="getDb">刷新数据库</el-button>
            </el-space>
            <el-button type="success" @click="execSql">执行选择的SQL</el-button>
        </el-header>
        <el-main>
            <splitpanes class="default-theme">
                <pane min-size="2" max-size="80" size="20">
                    <div class="db-container">
                        <el-tree :data="dbTree"/>
                    </div>
                </pane>
                <pane min-size="2" max-size="80" size="80" style="border-left: 1px solid var(--el-border-color)">
                    <splitpanes horizontal>
                        <pane min-size="2" max-size="80" size="20" style="border-bottom: 1px solid var(--el-border-color);">
                            <v-ace-editor
                                ref="aceRef"
                                v-model:value="content"
                                lang="sql"
                                theme="chrome"
                                :options="options"
                            />
                        </pane>
                        <pane min-size="2" max-size="80" size="20">
                            <el-auto-resizer>
                                <template #default="{ height, width }">
                                    <el-table :data="results" fit highlight-current-row :style="{width: `${width - 5}px`}" :height="height - 5" border style="margin: 2px;">
                                        <el-table-column v-for="header in headers" :prop="header" :label="header" show-overflow-tooltip />
                                    </el-table>
                                </template>
                            </el-auto-resizer>
                        </pane>
                    </splitpanes>
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

const selectedDb = ref();
const dbOptions = ref<any[]>([]);
const headers = ref<any[]>([]);
const results = ref<any[]>([]);
const aceRef: any = ref(null);
const content = ref('select * from OpLog limit 10');
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

const execSql = async () => { 
    if (!aceRef.value) return;
    let sql = aceRef.value.getAceInstance().getSelectedText();
    if (!sql) return;
    let result = await wcf_api.sql(selectedDb.value, sql);
    if (result && result.length > 0) { 
        let item = result[0];
        headers.value = Object.keys(item);
        console.log(headers);
        results.value = result;
    }
}

const getDb = async () => { 
    let dbs: any = await wcf_api.dbs();
    console.log(dbs);
    let dbNames = dbs.names;
    let tree: any[] = [];
    dbOptions.value = [];
    for (let i = 0; i < dbNames.length; i++) { 
        let name = dbNames[i];
        dbOptions.value.push({
            label: name,
            value: name
        });
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
    if (!selectedDb.value) { 
        selectedDb.value = dbOptions.value.length > 0 ? dbOptions.value[0].value : null;
    }
}

onMounted(async () => { 
    await getDb();
})
</script>

<style lang="scss" scoped>

.el-container {
    padding: 0;
    height: calc(100vh - var(--header-height));

    :deep(.splitpanes__pane) {
        display: block;
        font-family: Helvetica, Arial, sans-serif;
        font-size: 5em;
    }

    >header {
        height: 35px;
        display: flex;
        justify-content: space-between;
        padding: 6px 10px;
        border-bottom: 1px solid var(--el-border-color);
    }

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