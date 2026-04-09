---
paths:
- "**/*.rs"
---

# Rust 代码规范

使用 **rustfmt**（格式化）和 **clippy**（静态分析）作为代码质量工具链。

## 快速参考

- **格式化代码**: `cargo fmt`
- **检查并修复**: `cargo clippy --fix`
- **编译检查**: `cargo check`

---

## 核心原则

编写**安全、零成本抽象、表达力强**的 Rust 代码。利用编译器保障正确性，而非依赖运行时检查。

### 类型系统

- 善用 newtype 模式封装语义（`struct UserId(u64)` 而非裸 `u64`）
- 使用枚举建模状态机和互斥变体，避免用布尔标志组合表达状态
- 优先使用 `impl Trait` 作为参数和返回类型，仅在需要动态分发时使用 `dyn Trait`
- 利用类型状态模式（typestate pattern）在编译期保证 API 使用顺序
- 为公开 API 的类型实现常用 trait：`Debug`、`Clone`、`PartialEq`，按需 `Display`、`Hash`

```rust
// Good — 枚举建模互斥状态
enum ConnectionState {
    Disconnected,
    Connecting { attempt: u32 },
    Connected(TcpStream),
}

// Bad — 布尔标志组合
struct Connection {
    is_connected: bool,
    is_connecting: bool,
    stream: Option<TcpStream>,
}
```

### 所有权与借用

- 函数参数优先使用借用（`&T`/`&mut T`），仅在需要所有权时按值传递
- 使用 `&str` 而非 `&String`，使用 `&[T]` 而非 `&Vec<T>`，使用 `&Path` 而非 `&PathBuf`
- 避免不必要的 `.clone()`，先考虑能否通过借用或重构生命周期解决
- 显式标注生命周期仅在编译器无法推导时使用，不要为标注而标注
- 使用 `Cow<'_, str>` 在"可能借用也可能拥有"的场景中避免不必要的分配

```rust
// Good — 接受借用，调用者决定是否转移所有权
fn process(data: &[u8]) -> Result<Output> { ... }

// Bad — 强制调用者交出所有权
fn process(data: Vec<u8>) -> Result<Output> { ... }
```

### 惯用语法

- 使用 `match` 和 `if let` 进行模式匹配，用 `let-else` 处理提前返回
- 优先使用迭代器链（`.iter().filter().map().collect()`）而非手动循环
- 使用 `?` 运算符传播错误，而非手动 `match` 拆包
- 字符串格式化使用 `format!` 宏，而非手动拼接
- 使用 `Default::default()` 初始化结构体默认值
- 使用解构绑定提取结构体和元组字段

```rust
// Good — let-else 提前返回
let Some(config) = load_config() else {
    return Err(anyhow!("missing config"));
};

// Good — 迭代器链
let active_users: Vec<_> = users
    .iter()
    .filter(|u| u.is_active)
    .map(|u| &u.name)
    .collect();

// Bad — 手动循环
let mut active_users = Vec::new();
for u in &users {
    if u.is_active {
        active_users.push(&u.name);
    }
}
```

### 错误处理

- 库代码定义自己的错误类型（使用 `thiserror`），应用代码使用 `anyhow`
- 用 `?` 运算符传播错误，保持函数签名清晰
- 避免在库代码中使用 `.unwrap()` 和 `.expect()`，仅在确实不可能失败且有注释说明时使用
- 用 `Option` 表达"可能没有"，用 `Result` 表达"可能失败"，不要混用
- 为错误类型实现 `std::error::Error`，保留错误链（`#[source]`/`#[from]`）

```rust
// Good — thiserror 定义库错误
#[derive(Debug, thiserror::Error)]
enum ParseError {
    #[error("invalid header: {0}")]
    InvalidHeader(String),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

// Bad — 字符串作为错误
fn parse(input: &str) -> Result<Data, String> { ... }
```

### 并发安全

- 利用 `Send`/`Sync` trait 约束保证线程安全，编译器会替你检查
- 优先使用消息传递（`mpsc`/`crossbeam` channel）而非共享状态
- 必须共享状态时，使用 `Arc<Mutex<T>>` 或 `Arc<RwLock<T>>`，保持锁粒度最小
- 避免在持有锁的同时执行 I/O 或获取其他锁（防止死锁）
- 考虑使用无锁数据结构（`dashmap`、`atomic` 类型）减少竞争

### 异步代码

- 使用 `async/await`，选择成熟的运行时（`tokio` 为主流选择）
- 不要在 async 函数中执行阻塞操作，使用 `tokio::task::spawn_blocking` 卸载
- 使用 `tokio::select!` 处理多个异步分支，注意 cancellation safety
- 异步 trait 方法使用 `async fn in trait`（Rust 1.75+），或在需要 `dyn` 时使用 `async-trait`
- 注意 future 的 `Send` 约束——跨 `.await` 持有非 `Send` 类型会导致编译错误

```rust
// Good — 卸载阻塞操作
let result = tokio::task::spawn_blocking(|| {
    heavy_computation()
}).await?;

// Bad — 在 async 上下文中阻塞
async fn handle_request() {
    let result = heavy_computation(); // 阻塞了整个线程
}
```

### Unsafe 代码

- 最小化 `unsafe` 块的范围，仅包裹真正需要的语句
- 每个 `unsafe` 块必须有 `// SAFETY:` 注释，说明为何此操作是安全的
- 将 `unsafe` 操作封装为安全的公开 API，在边界处验证不变量
- 绝不使用 `mem::transmute` 进行类型转换，优先使用 `from_ne_bytes` 等安全替代
- 裸指针操作必须确保对齐、有效性和生命周期

```rust
// Good — 最小化 unsafe 范围并文档化
// SAFETY: `ptr` 由 `alloc` 分配，大小和对齐满足 Layout 要求
unsafe {
    std::ptr::write(ptr, value);
}
```

### 性能

- 避免不必要的堆分配：优先使用栈上数据和借用
- 使用 `&str` 接收字符串参数，仅在需要所有权时使用 `String`
- 大结构体传递使用引用或 `Box`，避免栈上拷贝
- 使用 `Vec::with_capacity` 预分配已知大小的集合
- 优先使用迭代器的惰性求值，避免中间集合分配
- 热路径中避免 `format!` 和频繁的小分配，考虑使用 `write!` 写入已有缓冲区

### 安全

- 不要用 `unsafe` 绕过借用检查器——如果编译器拒绝了你的代码，先重新审视设计
- 对用户输入进行验证和边界检查，不信任外部数据
- 使用 `secrecy` crate 处理敏感数据，防止意外日志泄露
- 序列化/反序列化（`serde`）时注意拒绝服务风险：限制输入大小、嵌套深度
- 使用 `cargo audit` 检查依赖中的已知漏洞

---

## 测试

- 单元测试放在同文件的 `#[cfg(test)] mod tests` 中
- 集成测试放在 `tests/` 目录
- 每个测试验证一个行为，测试函数名描述被测场景
- 使用 `#[should_panic(expected = "...")]` 测试预期 panic
- 使用 `proptest` 或 `quickcheck` 进行属性测试
- 不要在提交的代码中保留 `#[ignore]`

---

## clippy 无法覆盖的关注点

clippy 会自动捕获大多数惯用写法问题，手动关注：

1. **业务逻辑正确性** — 工具无法验证算法是否符合需求
2. **有意义的命名** — 变量、函数、类型名应清晰表达意图
3. **架构决策** — 模块划分、trait 设计、crate 边界
4. **边界条件** — 整数溢出、空集合、并发竞争、资源耗尽
5. **API 设计** — 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html/)，接口对误用具有抵抗力

---

提交前运行 `cargo fmt && cargo clippy -- -D warnings && cargo test` 确保合规。
