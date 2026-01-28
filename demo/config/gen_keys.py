import json, secrets, pathlib

tmpl_path = pathlib.Path("demo/config/keys.json.template")
out_path  = pathlib.Path("demo/config/keys.json")

tmpl = json.loads(tmpl_path.read_text(encoding="utf-8"))

out = {}
for k, v in tmpl.items():
    if isinstance(v, str):
        out[k] = secrets.token_hex(32)  # 64 hex chars
    else:
        out[k] = v

out_path.write_text(json.dumps(out, indent=2) + "\n", encoding="utf-8")
print(f"Wrote {out_path} bytes: {out_path.stat().st_size}")
