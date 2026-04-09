# Nominal Equivalence

This file describes how to decide if two types are the same.

If two function calls produce the same type, meaning they come from the same
`SynRef` (nominal typing) and all the items evaluate identically, but they come from different
function calls (perhaps there's an unused argument), should they be identical types? I think so.
Here's an example:

```
fn Pick(const T: type, const N: i32): type {
    if N < 4 {
        T
    } else {
        // Any N >= 4 will go down this branch.
        struct Other {
            x: T
        }
        Other
    }
}
```

Here, `Pick(bool, 10)` and `Pick(bool, 12)` will both produce structurally identical types from the
same `SynDef`, even though they were constructed from different paths. Thus, it makes sense to
define nominal equivalence as:

- From the same `SynDef` (syntactic definition, i.e., same place in source code), and
- structural equivalence.
