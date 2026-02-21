# Observation Dashboard

`index.html` is a thin, local-only human view over Observation Surface v0.

It does not compute semantics. It reads from the UX HTTP API exposed by:

```bash
mise run ci-observation-serve
```

Typical local loop:

```bash
mise run ci-observation-build
mise run ci-observation-serve
python3 -m http.server 43173 --directory docs
```

Open `http://127.0.0.1:43173/observation/`.

Default API base in the page is `http://127.0.0.1:43174` and can be changed in
the UI.
