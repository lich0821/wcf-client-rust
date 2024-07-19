<template>
    <el-container>
        <el-main>
            <el-tabs tab-position="left" h="full">
                <el-tab-pane label="JSON格式化">
                    <JsonFormat />
                </el-tab-pane>
                <el-tab-pane label="XML格式化">
                    <XmlFormat />
                </el-tab-pane>
            </el-tabs>
        </el-main>
    </el-container>
</template>

<script lang="ts" setup>
import JsonFormat from './tools/JsonFormat.vue';
import XmlFormat from './tools/XmlFormat.vue';

</script>

<style lang="scss" scoped>
.el-container {
    padding: 0;
    height: calc(100vh - var(--header-height));

    >.el-main {
        padding: 0;
    }
}
</style>