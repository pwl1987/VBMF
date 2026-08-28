# TECHNOLOGY_PORTABILITY_MATRIX — 技术可移植性矩阵

> Phase 0.6 门禁依据（替换轴）。综合论述见 [`IMPLEMENTATION_ADDENDUM.md §1`](./IMPLEMENTATION_ADDENDUM.md)。

## 1. 替换轴（9 轴，含 Audio Backend/Routing 第 9 轴）
| # | 轴 | 当前 Reference | 阶段 |
|---|---|---|---|
| 1 | Hardware Vendor | BMD | P0 Provider SPI |
| 2 | Hardware SDK | DeckLink SDK | P0 Provider SPI |
| 3 | Media Backend | GStreamer | P0 Backend SPI |
| 4 | Encoder Backend | FFmpeg | P1 Contract |
| 5 | Stream Gateway | SRS | P1 Contract |
| 6 | Clock / Timecode Provider | BMD hw / GStreamer clock | P1 Contract |
| 7 | Acceleration Provider | CUDA/NVENC/NVDEC/VAAPI/QSV/AMF/CPU | P1 Contract |
| 8 | Infrastructure / Deployment | Docker / runc / Nginx | P2 Adapter |
| 9 | **Audio Backend / Routing** | Embedded SDI（隐含） | P1 Contract |

> 第 9 轴是此前模型遗漏的真实缺口：当前 `decklinkaudiosrc` 隐含"audio 内嵌 SDI"，但未来 AES / Audio Matrix / MADI / Dante / IP Audio 使 Video Resource 与 Audio Resource 不再一一对应。

## 2. 门禁判据
- 每个替换轴的「当前 Reference」仅是实现资源，不是业务身份。
- 替换任意轴时，上层（Domain/Graph/Session/Supervisor/Health）语义与代码不变（见 IMPLEMENTATION_BOUNDARIES §6 验收表）。
- `ARCH-PORTABILITY-01` Test C：换 Mock Provider B，不得修改 Domain / Graph / UI semantic schema。
- 不做：动态 .so plugin、10+ Rust crate、AJA 全实现、通用硬件库、信号 AI（无消费方不建抽象）。
