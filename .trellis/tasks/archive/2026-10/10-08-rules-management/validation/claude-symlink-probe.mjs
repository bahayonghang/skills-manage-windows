// SDK control requests only. No user turn, real credentials, or model endpoint.
// Protocol source: anthropics/claude-agent-sdk-python src/claude_agent_sdk/_internal/query.py.
import { spawn } from "node:child_process";
import { mkdtemp, mkdir, writeFile, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import assert from "node:assert/strict";

const binary = process.env.RULES_CLAUDE_PATH;
assert(binary, "RULES_CLAUDE_PATH is required");
const root = await mkdtemp(join(tmpdir(), "skillport-claude-rules-"));
await mkdir(join(root, ".skillport/rules"), {recursive:true});
await mkdir(join(root, ".claude/rules"), {recursive:true});
const name = "synthetic-symlink-rule.md";
await writeFile(join(root, ".skillport/rules", name), "---\nalwaysApply: true\ndescription: isolated probe\n---\n# Synthetic rule\nUse exact names.\n");
await symlink("../../.skillport/rules/" + name, join(root, ".claude/rules", name), "file");
const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
  !/ANTHROPIC|CLAUDE|BEDROCK|FOUNDRY|VERTEX|AWS_|GOOGLE_|API_KEY|TOKEN|AUTH|SECRET|PASSWORD/i.test(key)));
Object.assign(env, {HOME:root,USERPROFILE:root,CLAUDE_CONFIG_DIR:join(root,".claude"),
  ANTHROPIC_API_KEY:"synthetic-probe-value",ANTHROPIC_BASE_URL:"http://127.0.0.1:1",
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC:"1",DISABLE_AUTOUPDATER:"1"});
const child = spawn(binary, ["-p","--input-format","stream-json","--output-format","stream-json","--verbose","--setting-sources","user"],
  {cwd:root,env,windowsHide:true,stdio:["pipe","pipe","pipe"]});
let buffer="";
const responses=[];
let timedOut=false;
const timer=setTimeout(()=>{timedOut=true;child.kill();},30000);
child.stderr.resume();
child.stdout.on("data",chunk=>{
  buffer += chunk.toString();
  while(buffer.includes("\n")) {
    const index=buffer.indexOf("\n");
    const line=buffer.slice(0,index);buffer=buffer.slice(index+1);
    let message;try{message=JSON.parse(line);}catch{continue;}
    if(message.type!=="control_response")continue;
    responses.push(message.response);
    if(message.response?.request_id==="init") child.stdin.write(JSON.stringify({type:"control_request",request_id:"context",request:{subtype:"get_context_usage"}})+"\n");
    if(message.response?.request_id==="context") child.stdin.end();
  }
});
child.stdin.write(JSON.stringify({type:"control_request",request_id:"init",request:{subtype:"initialize",systemPromptSnapshot:true}})+"\n");
const code=await new Promise((resolve,reject)=>{child.on("exit",resolve);child.on("error",reject);});
clearTimeout(timer);
const context=responses.find(r=>r?.request_id==="context");
const contextSucceeded=context?.subtype==="success";
const found=contextSucceeded && JSON.stringify(context??{}).includes(name);
const sourceEvidence=[];
function inspect(value) {
  if (!value || typeof value !== 'object') return;
  if (Object.values(value).some(v => typeof v === 'string' && v.includes(name))) {
    sourceEvidence.push({rule:name,keys:Object.keys(value),numericFields:Object.fromEntries(Object.entries(value).filter(([,v])=>typeof v==='number'))});
  }
  for (const child of Object.values(value)) inspect(child);
}
inspect(context);
console.log(JSON.stringify({result:found?"DISCOVERED":"UNVERIFIED",boundary:"isolated Claude SDK initialize/context control requests; no user turn",
  exitCode:code,timedOut,handshake:responses.some(r=>r?.request_id==="init"&&r.subtype==="success"),
  contextResponse:!!context,contextSucceeded,ruleNameInContext:found,sourceEvidence,temporaryRoot:root}));
