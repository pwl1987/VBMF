# Brainstorm Summary

- Change: alpha1-multi-input-channel
- Date: 2026-09-02

## 确认的技术方案

（用户 Alpha 路线裁定 + probe 实证; brainstorming 技能盘上不存在, 按前例 handoff+Design Doc 同态, 用户已预授权）

- D10 激活: start() 实例化全部 plans + 失败中途回滚零孤儿; MediaSession.inputs 句柄表加法（pipeline 保留=首输入兼容）
- Channel = 保守子集: 多输入聚合投影（SessionRuntimeState.inputs）; **命名=控制台侧规约 CH+显示序**（状态不携带序, HashMap 无序防漂移）; V0.2 failover 全语义不进
- 输出策略: 仅首输入物化输出段（单输出承诺, P1a/P1b gate 不变）, 次输入纯分析
- 诊断 VBMF_DIAG_INPUTS（默认 1=现行为）; 控制台输入行 + 聚合色

## 关键取舍与风险

- session 核心改动 → 单输入逐字节兼容断言 + 全回归
- 双卡并发打开资源争抢 → gate 实证
- 停止漏停孤儿 → 逆序 + 零孤儿 Unit + A1-05

## 测试策略

Unit（句柄表/回滚/投影/仅首输出/单输入零回退）+ Hardware A1-01..07（双卡真机, 信号实况自适应）+ 全回归

## Spec Patch

无（skip_specs 同前例）
