# layer-tool

一个用于导出、导入和检查Docker容器层的命令行工具。

## 功能特性

- **导出**: 将Docker容器的读写层、元数据和Docker信息导出到文件
- **导入**: 将导出的文件导入到现有容器的读写层
- **检查**: 验证导出文件的完整性和兼容性

## 安装

### 从源码构建：

```bash
cargo build --release
```

编译后的二进制文件位于 `target/release/layer-tool`。

### 静态二进制构建：

```bash
cargo build --release --target=x86_64-unknown-linux-musl
```

编译后的二进制文件位于 `target/x86_64-unknown-linux-musl/release/layer-tool`。

## 使用方法

### 导出容器层

将容器的读写层和元数据导出到文件：

```bash
layer-tool export <容器ID> <输出文件> [--compress]
```

**示例：**
```bash
# 导出容器到未压缩文件
layer-tool export my-container container-export.tar

# 导出容器到压缩文件
layer-tool export my-container container-export.tar.gz --compress
```

### 导入容器层

从导出文件将层数据导入到现有容器：

```bash
layer-tool import <输入文件> <容器ID> [--no-backup]
```

**选项：**
- `--no-backup`: 导入前跳过备份现有层（警告：这将永久删除现有层数据）

**示例：**
```bash
# 从未压缩文件导入（带备份）
layer-tool import container-export.tar target-container

# 从压缩文件导入（自动检测）
layer-tool import container-export.tar.gz target-container

# 导入时不备份现有层
layer-tool import container-export.tar target-container --no-backup
```

### 检查导出文件

验证导出文件的完整性和兼容性：

```bash
layer-tool check <输入文件> [选项]
```

**选项：**
- `--skip-image`: 跳过镜像SHA256验证
- `--skip-storage`: 跳过存储驱动兼容性检查
- `--skip-os`: 跳过操作系统兼容性检查
- `--skip-arch`: 跳过架构兼容性检查

**示例：**
```bash
# 完整检查
layer-tool check container-export.tar

# 跳过某些兼容性检查
layer-tool check container-export.tar --skip-os --skip-arch
```

## 导出文件格式

导出文件包含：
- 容器元数据（JSON格式）
- Docker守护进程信息（JSON格式）
- 容器的上层目录（tar归档）
- 可选的gzip压缩

## 系统要求

- Docker守护进程必须运行且可访问
- 需要足够的权限访问Docker和容器层目录
- 导入操作需要目标容器已存在

## 安全注意事项

- 该工具需要访问Docker守护进程和容器层目录
- 导出文件可能包含容器文件系统中的敏感数据
- 在导入到生产容器之前，请务必验证导出文件
- 为导出文件使用适当的文件权限

## 错误处理

该工具为常见问题提供详细的错误消息：
- 容器未找到
- 权限被拒绝
- 无效的导出文件格式
- 校验和不匹配
- 兼容性问题

## 限制

- 目前支持overlay2存储驱动
- 需要Docker CLI可用
- 不处理正在运行的容器（导出/导入前请停止容器）
- 仅限于Linux系统

## 工作原理

### 导出过程
1. 获取容器元数据和Docker守护进程信息
2. 定位容器的读写层目录（upper目录）
3. 创建层数据的tar归档
4. 计算校验和以确保完整性
5. 将元数据、Docker信息和层数据打包
6. 可选择性地压缩最终文件

### 导入过程
1. 读取并验证导出文件
2. 提取元数据和Docker信息
3. 如需要则解压缩
4. 备份目标容器的现有层（如果存在且未指定--no-backup）
5. 将层数据提取到目标容器的upper目录
6. 验证导入数据的校验和

### 检查过程
1. 验证文件结构和格式
2. 检查元数据完整性
3. 验证层归档的可读性
4. 与当前Docker环境进行兼容性检查
5. 生成详细的验证报告

## 使用场景

- **容器备份**: 备份容器的读写层以便后续恢复
- **容器迁移**: 在不同环境间迁移容器状态
- **开发环境**: 在开发团队间共享容器状态
- **测试**: 创建一致的测试环境快照
- **灾难恢复**: 快速恢复容器到已知状态

## 故障排除

### 常见问题

**权限错误**
```bash
# 确保用户在docker组中
sudo usermod -aG docker $USER
# 或使用sudo运行
sudo layer-tool export container-name backup.tar
```

**容器未找到**
```bash
# 检查容器是否存在
docker ps -a
# 使用完整的容器ID或正确的名称
```

**存储驱动不兼容**
```bash
# 检查Docker存储驱动
docker info | grep "Storage Driver"
# 使用--skip-storage跳过检查（如果安全）
layer-tool check backup.tar --skip-storage
```

## 贡献

1. Fork 仓库
2. 创建功能分支
3. 进行更改
4. 如适用，添加测试
5. 提交拉取请求

## 许可证

本项目采用MIT许可证。
