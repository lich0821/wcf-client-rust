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
        path: '/sql',
        hidden: false,
        name: '数据库',
        meta: {
            keepAlive: true
        },
        component: () => import('@/components/Sql.vue')          
    },
    {
        path: '/tools',
        hidden: false,
        name: '工具',
        meta: {
            keepAlive: true
        },
        component: () => import('@/components/Tools.vue')          
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

router.beforeEach(async (_from: any, _to: any) => {
    NProgress.start();
});

router.afterEach(async (_from: any, _to: any) => {
    NProgress.done();
});