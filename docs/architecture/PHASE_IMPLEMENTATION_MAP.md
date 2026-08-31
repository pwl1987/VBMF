# PHASE_IMPLEMENTATION_MAP — 实施阶段地图（Implementation Roadmap SoT）

> 状态：**ACTIVE SoT**（实施路线的唯一事实源；状态列随阶段推进更新）。
> 性质声明：本地图是 **Implementation Roadmap**，不改变冻结 Architecture Contract（V0.2 LOCK FINAL 与各 `CONTRACT = FROZEN` 契约的语义不受本文件影响）。
> 建立：2026-08-30（p07b-consolidation，0.7B 收口对账）——取代 `PHASE_0_6_MASTER_PRD.md` §5 中规划期的 0.7A-G 标签体系（原文在该文档保留并注记）。

## 1. 阶段表

| Phase | 内容 | 状态 | Merge | Baseline Tag | 证据 |
|---|---|---|---|---|---|
| **0.6** | Runtime Abstraction（SPI/契约对齐/remove-adapter 证明/六→七 CI 门禁） | ✅ COMPLETE | `d1cfaa9` (PR#1) | `phase-0.6-runtime-abstraction-baseline` | verify 报告 2026-08-29-p06-final-merge-hardening |
| **0.7A** | Session Runtime（SessionManager/Resource 编排/Lease/Preflight；四轮 Merge Gate Hardening） | ✅ COMPLETE | `adc6f19` (PR#2) | `phase-0.7A-session-runtime-baseline` | verify 报告 §7/§8/§9 + SESSION-RT-01/RESOURCE-RT-01 真机 ALL PASS |
| **0.7B-1** | Normalize Foundation（CanonicalMediaDescriptor + normalize 纯函数） | ✅ COMPLETE | `30671b5` (PR#3) | `phase-0.7B1-normalize-foundation` | NORMALIZE-RT-01 三层 |
| **0.7B-2A** | Clock Domain（CanonicalClockDomain，#147 观测词表，零决策） | ✅ COMPLETE | `f90c6f8` (PR#4) | `phase-0.7B2A-clock-domain` | MEDIA-SEMANTICS-RT-01 (Clock) 三层 |
| **0.7B-2B** | Audio Semantics（CanonicalAudioStream/Role/Layout + AudioRouteIntent 语义意图） | ✅ COMPLETE | `04d6f4f` (PR#5) | `phase-0.7B2B-audio-semantics` | AUDIO-SEMANTICS-RT-01 三层 |
| **0.7B-2C** | Timecode Foundation（CanonicalTimecode，#148 词表，不实现 parser） | ✅ COMPLETE | `c574238` (PR#6) | `phase-0.7B2C-timecode-foundation` | TIMECODE-SEMANTICS-RT-01 三层 |
| **0.7B** | Canonical Media Semantics（Video/Audio/Clock/Timecode 四基础 + 收口对账） | ✅ COMPLETE（本 change 收口：文档对账 + Integration Audit + Phase Map） | — | — | Integration Audit 报告 2026-08-30 |
| **0.7C-Foundation** | Canonical Runtime State（Canonical→Runtime 第一条生产聚合边 + D2/D4/D5 伴随清偿） | ✅ COMPLETE | (本 PR) | `phase-0.7C1-runtime-state`（合并后打） | RUNTIME-STATE-RT-01 三层 + Integration Audit 补边 |
| **0.7C-2** | Runtime Query Model（Pure Read / Snapshot 门面 + D6 capability projection/硬判定 + D14/D15 登记） | ✅ COMPLETE | (本 PR) | `phase-0.7C2-runtime-query`（合并后打） | RUNTIME-QUERY-RT-01 三层 |
| **0.7C-3** | Command Contract Foundation（请求语义非执行计划; 不可执行性三重守护; 三命令薄映射） | ✅ COMPLETE | (本 PR) | `phase-0.7C3-command-contract`（合并后打） | COMMAND-CONTRACT-RT-01 三层（真机 envelope 驱动全 Executed） |
| **0.7C-4** | Idempotency Foundation（D9-A~E: 同一命令 fingerprint 语义冻结 + 单临界区原子 claim + replay/conflict 两平面分层; **D9 Foundation CLOSED**——External/持久化语义 deferred to External API） | ✅ COMPLETE | `317d99d` (PR#11) | `phase-0.7C4-idempotency` | IDEMPOTENCY-RT-01 三层（真机 executed/replayed/outcome_equal/conflict） |
| **0.7C-5** | Error Model Foundation（失败归因分类平面 ErrorClassification 五词表 + classify_session_error 封闭映射 + outcome 分类不变量; **三平面分离红线: CommandStatus≠IdempotentDispatch≠ErrorClassification**） | ✅ COMPLETE | `a6c5925` (PR#12) | `phase-0.7C5-error-model` | ERROR-MODEL-RT-01 三层（真机 ghost-stop PermanentFailure 实证） |
| **0.7C-6** | Event Projection Foundation + D8 EventSink Decoupling（RuntimeEventSink trait + 组合根单表 + SessionManager 直连 + Supervisor 收窄纯决策 + project() 纯函数投影; 四语义零偷改; **D8 CLOSED**） | ✅ COMPLETE | `9b475c1` (PR#13) | `phase-0.7C6-event-projection` | EVENT-PROJECTION-RT-01 三层（真机投影 46 事件实证） |
| **0.7C-7** | External API Foundation（**API Boundary Model**——五大独立 API 资源类型 + to_api_* 纯转换 + Command/Event/Idempotency 三平面 API 模型 + 契约层 Idempotency 持久化边界三选项冻结; **非 Web Server, 零 transport/持久化**; API-BOUNDARY-01 白盒 + 终审禁清单 11 项） | ✅ COMPLETE | (本 PR) | `phase-0.7C7-external-api`（合并后打） | EXTERNAL-API-RT-01 三层（真机 verdict=OK 实证） |
| **0.7C** | External Integration（§3 下一项 = **Transport 实现**——std-only 纪律, 单独开 change; API Boundary Model 已完成, transport 只做模型到 wire 的序列化边界） | 📋 NEXT | — | — | — |
| **0.7D** | Event Projection / Integration（EventSink 解耦 D8 与此同期） | 📋 | — | — | — |
| **0.8** | Federation / Multi-site（P2） | 📋 | — | — | — |

## 2. 0.7 全阶段最高架构红线（终审 2026-08-30 冻结）

任何新模块必须证明：

1. **没有把 Observation 偷变成 Configuration**（观测 ≠ 写回 Graph/Backend 决策）；
2. **没有把 Semantic Intent 偷变成 Execution Plan**（Intent → Plan → Backend 的转换只发生在 Runtime/编排层）；
3. **没有把 Canonical 类型重新绑回 Vendor**（canonical 层零 vendor 字段/零 adapter 依赖，serde 反向断言 + 公开面 allowlist + remove-adapter proof 三重守护）。

## 3. 0.7C 前置顺序（终审裁定，不直接做 REST API）

```
Canonical Runtime State → Runtime Query Model → Command Contract →
Idempotency (✅ 0.7C-4) → Error Model (✅ 0.7C-5) → Event Projection → External API
```

避免把 API 做成"直接把 Rust 内部结构暴露出去"。

## 4. 0.7C 前必须清偿的债务（详见 PHASE_0_7A_POST_MERGE_DEBT.md 优先级分组）

- **D2** derive_claims FAIL 化（RESOURCE-RESOLUTION-01）
- **D4** PortAvailability 精确化（端口级：direction/capability/availability）
- **D5** IdentityBinding 实查（IDENTITY-BINDING-01：strength/verification，非 key-existence）
- **D6** BACKEND-CAPABILITY-01（真实能力探针 + 硬性判定）

## 5. 关联

- 冻结契约索引：`README.md`；综合契约：`IMPLEMENTATION_ADDENDUM.md`
- 债务登记：`PHASE_0_7A_POST_MERGE_DEBT.md`（D1-D13）
- Canonical→Runtime 接线现状审计：`docs/superpowers/reports/2026-08-30-p07b-consolidation-integration-audit.md`
