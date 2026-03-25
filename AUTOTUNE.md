# JSON-AutoTune 自主优化项目

> **目标：** 让 AI Agent 自主优化 JSON 解析器性能，直到超越 serde_json

---

## 一、核心机制

```
┌─────────────────────────────────────────────────────────┐
│                    实验循环                              │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐            │
│  │ 修改代码  │ → │ 运行测试  │ → │ 检查性能  │            │
│  └──────────┘   └──────────┘   └──────────┘            │
│       ↑                              ↓                  │
│       │         ┌──────────┐         │                  │
│       └─────────│保留/丢弃  │←────────┘                  │
│                 └──────────┘                            │
│                     ↓                                   │
│               重复循环直到目标达成                        │
└─────────────────────────────────────────────────────────┘
```

**关键约束：**
- 唯一指标：性能 vs serde_json（越快越好）
- 固定测试用例：benchmarks/
- 单文件修改原则：主要修改 `src/parser.rs`

---

## 二、文件结构

| 文件 | 作用 | 谁来修改 |
|------|------|----------|
| `benches/benchmark.rs` | 性能测试框架 | **不可修改**（固定基准） |
| `src/parser.rs` | 核心解析逻辑 | **AI Agent 修改** |
| `src/simd.rs` | SIMD 优化 | **AI Agent 修改** |
| `AUTOTUNE.md` | 本文件 | 人类修改 |
| `results.tsv` | 实验记录 | 自动生成 |

---

## 三、性能目标

| 测试用例 | serde_json | 目标 |
|----------|------------|------|
| small | 296ns | < 300ns ✅ |
| medium | 37µs | < 40µs |
| large | 53 MiB/s | > 50 MiB/s |

**最终目标：所有测试用例性能 ≥ serde_json**

---

## 四、实验循环规则

### 4.1 循环步骤

```
LOOP UNTIL 所有目标达成:
1. 分析当前性能瓶颈
2. 提出优化方案
3. 修改代码
4. 运行: cargo bench 2>&1 | tee bench.log
5. 解析性能结果
6. 记录到 results.tsv
7. 决策：
   - 改进 → git commit，继续
   - 退步 → git checkout 回退
   - 崩溃 → 记录错误，回退
```

### 4.2 决策规则

| 结果 | 行为 |
|------|------|
| 性能提升 ≥ 5% | 保留 commit，标记 "keep" |
| 性能变化 < 5% | 视情况保留或回退 |
| 性能下降 | git checkout 回退，标记 "discard" |
| 编译失败/测试失败 | 记录错误，回退，标记 "fail" |

### 4.3 结果记录格式

```
# results.tsv
timestamp	test	autotune	serde_json	ratio	status	description
2026-03-24_13:52	small	438ns	296ns	0.68	keep	SIMD string detect
```

---

## 五、优化方向参考

### Phase 1: 算法优化
- [ ] 零拷贝字符串
- [ ] 快速数字解析
- [ ] 内存预分配

### Phase 2: SIMD 优化
- [x] 空白符跳过 (SSE2)
- [x] 字符串结束检测
- [ ] SIMD 数字解析
- [ ] SIMD 结构字符查找
- [ ] AVX-512 支持

### Phase 3: 并行优化
- [ ] 多线程解析（大 JSON）
- [ ] 流式解析

### Phase 4: 其他
- [ ] 内联优化
- [ ] 分支预测优化
- [ ] 缓存友好布局

---

## 六、当前状态

**最新基准 (2026-03-25 13:55):**

| 测试 | json-autotune | serde_json | 比值 | 状态 |
|------|--------------|------------|------|------|
| small | 347ns | 295ns | 0.85x | ✅ 接近目标 |
| medium | 42.5µs | 37.1µs | 0.87x | ✅ 接近目标 |
| large | 47.3 MiB/s | 53.7 MiB/s | 0.88x | ✅ 接近目标 |

**已完成的优化：**
- ✅ SIMD 空白符跳过 (SSE2)
- ✅ SIMD 字符串结束检测
- ✅ FxHashMap 替代 HashMap
- ✅ 预扫描估算容器大小
- ✅ u32 批量比较关键字 (null/true)
- ✅ inline skip_ws 减少 call overhead
- ✅ 展开整数解析前4位
- ✅ ptr::copy_nonoverlapping 字符串复制
- ✅ lexical-core 浮点解析
- ✅ if-else 分支重排序

**性能提升历程：**
- 初始: large 10.5 MiB/s → 现在 47.3 MiB/s (**+350%**)
- 与 serde_json 差距: 从 3x → 现在 **12-15%**

---

## 七、永不停止原则

> 一旦开始优化循环，不要停下来问用户是否继续。
> 继续迭代直到所有性能目标达成或遇到不可逾越的障碍。

---

## 八、快速参考命令

```bash
# 运行基准测试
cargo bench 2>&1 | tee bench.log

# 运行单元测试
cargo test --release

# 提交改进
git add -A && git commit -m "optimize: $description"

# 回退
git checkout src/parser.rs src/simd.rs

# 推送
git push
```

---

*基于 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架*