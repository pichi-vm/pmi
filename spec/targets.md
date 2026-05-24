# Targets

A PMI **target** is a launch recipe — a CBOR-encoded specification,
carried in a `.pmi.<target>` PE section, that tells a VMM how to
assemble and start a guest VM. Different targets express different
launch paths: a non-CC virtual machine, a confidential VM on AMD
SEV-SNP, on Intel TDX, on Arm CCA. Each target has its own
`.pmi.<target>` PE section, schema, launch model, and per-target
action kinds.

## Shape

Every PMI target's CBOR map follows this skeleton:

```cddl
target = {
  "version" => uint,                       ; schema version
  "actions" => [+ action],                 ; ordered launch recipe
  ; per-target firmware-bound fields and extension attributes
}

action = {
  "type" => tstr,                          ; selects load / fill / ...
  ; per-type fields
}
```

`type` is the only universal action field. Everything else is
defined per action type.

## Launch model

A VMM launches a target by executing this ordered sequence:

1. **Read `.pmi.<target>`.** Locate and decode the target's PE
   section. Refuse to launch if absent.
2. **Initialize.** Perform target-specific setup before processing
   actions (e.g., on confidential targets, call the CC firmware's
   launch-start API).
3. **Process actions.** Execute each entry in the `actions` array
   in array order. Each action's `type` selects the operation; the
   per-type fields parameterize it.
4. **Finalize.** Apply post-action state (e.g., write boot-vCPU
   registers, finalize the CC measurement).
5. **Start the guest.**

Per-target chapters specialize each step.

## Validation

A loader MUST refuse to launch on any of:

- unrecognized `version`;
- unknown key in any CBOR map in the spec;
- unknown action `type`;
- any action's `section` does not name a PE section present in
  the image;
- the same PE section name is referenced by more than one action;
- two action-referenced PE sections have overlapping
  `[VirtualAddress, VirtualAddress + VirtualSize)` ranges.

Per-target specs MAY add further validation rules.
