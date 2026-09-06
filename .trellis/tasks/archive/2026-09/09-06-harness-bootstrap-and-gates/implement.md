# 执行计划
依赖用户批准。与session child修改不重叠，但集成验证需其完成；与rules child串行协调Kimi文件ignore例外，不改其内容。

1. 核对仅两个inject hooks及三Kimi skill的受控范围，检查它们没有私有路径/凭据；保留定制hash。
2. 精确调整.gitignore，纳入两个hook。不执行git add/commit，交付工作区diff即可。
3. 在现有runtime tests把必要hook缺失由skip改为失败，补缺失fixture；保持平台不适用skip。
4. 接入既有rust-platform lane的unittest，调整runCi与CI合同测试。
5. `python -X utf8 -m unittest discover -s .trellis/scripts/tests -p 'test_*.py' -v`。
6. `pnpm exec vitest run src/test/scripts/runCi.test.ts src/test/contracts/ciWorkflowContract.test.ts`；阻塞时先直接本地Vitest，明确非canonical证据。
7. 按design的 `git ls-files -z` +当前工作区字节+明确新增allowlist建立临时Git快照，记录输入hash，主工作区index不变。使用已安装匹配Trellis运行init命令，再比较原受控文件hash、运行Python套件。按父research/harness-checks.md检查五工具静态入口及Grok实际inspect；四套无只读registry的真实发现留UNVERIFIED。记录实际测试数，必要hook相关用例必须执行。
8. 对故意缺hook的临时fixture验证非零，再验证正常检出通过；不删除真实hook来测试。
9. 父任务集成跑just ci。远程三host run只有明确用户授权才触发；本轮/未授权阶段标缺失证据。
10. 将命令、来源清单和四层状态交给rules child回写文档。

rollback仅所拥有diff；不更新全局CLI，不用 --force 或清理用户会话。
