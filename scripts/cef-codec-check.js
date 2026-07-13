#!/usr/bin/env node
//
// Verifica se o CEF em execução expõe os codecs proprietários (H.264/AAC) que o
// vídeo do WhatsApp Web exige. Fala com o DevTools/CDP do Syltr.
//
// Uso:
//   1) rode o Syltr com DevTools remoto:  SYLTR_DEBUG=1 syltr
//   2) node scripts/cef-codec-check.js [filtro-de-pagina]
//
// [filtro-de-pagina] casa por substring no title/url do alvo (default: whatsapp).
// Sem deps externas: cliente WebSocket mínimo sobre socket cru.
//
// Esperado num build COM codecs:  h264 → "probably", mse_h264 → true
// Build codec-free (default):     h264 → "",          mse_h264 → false

const net = require('net');
const http = require('http');
const crypto = require('crypto');

const HOST = process.env.CDP_HOST || '127.0.0.1';
const PORT = Number(process.env.CDP_PORT || 9222);
const FILTER = (process.argv[2] || 'whatsapp').toLowerCase();

const PROBE = `JSON.stringify({
  h264: document.createElement('video').canPlayType('video/mp4; codecs="avc1.42E01E, mp4a.40.2"'),
  aac: document.createElement('video').canPlayType('audio/mp4; codecs="mp4a.40.2"'),
  mp4_generic: document.createElement('video').canPlayType('video/mp4'),
  vp9: document.createElement('video').canPlayType('video/webm; codecs="vp9, opus"'),
  mse_h264: (window.MediaSource ? MediaSource.isTypeSupported('video/mp4; codecs="avc1.42E01E, mp4a.40.2"') : 'noMSE')
})`;

function getJSON(path) {
  return new Promise((res, rej) => {
    http.get({ host: HOST, port: PORT, path }, (r) => {
      let d = '';
      r.on('data', (c) => (d += c));
      r.on('end', () => { try { res(JSON.parse(d)); } catch (e) { rej(e); } });
    }).on('error', rej);
  });
}

function evaluate(wsUrl, expr) {
  return new Promise((resolve, reject) => {
    const u = new URL(wsUrl);
    const key = crypto.randomBytes(16).toString('base64');
    const sock = net.connect(Number(u.port), u.hostname, () => {
      sock.write(
        `GET ${u.pathname} HTTP/1.1\r\nHost: ${u.host}\r\n` +
        `Upgrade: websocket\r\nConnection: Upgrade\r\n` +
        `Sec-WebSocket-Key: ${key}\r\nSec-WebSocket-Version: 13\r\n\r\n`
      );
    });

    let handshaken = false, buf = Buffer.alloc(0), id = 0;
    const pending = new Map();

    const send = (obj) => {
      const p = Buffer.from(JSON.stringify(obj));
      const n = p.length;
      let h;
      if (n < 126) h = Buffer.from([0x81, 0x80 | n]);
      else if (n < 65536) { h = Buffer.alloc(4); h[0] = 0x81; h[1] = 0x80 | 126; h.writeUInt16BE(n, 2); }
      else { h = Buffer.alloc(10); h[0] = 0x81; h[1] = 0x80 | 127; h.writeBigUInt64BE(BigInt(n), 2); }
      const m = crypto.randomBytes(4), b = Buffer.from(p);
      for (let i = 0; i < b.length; i++) b[i] ^= m[i % 4];
      sock.write(Buffer.concat([h, m, b]));
    };
    const cmd = (method, params = {}) =>
      new Promise((r) => { const mid = ++id; pending.set(mid, r); send({ id: mid, method, params }); });

    const parse = () => {
      while (buf.length >= 2) {
        let len = buf[1] & 0x7f, off = 2;
        if (len === 126) { if (buf.length < 4) return; len = buf.readUInt16BE(2); off = 4; }
        else if (len === 127) { if (buf.length < 10) return; len = Number(buf.readBigUInt64BE(2)); off = 10; }
        if (buf.length < off + len) return;
        const payload = buf.slice(off, off + len);
        buf = buf.slice(off + len);
        try {
          const msg = JSON.parse(payload.toString());
          if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
        } catch { /* ignore non-json / control frames */ }
      }
    };

    sock.on('data', async (d) => {
      if (!handshaken) {
        buf = Buffer.concat([buf, d]);
        const i = buf.indexOf('\r\n\r\n');
        if (i === -1) return;
        handshaken = true;
        buf = buf.slice(i + 4);
        await cmd('Runtime.enable');
        const r = await cmd('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
        sock.end();
        // CDP nests: msg.result (evaluate) → .result (RemoteObject) → .value
        resolve(r.result && r.result.result && r.result.result.value);
      } else {
        buf = Buffer.concat([buf, d]);
        parse();
      }
    });
    sock.on('error', reject);
    setTimeout(() => { sock.destroy(); reject(new Error('timeout')); }, 8000);
  });
}

(async () => {
  let targets;
  try {
    targets = await getJSON('/json/list');
  } catch (e) {
    console.error(`Não consegui falar com o CDP em ${HOST}:${PORT}. Rode o Syltr com SYLTR_DEBUG=1.`);
    process.exit(1);
  }
  const page = targets.find(
    (t) => t.type === 'page' && t.webSocketDebuggerUrl &&
      ((t.url || '').toLowerCase().includes(FILTER) || (t.title || '').toLowerCase().includes(FILTER))
  );
  if (!page) {
    console.error(`Nenhuma página casando "${FILTER}". Páginas abertas:`);
    targets.filter((t) => t.type === 'page').forEach((t) => console.error(`  - ${t.title} | ${t.url}`));
    process.exit(1);
  }
  console.log(`Alvo: ${page.title} (${page.url})\n`);
  const raw = await evaluate(page.webSocketDebuggerUrl, PROBE);
  const r = JSON.parse(raw);
  const ok = r.h264 && r.h264 !== '' && r.mse_h264 === true;
  console.log(`  H.264 canPlayType : ${JSON.stringify(r.h264)}`);
  console.log(`  AAC   canPlayType : ${JSON.stringify(r.aac)}`);
  console.log(`  MSE H.264 support : ${r.mse_h264}`);
  console.log(`  video/mp4 (gen.)  : ${JSON.stringify(r.mp4_generic)}`);
  console.log(`  VP9/WebM          : ${JSON.stringify(r.vp9)}`);
  console.log(`\n${ok ? '✓ Codecs proprietários ATIVOS — vídeo do WhatsApp deve tocar.'
                     : '✗ Sem H.264/AAC — build codec-free (rode scripts/build-cef-codecs.sh).'}`);
  process.exit(ok ? 0 : 2);
})().catch((e) => { console.error(e.message || e); process.exit(1); });
