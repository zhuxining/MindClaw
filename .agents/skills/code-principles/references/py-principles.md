---
paths: 
- "**/*.py"
---

# Python 代码规范

使用 **ruff**（格式化与静态分析）和 **ty**（类型检查）作为代码质量工具链。

## 快速参考

- **格式化代码**: `ruff format .`
- **检查并自动修复**: `ruff check --fix .`
- **类型检查**: `ty check`

---

## 不确定时参考

### 权威来源

| 主题 | 官方参考 |
|------|---------|
| 类型系统 | [typing.readthedocs.io](https://typing.readthedocs.io/) |
| ty类型规则 | [docs.astral.sh](https://docs.astral.sh/ty/reference/rules/) |
| 语言特性 | [docs.python.org](https://docs.python.org/3/) |
| 标准库 | [docs.python.org/3/library](https://docs.python.org/3/library/) |
| 版本变更 | [Python Release Notes](https://www.python.org/downloads/) |

### 关键提示

- 从 `pyproject.toml` 确认 Python 版本
- 现有代码与文档冲突时，向用户确认后执行

---

## 核心原则

编写**类型安全、可读性强、可维护**的 Python 代码。以显式意图优先，避免不必要的技巧。

### 类型注解

- 所有函数的参数和返回值必须有类型注解
- 使用 `X | Y` 代替 `Union[X, Y]`，使用 `X | None` 代替 `Optional[X]`
- 使用 `list[T]`、`dict[K, V]`、`tuple[T, ...]` 而非 `List`、`Dict`、`Tuple`
- 避免使用 `Any`；如类型确实未知，优先使用 `object` 或 `Unknown`
- 对于复杂类型，使用 `TypeAlias` 或 `type` 语句定义别名（Python 3.12+）
- Python 3.14 中注解默认懒求值，无需 `from __future__ import annotations`

```python
# Good
def process(items: list[str], limit: int | None = None) -> dict[str, int]: ...


# Bad
from typing import Optional, List, Dict


def process(items: List[str], limit: Optional[int] = None) -> Dict[str, int]: ...
```

### 现代 Python 语法

- 使用 `match` 语句替代复杂的 `if/elif` 链（Python 3.10+）
- 使用 `dataclass`（或 `@dataclass(slots=True, frozen=True)`）定义数据结构
- 优先使用 `pathlib.Path` 而非 `os.path`
- 使用 f-string 进行字符串格式化；在 Python 3.14 中可用 t-string（PEP 750）进行安全模板化
- 使用海象运算符 `:=` 避免重复计算（谨慎使用，保持可读性）
- 用 `enumerate()` 替代手动索引，用 `zip()` 并行迭代

```python
# Good
for i, item in enumerate(items):
    ...

for key, value in mapping.items():
    ...

# Bad
for i in range(len(items)):
    item = items[i]
```

### 不可变性与常量

- 对不会修改的集合使用 `tuple` 而非 `list`
- 用 `Final` 标注模块级常量
- 用 `@dataclass(frozen=True)` 或 `NamedTuple` 定义不可变数据结构
- 避免全局可变状态

### 异常处理

- 捕获具体异常类型，而非裸 `except:` 或 `except Exception:`
- 用 `raise ... from err` 保留异常链
- 不要捕获异常后直接 `pass` 或无意义地重新抛出
- 优先用早返回（guard clause）减少嵌套

```python
# Good
try:
    result = parse(data)
except ValueError as err:
    raise ProcessingError("Invalid data format") from err

# Bad
try:
    result = parse(data)
except:
    pass
```

### 函数与模块设计

- 保持函数职责单一，认知复杂度低
- 使用关键字参数提升调用处可读性（`def func(*, key: str)`）
- 用 `__all__` 明确声明公开 API
- 避免在模块顶层执行有副作用的代码
- 优先使用纯函数（无副作用），将 I/O 推到边界层

### 异步代码

- 使用 `async/await`，避免直接调用 `asyncio.get_event_loop()`
- 用 `asyncio.TaskGroup`（Python 3.11+）并发管理任务，替代裸 `asyncio.gather`
- 不要在 async 函数中执行阻塞 I/O，使用 `asyncio.to_thread()` 卸载
- 用 `async with` 和 `async for` 管理异步资源

```python
# Good
async with asyncio.TaskGroup() as tg:
    task1 = tg.create_task(fetch(url1))
    task2 = tg.create_task(fetch(url2))
```

### 安全

- 不要用 `eval()` 或 `exec()` 执行动态代码
- 使用参数化查询，避免 SQL 字符串拼接
- 不要将密钥、密码硬编码在源码中，使用环境变量或 secrets 管理
- 对用户输入进行验证和清理（推荐 `pydantic`）
- 使用 `secrets` 模块生成安全随机数，而非 `random`

### 性能

- 优先使用生成器表达式而非列表推导式（当不需要随机访问时）
- 避免在循环内进行重复的属性查找，提前绑定到局部变量
- 用 `__slots__` 减少实例内存占用（或 `@dataclass(slots=True)`）
- 使用 `collections.deque` 代替列表实现队列
- 避免频繁的小字符串拼接，用 `"".join(parts)` 或 f-string

---

## 测试

- 使用 `pytest`，测试文件以 `test_` 前缀命名
- 每个测试只验证一个行为
- 使用 `pytest.fixture` 管理测试依赖，避免在测试间共享可变状态
- 用 `pytest.mark.parametrize` 替代重复的测试逻辑
- Mock 外部依赖（网络、文件系统、时间），保持单元测试快速稳定
- 不要在提交的代码中保留 `pytest.mark.skip` 或 `.only`

---

## ruff 无法覆盖的关注点

ruff 和 ty 会自动捕获大多数问题，手动关注：

1. **业务逻辑正确性** — 工具无法验证算法是否符合需求
2. **有意义的命名** — 变量、函数、类名应清晰表达意图
3. **架构决策** — 模块划分、依赖方向、接口设计
4. **边界条件** — 空集合、零值、超大输入、并发竞争
5. **文档** — 对复杂逻辑添加注释，但优先自文档化代码

---

提交前运行 `ruff format . && ruff check --fix . && ty check` 确保合规。
