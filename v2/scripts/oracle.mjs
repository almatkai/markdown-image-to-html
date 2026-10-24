// Execute the pinned original function, with real string-valued launch args.
// Node >=22.13. No Raycast, OS, browser, network, or rewritten expected algorithm.
import { readFileSync, writeFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../', import.meta.url));
const source = readFileSync(`${root}tests/fixtures/v1/index.ts`, 'utf8');
const stripped = stripTypeScriptTypes(source).replace(/^import .*from "@raycast\/api";\s*/m, '')
  .replace('export default async function main', 'async function main');
const factory = new Function('api', 'console',
  `const {Clipboard,showToast,Toast,getSelectedText,getPreferenceValues}=api;\n${stripped}\nreturn main;`);
function caseOf(name, args = {}, extra = {}) {
  return { name, payload: { argumentValues: args, preferences: {} }, ...extra };
}
const image = '![alt](https://example.test/a.png)';
const cases = [
  caseOf('canonical valid skips all reads', {markdownImage:image}),
  caseOf('legacy typo valid', {mardownImage:image}),
  caseOf('canonical wins', {markdownImage:'![canonical](x)',mardownImage:'![legacy](y)'}),
  caseOf('empty canonical ignores legacy', {markdownImage:'',mardownImage:image}, {selection:{value:'![selection](s)'}}),
  caseOf('invalid canonical ignores legacy', {markdownImage:'invalid',mardownImage:image}, {selection:{value:'invalid'},clipboard:{value:image}}),
  caseOf('selection first', {}, {selection:{value:'before ![selected](s) after'},clipboard:{error:true}}),
  caseOf('selection throws', {}, {selection:{error:true},clipboard:{value:image}}),
  caseOf('selection no match', {}, {selection:{value:'hello'},clipboard:{value:image}}),
  caseOf('clipboard throws', {}, {selection:{error:true},clipboard:{error:true}}),
  caseOf('no match', {}, {selection:{value:'nothing'},clipboard:{value:'nothing'}}),
  caseOf('paste throws', {markdownImage:image}, {paste:{error:true}}),
  caseOf('real payload fields only', {}, {payload:{arguments:{markdownImage:image},imageWidth:'999',preferences:{imageWidth:'234'}},clipboard:{value:image}}),
];
for (const width of ['', ' ', '\t\n', '\u00a0\ufeff', '0', '-0', '-12', '+12', '300.00', '03', '1e3', '1e9999', '1e-9999', '0xFF', '0o17', '0b11', '-0x1', '+0b1', 'Infinity', '+Infinity', '-Infinity', 'NaN', 'nan', 'infinity', 'abc', '300px', '  7  ', '\u0085300', '\u180e300', '.5', '1.', '1e+', '0x', '0b2', '0o8', '１２', '1_000']) {
  cases.push(caseOf(`width ${JSON.stringify(width)}`, {markdownImage:image,imageWidth:width}, {
    payload:{argumentValues:{markdownImage:image,imageWidth:width},preferences:{imageWidth:'fallback'}}
  }));
}
for (const width of [null, '', ' ', 'bad" <script>', '0', '-0', '1e3', '$&', '$1', '$2', '$$', '$`', "$'", '$01/$02/$03/$00/$10/$12/$20/$23/$99/$0/$', '$$$&$1$2$`$\'$', true, false, 0, -0, 1e21, 1e-7, 9007199254740992, {}, {toString:null}, [{toString:'invalid'}], ['a',null,['b',null]], [1,true,{}]]) {
  cases.push(caseOf(`preference ${JSON.stringify(width)}`, {}, {
    payload:{argumentValues:{markdownImage:'PRE ![one](u) MID ![two](v) POST'},preferences:{imageWidth:width}}
  }));
}
const texts = [
  '', '![](x)', '![](x)![b](y)', 'pre ![a](x) middle ![b](y) post',
  '![a](x(y))', '![a](x)![b](y)', '![a](x))tail)', '![a](x y)', '![a]()',
  '![a]oops](u)', '![a](x) ![bad]() ![c](z)', '![a ![b](c)', '![a](x\n) ![b](y)',
  '![a\nb](x)', '![a\rb](x)', '![a\u2028b](x)', '![a\u2029b](x)',
  '![a\u0085b](x)', '![a\u000bb](x)', '![😀中文](https://例え.test/😃)',
  '!["<&>](javascript:evil())', '\\![escaped](x)', '![a](x\u0085y)', '![a](x\u180ey)',
];
for (const ws of ['\t','\n','\v','\f','\r',' ','\u00a0','\u1680','\u2000','\u2001','\u2002','\u2003','\u2004','\u2005','\u2006','\u2007','\u2008','\u2009','\u200a','\u2028','\u2029','\u202f','\u205f','\u3000','\ufeff','\u0085','\u180e','\u200b']) {
  texts.push(`![a](x${ws}y)`, `![a${ws}b](u)`);
}
for (let i=0;i<texts.length;i++) cases.push(caseOf(`regex ${i}`,{markdownImage:texts[i]}));
// Deterministic adversarial grammar exercises lazy alt and greedy URL boundaries.
let seed = 81719;
const pieces = ['![','](',')',']','[','!', 'a','x',' ', '\n','😀','\u0085','\u00a0'];
for (let i=0;i<250;i++) {
  let text = '';
  for(let j=0;j<30;j++) { seed=(Math.imul(seed,1664525)+1013904223)>>>0; text+=pieces[seed%pieces.length]; }
  cases.push(caseOf(`grammar ${i}`,{markdownImage:text}));
}
async function execute(test) {
  const calls=[];
  const read = async (key, channel) => {
    calls.push([channel,[]]);
    if (test[key]?.error) throw new Error(key);
    return test[key]?.value ?? '';
  };
  const api = {
    Clipboard:{
      readText:()=>read('clipboard','clipboard:read'),
      paste:async text=>{calls.push(['core:platform:paste-text',[text]]); if(test.paste?.error) throw new Error('paste');},
    },
    getSelectedText:()=>read('selection','core:platform:selected-text'),
    getPreferenceValues:()=>test.payload.preferences ?? {},
    showToast:async toast=>{throw new Error(toast.message);}, Toast:{Style:{Failure:'Failure'}},
  };
  const fn = factory(api,{log:()=>{}});
  const args={...(test.payload.argumentValues??{})};
  // Explicit V2 bug fix only: canonical presence wins, even when empty/invalid.
  if(Object.hasOwn(args,'markdownImage')) args.mardownImage=args.markdownImage;
  try { await fn({arguments:args}); return {status:'ok',calls}; }
  catch {return {status:'error',calls};}
}
const wireCases = JSON.parse(JSON.stringify(cases)); // Exactly the JSON transport values.
const expected = [];
for(const test of wireCases) expected.push(await execute(test));
const fixture=wireCases.map((test,i)=>({...test,expected:expected[i]}));
if(process.argv.includes('--write')) {
  writeFileSync(`${root}tests/fixtures/oracle.json`,JSON.stringify(fixture,null,2)+'\n');
} else {
  assert.deepEqual(JSON.parse(readFileSync(`${root}tests/fixtures/oracle.json`)),fixture,'stored oracle must match live pinned TS');
}
const probe=spawnSync('cargo',['run','--quiet','--locked','--example','oracle_probe'],{cwd:root,input:JSON.stringify(cases),encoding:'utf8',maxBuffer:32*1024*1024});
assert.equal(probe.status,0,probe.stderr);
const actual=JSON.parse(probe.stdout);
for(let i=0;i<cases.length;i++) assert.deepEqual({status:actual[i].status,calls:actual[i].calls},expected[i],cases[i].name);
console.log(`Pinned TS oracle: ${cases.length} cases matched production-adapter calls and output`);
