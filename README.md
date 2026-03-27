# JSON-AutoTune

> 高性能 JSON 解析器，使用 SIMD 优化 + AI 自主调优，**所有测试场景超越 serde_json 45%+**

**基于 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架**

## 性能对比

| 测试 | json-autotune | serde_json | 领先幅度 |
|------|--------------|------------|----------|
| small (小对象) | **186ns** | 294ns | **+58%** |
| medium (中等) | **20.5µs** | 37.2µs | **+45%** |
| large (大文件) | **78.6 MiB/s** | 52 MiB/s | **+51%** |

## 核心优化技术

### 1. Tiny Object Optimization ⚡ 最大提升
- 小对象（≤3 字段）使用 `Box<[(K,V); 3]>` 固定数组
- 完全避免 HashMap 开销，栈上分配零堆开销

### 2. SmartString 内联字符串
- 短字符串（≤23字节）内联存储，避免堆分配

### 3. hashbrown + foldhash
- `hashbrown::HashMap` 替代标准 HashMap
- `foldhash::fast::FixedState` 提供极速哈希

### 4. SIMD 优化
- SSE2 空白符跳过
- 范围比较优化
- SIMD 字符串结束检测

### 5. 数字解析
- 整数快速路径
- `fast-float` 浮点解析
- DIGIT lookup table

## 项目结构

```
json-autotune-rs/
├── README.md         # 本文件
├── AUTOTUNE.md       # AI 优化过程记录
├── src/
│   ├── lib.rs        # 库入口
│   ├── parser.rs     # 解析器核心
│   ├── simd.rs       # SIMD 优化
│   ├── value.rs      # JSON 值类型
│   ├── number.rs     # 数字解析
│   └── error.rs      # 错误处理
├── benches/
│   └── benchmark.rs  # 性能测试
├── examples/
│   └── parse.rs      # 使用示例
├── results.tsv       # 实验历史
└── Cargo.toml
```

## 快速开始

```rust
use json_autotune::parse;

let value = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
assert_eq!(value["name"].as_str(), Some("Alice"));
```

## 运行 benchmark

```bash
cargo bench
```

## 依赖

- Rust 2021 edition
- 仅使用纯 Rust 依赖（无 C++ 绑定）

## 参考

- [karpathy/autoresearch](https://github.com/karpathy/autoresearch) - AI 自主研究框架
- [simdjson](https://github.com/simdjson/simdjson) - C++ SIMD JSON 解析器
- [serde_json](https://github.com/serde-rs/json) - 标准参考实现
