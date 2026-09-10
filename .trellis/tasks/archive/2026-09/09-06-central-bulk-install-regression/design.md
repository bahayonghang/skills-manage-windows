# 测试设计
唯一owner：src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx；先复用同文件renderCentralSkillsView与现有store mock，不重建mock层。

R1通过当前toolbar菜单项触发过滤，等待可见集合后用checkbox选择，再通过bulk bar打开batch dialog。R2观察mockBatchInstallSkills的skill IDs/agent IDs/方式以及过滤后DOM，不直接调用store替代用户链；保留同文件已有部分成功反馈测试。R3不编辑生产UI。

适合任一已验证shell/test入口的harness交给较便宜模型执行（Codex有限worker、Claude较低成本子代理、Grok/Kimi有限coder、OMP已解析task role）；强模型先给断言、最后审查交互真实性。模型实际价格/可用性运行时确认，不按品牌推测。

回滚仅此测试diff；无需业务数据迁移。
