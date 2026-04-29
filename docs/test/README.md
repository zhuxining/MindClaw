> **Status**: `active`

# MindClaw 测试说明

## 目标

本目录用于记录项目中的测试策略、运行方式和约定。

当前优先覆盖 `src-tauri/src/providers` 的测试说明，因为 Provider 层同时包含：

- 纯本地可重复执行的单元测试
- 依赖外部模型服务和 API Key 的联网集成测试

## Provider 测试分层

Provider 测试位于 `src-tauri/src/providers/tests/`，当前分为两类：

| 类型     | 文件                                                           | 特点                                          |
| -------- | -------------------------------------------------------------- | --------------------------------------------- |
| 本地测试 | `registry.rs` + `*_config` 测试                                | 不依赖网络，不依赖真实密钥，适合默认执行      |
| 联网测试 | `openai.rs`、`deepseek.rs` 中的 `chat` / `stream` / `reasoner` | 依赖外部 API 和有效密钥，默认标记为 `ignored` |

这样设计的原因是：

- 默认 `cargo test` 应该稳定、快速、可重复
- 联网测试应当只在需要联调真实 Provider 时显式运行
- 测试结果需要能区分“代码没问题”和“外部环境没准备好”

## 默认运行

在 `src-tauri` 目录执行：

```bash
cargo test --package mindclaw --lib providers::tests -- --nocapture
```

预期行为：

- `registry` 和配置类测试会实际执行
- OpenAI / DeepSeek 的联网测试会显示为 `ignored`

这类命令适合：

- 本地开发时快速回归
- CI 中执行稳定测试
- 验证 Provider 注册、模型选择、错误分支没有回归

## 显式运行联网测试

当需要验证真实 Provider API 时，使用 `--ignored`：

```bash
# 跑所有 Provider 联网测试
cargo test --package mindclaw --lib providers::tests -- --ignored --nocapture

# 只跑 DeepSeek chat
cargo test --package mindclaw --lib providers::tests::deepseek::test_deepseek_chat -- --ignored --nocapture

# 只跑 OpenAI stream
cargo test --package mindclaw --lib providers::tests::openai::test_openai_stream -- --ignored --nocapture
```

适用场景：

- 联调新 Provider 接入
- 排查 API Base、模型 ID、流式输出是否正常
- 验证真实 token usage 和 stop reason

## 环境变量约定

Provider 配置当前使用以下环境变量：

| Provider | 环境变量            |
| -------- | ------------------- |
| OpenAI   | `OPENAI_API_KEY`    |
| DeepSeek | `DEEPSEEK_API_KEY`  |
| Claude   | `ANTHROPIC_API_KEY` |

联网测试会优先从当前测试进程读取环境变量。

如果当前 shell / IDE 任务没有传入这些变量，测试辅助逻辑还会尝试加载以下文件：

1. `src-tauri/.env.test.local`
2. `src-tauri/.env.local`
3. `src-tauri/.env.test`
4. `src-tauri/.env`
5. `<repo>/.env.test.local`
6. `<repo>/.env.local`
7. `<repo>/.env.test`
8. `<repo>/.env`

建议优先使用：

- 仓库根目录的 `.env.local`
- 或 `src-tauri/.env.local`

示例：

```env
DEEPSEEK_API_KEY=your_deepseek_key
OPENAI_API_KEY=your_openai_key
ANTHROPIC_API_KEY=your_anthropic_key
```

## 常见问题

### 1. 已经配置了环境变量，测试还是跳过

优先检查以下几点：

1. 你运行的是不是 `--ignored` 版本的命令
2. 变量是否真的传进了 `cargo test` 的进程
3. 如果是通过 IDE/任务面板启动，任务进程是否继承了 shell 环境
4. `.env.local` 是否放在上面约定的搜索路径中

说明：

- 默认命令会把联网测试标记为 `ignored`，这不是失败
- 如果看到 `ignored`，说明测试被正常过滤了
- 如果显式使用 `--ignored` 后仍然提示未设置环境变量，说明测试进程看不到对应 key

### 2. 为什么不把联网测试默认打开

因为这会引入以下不稳定因素：

- 外部服务可用性
- 网络波动
- API 限流
- 账户权限和额度
- 密钥在不同终端、IDE、CI 环境中的传递差异

默认 `ignored` 可以把“代码正确性”与“外部依赖可用性”分开。

### 3. 为什么配置测试和 Registry 测试要单独保留

这些测试提供最基础的行为保障：

- 内置 Provider 是否正确注册
- tier 到模型的解析是否正确
- 默认模型是否正确
- 未知 Provider、缺失环境变量配置时是否返回明确错误

即使没有任何外部 API Key，这些测试也应当始终可运行。

## 当前命令清单

```bash
# 默认跑稳定测试
cd /Users/zhuxining/Code/MindClaw/src-tauri
cargo test --package mindclaw --lib providers::tests -- --nocapture

# 跑全部联网 Provider 测试
cd /Users/zhuxining/Code/MindClaw/src-tauri
cargo test --package mindclaw --lib providers::tests -- --ignored --nocapture

# 跑单个 DeepSeek 联网测试
cd /Users/zhuxining/Code/MindClaw/src-tauri
cargo test --package mindclaw --lib providers::tests::deepseek::test_deepseek_chat -- --ignored --nocapture
```

## 后续建议

后续如果测试规模继续扩大，建议按主题拆分本目录文档：

- `provider-tests.md`
- `frontend-tests.md`
- `storage-tests.md`
- `ci-test-matrix.md`

当前阶段，一份总览文档已经足够支撑日常开发和联调。
