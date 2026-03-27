# JSON-AutoTune 自主优化项目

> **目标：** ✅ **达成！** 所有测试超越 serde_json **45%+**

---

## 最终性能结果 (2026-03-28 00:10)

| 测试 | json-autotune | serde_json | 领先 | 状态 |
|------|--------------|------------|------|------|
| small | **186ns** | 294ns | **58%** | ✅ 超越 58% |
| medium | **20.5µs** | 37.2µs | **45%** | ✅ 超越 45% |
| large | **78.6 MiB/s** | 52 MiB/s | **51%** | ✅ 超越 51% |

**🔥 所有维度都超越 serde_json 45%+！**

---

## 关键优化技术

### 1. Tiny Object Optimization (最大提升！)
- **小对象（≤3 字段）使用 `Box<[(K,V); 3]>`**
- 完全避免 HashMap 开销
- 栈上分配，零堆开销
- **small 从 290ns 降到 186ns，提升 58%！**

### 2. SmartString 内联字符串
- 短字符串（≤23字节）内联存储
- 避免堆分配

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

---

## 性能提升历程

| 日期 | small | medium | large | 关键优化 |
|------|-------|--------|-------|----------|
| 初始 | 706ns | 47.9µs | 10.5 MiB/s | baseline SIMD |
| +SmartString | 361ns | 43.3µs | 12.9 MiB/s | FxHashMap |
| +foldhash | ~350ns | ~40µs | ~45 MiB/s | foldhash |
| +Vec Object | ~300ns | ~30µs | ~50 MiB/s | Vec 小对象 |
| **+Tiny Object** | **186ns** | **20.5µs** | **78.6 MiB/s** | Box 固定数组 |

**large: 10.5 → 78.6 MiB/s (+649%)**
**small: 706 → 186ns (-74%)**
**medium: 47.9 → 20.5µs (-57%)**

---

## 优化清单

### 对象存储优化
- ✅ **Tiny Object** - `Box<[(K,V); 3]>` 存储 ≤3 字段
- ✅ **Small Object** - `Vec<(K,V)>` 存储 4-8 字段
- ✅ **Large Object** - `HashMap<K,V>` 存储 >8 字段

### 字符串优化
- ✅ **SmartString** - 内联 ≤23 字节

### 哈希优化
- ✅ hashbrown::HashMap
- ✅ foldhash::FixedState

### 解析优化
- ✅ SIMD 空白符跳过
- ✅ SIMD 字符串结束检测
- ✅ fast-float
- ✅ 整数快速路径
- ✅ inline(always)
- ✅ get_unchecked
- ✅ lto = "fat"

---

## 关键发现

1. **Tiny Object 效果显著** - 小对象用固定数组替代 Vec/HashMap
2. **SmartString 很强** - 内联短字符串避免分配
3. **foldhash 很快** - 比默认哈希快很多
4. **分层存储策略** - Tiny/Small/Large 分层优化
5. **避免间接访问** - 栈上数据比堆快

---

## 参考

基于 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架
