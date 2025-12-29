
# MyFS - 模拟文件系统 (Rust)

这是一个基于 Rust 标准库实现的模拟文件系统。系统建立了一个名为 `disk.img` 的虚拟磁盘文件，在其上模拟类 Unix 系统中的超级块、位图、索引节点（Inode）及目录结构等文件系统特性。

安装 Rust 后在项目根目录下可直接运行：

```bash
cargo run
```

启动后是一个模拟的命令行界面，如果是初次启动，务必首先执行格式化操作。

```bash
MyFS CLI started.
myfs:/> format
Formatting disk...
Done.
myfs:/> 
```

## 命令列表

| 命令 | 用法 | 行为 |
| :--- | :--- | :--- |
| **format** | `format` | 初始化磁盘镜像，建立根目录及基础元数据 |
| **ls** | `ls [path]` | 列出指定目录下的文件和子目录（默认为当前目录） |
| **cd** | `cd <path>` | 切换当前工作目录 |
| **mkdir** | `mkdir <path>` | 在指定路径创建一个新目录 |
| **touch** | `touch <path>` | 在指定路径创建一个空文件 |
| **write** | `write <path> <content>` | 向指定文件写入文本内容 |
| **cat** | `cat <path>` | 读取并打印指定文件的文本内容 |
| **rm** | `rm <path>` | 删除指定的文件或空目录，并回收磁盘空间 |
| **cp** | `cp <src> <dst>` | 将源文件内容复制到目标路径 |
| **mv** | `mv <src> <dst>` | 移动或重命名文件/目录 |
| **exit** | `exit` | 退出 |
