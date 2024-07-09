import { http } from '@/utils/http';

/** 查询登录状态 */
const isLogin = async () => { 
    return http.get('/islogin');
}

/** 获取登录账号信息 */
const userinfo = async () => { 
    return http.get('/userinfo');
}

export default {
    isLogin,
    userinfo
}