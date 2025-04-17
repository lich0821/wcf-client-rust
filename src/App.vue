<script lang="ts" setup>
import { ElConfigProvider } from 'element-plus';
import { listen } from '@tauri-apps/api/event';
import { onMounted } from 'vue';
import { confirm } from '@tauri-apps/plugin-dialog';
import wcf from './command/wcf';
import { useConfigStore } from '@/store/modules/config';
import { getName, getVersion } from '@tauri-apps/api/app';
import { getCurrentWindow } from '@tauri-apps/api/window';

let configStore = useConfigStore();

const confirmExit = async () => {
    const shouldExit = await confirm("退出将无法使用服务，确定要退出吗？");
    if (shouldExit) {
        await wcf.exit();
    }
}

onMounted(async () => {
    setWindowsTitle();
    await listen('log-message', (msg) => {
        console.log(msg);
    });
    await listen('request-exit', () => {
        confirmExit();
    });
    // 加载配置
    await configStore.read();
})

const setWindowsTitle = async () => {
    const app_name = await getName();
    const app_version = await getVersion();
    await getCurrentWindow().setTitle(app_name + "  V" + app_version);
}
</script>
<template>
    <el-config-provider>
        <el-container h="full">
            <el-header>
                <BaseHeader />
            </el-header>
            <el-main>
                <router-view v-slot="{ Component }">
                    <transition name="scale" mode="out-in" appear>
                        <keep-alive>
                            <component :is="Component" />
                        </keep-alive>
                    </transition>
                </router-view>
            </el-main>
        </el-container>
    </el-config-provider>
</template>


<style lang="scss">
#app {
    >section {
        >header {
            padding: 0;
            height: var(--header-height);
        }

        >main {
            padding: 0;

            >div {
                height: calc(100vh - var(--header-height) - 20px)
            }
        }
    }
}

body {
    width: 100% !important;
}

/* 路由切换动画 */
/* fade-transform */
.fade-leave-active,
.fade-enter-active {
    transition: all 0.5s;
}

/* 可能为enter失效，拆分为 enter-from和enter-to */
.fade-enter-from {
    opacity: 0;
    transform: translateY(-30px);
}

.fade-enter-to {
    opacity: 1;
    transform: translateY(0px);
}

.fade-leave-to {
    opacity: 0;
    transform: translateY(30px);
}

.scale-enter-active,
.scale-leave-active {
    transition: all 0.5s ease;
}

.scale-enter-from,
.scale-leave-to {
    opacity: 0;
    transform: scale(0.9);
}
</style>