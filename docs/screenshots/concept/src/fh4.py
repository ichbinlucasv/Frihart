exec(open('fh.py').read().split('# 1 new tab')[0])
card=lambda t,s,body,c='#F5C400':f'<div style="flex:1;background:#121212;border:1px solid #242424;border-radius:16px;padding:22px"><div style="color:{c};font-size:12px;letter-spacing:.12em" class="mono">{s}</div><div style="font-size:20px;font-weight:700;margin:8px 0 14px">{t}</div>{body}</div>'
li=lambda items:''.join(f'<div style="font-size:13px;color:#bbb;padding:5px 0">{i}</div>' for i in items)
modes=f'''<div style="display:flex;gap:16px">
{card("Sovereign","MODE · DEFAULT",li(["✓ HTTPS-only","✓ Partitioned cookies","✓ Native blocker, local lists","✓ Frozen UA · no Client Hints","✓ No telemetry · no account"]))}
{card("Anonymous circuit","MODE · OPT-IN",li(['<span class="pill tor" style="padding:2px 7px;font-size:10px">TOR</span> &nbsp;Ctrl+Shift+O','<span class="pill i2p" style="padding:2px 7px;font-size:10px">I2P</span> &nbsp;Ctrl+Shift+I',"✓ SOCKS only, system daemon","✓ Fail-closed · no clearnet fallback","✓ No local DNS on circuit tabs"]),"#b48cff")}
</div>'''
arch='''<div style="background:#0c0c0c;border:1px solid #242424;border-radius:16px;padding:22px;margin-top:16px">
<div class="mono y" style="font-size:12px;letter-spacing:.12em">ARCHITECTURE · NO FRIHART SERVERS</div>
<div style="display:flex;align-items:center;justify-content:space-between;margin-top:18px;font-size:14px">
<div style="padding:14px 18px;border:1px solid #F5C400;border-radius:12px;color:#F5C400;font-weight:700">Your machine</div>
<div style="flex:1;text-align:center;color:#555">────────▶</div>
<div style="display:flex;flex-direction:column;gap:8px">
<div style="padding:8px 14px;border:1px solid #333;border-radius:10px">Website (direct, HTTPS)</div>
<div style="padding:8px 14px;border:1px solid #4a3a7a;border-radius:10px;color:#b48cff">Tor network → .onion</div>
<div style="padding:8px 14px;border:1px solid #1f5a52;border-radius:10px;color:#4fd1c5">I2P network → eepsite</div></div>
<div style="flex:1;text-align:center;color:#555">┆</div>
<div style="padding:14px 18px;border:1px dashed #5a2a2a;border-radius:12px;color:#ff8a8a;text-align:center">Frihart cloud<br><span class="mono" style="font-size:11px">does not exist</span></div>
</div>
<div class="mono" style="font-size:12px;color:#777;margin-top:14px">No account · no sync server · no license server · no crash ping · nothing to seize, nothing to decrypt later</div></div>'''
pq='''<div style="background:#0c0c0c;border:1px solid #242424;border-radius:16px;padding:22px;margin-top:16px;display:flex;gap:24px;align-items:center">
<div style="flex:1"><div class="mono y" style="font-size:12px;letter-spacing:.12em">QUANTUM · HONEST ROADMAP</div>
<div style="font-size:14px;color:#bbb;margin-top:10px">Collect nothing, so a future quantum decrypt finds nothing. Post-quantum TLS (hybrid ML-KEM key exchange) arrives via <span class="mono">rustls</span> when it is stable — no home-made crypto.</div></div>
<div style="display:flex;gap:8px;flex:none"><span class="pill" style="border:1px solid #F5C400;color:#F5C400;padding:4px 10px">Zero data collected · today</span><span class="pill" style="border:1px solid #444;color:#aaa;padding:4px 10px">PQ hybrid TLS · planned</span></div></div>'''
body=f'<div class="content" style="justify-content:center;overflow:hidden"><div style="width:1100px;padding:34px 0"><div style="font-size:30px;font-weight:800">Two modes. <span class="y">Zero servers.</span></div><div style="color:#888;margin:6px 0 22px">Home for Linux users who already live private: Tor, I2P, Monero, Tails, Qubes.</div>{modes}{arch}{pq}</div></div>'
open('fh4.html','w').write(H.format(CSS,f'<div class="win br">{tabs(0,[("","Why Frihart",DIR)]+TL[1:])}{bar("about:frihart",DIR)}{body}{STATUS}</div>'))
