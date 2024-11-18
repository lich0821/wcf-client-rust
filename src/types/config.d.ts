export type WechatConfig = {
    cburl: string[];
    wsurl: string;
    http_server_port: number,
    front_msg_show: boolean,
    file_dir: string;
    msg_filter_regexp: string;
}