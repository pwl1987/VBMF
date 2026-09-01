# Brainstorm Summary

- Change: prototype-p1a-output-pipeline
- Date: 2026-09-02

## 确认的技术方案

（用户 2026-09-02 裁定即方案骨架 + 盒上 probe 实证；brainstorming 技能盘上不存在，按 p07c8/p07d/v03-d14 前例以 handoff+Design Doc 同态推进，用户已预授权自主推进）

- 输出物化在 domain（pipeline.rs，与 src_props 同层）；controller 纯执行 plan 输出段，不硬编码 element
- `PipelinePlan` 增 `outputs: Vec<OutputPlan>`（本 change 至多物化 1 项——单会话单输出；多输出留 Alpha）
- sink.kind 词表 {appsink=纯分析(默认), hls, rtmp}；未知 kind fail-closed；目标 env 缺失 ⇒ 降级纯分析（fail-soft，部署态非契约违约）
- tee 双分支：分析分支逐字符不动；编码分支独立 queue（结构性解耦背压）
- RTMP=rtmpsink（probe：属性面最简 location=URL）；HLS=hlssink2（playlist-location/location/target-duration/max-files 全在位）；flvmux streamable=true；x264enc bitrate 单位 kbit/s
- PrototypeOutputConfig 经 env（VBMF_OUTPUT_*），materialize 内读取（签名零改动），纯变体 materialize_with_output 供测试
- 运行时可见性：MediaSession/SessionRuntimeState 加法 outputs 摘要；顶层 8 键不动
- main.rs：VBMF_OUTPUT_KIND 覆盖诊断 intent sink kind（默认 rtmp 同今天）

## 关键取舍与风险

- env 读取入 materialize = demo 层缝（正式配置模型阶段收口；显式记录）
- SessionRuntimeState 去 Copy derive（加 Vec）——编译器全量核查
- x264 软编 CPU 余量 = 真机 gate P1a-05 持续运行验证项
- recover() 从存档 plan 重建 → 输出分支自动恢复（probe 实证 GstInstance 持 plan）

## 测试策略

Unit（物化三态/未知拒绝/默认值）+ Simulation（mock 计划驱动+221 基线零回退）+ Hardware Gate P1a-01..06（盒上真机；RTMP 接收端=ffmpeg -listen；HLS=分片文件+ffprobe 断言）+ 既有回归（E1-E8/MEDIA-RT-01/14 步矩阵）

## Spec Patch

无（skip_specs:true，同 v03-d14 前例）
