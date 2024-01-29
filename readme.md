# 文件隧道 (file tunnel)
## 愿景
不受限制的使用自己的文件

## 模块
### [服务端](./readme/tunnel.md)
### 客户端
#### [发送端](./readme/server.md)
#### [接收端](./readme/client.md)

## 流程图
```mermaid
sequenceDiagram
   发送端 ->> + 发送端: 配置 path, [password，可选]
   发送端 ->> + 发送端: 生成 share_key
   opt password 为空
   发送端 ->> + 发送端: 依据 share_key 生成 password
   end
   发送端 ->> + 服务端: 注册 share_key
   opt 注册成功
   服务端 -->> + 发送端: 创建 websocket 连接
   end
   接收端 ->> + 接收端: 配置 share_key, password
   接收端 ->> + 服务端: 验证 share_key
   opt 注册成功
   服务端 -->> + 接收端: 响应可用目录
   接收端 ->> + 服务端: 请求可用[目录/文件]内容
   服务端 ->> - 发送端: 发送读取指定[目录/文件]内容
   发送端 -->> + 服务端: 响应请求内容
   服务端 -->> - 接收端: 转发数据
   end
```
# 甘特图
```mermaid
gantt
   dateFormat YY-MM-DD
   title 文件隧道
   excludes weekends
   section 服务端(server)
      配置: srv_config, 2024-01-16, 2d
      生成 share_key && 注册(http_req): gen_sk, 2024-01-18, 1d
      websocket 交互: ws_interact, 2024-01-19, 4d
      升级检测(http_req): srv_update, 2024-01-25, 1d
   section 隧道(tunnel)
      注册 share_key(http_srv): reg_sk, 2024-01-26, 1d
      验证 share_key(http_srv): chk_sk, 2024-01-29, 1d
      websocket 服务: ws_srv, 2024-01-30, 2d
      请求文件信息(http_srv): file_data_srv, 2024-02-01, 1d
      升级检测(http_req): srv_update, 2024-02-02, 1d
   section 客户端(client)
      配置: cli_config, 2024-02-03, 1d
      验证 share_key(http_req): chk_sk, 2024-02-04, 1d
      请求文件信息: file_data_srv, 2024-02-05, 2d
   section 公共(common)
      生成密码: gen_password, 2024-02-06, 1d
      数据加密: data_encrpty, 2024-02-07, 1d
      数据加密: data_dencrpty, 2024-02-08, 1d
      http 请求: http_lib, 2024-02-09, 2d
```
[mermaid](https://mermaid.js.org/intro/)