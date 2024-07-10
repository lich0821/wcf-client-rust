import { http } from '@/utils/http';

/** 查询登录状态 */
const isLogin = async () => { 
    return http.get('/islogin');
}

/** 获取登录账号信息 */
const userinfo = async () => { 
    return http.get('/userinfo');
}

/** 获取所有可查询的数据库 */
const dbs = async () => { 
    return http.get('/dbs');
}

/** 查询数据库下的表信息 */
const tables = async (db) => { 
    return http.get(`/${db}/tables`);
}

/** 执行SQL */
const sql = async (db, sql) => { 
    return http.post('/sql', {
        data: {
            db: db,
            sql: sql
        }
    });
}

export default {
    isLogin,
    userinfo,
    dbs,
    tables,
    sql
}