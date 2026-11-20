globalThis.rbDecodePieces = function(entries) {
  const cells = Array(64).fill(null);
  for (const entry of entries) {
    const classes = String(entry.classes).split(/\s+/);
    const piece = classes.find(c => /^[wb][pnbrqk]$/.test(c));
    const square = classes.find(c => /^square-[1-8][1-8]$/.test(c));
    if (!piece || !square) continue;
    const file = Number(square[7]) - 1, rank = Number(square[8]) - 1;
    const index = rank * 8 + file;
    if (cells[index]) throw Error('Waiting for the piece animation to finish.');
    cells[index] = piece[0] === 'w' ? piece[1].toUpperCase() : piece[1];
  }
  if (cells.filter(p => p === 'K').length !== 1 || cells.filter(p => p === 'k').length !== 1)
    throw Error('Waiting for a complete chessboard.');
  const rows = [];
  for (let rank = 7; rank >= 0; rank--) {
    let row = '', empty = 0;
    for (let file = 0; file < 8; file++) {
      const piece = cells[rank * 8 + file];
      if (piece) { if (empty) row += empty; empty = 0; row += piece; }
      else empty++;
    }
    if (empty) row += empty;
    rows.push(row);
  }
  return rows.join('/');
};
