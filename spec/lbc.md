# Linux boot configuration (`lbc`)

The `lbc:bootconfig` fill supplies Linux boot configuration as a
[measured fill](core.md#measurement). A PMI image MAY declare this fill; each
target's action list MUST contain at most one.

```cbor-diag
{"type": "fill", "gpa": 0x2100000, "section": ".bconf",
 "kind": "lbc:bootconfig"}
```

## Wire representation

The fill MUST name a Zero section whose `VirtualSize` sets the reservation
capacity. Placement and zero page padding follow the core [`fill`](core.md#fill)
and [page granularity](granularity.md) rules.

For nonempty input, the VMM MUST place the complete native Linux bootconfig block
at the end of the reservation and zero the unused prefix. It MUST reject input
that exceeds the reservation capacity or native framing bounds:

```text
[unused zeros][bootconfig data and padding][size: LE32][checksum: LE32][#BOOTCONFIG\n]
```

The 12-byte magic ends exactly at the end of `VirtualSize`. Size counts the
bootconfig data, including its NUL and padding; checksum is the sum of those
bytes modulo 2^32.

The format, alignment, padding, syntax, operators and limits follow upstream
[Linux bootconfig](https://docs.kernel.org/admin-guide/bootconfig.html), including
its [implementation](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/lib/bootconfig.c)
and [native tool](https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/tools/bootconfig).

When input is absent or empty, the VMM MUST zero the entire reservation.

## Linux handoff

An image declaring this fill MUST pass the supplied block through Linux's native
bootconfig path, providing kernel support, activation, appropriate initrd bounds,
and the `/proc/bootconfig` interface. Linux handles parsing, interpretation,
exposure and errors. Consumers are responsible for checking settings they require.
