# crabsync

rsync 文件同步终端 UI。通过直观的分栏界面浏览、选择并同步本地与远程目录之间的文件。

## 功能特性

- **分栏文件树** — 左右分栏浏览源目录（本地）和目标目录（远程）
- **复选框选择** — 勾选单个文件或整个目录进行同步
- **上传与下载** — 推送文件到远程（`s`）或从远程拉取文件（`d`）
- **Dry-run 预览** — 执行前预览 rsync 将要进行的操作
- **SSH 密码支持** — 未配置 SSH 密钥时通过 PTY 交互式输入密码
- **密码错误重试** — 密码错误时自动重新提示，不会卡死
- **CLI 管理** — 从命令行添加、删除和查看同步配对
- **两级配置** — 局部（`./crabsync.toml`）和全局（`~/.config/crabsync/crabsync.toml`）配置文件；同名配对局部优先

## 系统要求

- **rsync** 必须在 `$PATH` 中
- **SSH** 用于远程目标（密钥认证或密码认证）
- Linux 或 macOS（交互式密码输入需要 PTY 支持）

## 安装

### 从源码构建

```bash
cargo install --git https://github.com/SHA-4096/crabsync.git --locked
```

可执行文件会安装到 `~/.cargo/bin/crabsync`（请确保 `~/.cargo/bin` 在 `$PATH` 中）。

### 从 Release 下载

从 [GitHub Releases](https://github.com/SHA-4096/crabsync/releases) 下载预编译包：

| 平台 | 文件 |
|------|------|
| Linux x86_64 | `crabsync-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `crabsync-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 | `crabsync-x86_64-apple-darwin.tar.gz` |
| macOS ARM64 | `crabsync-aarch64-apple-darwin.tar.gz` |

```bash
tar xzf crabsync-*.tar.gz
chmod +x crabsync
sudo mv crabsync /usr/local/bin/
```

## 使用方法

### 管理配对

"配对"（pair）将一个本地目录链接到一个远程目标。配对存储在两个配置文件中：

- **局部配置**：`./crabsync.toml`（当前目录，项目专用）
- **全局配置**：`~/.config/crabsync/crabsync.toml`（所有项目共享）

局部配对优先——如果两个文件中存在同名配对，全局配对会被遮蔽（shadowed）。

```bash
# 添加配对（默认存入局部配置）
crabsync add myproject ./data user@server:/backup/data

# 添加配对到全局配置
crabsync add myproject ./data user@server:/backup/data --global

# 查看已配置的配对（显示范围：local/global）
crabsync list

# 删除配对（先从局部删除，找不到再从全局删除）
crabsync remove myproject

# 仅从全局配置删除
crabsync remove myproject --global
```

### 交互式 TUI

```bash
# 启动 TUI，进入配对列表
crabsync

# 直接进入指定配对的文件树
crabsync sync myproject
```

### 快捷键

#### 配对列表

| 按键 | 操作 |
|------|------|
| `j` / `↓` | 下移 |
| `k` / `↑` | 上移 |
| `Enter` | 进入文件树 |
| `a` | 添加配对 |
| `d` | 删除配对 |

#### 文件树

| 按键 | 操作 |
|------|------|
| `j` / `↓` | 下移（当前面板） |
| `k` / `↑` | 上移（当前面板） |
| `Space` | 切换文件/目录选中状态 |
| `Enter` | 展开/折叠目录 |
| `a` | 全选/取消全选 |
| `s` | Dry-run 上传（源面板） |
| `d` | Dry-run 下载（目标面板） |
| `Tab` | 切换源/目标面板 |
| `r` | 重新加载远程目录树 |
| `p` | 输入 SSH 密码（需要认证时） |
| `?` | 显示帮助 |
| `Esc` / `q` | 返回 |

#### 同步预览

| 按键 | 操作 |
|------|------|
| `y` | 确认并执行同步 |
| `n` / `Esc` | 返回 |

#### 密码输入

| 按键 | 操作 |
|------|------|
| `Enter` | 提交密码 |
| `Esc` | 取消 |

#### 添加配对

| 按键 | 操作 |
|------|------|
| `Tab` / `Shift+Tab` | 下一个 / 上一个字段 |
| `Space` | 切换范围（局部 / 全局） |
| `Enter` | 保存配对 |
| `Esc` | 取消 |

## 工作流程

1. 从配对列表中**选择一个配对**，按 Enter 进入
2. 在源面板（左）或目标面板（右）中**浏览并选择**文件
3. **按 `s`**（上传）或 **`d`**（下载）预览将要同步的内容
4. **按 `y`** 确认并执行 rsync 操作
5. 如果需要 SSH 认证，会自动弹出密码输入框

上传将源端选中的文件同步到目标端，下载将目标端选中的文件同步到源端。底层 rsync 命令会相应地交换源和目标参数。

## 配置

配对存储在两个 TOML 文件中，格式相同：

**局部配置**（`./crabsync.toml`）：
```toml
[[pair]]
name = "myproject"
local = "./data"
remote = "user@server:/backup/data"
```

**全局配置**（`~/.config/crabsync/crabsync.toml`）：
```toml
[[pair]]
name = "photos"
local = "/home/user/photos"
remote = "nas:/backup/photos"
```

当两个文件中存在同名配对时，局部配对优先，全局配对在 TUI 中会显示为 "shadowed"（遮蔽）。全局配置目录和文件仅在首次添加全局配对时创建。

## 构建

```bash
cargo build          # 调试构建
cargo build --release # 发布构建
cargo test            # 运行测试
```

## 许可证

MIT
