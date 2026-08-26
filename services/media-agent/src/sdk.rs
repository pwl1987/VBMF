//! DeckLink SDK FFI smoke — Gate 2.5 (path A) prerequisite probe.
//!
//! 目的(仅验证, 非生产集成): 确认 `libDeckLinkAPI.so` 在目标运行环境
//! (BMD 宿主机 / Option B runc 容器) 可被动态加载, 且关键 C 导出符号
//! `GetDeckLinkAPIVersion` 可用。这是 Gate 6/7 (GStreamer pipeline / 信号)
//! 的前置条件 —— 没有可达的 SDK 运行时, 深度枚举/采集无从谈起。
//!
//! 设计取舍: 用 `libloading` 运行时 dlopen, 而非编译期 link。
//! 好处: CI (无 DeckLink) 仍能编译; 运行时缺失 SDK 只 warn 不 panic。
//! 真正的 C++ 接口 (IDeckLink / IDeckLinkInput) 集成放到 Gate 6/7。
#![allow(dead_code)] // Gate 2.5: probe 仅 main 调用, 其余接口预留。

use std::ffi::CString;

type GetDeckLinkAPIVersion = unsafe extern "C" fn(version: *mut u32) -> i32;

/// 加载 DeckLink SDK 运行时并读取版本。返回 Ok(version) 或 Err(reason)。
///
/// `version` 编码见 DeckLinkAPI.h: 高 16 位主版本, 低 16 位次版本
/// (e.g. 0x000A0000 = 10.0)。实际 SDK 16.0 为 0x00100000。
pub fn probe_sdk_version(lib_name: &str) -> Result<u32, String> {
    let cname = CString::new(lib_name).map_err(|e| format!("bad lib name: {e}"))?;
    let lib = unsafe { libloading::Library::new(cname) }
        .map_err(|e| format!("load '{lib_name}' failed: {e}"))?;
    let sym: libloading::Symbol<GetDeckLinkAPIVersion> = unsafe {
        lib.get(b"GetDeckLinkAPIVersion\0")
            .map_err(|e| format!("symbol GetDeckLinkAPIVersion not found: {e}"))?
    };
    let mut version: u32 = 0;
    let rc = unsafe { sym(&mut version) };
    if rc == 0 {
        Ok(version)
    } else {
        Err(format!("GetDeckLinkAPIVersion returned {rc}"))
    }
}

/// 把 SDK 编码版本拆成 (major, minor) 便于日志。
pub fn decode_version(v: u32) -> (u16, u16) {
    ((v >> 16) as u16, (v & 0xffff) as u16)
}
