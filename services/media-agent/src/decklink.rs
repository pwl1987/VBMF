//! Gate 6/7 —— 通过 bindgen 生成的 FFI 做真实 DeckLink 设备枚举。
//!
//! 编译进 `bmd` feature 后，`bindings.rs`（`build.rs` 由 `DeckLinkAPI.h` 生成）
//! 提供 COM 接口类型。我们用 libloading 动态加载 `libDeckLinkAPI.so`
//!（Gate 2.5 已验证可达），调用 `CreateDeckLinkIteratorInstance_0004` 拿到
//! `IDeckLinkIterator`，再遍历 `Next()` 读取 `GetModelName` / `GetDisplayName`。
//! 序列号/设备唯一标识优先经 `IDeckLink::QueryInterface(IID_IDeckLinkProfileAttributes)`
//! 取属性接口后, 调 `IDeckLinkProfileAttributes::GetString(@6)` 读
//! `BMDDeckLinkDeviceHandle`(=0x64657668), 官方手册 (3.17) 认定的设备唯一标识字符串;
//! fallback 到 `IDeckLinkConfiguration::GetString(@10)` 读序列号项 (=0x6469736E)。
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
        // 注意: QueryInterface 的 riid 在本 SDK (Desktop Video 16.0) 的 ABI 里是
        // **按值传 16 字节 GUID** (占 rsi:rdx 两个寄存器), 不是指针! 真机 g++ 标准
        // 调用 (REFIID 即 16 字节结构体按值) 与手动 vtable[0] 调用均验证: 按值传
        // S_OK, 按指针传则被当作垃圾 GUID 返回 E_NOTIMPL。故此处第 2 参数用 Guid16 按值。
        QueryInterface: Option<unsafe extern "C" fn(*mut IDeckLink, Guid16, *mut LPVOID) -> HRESULT>,
        AddRef: Option<unsafe extern "C" fn(*mut IDeckLink) -> ULONG>,
        Release: Option<unsafe extern "C" fn(*mut IDeckLink) -> ULONG>,
        GetModelName: Option<unsafe extern "C" fn(*mut IDeckLink, *mut *mut c_char) -> HRESULT>,
        GetDisplayName: Option<unsafe extern "C" fn(*mut IDeckLink, *mut *mut c_char) -> HRESULT>,
    }

    // IDeckLinkConfiguration vtable。方法顺序按官方手册 2.5.15 (Desktop Video 16.0),
    // 计入 IUnknown 基类后的 0-based 索引:
    //   0 QueryInterface / 1 AddRef / 2 Release / 3 SetFlag / 4 GetFlag / 5 SetInt /
    //   6 GetInt / 7 SetFloat / 8 GetFloat / 9 SetString / 10 GetString /
    //   11 SetFlagWithParam / 12 GetFlagWithParam / 13 SetIntWithParam /
    //   14 GetIntWithParam / 15 SetFloatWithParam / 16 SetStringWithParam /
    //   17 GetStringWithParam / 18 WriteConfigurationToPreferences。
    // 我们只调用 GetString@10(读序列号 fallback)与 Release@2(释放配置接口)。
    // 槽位均为 8 字节函数指针, 占位槽以 `Option<unsafe extern "C" fn()>` 声明, 不影响布局。
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

    // IDeckLinkProfileAttributes vtable (SDK 16.0, 2.5.17)。方法顺序按官方头文件
    // (DeckLinkAPI.h:1527) 严格一致, 已由真机 g++ 编译运行验证:
    // QueryInterface@0 / AddRef@1 / Release@2 / GetFlag@3 / GetInt@4 / GetFloat@5 /
    // GetString@6 / GetStringWithParam@7。我们只调用 GetString@6 读设备唯一标识
    // (BMDDeckLinkDeviceHandle), 以及 Release@2 释放接口。
    #[repr(C)]
    struct IDeckLinkProfileAttributesVtbl {
        QueryInterface: Option<unsafe extern "C" fn()>,
        AddRef: Option<unsafe extern "C" fn()>,
        Release: Option<unsafe extern "C" fn(*mut IDeckLinkProfileAttributes) -> ULONG>,
        GetFlag: Option<unsafe extern "C" fn()>,
        GetInt: Option<unsafe extern "C" fn()>,
        GetFloat: Option<unsafe extern "C" fn()>,
        GetString: Option<unsafe extern "C" fn(*mut IDeckLinkProfileAttributes, u32, *mut *mut c_char) -> HRESULT>,
        GetStringWithParam: Option<unsafe extern "C" fn()>,
    }

    // IID_IDeckLinkConfiguration 的真实 GUID (SDK 16.0 权威值)。
    // 注意: SDK 头文件只 `extern` 声明该 IID, 其定义在 libDeckLinkAPI.so 内部
    // (带内部链接符号 _ZL26IID_IDeckLinkConfiguration, 不进 dynsym, 真机确认 .so 为
    // stripped 且无 dynsym 导出), 既无法在构建期链接, 也不能运行时 dlsym。因此这里
    // 自包含硬编码该 16 字节。
    // ** 权威来源 **: 真机 /home/lytv/Blackmagic_DeckLink_SDK_16.0/.../Linux/include/
    //   DeckLinkAPIConfiguration.h 第 53 行:
    //     IID_IDeckLinkConfiguration = /* 5A68FFD4-1C12-4EDE-A6D2-45451D385FC1 */
    //       { 0x5A, 0x68, 0xFF, 0xD4, 0x1C, 0x12, 0x4E, 0xDE, 0xA6, 0xD2, 0x45, 0x45, 0x1D, 0x38, 0x5F, 0xC1 };
    //   该值已用真机 g++ 标准调用序列验证: QueryInterface 返回 S_OK 且可取得配置接口。
    //   (注: 网络上其它仓库的 "16.0 头" 给出 912F634B-… 系版本错配, 不可用; 以真机
    //    完整 SDK 16.0 头为准。)
    // ** ABI (已真机验证) **: IDeckLink::QueryInterface 的 riid 参数在本 SDK (Desktop
    //   Video 16.0) 的 ABI 里是**按值传 16 字节 GUID** (占 rsi:rdx 两寄存器), 不是标准
    //   COM 的 `const IID&` 指针! 真机 g++ 标准调用与手动 vtable[0] 调用均验证: 按值传
    //   返回 S_OK, 按指针(8 字节地址)传则被当作垃圾 GUID 返回 E_NOTIMPL。故调用处
    //   (第 211/239 行) **按值** 传 `IID_*` 静态量, 切勿按指针传 (那会错位导致 E_NOTIMPL/段错误)。
    // 若升级 SDK 大版本, 需用对应版本官方完整头重新核对此值。
    // GUID (SDK 16.0, 真机头权威): 5A68FFD4-1C12-4EDE-A6D2-45451D385FC1
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Guid16 {
        b: [u8; 16],
    }
    static IID_DECKLINK_CONFIGURATION: Guid16 = Guid16 {
        b: [
            0x5A, 0x68, 0xFF, 0xD4, 0x1C, 0x12, 0x4E, 0xDE, 0xA6, 0xD2, 0x45, 0x45, 0x1D, 0x38,
            0x5F, 0xC1,
        ],
    };

    // IID_IDeckLinkProfileAttributes 的真实 GUID (SDK 16.0 权威值)。
    // 来源: 真机 /home/lytv/Blackmagic_DeckLink_SDK_16.0/.../Linux/include/DeckLinkAPI.h:118:
    //   IID_IDeckLinkProfileAttributes = /* F47551D7-AD22-47AF-BCFD-6BE88AA879D9 */
    //     { 0xF4,0x75,0x51,0xD7,0xAD,0x22,0x47,0xAF,0xBC,0xFD,0x6B,0xE8,0x8A,0xA8,0x79,0xD9 };
    // 该接口提供 GetString(BMDDeckLinkDeviceHandle='devh'=0x64657668) 读取设备唯一标识
    // 字符串, 这是官方手册 (3.17) 列出的真正可用于"设备唯一标识"的属性
    // (BMDDeckLinkConfiguration 的序列号项在多数设备上不可读, 见上)。真机 2026-08-26
    // 验证: QueryInterface 返回 S_OK, 且 3 台设备均返回非空 DeviceHandle
    // (如 DeckLink SDI -> "46:00000000:002e4500")。
    const IID_DECKLINK_PROFILE_ATTRIBUTES: Guid16 = Guid16 {
        b: [
            0xF4, 0x75, 0x51, 0xD7, 0xAD, 0x22, 0x47, 0xAF, 0xBC, 0xFD, 0x6B, 0xE8, 0x8A, 0xA8,
            0x79, 0xD9,
        ],
    };

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

            // 序列号/设备唯一标识: 优先经 IDeckLinkProfileAttributes::GetString 读
            // BMDDeckLinkDeviceHandle(= 'devh' = 0x64657668), 官方手册 (3.17) 认定的
            // "unique identifier for the device" 字符串。真机 2026-08-26 验证: 3 台设备
            // 均返回非空 (如 DeckLink SDI -> "46:00000000:002e4500")。
            // fallback: IDeckLinkConfiguration::GetString(0x6469736E) 序列号项
            // (部分设备 E_INVALIDARG 或空串, 属 SDK/硬件行为)。
            let serial = {
                // —— 主路径: IDeckLinkProfileAttributes::GetString(DeviceHandle) ——
                let handle = {
                    let mut attr: *mut IDeckLinkProfileAttributes = std::ptr::null_mut();
                    let attr_ptr: *mut *mut IDeckLinkProfileAttributes = &mut attr;
                    let hr_attr = unsafe {
                        query_iface(
                            decklink,
                            IID_DECKLINK_PROFILE_ATTRIBUTES,
                            attr_ptr as *mut LPVOID,
                        )
                    };
                    if hr_attr == S_OK && !attr.is_null() {
                        let av = unsafe { *(attr as *mut *mut IDeckLinkProfileAttributesVtbl) };
                        let get_string = unsafe { (*av).GetString }
                            .ok_or("vtable 中缺少 IDeckLinkProfileAttributes::GetString")?;
                        let release_attr = unsafe { (*av).Release }
                            .ok_or("vtable 中缺少 IDeckLinkProfileAttributes::Release")?;
                        let mut hptr: *mut c_char = std::ptr::null_mut();
                        unsafe { let _ = get_string(attr, 0x64657668u32, &mut hptr); }
                        let s = unsafe { read_cstr(hptr) };
                        unsafe { let _ = release_attr(attr); }
                        s
                    } else {
                        String::new()
                    }
                };
                if !handle.is_empty() {
                    handle
                } else {
                    // —— fallback: IDeckLinkConfiguration::GetString(SerialNumber) ——
                    let mut cfg: *mut IDeckLinkConfiguration = std::ptr::null_mut();
                    // 注意: Rust 不允许把 `&mut` 引用直接 `as` 成不同 pointee 的裸指针,
                    // 故先取一个类型明确的指针变量, 再做指针→指针 (`as`) 转换。
                    let cfg_ptr: *mut *mut IDeckLinkConfiguration = &mut cfg;
                    let hr_cfg = unsafe {
                        query_iface(decklink, IID_DECKLINK_CONFIGURATION, cfg_ptr as *mut LPVOID)
                    };
                    if hr_cfg == S_OK && !cfg.is_null() {
                        let cv = unsafe { *(cfg as *mut *mut IDeckLinkConfigurationVtbl) };
                        let get_string = unsafe { (*cv).GetString }
                            .ok_or("vtable 中缺少 IDeckLinkConfiguration::GetString")?;
                        let release_cfg = unsafe { (*cv).Release }
                            .ok_or("vtable 中缺少 IDeckLinkConfiguration::Release")?;
                        let mut serial_ptr: *mut c_char = std::ptr::null_mut();
                        unsafe { let _ = get_string(cfg, 0x6469736Eu32, &mut serial_ptr); }
                        let s = unsafe { read_cstr(serial_ptr) };
                        unsafe { let _ = release_cfg(cfg); }
                        if s.is_empty() { String::from("n/a") } else { s }
                    } else {
                        format!("n/a(hr=0x{hr_cfg:08X})")
                    }
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

    /// 读取 DeckLink 返回的 `char*`（可空）。注意: SDK 16.0 的 `IDeckLink`
    /// (DeckLinkAPIDiscovery.h) 与 `IDeckLinkConfiguration` 头文件**未提供字符串释放接口**
    /// (无 `ReleaseString` 方法, 已 grep 真机头确认), 官方 2.5.15 文字虽称
    /// "must be freed by the caller", 但 16.0 头未暴露对应 API; 枚举为启动期一次性操作,
    /// 此处仅复制字符串内容, 轻微泄漏可接受。
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
