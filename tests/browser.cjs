const assert=require('node:assert/strict');
const fs=require('node:fs');const vm=require('node:vm');const path=require('node:path');
const decoder=fs.readFileSync(path.join(__dirname,'../assets/browser/decoder.js'),'utf8');
const coach=fs.readFileSync(path.join(__dirname,'../assets/browser/coach.js'),'utf8');
const initial='rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR';
let flipped=false, now=0, tick, nextFen=initial+' w KQkq - 0 1';
const pieces=[];const rect={left:200,top:100,width:600,height:600};
function piece(kind,file,rank){return {className:`piece ${kind} square-${file}${rank}`,get classList(){return this.className.split(' ');},getBoundingClientRect(){const sq=this.classList.at(-1),f=+sq[7]-1,r=+sq[8]-1,x=flipped?7-f:f,y=flipped?r:7-r;return {left:200+x*75,right:275+x*75,top:100+y*75,bottom:175+y*75,width:75,height:75};}};}
for(let f=1;f<=8;f++){pieces.push(piece('w'+'rnbqkbnr'[f-1],f,1),piece('wp',f,2),piece('bp',f,7),piece('b'+'rnbqkbnr'[f-1],f,8));}
const nodes=[];
class Node {constructor(tag){this.tag=tag;this.style={};this.attrs={};this.children=[];nodes.push(this);}setAttribute(k,v){this.attrs[k]=String(v);}append(...n){this.children.push(...n);}replaceChildren(...n){this.children=n;}addEventListener(k,fn){this[k]=fn;}remove(){this.removed=true;}}
const board={getBoundingClientRect:()=>rect,querySelectorAll:()=>pieces,game:{getFEN:()=>nextFen}};
const context=vm.createContext({location:{hostname:'www.chess.com'},document:{querySelectorAll:()=>[board],createElement:t=>new Node(t),createElementNS:(_,t)=>new Node(t),documentElement:new Node('html')},Date:{now:()=>now},setInterval:fn=>{tick=fn;return 1;},clearInterval:()=>{}});
vm.runInContext(decoder,context);vm.runInContext(coach,context);
const api=context.__rbChessCoach;
const detailToggle=nodes.find(n=>n.textContent==='Details');
assert.equal(detailToggle.attrs['aria-expanded'],'false');detailToggle.click();assert.equal(detailToggle.attrs['aria-expanded'],'true');detailToggle.click();assert.equal(detailToggle.attrs['aria-expanded'],'false');
const deeper=nodes.find(n=>n.textContent==='Analyze deeper · 15 seconds');deeper.click();assert.equal(api.snapshot().deep,1);
console.log('PASS: Details toggle and deeper-analysis request');
assert.equal(api.snapshot().placement,initial);assert.equal(api.snapshot().fen,nextFen);
assert.throws(()=>context.rbDecodePieces([...pieces.map(p=>({classes:p.className})),{classes:pieces[0].className}]),/animation/);
assert.throws(()=>context.rbDecodePieces([]),/complete/);
console.log('PASS: board decoding, duplicates, and missing kings');
const svg=nodes.find(n=>n.id==='rbchessassist-arrow'),line=nodes.find(n=>n.tag==='line');
const hint={placement:initial,uci:'e2e4',san:'e4',evaluation:'+0.2',details:'Move your pawn from e2 to e4.'};
hint.arrows=[{uci:'e2e4'},{uci:'e7e5'},{uci:'g1f3'}];
api.setHint(hint);assert.equal(svg.style.display,'block');assert.equal(svg.style.left,'200px');assert.equal(line.attrs.x1,'450');assert.equal(line.attrs.y1,'650');assert.equal(line.attrs.y2,'462');
console.log('PASS: arrow attaches to the browser board coordinates');
const extra=nodes.find(n=>n.id==='rbchessassist-detail-arrows');
assert.equal(extra.children.length,0);detailToggle.click();
assert.deepEqual(extra.children.filter(n=>n.tag==='line').map(n=>n.attrs.stroke),['#ff647c','#65a7ff']);
assert.deepEqual(extra.children.filter(n=>n.tag==='text').map(n=>n.textContent),['1','2','3']);
detailToggle.click();assert.equal(extra.children.length,0);
deeper.disabled=false;deeper.click();const request=api.snapshot().deep;deeper.click();assert.equal(api.snapshot().deep,request);
api.setStatus('Analyzing deeper (up to 15 seconds)...');assert.equal(svg.style.display,'block');
console.log('PASS: colored numbered continuation, Details visibility, and repeated deep-search prevention');
flipped=true;tick();assert.equal(api.snapshot().flipped,true);assert.equal(line.attrs.x1,'350');assert.equal(line.attrs.y1,'150');
api.setEnabled(false);api.setHint(hint);assert.equal(svg.style.display,'none');assert.equal(api.snapshot().paused,true);
api.setEnabled(true);api.setHint(hint);api.setArrows(false);assert.equal(svg.style.display,'none');api.setArrows(true);
console.log('PASS: rotated board and website Start/Stop controls');
pieces.find(p=>p.className==='piece wp square-52').className='piece wp square-54';tick();assert.equal(svg.style.display,'none');assert.equal(api.snapshot().fen,null);
console.log('PASS: changing position clears stale arrows and rejects stale FEN');
pieces.find(p=>p.className==='piece wp square-54').className='piece wp square-52';api.setHint(hint);api.touch();now=8000;tick();assert.equal(svg.style.display,'none');api.dispose();assert.equal(context.__rbChessCoach,undefined);assert.equal(svg.removed,true);
console.log('PASS: disconnect timeout and cleanup');
const other=vm.createContext({location:{hostname:'not-chess.com'}});vm.runInContext(coach,other);assert.equal(other.__rbChessCoach,undefined);
console.log('PASS: no injection on unrelated websites');
const bridge=fs.readFileSync(path.join(__dirname,'../assets/browser/bridge.js'),'utf8');
const sockets=[],timeouts=new Map();let timerId=0,paused=true;
class Socket {
  static OPEN=1;static CONNECTING=0;
  constructor(){this.readyState=0;this.handlers={};this.sent=[];sockets.push(this);}
  addEventListener(k,fn){this.handlers[k]=fn;}
  send(v){this.sent.push(JSON.parse(v));}
  close(){this.readyState=3;this.handlers.close?.();}
  open(){this.readyState=1;this.handlers.open();this.handlers.message({data:JSON.stringify({type:'connected',enabled:true,arrows:true})});}
}
const bridgeContext=vm.createContext({RB_ENDPOINT:'ws://127.0.0.1/test',WebSocket:Socket,
  __rbChessCoach:{snapshot:()=>({paused}),setStatus(){},touch(){},setArrows(){},setEnabled(v){paused=!v;}},
  setInterval:()=>1,clearInterval(){},setTimeout:(fn,ms)=>{timeouts.set(++timerId,{fn,ms});return timerId;},clearTimeout:id=>timeouts.delete(id)});
vm.runInContext(bridge,bridgeContext);sockets[0].open();assert.equal(sockets[0].sent.at(-1).paused,true);
paused=false;bridgeContext.__rbChessBridge.flush();assert.equal(sockets[0].sent.at(-1).paused,false);
sockets[0].close();[...timeouts.values()].find(t=>t.ms===2000).fn();assert.equal(sockets.length,2);
sockets[1].open();assert.equal(sockets[1].sent.at(-1).paused,false);
bridgeContext.__rbChessBridge.close();assert.equal(bridgeContext.__rbChessBridge,undefined);
console.log('PASS: immediate controls, automatic reconnect, and preserved Start/Stop choice');
