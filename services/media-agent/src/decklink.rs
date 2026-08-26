//! Gate 6/7 —— 通过 bindgen 生成的 FFI 做真实 DeckLink 设备枚举。
//!
//! 编译进 `bmd` feature 后，`bindings.rs`（`build.rs` 由 `DeckLinkAPI.h` 生成）
//! 提供 COM 接口类型。我们用 libloading 动态加载 `libDeckLinkAPI.so`
//!（Gate 2.5 已验证可达），调用 `CreateDeckLinkIteratorInstance_0004` 拿到
//! `IDeckLinkIterator`，再遍历 `Next()` 读取 `GetModelName` / `GetDisplayName`。
//! 序列号经 `IDeckLink::QueryInterface(IID_IDeckLinkConfiguration)` 取配置接口后,
//! 调 `IDeckLinkConfiguration::GetString(@10)` 读 `bmdDeckLinkConfigDeviceInformationSerialNumber`(=1684632430)。
//! bindgen 为这些接口生成的是“不透明 vtable 类型”，因此 vtable 槽位布局
//! 需要按 SDK 头文件手工声明，并必须在 BMD 真机上用真实枚举来验证
//!（槽位错一位会让 `Next`/`GetModelName`/`GetString` 落到错误函数上）。
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
        // 注意: QueryInterface 的 riid 在真实 libDeckLinkAPI.so ABI 里是
        // `const IID&` = 指针(8 字节), 不是 bindgen 可能生成的按值 16 字节。
        // 这里显式用 `*const c_void` 保证按指针 ABI 传参。
        QueryInterface: Option<unsafe extern "C" fn(*mut IDeckLink, *const std::ffi::c_void, *mut LPVOID) -> HRESULT>,
        AddRef: Option<unsafe extern "C" fn(*mut IDeckLink) -> ULONG>,
        Release: Option<unsafe extern "C" fn(*mut IDeckLink) -> ULONG>,
        GetModelName: Option<unsafe extern "C" fn(*mut IDeckLink, *mut *mut c_char) -> HRESULT>,
        GetDisplayName: Option<unsafe extern "C" fn(*mut IDeckLink, *mut *mut c_char) -> HRESULT>,
    }

    // IDeckLinkConfiguration vtable。方法顺序按 SDK 16.0 头文件(2.5.15)推导,
    // 已由 https://sdk-doc.blackmagicdesign.com/decklink-sdk/ 核对:
    // SetFlag@3 / GetFlag@4 / SetInt@5 / GetInt@6 / SetFloat@7 / GetFloat@8 /
    // SetString@9 / GetString@10 / ... / WriteConfigurationToPreferences@17。
    // 我们只调用 GetString@10(读序列号)与 Release@2(释放配置接口)。
    // 槽位均为 8 字节函数指针, 占位槽用 `Option<unsafe extern "C" fn()>` 不影响布局。
    #[repr(C)]
    struct IDeckLinkConfigurationVtbl {
        QueryInterface: Option<unsafe extern "C" fn()>,
        AddRef: Option<unsafe extern "C" fn()>,
        Release: Option<unsafe extern "C" fn(*mut IDeckLinkConfiguration) -> ULONG>,
        SetFlag: Option<unsafe extern "C" fn()>,
        GetFlag: Option<unsafe extern "C" fn()>,
        SetInt: Option<unsafe extern "C" fn()>,
        GetInt: Option<unsafe extern "C" fn()>,
        SetFloat: Option<unsafe extern "C" fn()>,
        GetFloat: Option<unsafe extern "C" fn()>,
        SetString: Option<unsafe extern "C" fn()>,
        GetString: Option<unsafe extern "C" fn(*mut IDeckLinkConfiguration, u32, *mut *mut c_char) -> HRESULT>,
    }

    // IID_IDeckLinkConfiguration 的候选 GUID 列表。
    // 注意: SDK 头文件只 `extern` 声明该 IID, 其定义在 libDeckLinkAPI.so 内部
    // (带内部链接符号 _ZL26IID_IDeckLinkConfiguration, 不进 dynsym, 真机确认 .so 为
    // stripped 且无 dynsym 导出), 既无法在构建期链接, 也不能运行时 dlsym。因此这里
    // 自包含硬编码。
    // ** 该 IID 随 SDK 主版本变化, 且 BMD 存在版本化接口 (真机 .so RTTI 含
    //    IDeckLinkConfiguration_v10_11), 不同设备上 base / v10_11 返回行为不同
    //    (base 曾实测 E_NOTIMPL=0x80000004, v10_11 才是 .so 实际注册的接口)。**
    //    故 iter_devices 遍历下列候选, 取第一个 QueryInterface 成功者。
    // 来源:
    //   [0] 912F634B-2D4E-40A4-8AAB-8D80B73F1289 = Desktop Video 16.0 SDK (2026-03) 官方文档权威 base 值
    //   [1] EF90380B-4AE5-4346-9077-E288E149F129 = 同代头 IDeckLinkConfiguration_v10_11 版本化 IID
    //   [2] 5A68FFD4-1C12-4EDE-A6D2-45451D385FC1 = 真机旧头误用值(诊断用, 应失效)
    //   [3] CB71734A-FE37-4E8D-8E13-802133A1C3F2 = SDK 10.6.6 头值(诊断用, 应失效)
    // 若升级 SDK 大版本, 需用对应版本官方头重新核对候选序。
    #[repr(C)]
    struct Guid16 {
        b: [u8; 16],
    }
    static CANDIDATE_CONFIG_IIDS: &[Guid16] = &[
        Guid16 { b: [0x91, 0x2F, 0x63, 0x4B, 0x2D, 0x4E, 0x40, 0xA4, 0x8A, 0xAB, 0x8D, 0x80, 0xB7, 0x3F, 0x12, 0x89] },
        Guid16 { b: [0xEF, 0x90, 0x38, 0x0B, 0x4A, 0xE5, 0x43, 0x46, 0x90, 0x77, 0xE2, 0x88, 0xE1, 0x49, 0xF1, 0x29] },
        Guid16 { b: [0x5A, 0x68, 0xFF, 0xD4, 0x1C, 0x12, 0x4E, 0xDE, 0xA6, 0xD2, 0x45, 0x45, 0x1D, 0x38, 0x5F, 0xC1] },
        Guid16 { b: [0xCB, 0x71, 0x73, 0x4A, 0xFE, 0x37, 0x4E, 0x8D, 0x8E, 0x13, 0x80, 0x21, 0x33, 0xA1, 0xC3, 0xF2] },
    ];

    // HRESULT 的 S_OK
    const S_OK: i32 = 0;

    // 注意: CreateDeckLinkIteratorInstance_0004 实际直接返回 `IDeckLinkIterator*`
    // 指针（不是 HRESULT + 出参）。真机实测(2026-08-26)确认: 若按出参解读,
    // 会把返回的指针误当成 hr, 而出参 iter 恒为 null。
    type CreateIter = unsafe extern "C" fn() -> *mut IDeckLinkIterator;

    /// 遍历 IDeckLinkIterator::Next，按顺序返回 (型号, 显示名, 序列号) 三元组。
    /// 序列号经 IDeckLinkConfiguration::GetString 读取；若 QueryInterface/GetString
    /// 失败则回退为 "n/a"。
    fn iter_devices() -> Result<Vec<(String, String, String)>, String> {
        let lib = unsafe { Library::new(OsStr::new("libDeckLinkAPI.so")) }
            .map_err(|e| format!("加载 libDeckLinkAPI.so 失败: {e}"))?;

        let create: Symbol<CreateIter> = unsafe {
            lib.get(b"CreateDeckLinkIteratorInstance_0004\0")
                .map_err(|e| format!("未找到符号 CreateDeckLinkIteratorInstance_0004: {e}"))?
        };

        // 该函数直接返回迭代器指针（NULL 表示无设备或初始化失败）。
        let iter: *mut IDeckLinkIterator = unsafe { create() };
        if iter.is_null() {
            return Err(
                "CreateDeckLinkIteratorInstance_0004 返回空指针（未检测到 DeckLink 设备？）".into(),
            );
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
            let query_iface = unsafe { (*dv).QueryInterface }
                .ok_or("vtable 中缺少 IDeckLink::QueryInterface")?;
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

            // 序列号: 经 IDeckLink::QueryInterface(IID_IDeckLinkConfiguration)
            // 取配置接口, 再调 IDeckLinkConfiguration::GetString(@10) 读
            // bmdDeckLinkConfigDeviceInformationSerialNumber (= 1684632430)。
            // 注意: BMD 的 IDeckLinkConfiguration 接口随 SDK 演进有多个版本化 IID
            // (真机 .so RTTI 含 _v10_11)。base IID 在某些设备上会返回 E_NOTIMPL(0x80000004),
            // 故这里遍历候选 IID, 取第一个 QueryInterface 成功者; 并逐条打印 HRESULT
            // 供真机定位。候选顺序: 16.0 base / v10_11 / 旧头误用值 / 10.6.6。
            let serial = {
                let mut chosen: String = String::from("n/a");
                let mut last_hr: i32 = 0;
                for (idx, iid) in CANDIDATE_CONFIG_IIDS.iter().enumerate() {
                    let mut cfg: *mut IDeckLinkConfiguration = std::ptr::null_mut();
                    let cfg_ptr: *mut *mut IDeckLinkConfiguration = &mut cfg;
                    let hr_cfg = unsafe {
                        query_iface(
                            decklink,
                            iid as *const _ as *const std::ffi::c_void,
                            cfg_ptr as *mut LPVOID,
                        )
                    };
                    last_hr = hr_cfg;
                    eprintln!(
                        "dv serial probe: candidate[{idx}] hr=0x{hr_cfg:08X} cfg_null={}",
                        cfg.is_null()
                    );
                    if hr_cfg == S_OK && !cfg.is_null() {
                        let cv = unsafe { *(cfg as *mut *mut IDeckLinkConfigurationVtbl) };
                        let get_string = unsafe { (*cv).GetString }
                            .ok_or("vtable 中缺少 IDeckLinkConfiguration::GetString")?;
                        let release_cfg = unsafe { (*cv).Release }
                            .ok_or("vtable 中缺少 IDeckLinkConfiguration::Release")?;
                        let mut serial_ptr: *mut c_char = std::ptr::null_mut();
                        unsafe { let _ = get_string(cfg, 1684632430u32, &mut serial_ptr); }
                        let s = unsafe { read_cstr(serial_ptr) };
                        unsafe { let _ = release_cfg(cfg); }
                        chosen = s;
                        break;
                    }
                }
                if chosen == "n/a" {
                    format!("n/a(hr=0x{last_hr:08X})")
                } else {
                    chosen
                }
            };
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
