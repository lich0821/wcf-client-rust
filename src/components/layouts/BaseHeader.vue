<script lang="ts" setup>
import { isDark, toggleDark } from "@/composables";
import { onMounted, ref } from "vue";
import { routes } from '@/router';
import { useWechatStore } from "~/store";
import { ElLoading, ElMessage } from "element-plus";
import { VxeUI } from 'vxe-table'

const wechatStore = useWechatStore();
const activeMenu = ref<string>('/');

const handleMenuItemClick = (path: string) => {
    activeMenu.value = path;
}

const switchTheme = () => {
    VxeUI.setTheme(isDark.value ? 'light' : 'dark');
    toggleDark();
}

const startOrStop = async () => {
    const loading = ElLoading.service({
        text: wechatStore.isServerRunning ? '停止中...' : '启动中...'
    });
    try {
        if (wechatStore.isServerRunning) {
            await wechatStore.stop();
        } else {
            await wechatStore.start();
            // todo 循环检测，一直等到加载成功
            // 因为升级tauri v2 后此提示 window.__TAURI_IPC__ is not a function
            // 暂时停止获取头像，后期修复
            // var getUserInfoTask = window.setInterval(async function logname() {
            //     await wechatStore.updateSlefInfo();
            //     clearInterval(getUserInfoTask)
            // }, 5000);
        }
    } catch (err: any) {
        console.error(err);
        ElMessage.error(err.message || err);
    } finally {
        loading.close();
    }

}

onMounted(async () => {
    const wechatStore = useWechatStore();
    wechatStore.getRunningFlag();
    const path = location.hash.replace('#', '');
    activeMenu.value = path;
})
</script>

<template>
    <div w="full" h="full" class="menu-container">
        <el-menu class="unselect menu" :default-active="activeMenu" :router="true" mode="horizontal" :collapse="false"
            :ellipsis="false">
            <template v-for="item in (routes as any)">
                <el-menu-item v-ripple :index="item.path" :key="item.path" v-if="!item.hidden"
                    @click="handleMenuItemClick(item.path)">
                    <el-space>
                        <!-- <Icon :icon="item.icon"/> -->
                        {{ item.name }}
                    </el-space>
                </el-menu-item>
            </template>
            <div class="flex-grow" />
            <el-menu-item v-ripple h="full" @click="startOrStop()">
                <button class="border-none w-full bg-transparent cursor-pointer"
                    style="height: var(--el-menu-item-height)">
                    <el-text type="success" v-if="!wechatStore.isServerRunning">启动</el-text>
                    <el-text type="danger" v-else>停止</el-text>
                </button>
            </el-menu-item>
            <el-menu-item v-ripple h="full" @click="switchTheme()">
                <button class="border-none w-full bg-transparent cursor-pointer"
                    style="height: var(--el-menu-item-height)">
                    <i inline-flex i="dark:ep-moon ep-sunny" />
                </button>
            </el-menu-item>
            <el-menu-item v-if="!!wechatStore.selfInfo.small_head_url">
                <el-avatar shape="square" :size="36" :src="wechatStore.selfInfo.small_head_url" />
            </el-menu-item>
        </el-menu>
    </div>
</template>

<style>
.menu-container {
    width: 100%;
    height: 100%;

    .menu {
        width: 100%;
    }
}
</style>