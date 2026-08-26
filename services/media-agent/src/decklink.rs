//! Gate 6/7 —— 通过 bindgen 生成的 FFI 做真实 DeckLink 设备枚举。
//!
//! 编译进 `bmd` feature 后，`bindings.rs`（`build.rs` 由 `DeckLinkAPI.h` 生成）
//! 提供 COM 接口类型。我们用 libloading 动态加载 `libDeckLinkAPI.so`
//!（Gate 2.5 已验证可达），调用 `CreateDeckLinkIteratorInstance_0004` 拿到
//! `IDeckLinkIterator`，再遍历 `Next()` 读取 `GetModelName` / `GetDisplayName`。
//! bindgen 为这些接口生成的是“不透明 vtable 类型”，因此 vtable 槽位布局
//! 需要按 SDK 头文件手工声明，并必须在 BMD 真机上用真实枚举来验证
//!（槽位错一位会让 `Next`/`GetModelName` 落到错误函数上）。
//!
//! `hardware-test` 额外输出一份详细的设备注册表（型号/序列号/状态）供 BMD 使用。
//!
//! 未编译 `bmd` feature 时，`enumerate()` 返回说明性错误，骨架仍能链接、CI 保持绿。

#![allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals)]

#[cfg(feature = "bmd")]
mod imp {
    use libloading::{Library, Symbol};
    use std::ffi::{CStr, OsStr};
    use std::os::raw::c_char;

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    // bindgen 为这些 COM 接口生成的是不透明 vtable 类型，因此这里按
    // DeckLinkAPI.h / DeckLinkAPIDiscovery.h 手工声明槽位布局。槽位顺序必须
    // 与 SDK 头文件一致；由 BMD 真机枚举验证（错一位会让 Next/GetModelName
    // 落到错误函数上，导致崩溃或返回垃圾数据）。
    #[repr(C)]
    struct IDeckLinkIteratorVtbl {
        QueryInterface: Option<unsafe extern "C" fn(*mut IDeckLinkIterator, REFIID, *mut LPVOID) -> HRESULT>,
        AddRef: Option<unsafe extern "C" fn(*mut IDeckLinkIterator) -> ULONG>,
        Release: Option<unsafe extern "C" fn(*mut IDeckLinkIterator) -> ULONG>,
        Next: Option<unsafe extern "C" fn(*mut IDeckLinkIterator, *mut *mut IDeckLink) -> HRESULT>,
    }

    #[repr(C)]
    struct IDeckLinkVtbl {
        QueryInterface: Option<unsafe extern "C" fn(*mut IDeckLink, REFIID, *mut LPVOID) -> HRESULT>,
        AddRef: Option<unsafe extern "C" fn(*mut IDeckLink) -> ULONG>,
        Release: Option<unsafe extern "C" fn(*mut IDeckLink) -> ULONG>,
        GetModelName: Option<unsafe extern "C" fn(*mut IDeckLink, *mut *mut c_char) -> HRESULT>,
        GetDisplayName: Option<unsafe extern "C" fn(*mut IDeckLink, *mut *mut c_char) -> HRESULT>,
    }

    // HRESULT 的 S_OK
    const S_OK: i32 = 0;

    type CreateIter = unsafe extern "C" fn(*mut *mut IDeckLinkIterator) -> i32;

    /// 遍历 IDeckLinkIterator::Next，按顺序返回 (型号, 显示名, 序列号) 三元组。
    /// 序列号在接入 IDeckLinkConfiguration::GetString 之前先记为 "n/a"。
    fn iter_devices() -> Result<Vec<(String, String, String)>, String> {
        let lib = unsafe { Library::new(OsStr::new("libDeckLinkAPI.so")) }
            .map_err(|e| format!("加载 libDeckLinkAPI.so 失败: {e}"))?;

        let create: Symbol<CreateIter> = unsafe {
            lib.get(b"CreateDeckLinkIteratorInstance_0004\0")
                .map_err(|e| format!("未找到符号 CreateDeckLinkIteratorInstance_0004: {e}"))?
        };

        let mut iter: *mut IDeckLinkIterator = std::ptr::null_mut();
        let hr = unsafe { create(&mut iter) };
        if hr != S_OK || iter.is_null() {
            return Err(format!("CreateDeckLinkIteratorInstance_0004 失败 hr={hr}"));
        }

        // 通过 ABI 读取 COM vtable 指针：它是对象第一个指针大小字段，
        // 与 bindgen 生成的具体字段名无关。
        let vtbl = unsafe { *(iter as *mut *mut IDeckLinkIteratorVtbl) };
        let next = unsafe { (*vtbl).Next }.ok_or("vtable 中缺少 IDeckLinkIterator::Next")?;
        let release_iter =
            unsafe { (*vtbl).Release }.ok_or("vtable 中缺少 IDeckLinkIterator::Release")?;

        let mut out = Vec::new();
        loop {
            let mut decklink: *mut IDeckLink = std::ptr::null_mut();
            let hr = unsafe { next(iter, &mut decklink) };
            if hr != S_OK || decklink.is_null() {
                break;
            }
            let dv = unsafe { *(decklink as *mut *mut IDeckLinkVtbl) };
            let get_model =
                unsafe { (*dv).GetModelName }.ok_or("vtable 中缺少 IDeckLink::GetModelName")?;
            let get_display = unsafe { (*dv).GetDisplayName }
                .ok_or("vtable 中缺少 IDeckLink::GetDisplayName")?;
            let release_dev =
                unsafe { (*dv).Release }.ok_or("vtable 中缺少 IDeckLink::Release")?;

            let mut model_ptr: *mut c_char = std::ptr::null_mut();
            let mut display_ptr: *mut c_char = std::ptr::null_mut();
            unsafe {
                let _ = get_model(decklink, &mut model_ptr);
                let _ = get_display(decklink, &mut display_ptr);
            }
            let model = unsafe { read_cstr(model_ptr) };
            let display = unsafe { read_cstr(display_ptr) };
            // 序列号需要 IDeckLinkConfiguration::GetString(
            //   bmdDeckLinkConfigDeviceInformationSerialNumber)；
            // 该接口的 vtable 槽位顺序尚未推导/验证，故暂省略。
            let serial = String::from("n/a");
            out.push((model, display, serial));

            unsafe {
                let _ = release_dev(decklink);
            }
        }

        unsafe {
            let _ = release_iter(iter);
        }
        Ok(out)
    }

    /// 读取 DeckLink 返回的 `char*`（可空）。SDK 用自有字符串分配器分配；
    /// 生产环境应调用 `ReleaseString` 释放。枚举阶段只读取（一次性启动期
    /// 轻微泄漏可接受）。
    unsafe fn read_cstr(p: *mut c_char) -> String {
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p as *const c_char)
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn enumerate() -> Result<Vec<(String, String, String)>, String> {
        iter_devices()
    }

    /// 生成人类可读的 “DeckLink 设备注册表” 表格（Gate 7 目标输出）。
    pub fn registry() -> Result<String, String> {
        let devs = iter_devices()?;
        let mut s = String::from("DeckLink 设备注册表\n────────────────────────\n");
        for (i, (model, display, serial)) in devs.iter().enumerate() {
            s.push_str(&format!(
                "ID: {i}\n型号: {model}\n显示名: {display}\n序列号: {serial}\n状态: 可用\n\n"
            ));
        }
        Ok(s)
    }
}

#[cfg(not(feature = "bmd"))]
mod imp {
    pub fn enumerate() -> Result<Vec<(String, String, String)>, String> {
        Err(
            "未编译 bmd feature —— 请使用 `--features bmd` 并设 \
             DECKLINK_SDK_INCLUDE=<SDK 的 Linux/include 路径>（需要 libclang）重新构建"
                .into(),
        )
    }
}

#[cfg(feature = "bmd")]
pub use imp::{enumerate, registry};
#[cfg(not(feature = "bmd"))]
pub use imp::enumerate;
