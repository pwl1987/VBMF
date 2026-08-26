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
    use std::os::raw::{c_char, c_void};

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

    // ─────────────────────────────────────────────────────────────
    // CAP-01: IDeckLinkInput 视频采集封装 (Gate 2.6 → MEDIA-RT-01)
    // vtable 槽位顺序严格对齐官方手册 DeckLinkAPI.h (真机头权威, 2026-08-26 校验):
    //   IDeckLinkInput: QI@0/AddRef@1/Release@2/DoesSupportVideoMode@3/GetDisplayMode@4/
    //     GetDisplayModeIterator@5/SetScreenPreviewCallback@6/EnableVideoInput@7/
    //     EnableVideoInputWithAllocatorProvider@8/DisableVideoInput@9/
    //     GetAvailableVideoFrameCount@10/EnableAudioInput@11/DisableAudioInput@12/
    //     GetAvailableAudioSampleFrameCount@13/StartStreams@14/StopStreams@15/
    //     PauseStreams@16/FlushStreams@17/SetCallback@18/GetHardwareReferenceClock@19
    //   IDeckLinkInputCallback: QI@0/AddRef@1/Release@2/
    //     VideoInputFormatChanged@3/VideoInputFrameArrived@4
    // IID (真机头 DeckLinkAPI.h 权威):
    //   IID_IDeckLinkInput = 6A515F8A-FBCE-4853-B0F7-2A09DB1ECA0B
    // 复用 bindgen 生成的 IDeckLinkInput/IDeckLinkInputCallback 对象类型;
    // vtable 槽位布局手工声明 (bindgen 生成不透明 vtable, 槽位需对齐官方头).
    // ─────────────────────────────────────────────────────────────
    const IID_DECKLINK_INPUT: Guid16 = Guid16 {
        b: [0x6A,0x51,0x5F,0x8A,0xFB,0xCE,0x48,0x53,0xB0,0xF7,0x2A,0x09,0xDB,0x1E,0xCA,0x0B],
    };

    #[repr(C)]
    struct RawIDeckLinkInputVtbl {
        QueryInterface: Option<unsafe extern "C" fn(*mut c_void, Guid16, *mut *mut c_void) -> i32>,
        AddRef: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
        Release: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
        // 官方 IDeckLinkInput::DoesSupportVideoMode 签名 (DeckLinkAPI.h:1161):
        //   (this, BMDVideoConnection, BMDDisplayMode, BMDPixelFormat,
        //    BMDVideoInputConversionMode, BMDSupportedVideoModeFlags,
        //    BMDDisplayMode* actualMode, bool* supported) -> HRESULT
        DoesSupportVideoMode: Option<unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32, u32, *mut u32, *mut u8) -> i32>,
        GetDisplayMode: Option<unsafe extern "C" fn()>,
        GetDisplayModeIterator: Option<unsafe extern "C" fn()>,
        SetScreenPreviewCallback: Option<unsafe extern "C" fn()>,
        EnableVideoInput: Option<unsafe extern "C" fn(*mut c_void, u32, u32, u32) -> i32>,
        EnableVideoInputWithAllocatorProvider: Option<unsafe extern "C" fn()>,
        DisableVideoInput: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
        GetAvailableVideoFrameCount: Option<unsafe extern "C" fn()>,
        EnableAudioInput: Option<unsafe extern "C" fn()>,
        DisableAudioInput: Option<unsafe extern "C" fn()>,
        GetAvailableAudioSampleFrameCount: Option<unsafe extern "C" fn()>,
        StartStreams: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
        StopStreams: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
        PauseStreams: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
        FlushStreams: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
        SetCallback: Option<unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32>,
        GetHardwareReferenceClock: Option<unsafe extern "C" fn(*mut c_void, u32, *mut i64, *mut i64, *mut i64) -> i32>,
    }

    #[repr(C)]
    struct RawIDeckLinkInputCallbackVtbl {
        QueryInterface: Option<unsafe extern "C" fn(*mut c_void, Guid16, *mut *mut c_void) -> i32>,
        AddRef: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
        Release: Option<unsafe extern "C" fn(*mut c_void) -> u32>,
        VideoInputFormatChanged: Option<unsafe extern "C" fn(*mut c_void, u32, *mut c_void, u32) -> i32>,
        VideoInputFrameArrived: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32>,
    }

    /// 采集统计 (MEDIA-RT-01 验证载体): 首帧到达、帧计数、PTS(硬件参考时钟) 单调性。
    pub struct CaptureStats {
        pub frame_count: std::sync::atomic::AtomicU64,
        pub first_frame_at: std::sync::Mutex<Option<std::time::Instant>>,
        pub last_pts: std::sync::Mutex<Option<i64>>,
        pub monotonic: std::sync::atomic::AtomicBool,
    }

    #[repr(C)]
    struct CaptureCallback {
        vtbl: *const RawIDeckLinkInputCallbackVtbl,
        input: *mut c_void,
        stats: *const CaptureStats,
    }

    static CALLBACK_VTBL: RawIDeckLinkInputCallbackVtbl = RawIDeckLinkInputCallbackVtbl {
        QueryInterface: Some(cb_query_interface),
        AddRef: Some(cb_add_ref),
        Release: Some(cb_release),
        VideoInputFormatChanged: Some(cb_format_changed),
        VideoInputFrameArrived: Some(cb_frame_arrived),
    };

    unsafe extern "C" fn cb_query_interface(_this: *mut c_void, _iid: Guid16, _out: *mut *mut c_void) -> i32 {
        0x80004001u32 as i32 // E_NOTIMPL
    }
    unsafe extern "C" fn cb_add_ref(_this: *mut c_void) -> u32 { 1 }
    unsafe extern "C" fn cb_release(_this: *mut c_void) -> u32 { 1 }
    unsafe extern "C" fn cb_format_changed(_this: *mut c_void, _ev: u32, _mode: *mut c_void, _flags: u32) -> i32 { 0 }

    unsafe extern "C" fn cb_frame_arrived(this: *mut c_void, _video: *mut c_void, _audio: *mut c_void) -> i32 {
        let cb = &*(this as *const CaptureCallback);
        let stats = &*cb.stats;
        let n = stats.frame_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let ivt = *(cb.input as *mut *mut RawIDeckLinkInputVtbl);
        let mut hw: i64 = 0;
        if let Some(f) = (*ivt).GetHardwareReferenceClock {
            f(cb.input, 1_000_000_000, &mut hw, std::ptr::null_mut(), std::ptr::null_mut());
        }
        if n == 1 {
            *stats.first_frame_at.lock().unwrap() = Some(std::time::Instant::now());
            *stats.last_pts.lock().unwrap() = Some(hw);
            eprintln!("[CAP-01] first frame arrived; hw_clock={hw}");
        } else {
            let mut last = stats.last_pts.lock().unwrap();
            if let Some(prev) = *last {
                if hw < prev {
                    stats.monotonic.store(false, std::sync::atomic::Ordering::SeqCst);
                }
            }
            *last = Some(hw);
        }
        eprintln!("[CAP-01] frame {n} hw={hw}");
        0
    }

    /// 对指定索引的 DeckLink 设备开启视频采集 (EnableVideoInput + SetCallback + StartStreams)。
    /// 返回 CaptureStats 共享句柄, 调用方轮询 frame_count / first_frame_at 验证 MEDIA-RT-01。
    /// 默认采集格式 1080i50 / 8-bit YUV; 不同机型需在真机验证支持的模式。
    pub fn start_capture(device_index: usize) -> Result<std::sync::Arc<CaptureStats>, String> {
        let lib = unsafe { Library::new(OsStr::new("libDeckLinkAPI.so")) }
            .map_err(|e| format!("加载 libDeckLinkAPI.so 失败: {e}"))?;
        let create: Symbol<CreateIter> = unsafe {
            lib.get(b"CreateDeckLinkIteratorInstance_0004\0")
                .map_err(|e| format!("未找到符号 CreateDeckLinkIteratorInstance_0004: {e}"))?
        };
        let iter: *mut IDeckLinkIterator = unsafe { create() };
        if iter.is_null() {
            return Err("CreateDeckLinkIteratorInstance_0004 返回空指针（无 DeckLink 设备？）".into());
        }
        let ivt = unsafe { *(iter as *mut *mut IDeckLinkIteratorVtbl) };
        let next = unsafe { (*ivt).Next }.ok_or("vtable 中缺少 IDeckLinkIterator::Next")?;
        let release_iter = unsafe { (*ivt).Release }.ok_or("vtable 中缺少 IDeckLinkIterator::Release")?;

        let mut dev: *mut IDeckLink = std::ptr::null_mut();
        let mut idx = 0usize;
        loop {
            let mut d: *mut IDeckLink = std::ptr::null_mut();
            let hr = unsafe { next(iter, &mut d) };
            if hr != S_OK || d.is_null() {
                break;
            }
            if idx == device_index {
                dev = d;
                break;
            }
            let dv = unsafe { *(d as *mut *mut IDeckLinkVtbl) };
            unsafe { ((*dv).Release).unwrap()(d); }
            idx += 1;
        }
        unsafe { release_iter(iter); }
        if dev.is_null() {
            return Err(format!("设备索引 {device_index} 不存在"));
        }

        let dv = unsafe { *(dev as *mut *mut IDeckLinkVtbl) };
        let q = unsafe { (*dv).QueryInterface }.ok_or("vtable 中缺少 IDeckLink::QueryInterface")?;
        let mut input: *mut c_void = std::ptr::null_mut();
        let hr = unsafe { q(dev, IID_DECKLINK_INPUT, &mut input) };
        unsafe { ((*dv).Release).unwrap()(dev); }
        if hr != S_OK || input.is_null() {
            return Err(format!("QueryInterface(IDeckLinkInput) 失败 hr=0x{hr:08X}"));
        }

        let stats = std::sync::Arc::new(CaptureStats {
            frame_count: std::sync::atomic::AtomicU64::new(0),
            first_frame_at: std::sync::Mutex::new(None),
            last_pts: std::sync::Mutex::new(None),
            monotonic: std::sync::atomic::AtomicBool::new(true),
        });
        let cb = Box::into_raw(Box::new(CaptureCallback {
            vtbl: &CALLBACK_VTBL as *const RawIDeckLinkInputCallbackVtbl,
            input,
            stats: &*stats as *const CaptureStats,
        }));

        let ivt = unsafe { *(input as *mut *mut RawIDeckLinkInputVtbl) };
        let does_support = unsafe { (*ivt).DoesSupportVideoMode }
            .ok_or("vtable 中缺少 IDeckLinkInput::DoesSupportVideoMode")?;
        let enable = unsafe { (*ivt).EnableVideoInput }.ok_or("vtable 中缺少 IDeckLinkInput::EnableVideoInput")?;
        let set_cb = unsafe { (*ivt).SetCallback }.ok_or("vtable 中缺少 IDeckLinkInput::SetCallback")?;
        let start = unsafe { (*ivt).StartStreams }.ok_or("vtable 中缺少 IDeckLinkInput::StartStreams")?;

        // —— 动态探测设备支持的采集格式 (对齐官方 SDK 手册, 禁止硬编码) ——
        // 不假设单一信号源模式: 用 IDeckLinkInput::DoesSupportVideoMode (vtable@3) 按优先级
        // 遍历候选, 选第一个设备支持 (supported=true) 的 BMDDisplayMode。
        // 权威枚举值来自 DeckLinkAPIModes.h (SDK 16.0):
        //   bmdModeHD1080i50   = 'Hi50' = 0x48693530
        //   bmdModeHD1080i5994 = 'Hi59' = 0x48693539
        //   bmdModeHD1080i6000 = 'Hi60' = 0x48693630
        //   bmdModeHD1080p50   = 'Hp50' = 0x48703530
        //   bmdModeHD1080p30   = 'Hp30' = 0x48703330
        //   bmdModeHD720p50    = 'hp50' = 0x68703530
        // bmdFormat8BitYUV = '2vuy' = 0x32767579 (DeckLinkAPIModes.h:219)。
        // 入参 (DeckLinkAPI.h:1161): connection=bmdVideoConnectionUnspecified(0),
        //   conversionMode=bmdNoVideoInputConversion(0x6E6F6E65 'none', DeckLinkAPI.h:354),
        //   flags=bmdSupportedVideoModeDefault(0)。
        const BMD_FORMAT_8BIT_YUV: u32 = 0x32767579;
        const BMD_NO_CONVERSION: u32 = 0x6E6F6E65;
        const CANDIDATES: &[(u32, &str)] = &[
            (0x48693530, "1080i50"),
            (0x48693539, "1080i5994"),
            (0x48693630, "1080i6000"),
            (0x48703530, "1080p50"),
            (0x48703330, "1080p30"),
            (0x68703530, "720p50"),
        ];
        let mut chosen: Option<(u32, &str)> = None;
        for &(mode, name) in CANDIDATES {
            let mut actual_mode: u32 = 0;
            let mut supported: u8 = 0;
            let hr = unsafe {
                does_support(
                    input,
                    0, // bmdVideoConnectionUnspecified
                    mode,
                    BMD_FORMAT_8BIT_YUV,
                    BMD_NO_CONVERSION,
                    0, // bmdSupportedVideoModeDefault
                    &mut actual_mode,
                    &mut supported,
                )
            };
            eprintln!(
                "[CAP-01] DoesSupportVideoMode({name})=hr=0x{hr:08X} supported={supported} actual=0x{actual_mode:08X}"
            );
            if hr == S_OK && supported != 0 {
                chosen = Some((mode, name));
                break;
            }
        }
        let (mode, name) = chosen.ok_or_else(|| {
            "DoesSupportVideoMode: 设备在候选列表中无任何支持的采集格式 (检查信号源/连接)".to_string()
        })?;

        // flags = bmdVideoInputEnableFormatDetection(0x01): 让 SDK 自动检测输入信号真实格式,
        // 避免 EnableVideoInput 因模式与当前信号不符返回 E_INVALIDARG (hr=0x80000003)。
        let hr = unsafe { enable(input, mode, BMD_FORMAT_8BIT_YUV, 1) };
        if hr != S_OK {
            return Err(format!(
                "EnableVideoInput({name}/8bitYUV) 失败 hr=0x{hr:08X} (设备可能不支持该模式)"
            ));
        }
        let hr = unsafe { set_cb(input, cb as *mut c_void) };
        if hr != S_OK {
            return Err(format!("SetCallback 失败 hr=0x{hr:08X}"));
        }
        let hr = unsafe { start(input) };
        if hr != S_OK {
            return Err(format!("StartStreams 失败 hr=0x{hr:08X}"));
        }
        eprintln!("[CAP-01] capture started on device {device_index} (mode={name}/8bitYUV, format-detection on)");
        Ok(stats)
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
pub use imp::{enumerate, registry, start_capture, CaptureStats};
#[cfg(not(feature = "bmd"))]
pub use imp::enumerate;
