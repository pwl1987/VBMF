//! DeckLink SDK FFI smoke — Gate 2.5 (path A) prerequisite probe.
//!
//! 目的(仅验证, 非生产集成): 确认 `libDeckLinkAPI.so` 在目标运行环境
//! (BMD 宿主机 / Option B runc 容器) 可被动态加载, 且 SDK 入口符号可见。
//! 这是 Gate 6/7 (GStreamer pipeline / 信号) 的前置条件。
//!
//! 实测事实 (BMD 2026-08-26): `libDeckLinkAPI.so` 不导出纯 C 的
//! `GetDeckLinkAPIVersion`; 它导出 C++ COM 风格工厂函数, 例如
//! `CreateDeckLinkAPIInformationInstance_0001` / `CreateDeckLinkIteratorInstance_0004`
//! (见 `nm -D`)。故本 probe 校验真实存在的入口符号, 验证"SDK 运行时可达"。
//! 真正的版本/设备枚举经 C++ vtable, 放到 Gate 6/7 用 bindgen 安全包装实现。
//!
//! 设计取舍: 运行时 dlopen (libloading), 而非编译期 link —— CI (无 DeckLink)
//! 仍能编译; 运行时缺失 SDK 只 warn, 不 panic。
#![allow(dead_code)] // Gate 2.5: probe 仅 main 调用, 其余预留。

use std::ffi::OsStr;

/// 工厂函数签名: C 链接, 返回 COM 接口指针 (void*)。这里只需符号存在性。
type CreateInstance = unsafe extern "C" fn() -> *mut std::ffi::c_void;

/// 校验 DeckLink SDK 运行时可达且入口符号存在。
/// Ok(()) = 库可加载 + 入口符号可见 (Gate 6/7 可在此基础上集成)。
/// Err(reason) = 缺失 (容器内未 bind-mount 库时预期发生)。
pub fn probe_sdk(lib_name: &str) -> Result<(), String> {
    let lib = unsafe { libloading::Library::new(OsStr::new(lib_name)) }
        .map_err(|e| format!("load '{lib_name}' failed: {e}"))?;

    // 入口符号: SDK 信息工厂 (存在即证明 SDK 运行时与 ABI 可见)。
    let _info: libloading::Symbol<CreateInstance> = unsafe {
        lib.get(b"CreateDeckLinkAPIInformationInstance_0001\0")
            .map_err(|e| {
                format!("entry symbol CreateDeckLinkAPIInformationInstance_0001 not found: {e}")
            })?
    };
    // 同类入口, 一并确认迭代器工厂 (Gate 2.5 深度枚举将用它)。
    let _iter: libloading::Symbol<CreateInstance> = unsafe {
        lib.get(b"CreateDeckLinkIteratorInstance_0004\0")
            .map_err(|e| {
                format!("entry symbol CreateDeckLinkIteratorInstance_0004 not found: {e}")
            })?
    };

    Ok(())
}
