# PMI Index

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

The `.pmi` PE section contains a CBOR-encoded **index** that names the
platforms this image supports and points each one at the PE section containing
its per-platform [manifest](manifest/README.md).

The index is the single, well-known entry point for PMI-aware VMMs. Everything
else about how the image launches on a given platform lives in that platform's
manifest, not here.

## Schema

```cddl
index = {
  "version"   => uint,                 ; schema version, currently 1
  "platforms" => { + tstr => tstr },   ; platform name => PE section name
  * tstr => any,                       ; extension point
}
```

- **`version`** — the index schema version. Currently `1`. VMMs MUST reject
  indexes with an unrecognized version.

- **`platforms`** — a map from platform name (e.g., `"native"`, `"sev"`,
  `"tdx"`, `"cca"`) to the name of the PE section containing that platform's
  [manifest](manifest/README.md). VMMs MUST reject an index whose `platforms`
  map is empty.

## Selection

To launch the image, the VMM:

1. Identifies its target platform (from configuration or hardware detection).
2. Reads the `.pmi` PE section and parses the index.
3. Looks up the target platform name in the `platforms` map.
4. If absent, the image does not support that platform; the VMM MUST refuse
   to launch.
5. If present, the VMM reads the PE section named by the map's value and
   parses it as a per-platform [manifest](manifest/README.md), then follows
   the recipe described there.

There is no fallback. If the image does not declare support for a given
platform, the VMM does not attempt to launch on it.

## PE section naming convention

Per-platform manifest sections SHOULD follow the convention `.pmi.<plat>`
where `<plat>` is the platform name truncated to fit the 8-byte PE section
name limit:

- `.pmi.nat` — native
- `.pmi.sev` — AMD SEV 3.0
- `.pmi.tdx` — Intel TDX
- `.pmi.cca` — Arm CCA

Image authors MAY use any other names; the index is authoritative.

## Extensibility

The index map accepts additional keys beyond `version` and `platforms`.
Extension keys MUST use a collision-resistant namespaced form
(`"namespace:key"`). VMMs MUST ignore unknown index-level keys.

Adding a new platform requires only:

1. A new PE section containing the platform's manifest.
2. A new entry in the index's `platforms` map.

No changes to existing per-platform manifests are needed.
