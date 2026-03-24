# JSON-AutoTune

JSON 解析器性能自动优化项目。让 AI Agent 自主探索优化路径。

## 目标

- 实现一个完整的 JSON 解析器
- 使用 SIMD 指令加速关键路径
- 与 serde_json 进行性能对比
- 记录优化过程

## 参考

- [simdjson](https://github.com/simdjson/simdjson) - C++ SIMD JSON 解析器
- [simd-json.rs](https://github.com/simd-lite/simd-json) - Rust 移植
- [serde_json](https://github.com/serde-rs/json) - 标准参考实现

## 项目结构

```
json-autotune-rs/
├── AUTOTUNE.md      # AI 优化过程记录
├── src/
│   ├── lib.rs       # 库入口
│   ├── parser.rs    # 解析器实现
│   ├── simd.rs      # SIMD 优化
│   ├── value.rs     # JSON 值类型
│   └── error.rs     # 错误处理
├── benches/
│   └── benchmark.rs # 性能测试
├── examples/
│   └── parse.rs     # 示例
└── results.tsv      # 性能结果
```

## 使用

```rust
use json_autotune::parse;

let value = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
println!("{}", value["name"].as_str().unwrap());
```