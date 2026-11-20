(() => {
  globalThis.__rbChessBridge?.close();
  const coach = globalThis.__rbChessCoach;
  let socket, timer, closed = false;
  const connection = { close() { closed = true; clearInterval(timer); socket?.close(); if (globalThis.__rbChessBridge === connection) delete globalThis.__rbChessBridge; } };
  globalThis.__rbChessBridge = connection;
  coach.setStatus('Connecting to RbChessAssist on your computer…');
  try { socket = new WebSocket(RB_ENDPOINT); }
  catch (_) { coach.setStatus('Connection blocked. Start the app and allow Chrome’s local connection prompt, then click the bookmark again.'); return; }
  socket.addEventListener('open', () => {
    timer = setInterval(() => {
      if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(coach.snapshot()));
    }, 400);
  });
  socket.addEventListener('message', event => {
    let message; try { message = JSON.parse(event.data); } catch (_) { return; }
    coach.touch();
    switch (message.type) {
      case 'connected': coach.setEnabled(message.enabled); coach.setArrows(message.arrows); break;
      case 'enabled': coach.setEnabled(message.value); break;
      case 'arrows': coach.setArrows(message.value); break;
      case 'hint': coach.setHint(message.value); break;
      case 'status': coach.setStatus(message.message); break;
      case 'dispose': connection.close(); coach.dispose(); break;
    }
  });
  socket.addEventListener('close', () => { clearInterval(timer); if (!closed) coach.setStatus('Disconnected. Start RbChessAssist, then click the Connect bookmark again.'); });
  socket.addEventListener('error', () => coach.setStatus('Could not connect. Start RbChessAssist and allow Chrome’s local connection prompt. Then click the bookmark again.'));
})();
