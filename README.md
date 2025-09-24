# npm_package_check

一个用于检查 pnpm-lock.yaml 文件中包版本的命令行工具，支持单包查询和批量检查模式。  
主要目的是用来检查近期 NPM 包投毒问题。

## ✨ 功能特点

- 🔍 **全面检查**: 支持检查 `importers`、`packages`、`snapshots` 三个节点
- 📦 **单包模式**: 快速查找指定包的版本信息
- 📋 **批量模式**: 支持从文件批量检查多个包
- 🎯 **精确匹配**: 支持版本号精确匹配和模糊匹配
- 📊 **详细报告**: 生成 TSV 格式的检查报告
- 🌐 **多格式支持**: 支持标准包列表和安全报告两种输入格式
- 🎨 **友好输出**: 彩色控制台输出，直观显示检查结果

## 🚀 快速开始

### 安装依赖

确保已安装 Rust 开发环境，然后克隆项目：

```bash
git clone <repository-url>
cd npm_package_check
cargo build --release
```

### 基本用法

```bash
# 查找指定包（任意版本）
cargo run -- react

# 查找指定包的特定版本
cargo run -- react 18.3.1

# 查找带作用域的包
cargo run -- "@ant-design/icons" 4.8.3

# 显示详细信息
cargo run -- react --verbose

# 指定 pnpm-lock.yaml 文件路径
cargo run -- react --file ./path/to/pnpm-lock.yaml
```

## 📋 批量检查模式

### 支持的文件格式

#### 格式一：标准包列表 (version1.txt)
检查列表可由 [Shai-Hulud: Self-Replicating Worm Compromises 500+ NPM Packages](https://www.stepsecurity.io/blog/ctrl-tinycolor-and-40-npm-packages-compromised#affected-packages) 直接复制来  
```
Row	Package Name	Version(s)
1	react	18.3.1
2	@ant-design/icons	4.8.3
3	lodash	4.17.21, 4.17.20
```

#### 格式二：安全报告 (version2.txt)
检查列表可由 [Shai-Hulud, The Most Dangerous NPM Breach In History Affecting CrowdStrike and Hundreds of Popular Packages](https://www.koi.security/incident/shai-hulud-npm-supply-chain-attack-crowdstrike-tinycolor) 直接复制来

```
Package Name	Compromised Version(s)	Detection Date	Status
react-malicious	1.0.0	2025-09-16	Removed from NPM
vulnerable-pkg	2.1.0, 2.1.1	2025-09-16	⚠️ Active
```

### 批量检查命令

```bash
# 批量检查标准格式
cargo run -- --batch version1.txt --output report.tsv

# 批量检查安全报告格式
cargo run -- --batch version2.txt --verbose --output security_report.tsv
```

## 📊 输出格式

### 控制台输出

```
✅ 找到包: react @ 18.3.1
   根目录 @ 18.3.1 (dependencies)
   packages节点 @ 18.3.1 (packages)
   snapshots节点 @ 18.3.1 (snapshots[...].dependencies)

❌ 未找到包: non-existent-package

⚠️ 找到包 'react' 但版本不匹配
   期望版本: 17.0.0
   实际版本:
   - 18.3.1 (根目录)
```

### 批量检查统计

```
🎯 统计信息:
   总数: 195
   ✅ 找到: 150
   🟡 部分匹配: 10
   ⚠️ 版本不匹配: 25
   ❌ 未找到: 10
```

### TSV 报告格式

生成的报告包含以下列：
- Package Name: 包名
- Status: 检查状态
- Expected Versions: 期望版本
- Found Versions: 实际找到的版本
- Locations: 包所在位置
- Original Status: 原始状态（安全报告格式）
- Detection Date: 检测日期（安全报告格式）

## 🔧 命令行参数

```
检查 pnpm-lock.yaml 文件中是否包含指定的包和版本

Usage: npm_package_check [OPTIONS] [PACKAGE] [VERSION]

Arguments:
  [PACKAGE]  要查找的包名（例如：antd 或 @ant-design/icons）
  [VERSION]  版本号（可选，不指定则匹配任意版本）

Options:
  -f, --file <FILE>      pnpm-lock.yaml 文件路径 [default: pnpm-lock.yaml]
  -v, --verbose          显示详细信息
  -b, --batch <BATCH>    批量检查模式：指定包列表文件路径
      --output <OUTPUT>  输出报告文件路径（批量模式）
  -h, --help             Print help
```

## 🎯 使用场景

### 1. 依赖审计
快速检查项目是否使用了特定版本的依赖：
```bash
cargo run -- lodash 4.17.20
```

### 2. 安全检查
批量检查项目中是否包含已知有安全问题的包：
```bash
cargo run -- --batch security-vulnerabilities.txt --output security-audit.tsv
```

### 3. 版本升级验证
验证包升级后的版本是否正确：
```bash
cargo run -- react 18.3.1 --verbose
```

### 4. 依赖分析
分析包在项目中的分布情况：
```bash
cargo run -- @types/react --verbose
```

## 📝 检查逻辑

工具会在以下三个节点中查找包：

1. **importers**: 直接依赖
   - `dependencies`
   - `devDependencies` 
   - `optionalDependencies`

2. **packages**: 包定义信息
   - 所有包的版本定义

3. **snapshots**: 包快照
   - 包的实际安装快照
   - 间接依赖关系

## 🔍 版本匹配规则

- **精确匹配**: `1.0.0` 完全匹配版本号
- **前缀匹配**: `1.0` 匹配 `1.0.x` 系列版本
- **多版本支持**: 支持检查多个版本 `1.0.0, 1.0.1, 1.1.0`

## 📦 项目结构

```
npm_package_check/
├── src/
│   └── main.rs           # 主程序文件
├── Cargo.toml            # Rust 项目配置
├── pnpm-lock.yaml        # 示例 pnpm 锁定文件
├── version1.txt          # 标准包列表示例
├── version2.txt          # 安全报告示例
└── README.md            # 项目说明文档
```

## 🛠️ 技术栈

- **语言**: Rust
- **CLI 解析**: clap
- **YAML 解析**: serde_yaml
- **序列化**: serde
- **错误处理**: anyhow

## 📈 性能特点

- ✅ 快速解析大型 pnpm-lock.yaml 文件
- ✅ 内存高效的批量处理
- ✅ 并行处理能力
- ✅ 智能缓存机制

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

MIT License

## 🔄 更新日志

### v0.1.0
- ✨ 支持单包查询模式
- ✨ 支持批量检查模式
- ✨ 支持两种输入文件格式
- ✨ 完整的三节点检查
- ✨ TSV 报告导出
- ✨ 彩色控制台输出

---

*🤖 该工具由 Claude Code 辅助开发*