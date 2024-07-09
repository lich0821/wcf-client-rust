import { createRouter, createWebHashHistory } from 'vue-router';
import NProgress from '@/utils/progress';

export const routes = [
    {
        path: '/',
        icon: 'mingcute:contacts-fill',
        hidden: false,
        name: '首页',
        meta: {
            keepAlive: true
        },
        component: () => import('@/components/Home.vue')          
    },
    {
        path: '/setting',
        icon: 'material-symbols:settings',
        hidden: false,
        name: '设置',
        meta: {
            keepAlive: true
        },
        component: () => import('@/components/Setting.vue')          
    },
    {
        path: '/:pathMatch(.*)*',
        hidden: true,
        component: () => import('@/components/404.vue')
    }
]

export const router = createRouter({
    history: createWebHashHistory(),
    routes: routes
});

router.beforeEach(async (from: any, to: any) => {
    NProgress.start();
});

router.afterEach(async (from: any, to: any) => {
    NProgress.done();
});