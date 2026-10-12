CSS='''
body{background:radial-gradient(1200px 700px at 80% 0%,#1d1700 0%,#07070a 55%,#000 100%)}
.y{color:#F5C400}
.br{width:1480px;height:880px;display:flex;flex-direction:column}
.tabs{display:flex;gap:4px;padding:10px 12px 0;background:#000}
.t{display:flex;align-items:center;gap:8px;padding:10px 14px;border-radius:10px 10px 0 0;font-size:13px;color:#A09040;width:230px}
.t.a{background:#161616;color:#F2F2F2}
.bar{display:flex;align-items:center;gap:10px;padding:10px 14px;background:#161616;border-bottom:1px solid #2A2A2A}
.nb{width:32px;height:32px;border-radius:8px;display:grid;place-items:center;color:#A09040;font-size:16px}
.url{flex:1;display:flex;align-items:center;gap:10px;background:#000;border:1px solid #2A2A2A;border-radius:10px;padding:9px 14px;font-size:14px}
.content{flex:1;background:#0A0A0A;position:relative;display:flex}
.status{display:flex;gap:18px;padding:6px 14px;font-size:12px;color:#A09040;background:#000;border-top:1px solid #2A2A2A}
.sw{width:36px;height:20px;border-radius:999px;background:#F5C400;position:relative;flex:none}.sw:after{content:"";position:absolute;right:3px;top:3px;width:14px;height:14px;border-radius:50%;background:#111}
.sw.off{background:#333}.sw.off:after{left:3px;right:auto;background:#777}
.row{display:flex;align-items:center;gap:14px;padding:12px 0;border-bottom:1px solid #1c1c1c;font-size:14px}
.row .k{font-size:12px;color:#6f6a50}
'''
def tabs(active,list_):
    s='<div class="tabs">'
    for i,(icon,t,p) in enumerate(list_):
        s+=f'<div class="t {"a" if i==active else ""}">{p}<span style="overflow:hidden;white-space:nowrap">{t}</span></div>'
    return s+'<div class="t" style="width:auto">＋</div></div>'
TOR='<span class="pill tor" style="padding:2px 7px;font-size:10px">TOR</span>'
I2P='<span class="pill i2p" style="padding:2px 7px;font-size:10px">I2P</span>'
DIR='<span class="pill" style="padding:2px 7px;font-size:10px;border:1px solid #333;color:#aaa">DIRECT</span>'
TL=[('','New tab',DIR),('','Onion library',TOR),('','eepsite forum',I2P)]
def bar(url,pill):
    return f'<div class="bar"><div class="nb">←</div><div class="nb">→</div><div class="nb">↻</div><div class="url">{pill}<span class="mono">{url}</span><span class="sp" style="flex:1"></span><span class="y">🛡 12</span></div><div class="nb y">☰</div></div>'
STATUS='<div class="status mono"><span class="y">● JS off</span><span>HTTPS-only</span><span>letterbox 1400×700</span><span>blocker on</span><span style="flex:1"></span><span>no telemetry · no account</span></div>'
# 1 new tab
nt=f'''<div class="content" style="flex-direction:column;align-items:center;justify-content:center">
<div style="font-size:64px;font-weight:800;letter-spacing:.02em">Fri<span class="y">hart</span></div>
<div style="color:#A09040;margin:8px 0 30px">Your browser. Your rules. Nothing phones home.</div>
<div style="width:680px;display:flex;align-items:center;gap:12px;background:#000;border:1px solid #3a3000;border-radius:14px;padding:16px 20px;color:#777;box-shadow:0 0 40px rgba(245,196,0,.08)">🔍 Search privately or enter address<span style="flex:1"></span><span class="mono" style="font-size:12px">no suggestions sent</span></div>
<div style="display:flex;gap:16px;margin-top:36px">'''+''.join(f'<div style="width:200px;background:#121212;border:1px solid #242424;border-radius:14px;padding:18px"><div class="y" style="font-size:22px">{a}</div><div style="font-weight:600;margin-top:10px">{b}</div><div style="color:#888;font-size:12px;margin-top:4px" class="mono">{c}</div></div>' for a,b,c in [('◎','New Tor tab','fail-closed'),('◇','New I2P tab','about:i2p'),('▣','Containers','about:containers'),('⌫','Shred site data','about:shred')])+'''</div>
<div class="mono" style="margin-top:40px;font-size:12px;color:#6f6a50">0 trackers followed you here · no sponsored tiles</div></div>'''
open('fh1.html','w').write(H.format(CSS,f'<div class="win br">{tabs(0,TL)}{bar("about:newtab",DIR)}{nt}{STATUS}</div>'))
# 2 browsing tor + shields
page='''<div style="flex:1;padding:28px;display:flex;justify-content:center;background:#050505"><div style="width:820px;background:#0E0E0E;border:1px dashed #2a2a2a;padding:36px 44px;position:relative">
<div class="mono" style="position:absolute;top:-1px;right:8px;font-size:10px;color:#555;transform:translateY(-100%)">letterboxed 1400×700</div>
<div style="font-size:30px;font-weight:700">The Onion Library</div><div style="color:#888;margin:6px 0 22px">Public-domain texts, mirrored over Tor</div>'''+''.join(f'<div style="padding:14px 0;border-bottom:1px solid #1d1d1d"><div class="y" style="font-weight:600">{t}</div><div style="color:#b8b8b8;font-size:14px;margin-top:4px">{d}</div></div>' for t,d in [('On Liberty — J. S. Mill','HTML · 412 KB · rendered with the Frihart HTML subset'),('Areopagitica — J. Milton','HTML · 96 KB'),('The Federalist Papers','HTML · 1.2 MB'),('Two Treatises of Government','HTML · 640 KB')])+'</div></div>'
sh='''<div style="width:360px;background:#111;border-left:1px solid #2a2a2a;padding:22px">
<div style="display:flex;align-items:center;gap:10px"><span class="y" style="font-size:22px">🛡</span><div style="font-weight:700;font-size:17px">Privacy shields</div></div>
<div style="margin:16px 0;padding:14px;border-radius:12px;background:rgba(155,110,255,.08);border:1px solid rgba(155,110,255,.35)"><div style="color:#C4A8FF;font-weight:600">◎ Tor circuit · fail-closed</div><div style="color:#aaa;font-size:12px;margin-top:6px">If Tor drops, this tab stops. No clearnet fallback. DNS resolved inside Tor, never locally.</div></div>'''+''.join(f'<div class="row"><div style="flex:1">{a}<div class="k">{b}</div></div><div class="sw {c}"></div></div>' for a,b,c in [('Tracker blocker','12 requests blocked',''),('Uniform identity headers','frozen UA · en · Sec-GPC',''),('Letterboxing','page sees 1400×700',''),('JavaScript','no engine · off','off'),('Third-party cookies','blocked','off'),('Strip tracking params','utm_*, fbclid…','')])+'<div class="mono" style="margin-top:16px;font-size:11px;color:#6f6a50">container: Reading · HTTPS-only · referrer origin-only</div></div>'
open('fh2.html','w').write(H.format(CSS,f'<div class="win br">{tabs(1,TL)}{bar("http://library…onion/",TOR)}<div class="content">{page}{sh}</div>{STATUS}</div>'))
# 3 settings
nav=''.join(f'<div style="padding:10px 14px;border-radius:10px;{"background:#1a1600;color:#F5C400" if i==1 else "color:#aaa"}">{n}</div>' for i,n in enumerate(['General','Privacy','Networks · Tor / I2P','Search','Containers','Blocker','Forced off']))
rows=''.join(f'<div class="row"><div style="flex:1">{a}<div class="k mono">{k}</div></div><div class="sw {c}"></div></div>' for a,k,c in [('HTTPS-only','privacy.https_only',''),('Resist fingerprinting','privacy.resist_fingerprinting',''),('Letterboxing','privacy.letterboxing',''),('Strip tracking parameters','privacy.strip_tracking',''),('Partition first-party state','privacy.partition_first_party_state',''),('Send Global Privacy Control','privacy.send_gpc',''),('WebRTC','privacy.webrtc','off'),('Search suggestions','privacy.search_suggestions','off'),('Clear data on shutdown','privacy.clear_on_shutdown','off')])
forced='<div style="margin-top:22px;padding:16px;border-radius:12px;border:1px solid #3a2020;background:rgba(192,64,64,.06)"><div style="color:#ff8a8a;font-weight:600">Forced off — not a setting</div><div class="mono" style="color:#aaa;font-size:12px;margin-top:6px">telemetry · safe_browsing (Google) · DRM/EME · stored logins</div></div>'
st=f'<div class="content"><div style="width:280px;padding:22px;border-right:1px solid #2a2a2a;background:#0c0c0c"><div style="font-weight:800;font-size:20px;margin-bottom:16px">Settings</div>{nav}</div><div style="flex:1;padding:28px 40px;overflow:hidden"><div style="font-size:24px;font-weight:700">Privacy</div><div style="color:#888;margin:6px 0 14px">Every default is documented in docs/defaults.md.</div>{rows}{forced}</div></div>'
open('fh3.html','w').write(H.format(CSS,f'<div class="win br">{tabs(0,[("","Settings",DIR)]+TL[1:])}{bar("about:settings",DIR)}{st}{STATUS}</div>'))
