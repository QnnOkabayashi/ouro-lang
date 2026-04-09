# Comptime evaluation

## Level 1: syntactic non-usage

1. Go through all the syntactic function (so totally independent of args or calls) and determine which
   parameters are used or not. Do this using depth-first search, and assume that cycles do not
   contribute anything.
2. Once we figure out which args do not contribute anything, we will basically ignore them for the
   rest of the program because they are never used. This means that when we cache a fn call, we will
   ignore those parameters which we have proven are never used. Possibly make them raise a warning?

```
fn ReturnJustT(const T: type, N: i32): type {
    // I do not use N
    T
}

fn LinkedListNode(const T: type, N: i32): type {
    // HEY! Do you use N (we refer to it via its SynRef)
    fn MakeImpl(): type {
        struct Impl {
            data: ReturnJustT(T, N),
            next: Option(Box(LinkedListNode(T, N))),
        }
        Impl
        // I use N iff ReturnJustT uses it or if LinkedListNode(T, N) uses it.
        // Okay ReturnJustT does not use it
        // Hit a cycle when checking myself, so assume nothing
        // Okay neither used it, I guess I don't use it.
    }

    MakeImpl()
}
```

## Level 2: semantic non-usage

The next question is: if two function calls produce the same type, meaning they come from the same
`SynRef` (nominal typing) and all the items evaluate identically, but they come from different
function calls (perhaps there's an unused argument), should they be identical types? I think so.
Here's an example:

```
fn Pick(const T: type, const N: i32): type {
    if N < 4 {
        T
    } else {
        struct Other {
            field: T
        }
        Other
    }
}
```

Here, `Pick(bool, 10)` and `Pick(bool, 12)` will both produce structurally identical types from the
same `SynDef`, even though they were constructed from different paths. Thus, it makes sense to
define nominal equivalence as:

- From the same `SynDef` (syntactic definition, i.e., same place in source code), and
- structurally equivalent.

Therefore, it's really easy to decide nominal equivalence.
