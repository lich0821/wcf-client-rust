<template>
    <el-container>
        <template>
            <svg viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="5441" width="256"
                height="256">
                <symbol id="icon-copy" viewBox="0 0 1024 1024">
                    <path :fill="isDark ? '#fff' : '#000'"
                        d="M979.2 192H832V44.8A44.8 44.8 0 0 0 787.2 0h-742.4A44.8 44.8 0 0 0 0 44.8v742.4a44.8 44.8 0 0 0 44.8 44.8H192v147.2A44.8 44.8 0 0 0 236.8 1024h742.4A44.8 44.8 0 0 0 1024 979.2v-742.4a44.8 44.8 0 0 0-44.8-44.8zM64 64H768V768H64z m896 896H256v-128h531.2a44.8 44.8 0 0 0 44.8-44.8V256h128z">
                    </path>
                </symbol>
            </svg>
            <svg viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" width="256" height="256">
                <symbol id="icon-ddl" viewBox="0 0 1024 1024">
                    <path :fill="isDark ? '#fff' : '#000'"
                        d="M140.8 517.2l169.4-164.1 0.1-15.5v-70.4l-258.1 250 258.1 249.9v-70.4l-0.1-15.5zM865.1 512.4l-169.4-164-0.1-15.5v-70.4l258.1 249.9-258.1 250v-70.5l0.1-15.5zM462.2 855h-77.6l159.1-695.4h77.6z">
                    </path>
                </symbol>

            </svg>
        </template>
        <el-header>
            <el-space>
                <el-select v-model="selectedDb" @change="getTables" style="width: 240px">
                    <el-option v-for="item in dbOptions" :key="item.value" :label="item.label" :value="item.value" />
                </el-select>
                <el-button @click="getDb">刷新数据库</el-button>
            </el-space>
            <el-space>
                <el-button type="success" @click="execCurrentLine">执行当前行</el-button>
                <el-button type="success" @click="execSql">执行选择的SQL</el-button>
                <el-button @click="formatSql">格式化</el-button>
            </el-space>
        </el-header>
        <el-main>
            <splitpanes>
                <pane min-size="10" max-size="90" size="20">
                    <el-auto-resizer>
                        <template #default="{ height }">
                            <el-table :data="currentDbTables" :height="height" w="full" :show-header="false"
                                highlight-current-row @row-contextmenu="rightClick" size="default"
                                show-overflow-tooltip>
                                <el-table-column prop="name" label="表名" />
                            </el-table>
                        </template>
                    </el-auto-resizer>
                </pane>
                <pane min-size="10" max-size="90" size="80" style="border-left: 1px solid var(--el-border-color)">
                    <splitpanes horizontal>
                        <pane min-size="10" max-size="90" size="20"
                            style="border-bottom: 1px solid var(--el-border-color);">
                            <v-ace-editor ref="aceRef" v-model:value="content" lang="sql"
                                :theme="isDark ? 'monokai' : 'chrome'" :options="options" />
                        </pane>
                        <pane min-size="10" max-size="90" size="20">
                            <el-auto-resizer>
                                <template #default="{ height }">
                                    <vxe-toolbar ref="toolbarRef" custom></vxe-toolbar>
                                    <vxe-table ref="tableRef" :data="results" :column-config="{ resizable: true }"
                                        :height="height - 60" show-header-overflow border style="margin: 5px;">
                                        <vxe-column v-for="header in headers" :field="header" :title="header"
                                            show-overflow min-width="60" width="120">
                                        </vxe-column>
                                    </vxe-table>
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
import { isDark } from "@/composables";
import ContextMenu from '@imengyu/vue3-context-menu'
import { VAceEditor } from 'vue3-ace-editor';
import '@/components/ace/vace.config';
import 'ace-builds/src-noconflict/mode-sql'; // Load the language definition file used below
import 'ace-builds/src-noconflict/theme-chrome'; // Load the theme definition file used below
import 'ace-builds/src-noconflict/ext-language_tools';
import wcf_api from '~/api/wcf_api';
import { format } from 'sql-formatter';
import useClipboard from 'vue-clipboard3';
import { VxeUI } from 'vxe-table';

const toolbarRef = ref()
const tableRef = ref()
const selectedDb = ref();
const dbOptions = ref<any[]>([]);
const headers = ref<any[]>([]);
const results = ref<any[]>([]);
const aceRef: any = ref(null);
const content = ref('');
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
const tableNames = ref<any[]>([]);
const currentDbTables = ref<any[]>([]);

const formatSql = () => {
    if (!aceRef.value) return;
    let instance = aceRef.value.getAceInstance();
    let selected = instance.getSelectedText();
    if (selected) {
        instance.session.replace(instance.selection.getRange(), format(selected, { language: 'sqlite' }));
    } else {
        content.value = format(content.value, { language: 'sqlite' })
    }
}

const execCurrentLine = async () => {
    if (!aceRef.value) return;
    let instance = aceRef.value.getAceInstance();
    let cursorPosition = instance.getCursorPosition();
    let currentRow = cursorPosition.row;
    let currentLine = instance.session.getLine(currentRow);
    if (!currentLine) return;
    headers.value = [];
    results.value = [];
    let result = await wcf_api.sql(selectedDb.value, currentLine);
    if (result && result.length > 0) {
        let item = result[0];
        headers.value = Object.keys(item).sort();
        results.value = result;
    }
}

const execSql = async () => {
    if (!aceRef.value) return;
    let instance = aceRef.value.getAceInstance();
    let sql = instance.getSelectedText();
    if (!sql) return;
    headers.value = [];
    results.value = [];
    let result = await wcf_api.sql(selectedDb.value, sql);
    if (result && result.length > 0) {
        let item = result[0];
        headers.value = Object.keys(item).sort();
        results.value = result;
    }
}

const getDb = async () => {
    let dbs: any = await wcf_api.dbs();
    let dbNames = dbs.names;
    dbOptions.value = [];
    // 获取所有表名，用于关键字提示
    for (let i = 0; i < dbNames.length; i++) {
        let name = dbNames[i];
        dbOptions.value.push({
            label: name,
            value: name
        });
        let tableData: any = await wcf_api.tables(name);
        let tables = tableData.tables;
        tables.forEach((item: any) => {
            tableNames.value.push(item.name);
        });
    }
    if (!selectedDb.value) {
        selectedDb.value = dbOptions.value.length > 0 ? dbOptions.value[0].value : null;
    }
    await getTables();
    // 动态添加关键字提示
    if (!aceRef.value) return;
    let instance = aceRef.value.getAceInstance();
    let completers = instance.completers;
    let index = completers.findIndex((item: any) => item.id && item.id == 'tableCompleter');
    if (index > -1) {
        completers.splice(index, 1);
    }
    completers.push({
        id: "tableCompleter",
        getCompletions: function (_editor: any, _session: any, _pos: any, _prefix: any, callback: any) {
            callback(
                null,
                tableNames.value.map(function (table) {
                    return {
                        caption: table,
                        value: table,
                        meta: "static",
                    };
                })
            );
        },
    });
}

const getTables = async () => {
    console.log(selectedDb.value);
    if (!selectedDb.value) return;
    let tableData: any = await wcf_api.tables(selectedDb.value);
    currentDbTables.value = tableData.tables;
}

const rightClick = async (row: any, _column: any, e: MouseEvent) => {
    console.log(row);
    e.preventDefault();
    ContextMenu.showContextMenu({
        theme: isDark.value ? 'win10 dark' : 'win10',
        x: e.x,
        y: e.y,
        items: [
            {
                label: "拷贝表名",
                svgIcon: '#icon-copy',
                divided: true,
                onClick: async () => {
                    const { toClipboard } = useClipboard()
                    await toClipboard(row.name);
                }
            },
            {
                label: "拷贝DDL",
                svgIcon: '#icon-ddl',
                onClick: async () => {
                    const { toClipboard } = useClipboard()
                    await toClipboard(format(row.sql, { language: 'sqlite' }));
                }
            }
        ]
    });
}

onMounted(async () => {
    const $table = tableRef.value
    const $toolbar = toolbarRef.value
    if ($table && $toolbar) {
        $table.connect($toolbar)
    }
    VxeUI.setTheme(isDark.value ? 'dark' : 'light');
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
        background-color: transparent;
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

            .el-tree {
                width: 100%;
            }
        }


        .ace_editor {
            width: 100%;
            height: 100%;
        }

        .vxe-toolbar {
            padding: 5px 10px 0 10px;
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