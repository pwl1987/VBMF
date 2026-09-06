# A2-8-04 R58 Step 10 全回归——真机证据入库审计副本

- 来源: 盒 lytv@10.30.15.10 `~/a2-8-02i-evidence/2026-09-06-r58-step10-regression/`
  （盒上原件为 origin·P8 不变; 本目录 = 审计便利副本, **本地验证口径非 GitHub CI**）。
- 采集: 2026-09-06 12:57-13:13 CST（header.txt 五件套含 REV=6ab24a3…）。
- 身份链: REV=6ab24a3c8fa3fe1d4a24aec964595966cc64cf88（e09ed97 为其
  docs-only 后继, .rs 源全等）; gates bin md5=440c761b79457dd51f6dd49ca6aa5bb2;
  manifest v5 md5=7521d17e7fd02e50eb2b0a84374a43dd。
- 四跑: run1 dual_input ALL PASS 10/10（L4 状态机收敛一行齐·fence 在链）;
  run2 obs N=10 dwell1000（#8 精确形态）; run3 obs N=30 dwell1000;
  run4 obs burst N=30 dwell0——全 EXIT=0·全 Preserved·PE(0)·NM=0·adv=0·
  R53 签名逐字。
- 验收层终裁: Step 10 = ✅ PASS / CLOSED（R57 报告 §21·主账 §86·2026-09-06）。
- 完整性: md5s.txt = 盒生成清单; 四跑 log md5 与 e09ed97 登记值逐一相符
  （8361e98a/2d2b724e/f9c60fbe/355330b3）, 入库时 md5sum -c 全过。
