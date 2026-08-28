# EXTERNAL_API_CONTRACT（外部 API 契约）

> 状态：🔧 待建 → ✅ 已建（Phase 0.7/P1）
> 来源：API PRD #1–#85, #108–#134, #151–#162；澄清 (G)(F)
> 关联：`EVENT_CONTRACT.md`、`DEVICE_INTEGRATION_CONTRACT.md`、`VENDOR_NEUTRALITY_RULES.md`

## 1. API 三平面（对齐 #150 / Portability #98/#99）
- **Product (External) API**：`/api/v1/*`，业务消费方
- **Diagnostics API**：工程排查，独立命名空间
- **Internal (Agent) API**：`/internal/v1/*`，Runtime Control，不与前两者混

## 2. Endpoint 规范
- 资源路径：`/devices` `/ports` `/sessions` `/routing` `/capabilities`（**禁** `/bmd/*` `/gstreamer/*`，#85）
- 厂商扩展隔离：`extensions.{blackmagic}` 不得污染 Canonical Schema（#86/#137）
- Versioning：`/api/v1/`，非破坏字段增加不影响旧客户端（#126）

## 3. Idempotency（#28–#34）
- Command 带 `command_id` / `Idempotency-Key`，重复提交返回首次结果
- 幂等方法：discover / bind / reserve / lease / start / stop / recover

## 4. Pagination / Filter / Sort / Concurrency（#117–#121）
- 可能增长集合（devices/ports/sessions/events/commands/audit/integrations）必须分页
- 过滤字段保持 Canonical；稳定排序（updated_at/created_at/id）
- 乐观并发：`If-Match: "revision-N"`，冲突返回 409（#120/#121）

## 5. Error Model（#122–#125）
统一结构：`{ "error": { "code", "message", "details", "request_id", "retryable" } }`
Categories：AUTHENTICATION_FAILED / AUTHORIZATION_DENIED / VALIDATION_ERROR / RESOURCE_NOT_FOUND / RESOURCE_CONFLICT / RESOURCE_UNAVAILABLE / CAPABILITY_UNSUPPORTED / COMMAND_REJECTED / COMMAND_FAILED / DEPENDENCY_UNAVAILABLE / RATE_LIMITED / INTERNAL_ERROR
每个 error 必须明确 `retryable`；需时 `Retry-After`。

## 6. Security（#108–#111, #152）
- 默认 authenticated / authorized / audited / rate-limited
- Nginx 只做 TLS/routing/rate-limit/IP filter，最终 authz 在应用层（#152），不直接连设备（#153）
- Secret 只存 Reference，不写 Manifest/Graph/Evidence/Git（#111）
- Device/Infra Adapter 可用 mTLS（#110）

## 7. Observability / Health（#113–#116）
- metrics：requests_total / latency / command_success / command_failed / webhook_success / webhook_failed / event_delivery_lag / external_device_state
- Health 分层：API / Integration / Runtime / Dependency，不合并单 status
- liveness / readiness / dependency health 区分

## 8. 大文件（#160–#162）
- 不走 JSON API 传视频；用 presigned URL + object storage（RustFS/S3）
- resumable / chunk / checksum / resume

## 9. Acceptance
- `EXT-API-01`（Query/Command/Event/Auth/Authz/Audit/Idempotency/Versioning）
- `ARCH-API-BOUNDARY-01`（External→Control→Runtime Contract→Rust，禁 External→Vendor SDK）
- Vendor Neutrality Gate（#136，引用 `VENDOR_NEUTRALITY_RULES.md`）
