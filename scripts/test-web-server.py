import pathlib, subprocess, os, time, ssl, urllib.request, signal, json, sys, tempfile, socket, itertools

root = pathlib.Path(tempfile.mkdtemp(prefix="jffi-web-options-"))
project = pathlib.Path(sys.argv[2]).resolve()
exe = str(pathlib.Path(sys.argv[1]).resolve())
config = project / "jffi.toml"
vite = project / "platforms/web/vite.config.js"
original = config.read_text()
package_json = project / "platforms/web/package.json"
original_package = package_json.read_text()
stale_lock = project / "platforms/web/pnpm-lock.yaml"
original_lock = stale_lock.read_bytes() if stale_lock.exists() else None
original_vite = vite.read_text()
results = []
with socket.socket() as probe:
    probe.bind(("127.0.0.1", 0))
    port = probe.getsockname()[1]
subprocess.run(
    [
        "openssl",
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-keyout",
        str(root / "test.key"),
        "-out",
        str(root / "test.crt"),
        "-days",
        "1",
        "-subj",
        "/CN=localhost",
        "-addext",
        "subjectAltName=DNS:localhost,IP:127.0.0.1",
    ],
    check=True,
    capture_output=True,
)


def launch(name, args, success, https=True, cors=False):
    log = open(root / (name + ".log"), "w")
    p = subprocess.Popen(
        [exe, *args],
        cwd=project,
        stdout=log,
        stderr=subprocess.STDOUT,
        start_new_session=True,
        env=dict(os.environ, BROWSER="none"),
    )
    try:
        ctx = ssl.create_default_context(cafile=str(root / "test.crt"))
        for _ in range(600):
            if p.poll() is not None:
                break
            try:
                with urllib.request.urlopen(
                    urllib.request.Request(
                        f"{'https' if https else 'http'}://localhost:{port}/",
                        headers={"Origin": "https://example.invalid"},
                    ),
                    context=ctx,
                    timeout=0.5,
                ) as r:
                    data = r.read()
                    assert r.status == 200 and b"<html" in data.lower()
                    if cors:
                        assert (
                            r.headers.get("Access-Control-Allow-Origin") == "*"
                        ), dict(r.headers)
                    results.append(dict(name=name, passed=success, tls_verified=https))
                    return
            except (OSError, urllib.error.URLError):
                time.sleep(0.3)
        log.flush()
        text = (root / (name + ".log")).read_text()
        if not success:
            assert "requires server.https certificates" in text, text
        else:
            raise AssertionError(text[-2000:])
        results.append(dict(name=name, passed=True, expected_rejection=True))
    finally:
        try:
            os.killpg(p.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        p.wait()
        log.close()


try:
    config.write_text(
        original.replace("https = false", "https = true").replace(
            "port = 5173", f"port = {port}"
        )
    )
    launch("https-missing-certificate", ["run", "-p", "web", "-v"], False)
    vite.write_text(
        "import fs from 'node:fs';\n"
        + original_vite.replace(
            "plugins: [wasmHotReload()],",
            f"plugins: [wasmHotReload()],\n  server: {{ https: {{key: fs.readFileSync('{root}/test.key'),cert: fs.readFileSync('{root}/test.crt')}} }},",
        )
    )
    tls_vite = vite.read_text()
    launch("https-run", ["run", "-p", "web", "-v"], True)
    launch("https-dev", ["dev", "-p", "web", "-v"], True)
    for command in ["run", "dev"]:
        for tls, host, cors, opened in itertools.product([False, True], repeat=4):
            text = original.replace("port = 5173", f"port = {port}")
            for key, value in [
                ("https", tls),
                ("host", host),
                ("cors", cors),
                ("open", opened),
            ]:
                text = text.replace(f"{key} = false", f"{key} = {str(value).lower()}")
            vite.write_text(tls_vite if tls else original_vite)
            config.write_text(text)
            launch(
                f"options-{command}-{int(tls)}{int(host)}{int(cors)}{int(opened)}",
                [command, "-p", "web", "-v"],
                True,
                https=tls,
                cors=cors,
            )
    config.write_text(
        original.replace('target = "es2020"', 'target = "es2015"')
        + '\n[bundle.web]\nbase_path = "/audit-subpath/"\nwasm_opt = false\n'
    )
    package = json.loads(original_package)
    package["packageManager"] = "npm@11.6.2"
    package_json.write_text(json.dumps(package))
    if original_lock is None:
        stale_lock.write_text("lockfileVersion: '9.0'\n")
    bundle = subprocess.run(
        [exe, "bundle", "-p", "web", "--print-commands"],
        cwd=project,
        capture_output=True,
        text=True,
        timeout=600,
    )
    (root / "bundle.log").write_text(bundle.stdout + bundle.stderr)
    assert bundle.returncode == 0, (bundle.stdout + bundle.stderr)[-4000:]
    assert "$ npm run build -- --config" in bundle.stdout, bundle.stdout
    assert (
        "/audit-subpath/assets/"
        in (project / "target/bundle/web/index.html").read_text()
    )
    script = "import { resolveConfig } from 'vite';const c=await resolveConfig({configFile:'.jffi-build.config.mjs'},'build');if(c.build.target!=='es2015'||c.base!=='/audit-subpath/')throw new Error('JFFI Web options not applied');"
    checked = subprocess.run(
        ["node", "--input-type=module", "-e", script],
        cwd=project / "platforms/web",
        env=dict(
            os.environ, JFFI_WEB_TARGET="es2015", JFFI_BASE_PATH="/audit-subpath/"
        ),
        capture_output=True,
        text=True,
    )
    assert checked.returncode == 0, checked.stderr
    results.append(dict(name="bundle-target-and-base", passed=True))
finally:
    config.write_text(original)
    package_json.write_text(original_package)
    if original_lock is None:
        stale_lock.unlink(missing_ok=True)
    else:
        stale_lock.write_bytes(original_lock)
    vite.write_text(original_vite)
    (root / "https-results.json").write_text(json.dumps(results, indent=2))
assert len(results) == 36 and all(r["passed"] for r in results), results
print(results)
print("Logs:", root)
