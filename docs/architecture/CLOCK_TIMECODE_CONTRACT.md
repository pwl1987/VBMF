# CLOCK_TIMECODE_CONTRACT（时钟 / 时码契约）

> 状态：🔧 待建 → ✅ 已建（Phase 0.7/P1）
> 来源：Portability PRD #57, #147, #148
> 关联：`CANONICAL_MEDIA_MODEL.md`、`RUNTIME_RESOURCE_MODEL.md`

## 1. Clock（#147）
状态：`Locked / Unlocked / Offset / Drift / Clock Lost / Clock Recovered`。
Clock 是运行时**观测**，不写回 Graph（R3 Observation≠Configuration）。

## 2. Timecode（#148）
状态：`Present / Absent / Invalid / Discontinuous / Recovered`。

## 3. 替换不变量
Clock/Timecode 源（BMD / GPIO / NTP）替换，GraphIntent 不变。

## 4. 验收
- #147 Clock Acceptance（Locked/Unlocked/Offset/Drift/Clock Lost/Clock Recovered）
- #148 Timecode Acceptance（Present/Absent/Invalid/Discontinuous/Recovered）
