# 问题:
1. 获取信息前需要验证 share_key 是否在线
2. 调用信息有时不匹配
3. 读取到的目录列表和实际目录不匹配(websocket 数据响应延迟导致)
# 新功能
1. server: 局域网内广播自己
2. server: 验证 client_key 是否被授权
3. server: UDP 服务局域网内 client
4. server: win 注册后台服务，用于维护于服务端的连接
5. client: win 注册后台服务，下载用户文件，维护下载器
6. client: 局域网嗅探可用 server
7. client: UDP 直接连接局域网 server
8. client: client_key: alias name 别名
9.  server: gui
10. client: gui
11. server 和 client 打包成可分发软件: windows 和 mac
12. github cli ci: 自动打包