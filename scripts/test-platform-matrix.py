import pathlib, subprocess, itertools, json, tomllib, sys, tempfile

root = pathlib.Path(tempfile.mkdtemp(prefix="jffi-platform-matrix-"))
exe = str(pathlib.Path(sys.argv[1]).resolve())
ps = ["ios", "android", "macos", "windows", "linux", "web"]
rows = []
for n in range(1, 7):
    for combo in itertools.combinations(ps, n):
        name = "-".join(combo)
        dest = root / "combinations" / name
        p = subprocess.run(
            [
                exe,
                "new",
                "combo_probe",
                "-p",
                ",".join(combo),
                "-t",
                "hello",
                "-d",
                str(dest),
                "--no-build",
            ],
            capture_output=True,
            text=True,
        )
        error = ""
        try:
            assert p.returncode == 0, p.stderr
            cfg = tomllib.loads((dest / "jffi.toml").read_text())
            assert set(cfg["platforms"]["enabled"]) == set(combo)
            for platform in combo:
                assert (dest / "platforms" / platform).is_dir()
            core = tomllib.loads((dest / "core/Cargo.toml").read_text())
            version = core["dependencies"]["uniffi"]["version"]
            assert ("0.29.4" in version) == ("windows" in combo), (combo, version)
        except Exception as e:
            error = repr(e)
        rows.append(dict(platforms=combo, passed=not error, error=error))
(root / "combinations.json").write_text(json.dumps(rows, indent=2))
print(len(rows), "cases;", sum(not r["passed"] for r in rows), "failures")
for r in rows:
    if not r["passed"]:
        print(r)

assert len(rows) == 63 and all(row["passed"] for row in rows), rows
print("Evidence:", root)
