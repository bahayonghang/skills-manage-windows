// Render the local browser fixtures with the installed Chrome in an isolated profile.
// Node 26 provides fetch and WebSocket. No browser package installation is required.
import { spawn } from "node:child_process";
import { mkdtemp, readFile, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import assert from "node:assert/strict";

const chromePath = process.env.RULES_CHROME_PATH;
assert(chromePath, "RULES_CHROME_PATH is required");
const outputDir = resolve(process.env.RULES_SCREENSHOT_DIR ?? "tmp/rules-ui");
await mkdir(outputDir, { recursive: true });
const profile = await mkdtemp(join(tmpdir(), "skillport-rules-browser-"));
const chrome = spawn(chromePath, ["--headless=new", "--remote-debugging-port=0",
  `--user-data-dir=${profile}`, "--no-first-run", "--no-default-browser-check",
  "--disable-extensions", "--disable-background-networking", "about:blank"],
{ windowsHide: true, stdio: "ignore" });
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
let socket;
try {
  let port;
  for (let attempt = 0; attempt < 100; attempt++) {
    try { port = (await readFile(join(profile, "DevToolsActivePort"), "utf8")).split("\n")[0]; break; }
    catch { await sleep(100); }
  }
  assert(port, "Chrome did not publish DevToolsActivePort");
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  socket = new WebSocket(version.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let nextId = 0;
  const pending = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (!message.id) return;
    const handler = pending.get(message.id);
    if (!handler) return;
    pending.delete(message.id);
    if (message.error) handler.reject(new Error(JSON.stringify(message.error)));
    else handler.resolve(message.result);
  };
  const send = (method, params = {}, sessionId) => new Promise((resolve, reject) => {
    const id = ++nextId;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
  });
  const { targetId } = await send("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await send("Target.attachToTarget", { targetId, flatten: true });
  const call = (method, params) => send(method, params, sessionId);
  await call("Page.enable");
  await call("Runtime.enable");
  const evaluate = async (expression) => {
    const response = await call("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (response.exceptionDetails) throw new Error(response.exceptionDetails.text + " " + (response.result.description ?? ""));
    return response.result.value;
  };
  const importLoaded = (path) => `import(performance.getEntriesByType('resource').filter(r => r.name.includes(${JSON.stringify(path)})).at(-1)?.name ?? ${JSON.stringify(path)})`;
  const wait = async (expression) => {
    for (let attempt = 0; attempt < 100; attempt++) {
      if (await evaluate(expression)) return;
      await sleep(100);
    }
    throw new Error(`UI condition timed out: ${expression}`);
  };
  await call("Page.addScriptToEvaluateOnNewDocument", { source: "localStorage.setItem('i18nextLng', 'zh');" });
  await call("Emulation.setDeviceMetricsOverride", { width: 1200, height: 800, deviceScaleFactor: 1, mobile: false });
  await call("Page.navigate", { url: process.env.RULES_UI_URL ?? "http://127.0.0.1:24202/rules" });
  await wait("document.querySelector('textarea') !== null");
  await evaluate("document.fonts.ready");
  const screenshot = async (name) => {
    const { data } = await call("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
    await writeFile(join(outputDir, `${name}.png`), Buffer.from(data, "base64"));
  };
  const report = { browser: version.Browser, boundary: "browser fixtures, isolated profile; no native Tauri IPC", cases: [] };
  const clickText = async (text) => evaluate(`(() => {
    const buttons = [...document.querySelectorAll('button')];
    const button = buttons.find(b => b.textContent.trim() === ${JSON.stringify(text)});
    if (!button) throw new Error('Button not found: ' + ${JSON.stringify(text)});
    button.click();
  })()`);
  for (const locale of ["zh", "en"]) {
    await evaluate(`import('/src/i18n/index.ts').then(m => m.default.changeLanguage('${locale}'))`);
    for (const [width, height] of [[1200, 800], [900, 800], [1200, 600], [900, 600]]) {
      await call("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
      await screenshot(`${locale}-${width}x${height}`);
      const metrics = await evaluate(`({ width:innerWidth, height:innerHeight,
        scrollWidth:document.documentElement.scrollWidth,
        textarea:document.querySelector('textarea').getBoundingClientRect().toJSON(),
        buttons:[...document.querySelectorAll('button')].filter(b=>!b.disabled).map(b=>({text:b.textContent.trim(),rect:b.getBoundingClientRect().toJSON()})),
        text:document.body.innerText })`);
      assert.equal(metrics.scrollWidth, width, "Page has horizontal overflow");
      report.cases.push({ locale, width, height, metrics });
    }
  }
  await evaluate("import('/src/lib/displayFont.ts').then(m => m.applyFontScale(1.125))");
  await screenshot("en-900x600-max-preset");
  await evaluate("import('/src/lib/displayFont.ts').then(m => m.applyFontScale(1.5))");
  await screenshot("en-900x600-max-scale");
  report.cases.push({ name: "max-scale", metrics: await evaluate(`({
    scale: getComputedStyle(document.documentElement).getPropertyValue('--font-scale'),
    buttons:[...document.querySelectorAll('main button')].map(b=>({text:b.textContent.trim(),rect:b.getBoundingClientRect().toJSON()}))
  })`) });
  const reachability = await evaluate(`(() => {
    const button = [...document.querySelectorAll('main button')].find(b => b.textContent.trim() === 'Replace with link');
    button.scrollIntoView({ block: 'nearest' });
    return button.getBoundingClientRect().toJSON();
  })()`);
  assert(reachability.bottom <= 600 && reachability.top >= 0, "Tool action cannot be scrolled into view at maximum scale");
  await screenshot("en-900x600-max-scale-tools");
  await evaluate("document.querySelector('main section').scrollTop=0");
  await evaluate("import('/src/lib/displayFont.ts').then(m => m.applyFontScale(1))");
  await clickText("Preview");
  await wait("document.querySelector('main section h1') !== null");
  assert.equal(await evaluate("document.querySelector('main section').innerText.includes('SKILL.md')"), false);
  await screenshot("en-preview");
  await clickText("Source");
  await wait("document.querySelector('main section pre') !== null");
  assert.match(await evaluate("document.querySelector('main section pre').textContent"), /alwaysApply: true/);
  await screenshot("en-source");
  await clickText("Body");
  await wait("document.querySelector('textarea') !== null");
  await evaluate(`(() => {
    const textarea = document.querySelector('textarea');
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(textarea, '# Changed draft\\n');
    textarea.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
  await wait("document.body.innerText.includes('Unsaved')");
  await evaluate("document.querySelector('[aria-label=\"Select writing-style.md\"]').click()");
  await wait("document.querySelector('[role=dialog]') !== null");
  await screenshot("en-unsaved-dialog");
  await clickText("Cancel");
  assert.equal(await evaluate("document.querySelector('textarea').value"), "# Changed draft\n");
  await clickText("Save");
  await wait("!document.body.innerText.includes('Unsaved')");
  await clickText("Replace with link");
  await wait("document.querySelector('[role=dialog]') !== null");
  await screenshot("en-takeover-dialog");
  await clickText("Cancel");
  await clickText("Import existing rules");
  await wait("document.body.innerText.includes('conditional-example.md')");
  await screenshot("en-import-dialog");
  await clickText("Close");
  await evaluate("document.querySelector('[aria-label=\"Select safety-basics.md\"]').click()");
  await wait("document.querySelector('h2')?.textContent === 'safety-basics.md'");
  await screenshot("en-conflict");
  for (const width of [719, 721, 1099, 1101]) {
    await call("Emulation.setDeviceMetricsOverride", { width, height: 600, deviceScaleFactor: 1, mobile: false });
    await screenshot(`en-boundary-${width}`);
    assert.equal(await evaluate("document.documentElement.scrollWidth"), width);
  }
  await call("Emulation.setDeviceMetricsOverride", { width:900, height:600, deviceScaleFactor:1, mobile:false });
  await evaluate("document.querySelector('[aria-label=\"Collapse sidebar\"]').click()");
  await screenshot("en-sidebar-collapsed");
  await evaluate("document.querySelector('[aria-label=\"Expand sidebar\"]').click()");
  await evaluate(`${importLoaded('/src/stores/rulesStore.ts')}.then(m => m.useRulesStore.setState(s => {
    const name = 'long-file-name-' + 'a'.repeat(180) + '.md';
    const detail = {...s.detail,name,targets:s.detail.targets.map(t=>({...t,path:t.path.replace('safety-basics.md',name)}))};
    return {selectedName:name,detail,snapshot:{...s.snapshot,rules:[detail]}};
  }))`);
  await screenshot("en-long-file-name");
  await evaluate(`${importLoaded('/src/stores/rulesStore.ts')}.then(m => m.useRulesStore.setState(s => ({ snapshot:{...s.snapshot,recoveryOperations:[{
    operationId:'00000000-0000-4000-8000-000000000001',name:s.selectedName,kind:'enable',phase:'prepared',tool:'omp',backupPath:s.snapshot.rootPath+'/.backups/probe/omp/'+s.selectedName
  }]} })))`);
  await screenshot("en-recovery");
  await evaluate(`${importLoaded('/src/stores/rulesStore.ts')}.then(m => m.useRulesStore.setState(s => ({ snapshot:{...s.snapshot,recoveryOperations:[]} })))`);
  await evaluate(`${importLoaded('/src/stores/rulesStore.ts')}.then(m => m.useRulesStore.setState({ errorCode: 'rules.permission_denied' }))`);
  await screenshot("en-permission-error");
  await evaluate(`${importLoaded('/src/stores/rulesStore.ts')}.then(m => m.useRulesStore.setState(s => ({ errorCode:null, selectedName:null, detail:null, snapshot:{...s.snapshot,rules:[]} })))`);
  await screenshot("en-empty");
  await evaluate(`${importLoaded('/src/stores/targetStore.ts')}.then(m => m.useTargetStore.setState(s => ({ activeTarget: {...s.activeTarget,id:'fixture-ssh',kind:'ssh'} })))`);
  await screenshot("en-nonlocal");
  report.interactions = ["Markdown preview", "read-only source with shared YAML header", "edit", "dirty switch dialog", "cancel keeps draft", "save", "takeover confirmation", "import preview", "conflict"];
  await writeFile(join(outputDir, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ result: "PASS", browser: version.Browser, cases: report.cases.length, outputDir }));
} finally {
  socket?.close();
  chrome.kill();
}
