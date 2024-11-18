<script lang="ts" setup>
import {
    Delete,
} from '@element-plus/icons-vue'
import { useConfigStore } from "@/store/modules/config";
import { ElMessage } from 'element-plus'

const configStore = useConfigStore();

const removeDomain = (item: string) => {
    const index = configStore.wechatConfig.cburl.indexOf(item)
    if (index !== -1) {
        configStore.wechatConfig.cburl.splice(index, 1)
    }
}

const addDomain = () => {
    configStore.wechatConfig.cburl.push('')
}

const submitForm = async () => {
    const res = await configStore.update();
    if (res) {
        ElMessage({
            message: '保存成功',
            type: 'success',
        })
    } else {
        ElMessage.error('保存错误')
    }
}
</script>

<template>
    <el-container>
        <el-main>
            <el-form ref="formRef" :model="configStore.wechatConfig" label-width="auto" class="demo-dynamic w-full">
                <el-button type="success" size="large" @click="submitForm()">保存</el-button>
                <el-card class="w-full mt-4">
                    <template #header>http 回调地址</template>
                    <el-form-item>
                        <el-row v-for="(http, index) in configStore.wechatConfig.cburl" :key="index"
                            class="w-full border-blue-600 m-b-2">
                            <el-col :span="22"><el-input v-model="configStore.wechatConfig.cburl[index]" /></el-col>
                            <el-col class="text-center" :span="2">
                                <el-button type="danger" @click.prevent="removeDomain(http)" :icon="Delete" circle />
                            </el-col>
                        </el-row>
                    </el-form-item>
                    <el-form-item>
                        <el-button @click="addDomain">新增回调</el-button>
                    </el-form-item>
                </el-card>
                <el-card class="w-full mt-4">
                    <template #header>http server 配置</template>
                    <el-form-item label="端口号：">
                        <el-input-number v-model="configStore.wechatConfig.http_server_port" :min="1" :max="65535" />
                    </el-form-item>
                </el-card>
                <el-card class="w-full mt-4">
                    <template #header>socket IO server 配置</template>
                    <el-form-item label="地址：">
                        <el-input v-model="configStore.wechatConfig.wsurl" />
                    </el-form-item>
                </el-card>
                <el-card class="w-full mt-4">
                    <template #header>消息过滤配置</template>
                    <el-form-item label="正则白名单过滤：">
                        <el-input v-model="configStore.wechatConfig.msg_filter_regexp" />
                    </el-form-item>
                </el-card>
            </el-form>
        </el-main>
    </el-container>
</template>

<style lang="scss" scoped>
.el-container {
    padding: 0;
    height: calc(100vh - var(--header-height));

    >.el-main {
        padding: 10px;
    }
}

.grid-content {
    border-radius: 4px;
    min-height: 36px;
}
</style>