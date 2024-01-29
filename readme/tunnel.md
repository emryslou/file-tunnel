# 模块
数据隧道

# 运行
部署在服务端

# 说明
主要实现数据中继功能

# 功能
1. 维护连接: 服务端 <=>客户端:发送端 by 共享key
2. 转发数据
3. webapi:
   1. 注册 share_key for  __server__
   2. 验证 share_key for __client__
   3. websocket: cmd for __server__
      1. 读取文件信息(meta data)
      2. 读取文件数据(content data)
   4. 请求文件信息 for __client__
   5. 升级检测 for __client__, __server__, __tunnel__ 