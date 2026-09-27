# ApiTest

本地桌面端 API 调试工具。  
技术栈：**Vue 2（vue-element-admin）+ Tauri 2 + SQLite**。无登录、无云同步，数据全部保存在本机。

详细设计见 `[docs/开发方案.md](./docs/开发方案.md)`。

---

## 功能概览


| 模块          | 说明                                                                    |
| ----------- | --------------------------------------------------------------------- |
| 请求调试        | Method / URL / Params / Headers / Body（json、raw、urlencoded、form-data；**支持文件字段**） |
| 发送与响应       | Rust `reqwest` 出站，无浏览器 CORS；查看状态码、耗时、Headers、Body（JSON 可折叠）           |
| 集合管理        | 集合树、新建 / 编辑 / 删除；支持**集合全局 URL 前缀**                                    |
| 环境变量        | 多环境切换，URL / Header / Body 中使用 `{{var}}` 替换                            |
| 工作区         | 多工作区隔离集合与环境                                                           |
| 历史          | 发送历史列表，点击回放请求与响应                                                      |
| 导入导出        | ApiTest JSON；Swagger 2 / OpenAPI 3（JSON 或 YAML）                       |
| Pre / Tests | 前置脚本、断言脚本与 Test Results                                               |
| Mock        | 按请求启用本地假响应（状态码 / Headers / Body / 延迟）                                 |
| JMeter      | 集合导出为 `.jmx`（可配置线程数、循环、Ramp-up）                                       |
| 内置压测        | 对当前请求多线程压测，输出吞吐、平均/P95、状态码分布等                                         |
| 主题          | 亮色 / 暗色                                                               |


---



## 环境要求

- **Node.js** 16+（建议 16 / 18 LTS）
- **Rust** stable（`[rustup](https://rustup.rs/)`）
- **Windows**：已安装 WebView2（Win10/11 一般自带）
- 可选：国内网络建议 npm 使用 `registry.npmmirror.com`；Rust crate 已在 `src-tauri/.cargo/config.toml` 配置 rsproxy

---



## 快速开始

```bash
# 1. 安装根依赖（含 @tauri-apps/cli）
npm install

# 2. 安装前端依赖
npm install --prefix vue-element-admin

# 3. 启动桌面应用（前端 dev server + Tauri）
npm run tauri:dev
```

仅预览前端（浏览器无法调用 Tauri / SQLite / 真实发送）：

```bash
npm run dev:web
```

开发时前端默认地址：`http://localhost:9527`。

---



## 常用脚本


| 命令                    | 说明                            |
| --------------------- | ----------------------------- |
| `npm run tauri:dev`   | 开发模式启动桌面应用                    |
| `npm run tauri:build` | 生产构建 + 打包（Windows NSIS 安装包）   |
| `npm run build:web`   | 仅构建前端 `dist`                  |
| `npm run dev:web`     | 仅启动前端开发服务                     |
| `npm test`            | 跑 Rust 测试 + 前端脚本单元测试          |
| `npm run test:rust`   | 仅 Rust（`cargo test`）          |
| `npm run test:unit`   | 仅前端 `scriptRunner` 相关 Jest 测试 |


---



## 目录结构

```
ApiTest/
├── docs/                      # 开发方案等文档
│   └── 开发方案.md
├── release/                   # 发布产物（安装包 / 便携版 exe）
├── scripts/                   # 辅助脚本（如图标生成）
├── vue-element-admin/         # 前端（Vue 2 + Element UI）
│   └── src/views/api-test/    # 工作台主界面
├── src-tauri/                 # Tauri + Rust + SQLite
│   ├── icons/                 # 应用图标
│   ├── src/
│   │   ├── commands/          # 前端 invoke 的命令
│   │   ├── db/                # SQLite 迁移与访问
│   │   ├── http/              # HTTP 发送与 URL 拼接
│   │   ├── jmeter.rs          # JMeter .jmx 导出
│   │   └── loadtest.rs        # 内置压测
│   └── tauri.conf.json
├── package.json               # 根脚本与 Tauri CLI
└── README.md
```

架构要点：外部 HTTP **一律走 Rust**；前端只负责 UI 与 `invoke`，不直接 `fetch` 业务接口。

---



## 使用说明（简要）



### 集合 URL 前缀

编辑集合可设置「全局 URL 前缀」（如 `https://api.example.com/v1`）。  
请求里写相对路径 `/users`，发送时自动拼接；若 URL 以 `http(s)://` 或 `{{` 开头则不拼接。

### 环境变量

在环境变量中配置 `baseUrl`、`token` 等，请求中使用 `{{baseUrl}}/path`。  
输入框会展示替换后的实际地址。

### Mock

打开请求的 **Mock** 页签 → 启用 Mock → 填写状态码 / Body 等 → Send。  
响应带 `MOCK` 标记，不访问真实网络。

### 导出 JMeter

选中集合 → 顶部 **JMeter**（或菜单「文件 → 导出 JMeter…」）→ 设置线程数 / 循环 / Ramp-up → 下载 `.jmx`，可用 Apache JMeter 打开。

### 内置压测

打开请求 → **压测** → 配置线程数、每线程循环、Ramp-up → 开始。  
限制：线程 ≤ 100，总量 threads × loops ≤ 10000。压测不写入历史。

### 快捷键


| 快捷键            | 作用   |
| -------------- | ---- |
| `Ctrl + Enter` | 发送请求 |
| `Ctrl + S`     | 保存请求 |
| `Ctrl + N`     | 新建请求 |
| `Ctrl + ,`     | 打开设置 |


---



## 打包与发布

```bash
npm run tauri:build
```

成功后产物通常在 cargo target 目录下：

- 可执行文件：`…/release/apitest.exe`
- NSIS 安装包：`…/release/bundle/nsis/ApiTest_0.1.0_x64-setup.exe`

仓库内整理后的发布目录：

```
release/
├── ApiTest_0.1.0_x64-setup.exe   # 安装包（推荐）
├── ApiTest.exe                   # 便携版
└── README.md
```

重新生成应用图标（在已有源图时）：

```bash
npx tauri icon src-tauri/app-icon-source.png
```

---



## 数据存储


| 项          | 路径                                     |
| ---------- | -------------------------------------- |
| SQLite 数据库 | `%APPDATA%\com.apitest.app\apitest.db` |


卸载安装包不会自动删除该数据库。需要重置时，退出应用后删除上述文件即可（下次启动会重新迁移 / 种子数据）。

---



## 测试

```bash
npm test
```

覆盖示例：

- SQLite 迁移、集合 / 请求 CRUD、环境变量替换、工作区隔离  
- Mock 字段、导入导出形态、OpenAPI 解析  
- HTTP（httpbin）、JMeter XML、压测参数校验  
- 前端 Pre / Tests 脚本沙箱（Jest）

---



## 相关文档

- [开发方案](./docs/开发方案.md) — 产品目标、架构、数据模型、分期与验证清单  
- [发布说明](./release/README.md) — 安装包与便携版用法

---



## 版本

当前版本：**0.1.0**