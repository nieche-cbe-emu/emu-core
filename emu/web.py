
import json, threading, time, http.server, socketserver, urllib.parse

PAGE = """<!doctype html><html lang="zh"><head><meta charset="utf-8">
<title>%(name)s — CBE 模拟器</title>
<style>
 :root{color-scheme:dark}
 body{margin:0;background:#111;color:#ddd;font:13px/1.5 system-ui,sans-serif;
      display:flex;gap:20px;padding:20px;flex-wrap:wrap}
 canvas{image-rendering:pixelated;background:#000;border:1px solid #333;
        border-radius:6px;touch-action:none}
 #side{min-width:260px;max-width:420px}
 h1{font-size:15px;margin:0 0 12px}
 .k{display:inline-block;min-width:52px;margin:2px;padding:6px 8px;background:#222;
    border:1px solid #3a3a3a;border-radius:5px;cursor:pointer;user-select:none;
    text-align:center;font-size:12px}
 .k:active,.k.on{background:#2d5cff;border-color:#2d5cff;color:#fff}
 pre{background:#181818;border:1px solid #2a2a2a;border-radius:6px;padding:8px;
     max-height:240px;overflow:auto;font-size:11px;white-space:pre-wrap}
 .row{margin:10px 0}
 label{color:#888}
</style></head><body>
<div><canvas id="c" width="%(w)d" height="%(h)d" style="width:%(cw)dpx;height:%(ch)dpx"></canvas>
 <div class="row"><label>帧 <span id="fno">0</span> · <span id="fps">0</span> fps</label></div></div>
<div id="side">
 <h1>%(name)s <span style="color:#888;font-weight:400">%(w)d×%(h)d</span></h1>
 <div class="row"><label>按键（位掩码）—— 键位由模块自己定义，逐位试即可</label><br>
   <div id="keys"></div></div>
 <div class="row"><label>键盘：方向键=bit0-3，Z=bit4，X=bit5，回车=bit12，退格=bit14</label></div>
 <div class="row"><label>模块日志</label><pre id="log"></pre></div>
</div>
<script>
const cv=document.getElementById('c'),cx=cv.getContext('2d');
let last=-1,frames=0,t0=performance.now();
const held=new Set();

function send(){fetch('/input',{method:'POST',body:JSON.stringify(
  {keys:[...held].reduce((a,b)=>a|b,0)})});}

const keys=document.getElementById('keys');
for(let b=0;b<16;b++){
  const d=document.createElement('div');d.className='k';d.textContent='bit'+b;
  const m=1<<b;
  const on=e=>{e.preventDefault();held.add(m);d.classList.add('on');send();};
  const off=e=>{e.preventDefault();held.delete(m);d.classList.remove('on');send();};
  d.addEventListener('pointerdown',on);d.addEventListener('pointerup',off);
  d.addEventListener('pointerleave',off);
  keys.appendChild(d);
}
const KB={ArrowUp:1,ArrowDown:2,ArrowLeft:4,ArrowRight:8,KeyZ:16,KeyX:32,
          Enter:1<<12,Backspace:1<<14,Space:1<<5};
addEventListener('keydown',e=>{const m=KB[e.code];if(m){e.preventDefault();held.add(m);send();}});
addEventListener('keyup',e=>{const m=KB[e.code];if(m){e.preventDefault();held.delete(m);send();}});

function touch(e,st){const r=cv.getBoundingClientRect();
  const x=Math.round((e.clientX-r.left)*cv.width/r.width);
  const y=Math.round((e.clientY-r.top)*cv.height/r.height);
  fetch('/input',{method:'POST',body:JSON.stringify({touch:[x,y,st]})});}
cv.addEventListener('pointerdown',e=>{cv.setPointerCapture(e.pointerId);touch(e,'down')});
cv.addEventListener('pointermove',e=>{if(e.buttons)touch(e,'move')});
cv.addEventListener('pointerup',e=>touch(e,'up'));

// 服务端直传 RGB565 原始帧缓冲，转换放在这里做——
// 在 Python 里逐像素转 RGB888 要 47ms/帧，是整条链路唯一的瓶颈。
const SW=%(sw)d, SH=%(sh)d, ROT=%(rot)d;
const imgd=cx.createImageData(cv.width,cv.height);
function draw(u16){
  const d=imgd.data;
  for(let i=0,n=SW*SH;i<n;i++){
    const v=u16[i];
    const r=(v>>11)&31,g=(v>>5)&63,b=v&31;
    let x=i%%SW,y=(i/SW)|0,j;
    if(ROT===90)      j=((SW-1-x)*SH+y);
    else if(ROT===270)j=(x*SH+(SH-1-y));
    else if(ROT===180)j=(n-1-i);
    else              j=i;
    j<<=2;
    d[j]=(r<<3)|(r>>2); d[j+1]=(g<<2)|(g>>4); d[j+2]=(b<<3)|(b>>2); d[j+3]=255;
  }
  cx.putImageData(imgd,0,0);
}
async function pump(){
  for(;;){
    try{
      const r=await fetch('/raw?since='+last);
      if(r.status===200){
        last=+r.headers.get('X-Frame');
        draw(new Uint16Array(await r.arrayBuffer()));
        frames++;document.getElementById('fno').textContent=last;
        const dt=(performance.now()-t0)/1000;
        if(dt>0.5){document.getElementById('fps').textContent=(frames/dt).toFixed(1);
                   frames=0;t0=performance.now();}
      }
    }catch(e){await new Promise(r=>setTimeout(r,300));}
  }
}
async function logs(){for(;;){
  try{const s=await (await fetch('/state')).json();
      document.getElementById('log').textContent=s.logs.join('\\n');}catch(e){}
  await new Promise(r=>setTimeout(r,700));}}
pump();logs();
</script></body></html>"""

class Session:

    def __init__(self, rt, fps=20, rotate=0):
        self.rt = rt
        self.fps = fps
        self.rotate = rotate
        self.lock = threading.Lock()
        self.frame_no = 0
        self.raw = b""
        self.pending_keys = 0
        self.latched = 0
        self.pending_touch = None
        self.running = True

    def loop(self):
        rt = self.rt
        period = 1.0 / self.fps
        while self.running:
            t = time.time()
            with self.lock:
                bits = self.pending_keys | self.latched
                self.latched = 0
                rt.keys_down = rt.keys_hold = bits
                if self.pending_touch:
                    x, y, st = self.pending_touch
                    rt.pointer = (x, y)
                    rt.touch_down = 1 if st == "down" else 0
                    rt.touch_hold = 1 if st in ("down", "move") else 0
                    rt.touch_up = 1 if st == "up" else 0
                    rt.touch_drag = 1 if st == "move" else 0
                    if st == "up":
                        self.pending_touch = None
                    elif st == "down":

                        self.pending_touch = (x, y, "move")
                try:
                    rt.frame()
                except Exception as e:
                    rt.host_errors[("frame", type(e).__name__, str(e)[:60])] += 1
                self.raw = rt.fb.raw565()
                self.frame_no += 1
            time.sleep(max(0, period - (time.time() - t)))

def serve(rt, port=8777, fps=20, rotate=0, scale=2):
    sess = Session(rt, fps, rotate)
    threading.Thread(target=sess.loop, daemon=True).start()
    w, h = (rt.fb.h, rt.fb.w) if rotate in (90, 270) else (rt.fb.w, rt.fb.h)

    class H(http.server.BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def _send(self, code, ctype, body, extra=None):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            for k, v in (extra or {}).items():
                self.send_header(k, v)
            self.end_headers()
            if body:
                self.wfile.write(body)

        def do_GET(self):
            u = urllib.parse.urlparse(self.path)
            if u.path == "/":
                page = PAGE % dict(name=rt.mod.name, w=w, h=h, rot=rotate,
                                   sw=rt.fb.w, sh=rt.fb.h,
                                   cw=w * scale, ch=h * scale)
                self._send(200, "text/html; charset=utf-8", page.encode())
            elif u.path in ("/raw", "/frame"):
                since = int(urllib.parse.parse_qs(u.query).get("since", ["-1"])[0])
                deadline = time.time() + 2.0
                while time.time() < deadline:
                    with sess.lock:
                        if sess.frame_no != since and sess.raw:
                            raw, no = sess.raw, sess.frame_no
                            break
                    time.sleep(0.002)
                else:
                    self._send(204, "text/plain", b"")
                    return
                if u.path == "/raw":
                    self._send(200, "application/octet-stream", raw,
                               {"X-Frame": str(no), "Cache-Control": "no-store"})
                else:
                    with sess.lock:
                        png = rt.fb.write_png_bytes(rotate=sess.rotate)
                    self._send(200, "image/png", png,
                               {"X-Frame": str(no), "Cache-Control": "no-store"})
            elif u.path == "/state":
                with sess.lock:
                    body = json.dumps({"name": rt.mod.name, "w": w, "h": h,
                                       "frame": sess.frame_no,
                                       "logs": rt.logs[-25:]}).encode()
                self._send(200, "application/json", body)
            else:
                self._send(404, "text/plain", b"")

        def do_POST(self):
            n = int(self.headers.get("Content-Length", 0))
            try:
                d = json.loads(self.rfile.read(n) or b"{}")
            except Exception:
                d = {}
            with sess.lock:
                if "keys" in d:
                    k = int(d["keys"])
                    sess.latched |= k & ~sess.pending_keys
                    sess.pending_keys = k
                if "touch" in d:
                    x, y, st = d["touch"]
                    sess.pending_touch = (int(x), int(y), st)
            self._send(200, "application/json", b"{}")

    socketserver.TCPServer.allow_reuse_address = True
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", port), H)
    return sess, srv
