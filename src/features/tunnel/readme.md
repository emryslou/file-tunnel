# API 数据格式
## 请求
```json
{
    "version": 1, // 类型: Number
    "data": {} // 类型: Object
}
```
## 响应
```json
{
    "version": 1,  // 类型: Number
    "status": 0, // 0 -- 成功；其他失败
    "message": "",
    "data": {},
}
```
# web api
1. websocket data format:
 - text: json
  ```json
  {
    "version": 1,  // 类型: Number
    "command": "", // enum: 
  }
  ```
  |命令|行为|命令参数|方向|样例数据|
  |--|--|--|--|--|
  | ReadConfig {}|读取配置|无| client &rightarrow; tunnel &rightarrow; server| input: `{"command": "read_config", params: {}}`, output: `{"data": {"path": "/want/to/share/path/dir", "modified_at": 1500000}}` |
  | ReadDirItem { dir_path }|获取目录内容| dir_path: 关联的目录| client &rightarrow; tunnel &rightarrow; server | |
  | ReadFileInfo { file_path }| 获取文件信息|file_path: 关联文件| client &rightarrow; tunnel &rightarrow; server ||
  | DownloadFile {file_path, block_idx, block_size }| 下载文件数据快 | file_path: 文件路径+文件名称，block_idx: 文件块id, block_size: 每个文件块大小 | client &rightarrow; tunnel &rightarrow; server ||
  |ModifiedFile { path, type: meta\|content }| 文件被修改| path: 修改的文件或者目录路径, type: 改动类型: meta -> 元数据，content -> 文件内容| client &leftarrow; tunnel &leftarrow; server||
  |DeleteFile { path } | 删除文件或者目录| path: 被删除的文件或者目录|| 
 - binary