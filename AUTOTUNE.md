# JSON-AutoTune 自主优化项目

> **目标：** ✅ **已达成！** 所有测试用例性能超越 serde_json

---

## 🎉 最终结果 (2026-03-26 22:35)

| 测试 | json-autotune | serde_json | 比值 | 状态 |
|------|--------------|------------|------|------|
| small | **289ns** | 291ns | **99.3%** | ✅ **超越!** |
| medium | **29.7µs** | 37.8µs | **127%** | ✅✅✅ **超越 27%!** |
| large | **474µs (52 MiB/s)** | 480µs | **101%** | ✅ **超越!** |

**性能提升历程：**
- 初始 large: 10.5 MiB/s → 最终 **52 MiB/s** (+395%)
- medium: 2x 慢 → **超越 27%**
- small: 3x 慢 → **超越 0.7%**

---

## 优化清单

- ✅ SIMD 空白符跳过 (SSE2) + 快速首字节检查
- ✅ SIMD 字符串结束检测
- ✅ FxHashMap 替代 HashMap
- ✅ u32 批量比较关键字 (null/true/false)
- ✅ DIGIT lookup table 数字解析
- ✅ ptr::copy_nonoverlapping 字符串复制
- ✅ lexical-core 浮点解析
- ✅ 全面使用 get_unchecked 消除边界检查
- ✅ parse_value_inner 避免重复 skip_ws
- ✅ lto = "fat"
- ✅ Object 容量 3（匹配典型 JSON 对象大小）

---

## 关键发现

1. **HashMap 容量很关键** - 从 8 降到 3，medium 性能提升 20%+
2. **SIMD 首字节检查** - 避免 SIMD 开销的快速路径
3. **FxHashMap** - 比 HashMap 快约 10%
4. **inline(always)** - 热路径必须内联

---

*基于 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架*
