# A2-8-04 R58 Step 11 新鲜 Final Gate——真机证据入库审计副本

- 来源: 盒 lytv@10.30.15.10 `~/a2-8-02i-evidence/2026-09-06-r58-step11-final-gate/`
  （盒上原件为 origin·P8 不变; 本目录 = 审计便利副本, **本地验证口径非 GitHub CI**）。
- 采集: 2026-09-06 13:48-13:57 CST（盒本地时间, header.txt 五件套含 date/date -u/
  timedatectl/REV）。
- 身份链: REV=e09ed975c3db1e2fdc6f253a304f97d549333b14; 源 sha 864/864
  盒==git-archive==HEAD 全等; gates bin md5=440c761b79457dd51f6dd49ca6aa5bb2
  （冻结复用 == Step 10 登记值, 未重建）; manifest v5 md5=
  7521d17e7fd02e50eb2b0a84374a43dd。
- 案 b 窗口: run1 OBS N=30 dwell=1000ms（=R56 失败窗同形）; run2 dual_input
  五层; run3 hw 矩阵 bmd,gstreamer（266=冻结字面 259+R58 验收层批准测试增量
  7·如实登记）; run3b = P3 检出能力定向补证（cfg(test,mock) 门控测试, 窗口外
  补充件, 附口径修正: R56 "∈259" 引用不准, 该测试 ∈ mock 397 套件）。
- 附件: run1-nm-extract.txt（NM 行抽取, 0 行=空文件亦为证据）;
  run1-av-delta-rows.txt/stats（P4 全序列 n=180·min 2.037ms/max 118.704ms/
  mean 51.124ms·无阈值案 a）。
- 完整性: md5s.txt = 盒生成清单（自 身不入清单）; 入库时 md5sum -c 全过。
  逐格 verdict 与冻结合取结果 = 谓词文档 §11（验收终裁归验收层）。
