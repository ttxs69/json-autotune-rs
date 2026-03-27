# JSON-AutoTune 自主优化项目

> **目标：** ✅ **已达成！** 所有测试用例性能超越 serde_json

---

## 🎉 最终结果 (2026-03-27 12:20)

| 测试 | json-autotune | serde_json | 比值 | 状态 |
|------|--------------|------------|------|------|
| small | **291ns** | 299ns | **103%** | ✅ **超越!** |
| medium | **30.0µs** | 37.7µs | **126%** | ✅✅✅ **超越 26%!** |
| large | **52.8 MiB/s** | 52.3 MiB/s | **101%** | ✅ **超越!** |

**性能提升历程：**
- 初始 large: 10.5 MiB/s → 最终 **52.8 MiB/s** (+403%)
- medium: 2x 慢 → **超越 26%**
- small: 3x 慢 → **超越 3%**

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
- ✅ Array 容量 4（匹配典型 JSON 数组大小）
- ✅ 内联尾部空白检查

---

## 关键发现

1. **HashMap 容量很关键** - 从 8 降到 3，medium 性能提升 20%+
2. **SIMD 首字节检查** - 避免 SIMD 开销的快速路径
3. **FxHashMap** - 比 HashMap 快约 10%
4. **inline(always)** - 热路径必须内联
5. **Array 初始容量** - 小 JSON 中数组通常只有 2-4 个元素

---

*基于 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架*
