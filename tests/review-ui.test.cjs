// Non-browser tests of the actual embedded script. These do not prove visual layout.
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const vm=require('node:vm');
const template=fs.readFileSync(require('node:path').join(__dirname,'../src/review.html'),'utf8');
const script=template.match(/<script>\s*([\s\S]*?)<\/script>/)[1];
class Element {
 constructor(tag='div'){this.tag=tag;this.children=[];this.value='';this.textContent='';this.listeners={};this.files=[];}
 append(...children){this.children.push(...children);}
 replaceChildren(...children){this.children=children;}
 setAttribute(name,value){this[name]=value;}
 addEventListener(name,fn){this.listeners[name]=fn;}
 click(){this.onclick?.();}
}
function fixture({stored,brokenStorage=false}={}){
 const data={version:1,repository:'synthetic-repository',snapshot:'synthetic-content-fingerprint',generated_at:0,scanned:1,coverage_skips:0,warning_count:0,warnings:[],findings:[
  {hit:{file:'sample.txt',line_number:1,matched:'synthetic-word',source:'synthetic-source',kind:'watchlist'},change:'added',context:'1: synthetic-word'},
  {hit:{file:'sample.txt',line_number:2,matched:'192.168.50.9',source:'synthetic-pattern',kind:'structural'},change:'existing',context:'2: 192.168.50.9'}]};
 const elements=Object.fromEntries([...template.matchAll(/id="([^"]+)"/g)].map(m=>[m[1],new Element()]));elements.data.textContent=JSON.stringify(data);
 const saved=new Map();if(stored)saved.set('doxguard-review-v1:'+data.snapshot,JSON.stringify(stored));
 let download,clipboard;
 const context=vm.createContext({document:{getElementById:id=>elements[id],createElement:tag=>new Element(tag)},localStorage:{getItem:key=>{if(brokenStorage)throw Error('disabled');return saved.get(key)||null;},setItem:(k,v)=>{if(brokenStorage)throw Error('disabled');saved.set(k,v);}},Blob,URL:{createObjectURL:blob=>{download=blob;return 'blob:synthetic';},revokeObjectURL(){}},setTimeout:fn=>fn(),navigator:{clipboard:{writeText:async text=>{clipboard=text;}}}});
 vm.runInContext(script,context);
 return {elements,data,saved,getDownload:()=>download,getClipboard:()=>clipboard};
}
const decisions=data=>({version:1,repository:data.repository,snapshot:data.snapshot,answers:[{decision:'replace',note:'synthetic memo'},{decision:'hold',note:''}]});

test('search and filters operate on real rendered finding cards',()=>{
 const {elements:e}=fixture();assert.equal(e.list.children.length,2);
 e.change.value='added';e.change.listeners.input();assert.equal(e.list.children.length,1);
 e.change.value='';e.search.value='absent';e.search.listeners.input();assert.equal(e.list.children.length,0);assert.equal(e.empty.hidden,false);
});
test('decision and memo save restore and exclude values from AI summary',async()=>{
 const f=fixture(),e=f.elements;
 const label=e.list.children[0].children.find(c=>c.tag==='label'),select=label.children[0];select.value='public';select.listeners.change();
 const note=e.list.children[0].children.find(c=>c.tag==='textarea');note.value='private synthetic memo';note.listeners.input();e.save.onclick();
 const saved=JSON.parse([...f.saved.values()][0]);assert.equal(saved.answers[0].decision,'public');assert.equal(saved.answers[0].note,note.value);
 const restored=fixture({stored:saved});assert.match(restored.elements.status.textContent,/復元/);
 await e.copy.onclick();assert.match(f.getClipboard(),/公開可能/);assert.ok(!f.getClipboard().includes('synthetic-word'));assert.ok(!f.getClipboard().includes('private synthetic memo'));
});
test('JSON roundtrip validates fingerprint and rejects stale or malicious decisions',async()=>{
 const f=fixture(),e=f.elements;
 e.import.files=[{size:200,text:async()=>JSON.stringify(decisions(f.data))}];await e.import.onchange();assert.match(e.status.textContent,/読み込みました/);
 e.export.onclick();const exported=JSON.parse(await f.getDownload().text());assert.equal(exported.answers[0].decision,'replace');assert.ok(!JSON.stringify(exported).includes('192.168.50.9'));
 const stale={...exported,snapshot:'other-content'};e.import.files=[{size:200,text:async()=>JSON.stringify(stale)}];await e.import.onchange();assert.match(e.status.textContent,/一致しない/);
 e.export.onclick();assert.equal(JSON.parse(await f.getDownload().text()).answers[0].decision,'replace');
 const bad={...exported,answers:[{decision:'<script>',note:''},{decision:'hold',note:''}]};e.import.files=[{size:200,text:async()=>JSON.stringify(bad)}];await e.import.onchange();assert.match(e.status.textContent,/形式/);
});
test('disabled browser storage shows fallback and still allows JSON export',()=>{
 const f=fixture({brokenStorage:true});assert.match(f.elements.status.textContent,/利用できません/);f.elements.save.onclick();assert.match(f.elements.status.textContent,/失敗/);f.elements.export.onclick();assert.ok(f.getDownload());
});
