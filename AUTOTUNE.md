# JSON-AutoTune 自主优化任务要求

> 本文件定义 AI Agent 的优化目标和要求，参考 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架

---

## 目标

**持续优化 JSON 解析器性能，目标是所有测试场景下达到原目标 2 倍（超越目标 100%）**

---

## 固定基准（不可修改）

### `benches/benchmark.rs` - 评估函数

```rust
// 小对象测试: {"name":"Alice","age":30,"active":true}
fn gen_small() -> String { r#"{"name":"Alice","age":30,"active":true}"#.into() }

// 中等对象测试: 100 个用户记录
fn gen_medium() -> String {
    let items: Vec<String> = (0..100).map(|i| format!(r#"{{"id":{},"name":"User{}"}}"#, i, i)).collect();
    format!(r#"{{"users":[{}]}}"#, items.join(","))
}

// 大文件测试: 1000 个记录
fn gen_large() -> String {
    let items: Vec<String> = (0..1000).map(|i| format!(r#"{{"id":{},"data":[1,2,3]}}"#, i)).collect();
    format!(r#"{{"items":[{}]}}"#, items.join(","))
}
```

### 评估指标

- **small/medium**: 解析时间（越低越好）
- **large**: 吞吐量 MiB/s（越高越好）

### 对比基准

- `serde_json::from_str::<serde_json::Value>`

---

## 可变模块（Agent 可修改）

### `src/parser.rs` - 解析器实现
Agent 可以修改任何部分：
- 对象存储结构
- 字符串存储方式
- 数字解析逻辑
- 内存分配策略

### `src/value.rs` - JSON 值类型
### `src/simd.rs` - SIMD 优化
### `src/number.rs` - 数字解析
### `Cargo.toml` - 依赖和编译参数

---

## 优化要求

### 1. 正确性优先
- 所有解析结果必须与 serde_json 一致
- 错误处理必须正确
- 不引入 panic 或 UB

### 2. 性能目标（超越目标 100%）
- **small**: 目标 < 100ns（当前 ~190ns 🔴）
- **medium**: 目标 < 12.5µs（当前 ~18.5µs 🔴）
- **large**: 目标 > 140 MiB/s（当前 ~80 MiB/s 🔴）

### 3. 约束条件
- 仅使用纯 Rust 依赖
- 不修改 `benches/benchmark.rs`
- 保持 API 兼容（`parse()` 函数签名不变）

### 4. 代码质量
- 遵循 Rust 最佳实践
- 避免不必要的依赖
- 保持代码简洁

---

## 实验循环

```
LOOP FOREVER:
1. 查看当前 git 状态
2. 修改 src/ 下的文件（尝试新优化）
3. git commit
4. 运行实验: cargo bench
5. 解析结果，对比 serde_json
6. 记录到 results.tsv
7. 如果改进 → 保留 commit；否则 → git reset 回退
```

---

## 决策规则

| 结果 | 行为 |
|------|------|
| 性能提升 | 保留 commit，继续迭代 |
| 性能下降 | git reset 回退 |
| 编译失败/测试失败 | 回退，跳过 |

---

## 结果记录格式 (results.tsv)

```
timestamp	test	autotune	serde_json	ratio	status	description
2026-03-28_00:10	small	186ns	294ns	0.63	keep	Tiny Object
```

---

## 设计原则

> **简单胜过复杂。同样的效果，更简单的代码更好。删除代码且效果相当 = 好结果。**

1. **分层优化**: 小/中/大对象使用不同策略
2. **避免堆分配**: 栈上能存的不放堆
3. **SIMD 加速**: 批量操作使用 SIMD 指令
4. **缓存友好**: 减少指针跳转，提高命中率

---

## 参考技术

- [simdjson](https://github.com/simdjson/simdjson) - C++ SIMD JSON 解析器
- [smartstring](https://github.com(optional) - 栈上内联字符串
- [foldhash](https://github.com/) - 快速确定性哈希
- [fast-float](https://github.com/) - 快速浮点解析
