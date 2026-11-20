(() => {
  if (globalThis.__rbChessCoach?.version === 2) return;
  globalThis.__rbChessCoach?.dispose();
  if (!/(^|\.)chess\.com$/.test(location.hostname)) return;
  const NS = 'http://www.w3.org/2000/svg';
  let enabled = true, arrows = true, hint = null, currentBoard = null, stopped = false, heartbeat = Date.now(), deep = 0;
  const overlay = document.createElementNS(NS, 'svg');
  overlay.id = 'rbchessassist-arrow';
  overlay.style.cssText = 'position:fixed;pointer-events:none;z-index:2147483646;display:none;overflow:visible';
  overlay.setAttribute('viewBox', '0 0 800 800');
  const defs = document.createElementNS(NS, 'defs');
  const marker = document.createElementNS(NS, 'marker');
  marker.id = 'rbchessassist-tip';
  for (const [key,value] of Object.entries({markerWidth:5,markerHeight:5,refX:3.8,refY:2.5,orient:'auto',markerUnits:'strokeWidth'})) marker.setAttribute(key,value);
  const triangle = document.createElementNS(NS,'path'); triangle.setAttribute('d','M0,0 L5,2.5 L0,5 Z');triangle.setAttribute('fill','#17e7cf');marker.append(triangle);defs.append(marker);overlay.append(defs);
  const line = document.createElementNS(NS,'line');
  for (const [key,value] of Object.entries({stroke:'#17e7cf','stroke-width':10,'stroke-linecap':'round','marker-end':'url(#rbchessassist-tip)'})) line.setAttribute(key,value);
  line.style.filter='drop-shadow(0 2px 2px #000b)'; overlay.append(line);
  const ring=document.createElementNS(NS,'circle');ring.setAttribute('r',22);ring.setAttribute('fill','none');ring.setAttribute('stroke','#17e7cf');ring.setAttribute('stroke-width',5);overlay.append(ring);
  const panel=document.createElement('section'); panel.id='rbchessassist-coach';
  panel.style.cssText='position:fixed;right:16px;bottom:16px;width:300px;max-width:85vw;max-height:75vh;overflow:auto;box-sizing:border-box;padding:16px;background:#111d29;color:#eef6ff;border:1px solid #54cfc2;border-radius:12px;font:14px/1.5 system-ui,sans-serif;z-index:2147483647;box-shadow:0 6px 25px #0007';
  const title=document.createElement('strong');title.textContent='RbChessAssist';title.style.color='#65dfd2';
  const button=document.createElement('button');button.type='button';button.textContent='Stop helping';button.style.cssText='display:block;width:100%;margin-top:12px;background:#65dfd2;color:#0e1923;padding:10px;border:0;border-radius:6px;cursor:pointer;font:600 14px system-ui';
  const detailButton=document.createElement('button');detailButton.type='button';detailButton.textContent='Details';detailButton.style.cssText='display:block;width:100%;margin-top:8px;background:#24384a;color:#eef6ff;padding:9px;border:1px solid #506476;border-radius:6px;cursor:pointer;font:600 14px system-ui';detailButton.setAttribute('aria-expanded','false');
  const status=document.createElement('div');status.textContent='Connected. Open a game to see a suggested move.';status.style.marginTop='10px';
  const details=document.createElement('div');details.hidden=true;
  const body=document.createElement('div');body.style.cssText='white-space:pre-line;margin-top:12px';
  const deeper=document.createElement('button');deeper.type='button';deeper.textContent='Analyze deeper · 15 seconds';deeper.style.cssText='width:100%;margin-top:12px;padding:9px;border:1px solid #65dfd2;border-radius:6px;background:#132c35;color:#b9fff5;cursor:pointer';deeper.addEventListener('click',()=>{deep++;status.textContent='Requesting deeper analysis…';});
  const sources=document.createElement('div');sources.style.cssText='margin-top:14px;font-size:12px';
  details.append(body,deeper,sources);panel.append(title,button,detailButton,status,details);document.documentElement.append(overlay,panel);
  detailButton.addEventListener('click',()=>{details.hidden=!details.hidden;detailButton.textContent=details.hidden?'Details':'Hide details';detailButton.setAttribute('aria-expanded',String(!details.hidden));});
  function clear() {hint=null;overlay.style.display='none';body.textContent='';sources.replaceChildren();}
  function setEnabled(value) {enabled=!!value;clear();button.textContent=enabled?'Stop helping':'Start helping';status.textContent=enabled?'Reading the board…':'Stopped. Click Start helping to resume.';}
  button.addEventListener('click',()=>setEnabled(!enabled));
  function read() {
    if (!enabled) return {paused:true,deep};
    const boards=[...document.querySelectorAll('wc-chess-board,chess-board,.chessboard')];
    const board=boards.find(b=>{const r=b.getBoundingClientRect();return r.width>100&&r.height>100&&b.querySelectorAll('.piece').length>1;});
    if (!board) throw Error('Open your Chess.com game in this tab.');
    const pieces=[...board.querySelectorAll('.piece')].filter(p=>{const r=p.getBoundingClientRect();return r.width>0&&r.height>0;});
    const placement=globalThis.rbDecodePieces(pieces.map(p=>({classes:p.className})));
    let fen=null;
    try { const game=board.game; const value=typeof game?.getFEN==='function'?game.getFEN():typeof game?.getFen==='function'?game.getFen():board.getAttribute('fen'); if(typeof value==='string'&&value.split(' ')[0]===placement)fen=value; } catch (_) {}
    const rect=board.getBoundingClientRect();let normal=0,reverse=0;
    for(const piece of pieces){const square=[...piece.classList].find(c=>/^square-[1-8][1-8]$/.test(c));if(!square)continue;const f=Number(square[7])-1,r=Number(square[8])-1,p=piece.getBoundingClientRect(),x=((p.left+p.right)/2-rect.left)/rect.width*8,y=((p.top+p.bottom)/2-rect.top)/rect.height*8;normal+=(x-f-.5)**2+(y-(7-r)-.5)**2;reverse+=(x-(7-f)-.5)**2+(y-r-.5)**2;}
    const flipped=reverse<normal;
    currentBoard=board;
    return {placement,fen,flipped,deep};
  }
  function draw(snapshot) {
    if(!hint||!enabled||!arrows||snapshot.placement!==hint.placement||!currentBoard){overlay.style.display='none';return;}
    const r=currentBoard.getBoundingClientRect();
    overlay.style.left=r.left+'px';overlay.style.top=r.top+'px';overlay.style.width=r.width+'px';overlay.style.height=r.height+'px';
    const point=s=>{const f=s.charCodeAt(0)-97,k=Number(s[1])-1;return snapshot.flipped?[(7-f)*100+50,k*100+50]:[f*100+50,(7-k)*100+50];};
    const [ax,ay]=point(hint.uci.slice(0,2)),[bx,by]=point(hint.uci.slice(2,4));
    line.setAttribute('x1',ax);line.setAttribute('y1',ay);line.setAttribute('x2',ax+(bx-ax)*.85);line.setAttribute('y2',ay+(by-ay)*.85);ring.setAttribute('cx',ax);ring.setAttribute('cy',ay);overlay.style.display='block';
  }
  function snapshot(){try{return read();}catch(e){return {error:String(e.message||e)};}}
  function setHint(value){hint=value;status.textContent=value?`${value.san}  ·  ${value.evaluation}`:'Analyzing the position…';body.textContent=value?value.details:'';sources.replaceChildren();if(value){
    if(value.references?.length){const heading=document.createElement('strong');heading.textContent='Related reading from the local library';sources.append(heading);const note=document.createElement('p');note.textContent='Historical book passages about related concepts. These are not analysis of this exact position; older notation and rules may differ.';sources.append(note);for(const ref of value.references){const block=document.createElement('blockquote');block.style.cssText='margin:12px 0;padding-left:10px;border-left:2px solid #65dfd2';block.textContent=ref.passage;const link=document.createElement('a');link.textContent=ref.title+' — '+ref.author;link.href=ref.url;link.target='_blank';link.rel='noopener noreferrer';link.style.color='#81ddf2';sources.append(block,link);}}
    draw(snapshot());}else overlay.style.display='none';}
  function setStatus(message){clear();status.textContent=String(message);}
  const timer=setInterval(()=>{if(stopped)return;if(Date.now()-heartbeat>7000){clear();status.textContent='App disconnected. Open RbChessAssist and click Start helping.';return;}const state=snapshot();if(state.error||state.paused){overlay.style.display='none';if(state.error)status.textContent=state.error;return;}if(hint&&hint.placement!==state.placement){clear();status.textContent='Position changed. Analyzing…';}draw(state);},160);
  globalThis.__rbChessCoach={version:2,snapshot,setHint,setStatus,setEnabled,setArrows(value){arrows=!!value;if(!arrows)overlay.style.display='none';},touch(){heartbeat=Date.now();},dispose(){stopped=true;clearInterval(timer);globalThis.__rbChessBridge?.close();overlay.remove();panel.remove();delete globalThis.__rbChessCoach;}};
})();
