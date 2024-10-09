import Axios, {
    type AxiosInstance,
    type AxiosRequestConfig,
    type CustomParamsSerializer,
    type AxiosResponse,
    AxiosError
} from "axios";
import { stringify } from "qs";
import NProgress from "../progress";
import { useWechatStore } from "~/store";
import { RequestMethods } from "./types";
import { ElMessage } from "element-plus";
import axiosTauriApiAdapter from 'axios-tauri-api-adapter';
import wcf from "~/command/wcf";

// 相关配置请参考：www.axios-js.com/zh-cn/docs/#axios-request-config-1
const defaultConfig: AxiosRequestConfig = {
    baseURL: 'http://' + (await (async () => { 
        return await wcf.ip();
    })()),
    adapter: axiosTauriApiAdapter,
    // 请求超时时间
    timeout: 10000,
    // headers: {
    //     Accept: "application/json, text/plain, */*",
    //     "Content-Type": "application/json;charset=UTF-8",
    //     "X-Requested-With": "XMLHttpRequest"
    // },
    // 数组格式参数序列化（https://github.com/axios/axios/issues/5142）
    paramsSerializer: {
        serialize: stringify as unknown as CustomParamsSerializer
    }
    // paramsSerializer: params => { 
    //     return qs.stringify(params, { indices: false });
    // }
};

class Request { 
    // axios 实例
    instance: AxiosInstance
    // 基础配置，url和超时时间
    baseConfig: AxiosRequestConfig = defaultConfig

    constructor(config?: AxiosRequestConfig) {
        // 使用axios.create创建axios实例，配置为基础配置和我们传递进来的配置
        this.instance = Axios.create(Object.assign(this.baseConfig, config))
        this.addRequestInterceptors();
        this.addResponseInterceptors();
    }

    // 通用请求
    public async request<T>(method: RequestMethods, url: string, param?: AxiosRequestConfig, axiosConfig?: AxiosRequestConfig): Promise<T> { 
        const config = {
            method,
            url,
            ...param,
            ...axiosConfig
        } as AxiosRequestConfig;
        return await this.instance.request(config);
    }

    /** 单独抽离的`post`工具函数 */
    public async post<T, P>(url: string, params?: AxiosRequestConfig<P>, config?: AxiosRequestConfig): Promise<T> {
        return await this.request<T>("post", url, params, config);
    }

    /** 单独抽离的`get`工具函数 */
    public async get<T, P>(url: string, params?: AxiosRequestConfig<P>, config?: AxiosRequestConfig): Promise<T> {
        return await this.request<T>("get", url, params, config);
    }

    // 添加请求拦截器
    private addRequestInterceptors(): void { 
        this.instance.interceptors.request.use(
            async (config: AxiosRequestConfig): Promise<any> => {
                // 开启进度条动画
                NProgress.start();
                const wechatStore = useWechatStore();
                if (wechatStore.isServerRunning) {
                    return config;
                } else { 
                    throw Error('请先启动HTTP服务');
                    // return config;
                }
            },
            (err: any) => {
                // 请求错误，这里可以用全局提示框进行提示
                return Promise.reject(err)
            },
        )
    }

    // 添加响应拦截器
    private addResponseInterceptors(): void { 
        this.instance.interceptors.response.use(
            (res: AxiosResponse) => {
                // 关闭进度条动画
                NProgress.done();
                // 直接返回res，当然你也可以只返回res.data
                // 系统如果有自定义code也可以在这里处理
                let data = res.data;
                if (data.status != 0) {
                    ElMessage({
                        showClose: true,
                        message: `${data.error}`,
                        type: "error",
                    });
                    return null;
                } else { 
                    return data.data;
                }
            },
            (err: any) => {
                console.log(err);
                // 这里用来处理http常见错误，进行全局提示
                let message = "";
                if (err && err instanceof AxiosError && err.response) {
                    switch (err.response.status) {
                        case 400:
                            message = "请求错误(400)";
                            break;
                        case 401:
                            message = "未授权，请重新登录(401)";
                            // 这里可以做清空storage并跳转到登录页的操作
                            break;
                        case 403:
                            message = "拒绝访问(403)";
                            break;
                        case 404:
                            message = "请求出错(404)";
                            break;
                        case 408:
                            message = "请求超时(408)";
                            break;
                        case 500:
                            message = "服务器错误(500)";
                            break;
                        case 501:
                            message = "服务未实现(501)";
                            break;
                        case 502:
                            message = "网络错误(502)";
                            break;
                        case 503:
                            message = "服务不可用(503)";
                            break;
                        case 504:
                            message = "网络超时(504)";
                            break;
                        case 505:
                            message = "HTTP版本不受支持(505)";
                            break;
                        default:
                            message = `连接出错(${err.response.status})!`;
                    }
                } else { 
                    message = err || err.message;
                }
                NProgress.done();
                // 这里错误消息可以使用全局弹框展示出来
                // 比如element plus 可以使用 ElMessage
                ElMessage({
                  showClose: true,
                  message: `${message}`,
                  type: "error",
                });
                // 这里是AxiosError类型，所以一般我们只reject我们需要的响应即可
                return Promise.reject(err.response)
            },
        )
    }
}

export const http = new Request();