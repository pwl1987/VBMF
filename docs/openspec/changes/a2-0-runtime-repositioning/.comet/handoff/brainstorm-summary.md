# Brainstorm Summary

- Change: a2-0-runtime-repositioning
- Date: 2026-09-02

## 确认的技术方案

（用户四刀裁定 + main.rs 1804 行区块地图 probe; brainstorming 技能盘上不存在按前例同态）

- lib + 双 bin（media-agent 组合根 <500 行 / media-agent-gates 薄壳）; gates 逻辑在 lib 模块保 crate:: 路径零漂移
- watchdog.rs 逐字节搬运（签名不变, Supervisor 边界零触碰）
- diagnostics.rs（C1/CAP-01/MEDIA-RT-01 自测/EXTERNAL-API 证据——诊断 boot 常跑, 只搬位置不搬执行位置=行为零变）
- bootstrap.rs DiagnosticWorld 单一构建器（main/gates 同源）
- Program Domain 腾位=lib.rs 注释锚, 零类型
- 头注释/Cargo description 修正为现实

## 关键取舍与风险

- gates 符号仍在 lib（链接进生产二进制）——语义证明生产零 gate 行为（代码路径+日志对照）, 如实记录此限度
- 大块搬运漂移风险 → 逐字节原则 + A20-01 日志逐段对照 + 全回归电池

## 测试策略

A20-01 行为零变日志对照 / A20-02 gates bin E1-E8+LOOPBACK / A20-03..06 P1a+P1b+A1+矩阵+mock251+CI

## Spec Patch

无
