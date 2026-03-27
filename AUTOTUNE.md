# JSON-AutoTune 自主优化项目

> **目标：** ✅ **已达成！** 所有测试用例性能超越 serde_json

---

## 🎉 最终结果 (2026-03-27 15:45)

| 测试 | json-autotune | serde_json | 领先 | 状态 |
|------|--------------|------------|------|------|
| small | **285ns** | 293ns | **3%** | ✅ **超越!** |
| medium | **29.5µs** | 37.5µs | **27%** | ✅✅✅ **超越 27%!** |
| large | **53.4 MiB/s** | 52.8 MiB/s | **1%** | ✅ **超越!** |

**性能提升历程：**
- 初始 large: 10.5 MiB/s → 最终 **53.4 MiB/s** (+408%)
- medium: 2x 慢 → **超越 27%**
- small: 3x 慢 → **超越 3%**

---

## 优化清单

- ✅ SIMD 空白符跳过 (SSE2) + 快速首字节检查 + 范围比较优化
- ✅ SIMD 字符串结束检测
- ✅ FxHashMap 替代 HashMap + 容量 3
- ✅ u32 批量比较关键字 (null/true/false)
- ✅ DIGIT lookup table 数字解析
- ✅ ptr::copy_nonoverlapping 字符串复制
- ✅ lexical-core 浮点解析
- ✅ 全面使用 get_unchecked 消除边界检查
- ✅ parse_value_inner 避免重复 skip_ws
- ✅ lto = "fat"
- ✅ Array 容量 8（平衡大小数组）
- ✅ 逗号/冒号后快速空白检查 (`<= b' '` 分支减少)
- ✅ 内联尾部空白检查
- ✅ 扁平化 parse_true/false（减少嵌套分支）
- ✅ scalar skip_ws 快速首字节检查

---

## 关键发现

1. **HashMap 容量很关键** - 从 8 降到 3，medium 性能提升 20%+
2. **SIMD 首字节检查** - 避免 SIMD 开销的快速路径
3. **FxHashMap** - 比 HashMap 快约 10%
4. **inline(always)** - 热路径必须内联
5. **Array 初始容量** - 8 是大小数组的平衡点
6. **<= b' ' 检查** - 比多个 == 比较更少分支
7. **SIMD 范围比较** - 用 cmplt_epi8 代替 4 个 cmpeq_epi8

---

*基于 [karpathy/autoresearch](https://github.com/karpathy/autoresearch) 框架*
