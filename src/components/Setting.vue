<script lang="ts" setup>
import { ref } from 'vue'
import type { FormInstance } from 'element-plus'
import {
    Delete,
} from '@element-plus/icons-vue'
import { useConfigStore } from "@/store/modules/config";
import { ElMessage } from 'element-plus'

const configStore = useConfigStore();
const formRef = ref<FormInstance>()

const removeDomain = (item: string) => {
    const index = configStore.wechatConfig.cburl.indexOf(item)
    if (index !== -1) {
        configStore.wechatConfig.cburl.splice(index, 1)
    }
}

const addDomain = () => {
    configStore.wechatConfig.cburl.push('')
}

const submitForm = (formEl: FormInstance | undefined) => {
    if (!formEl) return
    formEl.validate(async (valid) => {
        if (valid) {
            const res = await configStore.update();
            if (res) {
                ElMessage({
                    message: '保存成功',
                    type: 'success',
                })
            } else {
                ElMessage.error('保存错误')
            }
        } else {
            ElMessage.error('请检查输入内容')
        }
    })
}
</script>

<template>
    <el-container>
        <el-main>
            <el-card style="w-full">
                <template #header>http 回调地址</template>
                <el-form ref="formRef" :model="configStore.wechatConfig" label-width="auto" class="demo-dynamic w-full">
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
                        <el-button type="primary" @click="submitForm(formRef)">提交</el-button>
                        <el-button @click="addDomain">新增回调</el-button>
                    </el-form-item>
                </el-form>
            </el-card>

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