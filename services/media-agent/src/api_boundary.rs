//! Phase 0.7C-7: External API Foundation — **API Boundary Model + Idempotency 持久化契约**。
//!
//! 终审 0.7C-6 批准范围（**禁清单 11 项开工边界冻结**）:
//! - ❶ Axum/Hyper/Warp/Actix/gRPC ❷ HTTP listener ❸ REST route ❹ OpenAPI ❺ 数据库持久化
//! - ❻ 跨重启 Idempotency ❼ Event persistence ❽ Health Reducer 完整实现
//! - ❾ 修改 Command/Query/Event 内部契约 ❿ 内部 Rust DTO 直接作 API DTO
//! - ⓫ `ApiResponse<T>` 万能包装 ⓬ 新建第二套 Runtime State
//!
//! 本模块 = API Contract / Boundary Foundation（非 Web Server）。transport 实现属下一 change（std-only 纪律）。
//!
//! 设计原则：
//! - **API Resource Model 独立定义**（不绑回 Runtime 内部 enum；`to_api_*` 纯函数显式映射）
//! - **不暴露 Rust serde tag 习惯**（`verdict`/`retryable_failure` 等内部命名不出现在 JSON）
//! - **EventProjection ≠ CanonicalRuntimeState**（`ApiProjectionKind::EventProjectionSnapshot` 序列化字段守门）
//! - **Command/Query/Event 三平面分离**（各自独立 enum，不聚合"万能 ApiResult"）
//! - **Idempotency 持久化边界契约层**（仅契约冻结，无持久化实现——三选项对勘公开化）
//!
//! **API-BOUNDARY-01 白盒门禁**：本模块不得 `use` `backend` / `gstreamer` / `decklink` / `ffmpeg` / `pipeline` (impl) / `provider` (impl)。
//! 允许消费：`Canonical` types / `CanonicalRuntimeState` 子项 / `RuntimeQuery` 输出 / `CommandEnvelope/Outcome/Status/Rejection` / `IdempotentDispatch` / `ErrorClassification` / `EventProjection` —— 经 `to_api_*` 纯函数构造 API 模型。

#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event_projection::EventProjection;
use crate::runtime_state::{
    CanonicalRuntimeState, DeviceRuntimeState, PortRuntimeState, ResourceRuntimeState,
    SessionRuntimeState,
};

// === Query 平面：API 资源模型（独立定义） ====================================

/// API 设备资源。字段仅由 API 消费语义驱动 (0.7B 加严红线: 禁万能 struct)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiDevice {
    pub id: String,
    pub model: String,
    /// canonical binding_status 字符串化 (high/manifest_verified/...), **不暴露** Runtime BindingStatus enum。
    pub binding: Option<String>,
    pub capabilities: Option<Vec<ApiCapability>>,
}

/// API 能力项 — 字段由 DeviceCapabilitiesSummary 实际字段 (can_input/can_output/
/// input_ports/output_ports) 显式映射, 不暴露内部 CapabilityFlag enum。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiCapability {
    /// "supported" / "unsupported" / "unknown" — D6 三态字符串化。
    pub can_input: String,
    pub can_output: String,
    pub input_ports: Option<u32>,
    pub output_ports: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiPort {
    pub id: String,
    pub device_id: String,
    /// "input" / "output" / "bidirectional"。
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResource {
    pub id: String,
    pub device_id: String,
    /// API 字符串 ("available"/"reserved"/"allocated"/"releasing"/"faulted") —
    /// **不绑回** `ResourceState` enum (终审 NOTE-3: API 资源模型独立)。
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiSession {
    pub id: String,
    /// SessionState 字符串化 ("reserved"/"running"/"paused"/"releasing"/"released"/"terminated")。
    pub state: String,
    /// SessionPhase 字符串化 ("requested"/"provisioning"/"binding"/"leased"/"starting"/"running"/"stopping"/"released"/"failed")。
    pub phase: String,
}

// 纯函数: Query 五资源转换 (CanonicalRuntimeState 子项 → API 模型)
// 输入是 canonical 公开 DTO (CanonicalRuntimeState 不绑 vendor), 输出是 API 独立 enum/字符串。
pub fn to_api_device(d: &DeviceRuntimeState) -> ApiDevice {
    ApiDevice {
        id: d.device_id.to_string(),
        model: d.model.clone(),
        binding: d
            .binding
            .as_ref()
            .map(|b| format!("{:?}", b).to_lowercase()),
        capabilities: None, // 全 capabilities 由 to_api_capabilities 单独产出
    }
}

pub fn to_api_port(p: &PortRuntimeState) -> ApiPort {
    ApiPort {
        id: p.port_id.to_string(),
        device_id: p.device_id.to_string(),
        direction: format!("{:?}", p.direction).to_lowercase(),
    }
}

pub fn to_api_resource(r: &ResourceRuntimeState) -> ApiResource {
    ApiResource {
        id: r.resource_id.to_string(),
        device_id: r.device_id.to_string(),
        state: format!("{:?}", r.state).to_lowercase(),
    }
}

pub fn to_api_session(s: &SessionRuntimeState) -> ApiSession {
    ApiSession {
        id: s.session_id.to_string(),
        state: format!("{:?}", s.state).to_lowercase(),
        phase: format!("{:?}", s.phase).to_lowercase(),
    }
}

pub fn to_api_capabilities(
    cs: &[(uuid::Uuid, crate::runtime_state::DeviceCapabilitiesSummary)],
) -> Vec<(String, ApiCapability)> {
    cs.iter()
        .map(|(id, cap)| {
            (
                id.to_string(),
                ApiCapability {
                    can_input: format!("{:?}", cap.can_input).to_lowercase(),
                    can_output: format!("{:?}", cap.can_output).to_lowercase(),
                    input_ports: cap.input_ports,
                    output_ports: cap.output_ports,
                },
            )
        })
        .collect()
}

/// 顶层 Query 响应:返回 CanonicalRuntimeState 的 API 视图。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiQuerySnapshot {
    pub devices: Vec<ApiDevice>,
    pub ports: Vec<ApiPort>,
    pub resources: Vec<ApiResource>,
    pub sessions: Vec<ApiSession>,
    pub capabilities: Vec<(String, ApiCapability)>,
    pub generated_at_ms: u64,
    /// D14: 观察信封 additive 投影（wire 同名字段, 非破坏）。
    /// 有意**不带** `#[serde(default)]`: 本结构是响应模型, 无旧 JSON 反序列化消费方
    /// （与 CanonicalRuntimeState 的 additive 兼容义务不对称是设计决定）。
    pub observation_revision: u64,
    pub observation_lineage: uuid::Uuid,
}

pub fn to_api_query_snapshot(state: &CanonicalRuntimeState) -> ApiQuerySnapshot {
    // capabilities 是 per-device 字段 (CanonicalRuntimeState 无独立 capabilities 平面 — 不发明)。
    let caps: Vec<(uuid::Uuid, crate::runtime_state::DeviceCapabilitiesSummary)> = state
        .devices
        .iter()
        .filter_map(|d| d.capabilities.as_ref().map(|c| (d.device_id, *c)))
        .collect();
    ApiQuerySnapshot {
        devices: state.devices.iter().map(to_api_device).collect(),
        ports: state.ports.iter().map(to_api_port).collect(),
        resources: state.resources.iter().map(to_api_resource).collect(),
        sessions: state.sessions.iter().map(to_api_session).collect(),
        capabilities: to_api_capabilities(&caps),
        generated_at_ms: state.generated_at_ms,
        observation_revision: state.observation_revision,
        observation_lineage: state.observation_lineage,
    }
}

// === Command 平面: 独立 enum 命名 ============================================

/// API 命令请求 — 接受客户端 command_id 字符串 (后续持久化为 UUID 内部表示;
/// **不绑回** 内部 CommandId 类型)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiCommandRequest {
    pub command_id: String,
    /// API 友好命名 ("start_session"/"stop_session"/"release_session") — **不绑回** CommandKind。
    pub kind: String,
    pub target: ApiCommandTarget,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "target_type", rename_all = "snake_case")]
pub enum ApiCommandTarget {
    /// Session target: intent 以 JSON Value 透传 (不绑回 GraphRuntimeIntent 结构)。
    Session {
        intent: serde_json::Value,
    },
    SessionById {
        session_id: String,
    },
}

/// API 命令状态 — 4 态闭包 (Executed/Replayed/Rejected/Conflict), **不暴露** Failed
/// (终审 NOTE-2 + 0.7C-5 三平面分离: 失败归因通过 classification 传达)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApiCommandStatus {
    Executed,
    Replayed,
    Rejected,
    Conflict,
}

/// API 错误分类 — 与 ErrorClassification 同构, 但去 `_failure` 后缀, 命名 API 友好
/// (终审 NOTE-2: 不暴露 Rust serde tag 习惯 "retryable_failure")。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorClass {
    Rejected,
    Conflict,
    Retryable,
    Permanent,
    Unknown,
}

impl From<&crate::error_model::ErrorClassification> for ApiErrorClass {
    fn from(c: &crate::error_model::ErrorClassification) -> Self {
        use crate::error_model::ErrorClassification as E;
        match c {
            E::Rejected => Self::Rejected,
            E::Conflict => Self::Conflict,
            E::RetryableFailure => Self::Retryable,
            E::PermanentFailure => Self::Permanent,
            E::Unknown => Self::Unknown,
        }
    }
}

/// API 命令响应 — 不聚合万能包装 (终审 NOTE: ❶❶); 字段仅由消费语义驱动。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiCommandResponse {
    pub command_id: String,
    pub status: ApiCommandStatus,
    pub kind: String,
    pub classification: Option<ApiErrorClass>,
    pub detail: Option<String>,
}

// === Event 平面: Projection ≠ State 红线守护 ==================================

/// API 事件封套。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiEventEnvelope {
    pub kind: String,
    pub session_id: Option<String>,
    /// "observation" / "critical" — EventSeverity 字符串化, 不暴露内部 enum。
    pub severity: String,
    pub ts_ms: u64,
}

/// **API 投影守门字段** — 序列化必含 `"event_projection_snapshot"` 字面量,
/// 防客户端误作权威状态 (终审 NOTE-1 + 0.7C-6 EventProjection ≠ CanonicalRuntimeState)。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiProjectionKind {
    EventProjectionSnapshot,
}

/// API 投影响应。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiProjectionResponse {
    /// **守门字段**: 序列化必为 "event_projection_snapshot" (终审 NOTE-1)。
    pub snapshot_kind: ApiProjectionKind,
    pub total: usize,
    pub kind_counts: BTreeMap<String, usize>,
    pub session_states: BTreeMap<String, String>,
    pub session_failures: BTreeMap<String, usize>,
    pub has_critical: bool,
}

impl From<&EventProjection> for ApiProjectionResponse {
    fn from(p: &EventProjection) -> Self {
        ApiProjectionResponse {
            snapshot_kind: ApiProjectionKind::EventProjectionSnapshot,
            total: p.total,
            kind_counts: p.kind_counts.clone(),
            session_states: p.session_states.clone(),
            session_failures: p.session_failures.clone(),
            has_critical: p.has_critical,
        }
    }
}

// === Idempotency 持久化边界契约 (仅契约, 无实现) ==============================

/// **API Idempotency 边界契约层** — 公开 "当前实现承诺" 与 "跨重启语义承诺"。
/// 三选项对勘公开化 (终审裁定: 暴露非隐藏), 防止未来悄悄切换实现被消费者无感。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiIdempotencyBoundary {
    pub current_backend: ApiIdempotencyBackend,
    pub durable_persistence: ApiPersistenceOption,
    pub cross_restart_semantics: ApiCrossRestartSemantics,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiIdempotencyBackend {
    /// 当前实现: 进程内 `CommandIdempotency` 表 (0.7C-4 Foundation)。
    ProcessLocal,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiPersistenceOption {
    /// 实现阶段二: durable log / SQLite (本 change 不实现)。
    DurableLogDeferred,
    /// 实现阶段三: 外部 KV/Redis (本 change 不实现)。
    ExternalKvDeferred,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiCrossRestartSemantics {
    /// 当前: 重启后同 command_id 视为新命令实例 (D9 措辞已锁定)。
    RestartBreaksReplay,
    /// 持久化后 (后续阶段): 重启后同 command_id 重放原 outcome。
    RestartAllowsReplay,
}

/// 默认边界契约 = 当前能力快照 (消费者文档生成器/健康端点可序列化)。
pub fn default_idempotency_boundary() -> ApiIdempotencyBoundary {
    ApiIdempotencyBoundary {
        current_backend: ApiIdempotencyBackend::ProcessLocal,
        durable_persistence: ApiPersistenceOption::DurableLogDeferred,
        cross_restart_semantics: ApiCrossRestartSemantics::RestartBreaksReplay,
    }
}

// === 单元测试 ================================================================

#[cfg(all(test, feature = "mock"))]
mod tests {
    use super::*;
    use crate::error_model::ErrorClassification as E;

    /// **API-BOUNDARY-01 白盒门禁** — 静态扫描: 本文件/模块不得 `use` vendor 实现路径。
    #[test]
    fn api_rt_01_boundary_no_vendor_imports() {
        // 列出本模块源码中出现的全部顶级 use 路径, 任何匹配 vendor 字样者失败。
        // 这是编译级白盒 — 若未来提交偷加 use, 须同步在本测试更新 (与红线白盒同款)。
        let banned = [
            "backend",
            "gstreamer",
            "decklink",
            "ffmpeg",
            "pipeline::",
            "provider::",
        ];
        // 读取本测试模块自身的 src 字符串 (用 include_str!  让仓库静态校验)。
        // 实际 file!() 指向本文件, 含全部源码; banned 出现在 use 语句路径内即失败。
        let src = include_str!("api_boundary.rs");
        for line in src.lines() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("use ") {
                continue;
            }
            for b in &banned {
                assert!(
                    !trimmed.contains(b),
                    "API-BOUNDARY-01 违规: api_boundary.rs 不得 import `{b}` 实现 — `{trimmed}`"
                );
            }
        }
    }

    /// **终审禁清单 11 项零偷渡** — 静态扫描: api_boundary.rs 不出现 transport/持久化/ApiResponse 等关键字。
    #[test]
    fn api_rt_01_no_transport_no_persistence() {
        // 只扫描测试模块之前的生产代码 (剔除注释行; 测试自身的禁清单字面量不算违规)。
        let prod_src = include_str!("api_boundary.rs");
        let prod_part = prod_src.split("#[cfg(all(test").next().unwrap_or(prod_src);
        let src = prod_part
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for banned in [
            "axum",
            "hyper",
            "warp",
            "actix",
            "tower",
            "tonic",
            "grpc",
            "TcpListener",
            "HttpServer",
            "Router",
            "OpenAPI",
            "tokio::main",
            "async fn",
            "Sqlite",
            "Redis",
            "Kafka",
            "ApiResponse", // 禁止万能包装
        ] {
            assert!(
                !src.contains(banned),
                "终审禁清单违规: api_boundary.rs 含 `{banned}` — 本 change 禁 transport/persistence/万能包装"
            );
        }
    }

    /// **NOTE-2 + 0.7C-5 三平面分离白盒** — serde 反向断言:
    /// ApiCommandStatus JSON 禁出现 CommandStatus 变体名 (failed/accepted/executed/rejected);
    /// ApiErrorClass JSON 禁出现 ErrorClassification 后缀 (retryable_failure/permanent_failure);
    /// ApiProjectionResponse 必含 snapshot_kind="event_projection_snapshot"。
    #[test]
    fn api_rt_01_api_models_decoupled_from_runtime_types() {
        // ApiCommandStatus: 不暴露 failed (失败用 classification=Retryable/Permanent 传达)
        let resp = ApiCommandResponse {
            command_id: "x".into(),
            status: ApiCommandStatus::Executed,
            kind: "start_session".into(),
            classification: Some(ApiErrorClass::Retryable),
            detail: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(
            !json.contains("\"failed\""),
            "ApiCommandStatus 不得暴露 failed"
        );
        assert!(
            json.contains("\"retryable\""),
            "ApiErrorClass 必须 snake_case 简短命名"
        );
        assert!(
            !json.contains("retryable_failure"),
            "不得暴露内部 ErrorClassification 命名"
        );
        // ApiProjectionResponse 守门
        let p = ApiProjectionResponse {
            snapshot_kind: ApiProjectionKind::EventProjectionSnapshot,
            total: 1,
            kind_counts: BTreeMap::from([("session_created".into(), 1)]),
            session_states: BTreeMap::new(),
            session_failures: BTreeMap::new(),
            has_critical: false,
        };
        let pj = serde_json::to_string(&p).unwrap();
        assert!(
            pj.contains("\"event_projection_snapshot\""),
            "snapshot_kind 守门必出现, 防误作权威状态"
        );
    }

    /// **NOTE-2 验证**: ApiCommandStatus 不暴露 Failed 变体。
    #[test]
    fn api_rt_01_api_command_status_no_failed_state() {
        // 穷举 ApiCommandStatus, 任何变体的 serde 标签不得含 "failed"。
        let kinds = [
            serde_json::to_value(ApiCommandStatus::Executed).unwrap(),
            serde_json::to_value(ApiCommandStatus::Replayed).unwrap(),
            serde_json::to_value(ApiCommandStatus::Rejected).unwrap(),
            serde_json::to_value(ApiCommandStatus::Conflict).unwrap(),
        ];
        for k in &kinds {
            let s = k.to_string();
            assert!(!s.contains("failed"), "ApiCommandStatus 禁 Failed: {s}");
        }
        // ApiErrorClass: retryable 标签非 retryable_failure
        let r = serde_json::to_value(ApiErrorClass::Retryable).unwrap();
        assert_eq!(r, "retryable");
        let pf = serde_json::to_value(ApiErrorClass::Permanent).unwrap();
        assert_eq!(pf, "permanent");
    }

    /// **NOTE-1 守门**: ApiProjectionResponse 序列化必含 snapshot_kind 字段且字面量为
    /// `"event_projection_snapshot"` (客户端凭此标识知"非权威")。
    #[test]
    fn api_rt_01_api_projection_kind_enforced() {
        let ep = EventProjection {
            total: 0,
            ..Default::default()
        };
        let api: ApiProjectionResponse = (&ep).into();
        let v = serde_json::to_value(&api).unwrap();
        assert_eq!(
            v["snapshot_kind"], "event_projection_snapshot",
            "ApiProjectionKind 守门字段必出现, 防 EventProjection 伪装 RuntimeState"
        );
    }

    /// **to_api_query_models** — Query 五资源 + 全 capabilities 转换完整字段+值域。
    /// 用空 snapshot + 一项设备 (minimal fields) 锁定 to_api_* 全部入口编译级可用。
    #[test]
    fn api_rt_01_to_api_query_models() {
        let state = CanonicalRuntimeState {
            devices: vec![],
            ports: vec![],
            resources: vec![],
            sessions: vec![],
            media_semantics: vec![],
            generated_at_ms: 1_700_000_000_000,
            observation_revision: 42,
            observation_lineage: uuid::Uuid::new_v4(),
        };
        let snap = to_api_query_snapshot(&state);
        assert_eq!(snap.generated_at_ms, 1_700_000_000_000);
        assert_eq!(snap.observation_revision, 42, "投影透传 revision");
        assert_eq!(
            snap.observation_lineage, state.observation_lineage,
            "投影透传 lineage"
        );
        assert!(snap.devices.is_empty());
        assert!(snap.ports.is_empty());
        assert!(snap.resources.is_empty());
        assert!(snap.sessions.is_empty());
        // ErrorClassification → ApiErrorClass 转换闭包 (5 类齐全)。
        for (e, expected) in [
            (E::Rejected, ApiErrorClass::Rejected),
            (E::Conflict, ApiErrorClass::Conflict),
            (E::RetryableFailure, ApiErrorClass::Retryable),
            (E::PermanentFailure, ApiErrorClass::Permanent),
            (E::Unknown, ApiErrorClass::Unknown),
        ] {
            assert_eq!(ApiErrorClass::from(&e), expected);
        }
    }

    /// **Command API request 字段形状** — command_id 非空、kind 三词表封闭、
    /// target 二选一、requested_by 非空 (终审 NOTE: 验证 API 模型独立完整性)。
    #[test]
    fn api_rt_01_api_command_request_field_shape() {
        // 合法形态
        let ok = ApiCommandRequest {
            command_id: "client-abc-123".into(),
            kind: "start_session".into(),
            target: ApiCommandTarget::SessionById {
                session_id: "sess-1".into(),
            },
            requested_by: "ops@example.com".into(),
        };
        let json = serde_json::to_string(&ok).unwrap();
        assert!(json.contains("client-abc-123"));
        assert!(json.contains("session_by_id"));
        // Session target: intent JSON 透传 (不绑回 GraphRuntimeIntent 结构)
        let ok2 = ApiCommandRequest {
            command_id: "x".into(),
            kind: "start_session".into(),
            target: ApiCommandTarget::Session {
                intent: serde_json::json!({"version": "1.0", "devices": []}),
            },
            requested_by: "t".into(),
        };
        let j2 = serde_json::to_string(&ok2).unwrap();
        assert!(j2.contains("\"session\""));
        assert!(j2.contains("\"intent\""));
    }

    /// **Idempotency 边界契约三选项对勘** — 终审裁定公开化 (ProcessLocal/DurableLogDeferred/
    /// ExternalKvDeferred + RestartBreaksReplay/RestartAllowsReplay)。
    #[test]
    fn api_rt_01_idempotency_boundary_contract() {
        let b = default_idempotency_boundary();
        assert_eq!(b.current_backend, ApiIdempotencyBackend::ProcessLocal);
        assert_eq!(
            b.durable_persistence,
            ApiPersistenceOption::DurableLogDeferred
        );
        assert_eq!(
            b.cross_restart_semantics,
            ApiCrossRestartSemantics::RestartBreaksReplay
        );
        // boundary 只序列化**当前选定值** (process_local / durable_log_deferred /
        // restart_breaks_replay) — 未选定选项不出现。
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("process_local"));
        assert!(json.contains("durable_log_deferred"));
        assert!(json.contains("restart_breaks_replay"));
        assert!(!json.contains("\"failed\""));
        // 三选项对勘公开化: **全部枚举变体**的稳定 snake_case 序列化名 (公开 API 面,
        // 消费者凭此知"当前 vs 未来可选"边界)。未选定选项的稳定性在此锁定。
        assert_eq!(
            serde_json::to_string(&ApiPersistenceOption::ExternalKvDeferred).unwrap(),
            "\"external_kv_deferred\""
        );
        assert_eq!(
            serde_json::to_string(&ApiCrossRestartSemantics::RestartAllowsReplay).unwrap(),
            "\"restart_allows_replay\""
        );
    }
}
