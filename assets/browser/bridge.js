(() => {
  globalThis.__rbChessBridge?.close();
  const coach = globalThis.__rbChessCoach;
  let socket, sender, retry, deadline, closed = false, everConnected = false;
  function flush() {
    if (socket?.readyState === WebSocket.OPEN) socket.send(JSON.stringify(coach.snapshot()));
  }
  function connect() {
    if (closed || socket?.readyState === WebSocket.OPEN || socket?.readyState === WebSocket.CONNECTING) return;
    clearTimeout(retry);
    coach.setStatus('Connecting to RbChessAssist...');
    try { socket = new WebSocket(RB_ENDPOINT); }
    catch (_) { coach.setStatus('Open RbChessAssist and click Start helping. Then use Start helping here to reconnect.'); return; }
    const current = socket;
    deadline = setTimeout(() => { if (current.readyState === WebSocket.CONNECTING) current.close(); }, 8000);
    current.addEventListener('open', () => {
      if (closed || current !== socket) return;
      clearTimeout(deadline);everConnected=true;coach.touch();flush();
      sender = setInterval(flush, 250);
    });
    current.addEventListener('message', event => {
      if (closed || current !== socket) return;
      let message;try { message=JSON.parse(event.data); } catch (_) { return; }
      coach.touch();
      switch(message.type) {
        // Preserve the website's Start/Stop choice across reconnections.
        case 'connected':
          if(typeof RB_AUTOMATIC !== 'undefined' && RB_AUTOMATIC)coach.setEnabled(message.enabled);
          coach.setArrows(message.arrows);flush();break;
        case 'enabled': coach.setEnabled(message.value);flush();break;
        case 'arrows': coach.setArrows(message.value);break;
        case 'hint': coach.setHint(message.value);break;
        case 'status': coach.setStatus(message.message);break;
        case 'dispose': connection.close();coach.dispose();break;
      }
    });
    current.addEventListener('close', () => {
      if (current !== socket) return;
      clearInterval(sender);clearTimeout(deadline);socket=null;
      if (closed) return;
      coach.setStatus('Open RbChessAssist and click Start helping. This tab will reconnect automatically.');
      if (everConnected || (typeof RB_AUTOMATIC !== 'undefined' && RB_AUTOMATIC)) retry=setTimeout(connect,2000);
    });
    current.addEventListener('error', () => {
      coach.setStatus('Cannot reach the app. Open RbChessAssist, click Start helping, and allow Chrome local connection permission.');
    });
  }
  const connection = {
    flush, ensureConnected:connect,
    close() { closed=true;clearInterval(sender);clearTimeout(retry);clearTimeout(deadline);socket?.close();if(globalThis.__rbChessBridge===connection)delete globalThis.__rbChessBridge; }
  };
  globalThis.__rbChessBridge=connection;
  if (typeof RB_AUTOMATIC !== 'undefined' && RB_AUTOMATIC) {
    navigator.permissions.query({name:'loopback-network'}).then(permission => {
      if(permission.state==='granted')connect();
      else coach.setStatus('One-time connection: click Start helping, then allow access to the app on this computer.');
    }).catch(() => coach.setStatus('One-time connection: click Start helping to connect to the desktop app.'));
    coach.setEnabled(false);
  } else connect();
})();
