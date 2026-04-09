# References

References in Sea share a lot of similarities with references in Rust. For example, Rust has
lifetimes that dictate how long a reference is allowed to be used for:

```rust
fn identity<'a>(foo: &'a Foo) -> &'a Foo {
    foo
}
```

In Rust, you would read this as a function that returns a reference to a `Foo` that lives at least
as long as the input reference. In Sea, you would read this as a function that accepts a region `'a`
and an offset into that region to where a `Foo` value is located, and returns the offset which can
only be "dereferenced" when the region `'a` is in scope.

The key detail here is the offset part. Sea does not allow you to store addresses of memory
directly, but rather offsets into regions. Unlike Rust where lifetimes are purely compile-time
constructs, regions in Sea also exist at runtime in the form of pointers to regions, meaning that
when a reference is dereferenced, it actually takes a runtime provided pointer to a region and
dereferences it at the given offset. This all happens under the hood thanks to the compiler, so your
code can mostly still look like Rust code.

## Why

Want to be able to suspend functions at any point, and want those suspended functions to be able to
be relocated freely which can only happen if references are relative instead of direct.

## Coalescing lifetimes and what that looks like in Sea

In Rust, lifetimes are an example of subtyping: if any place that `'short` is requested, a `'long`
will suffice.

```rust
fn lifetime_subtyping<'a, 'b>(a: &'a str, b: &'b str) {
    let chosen: &str = if random() { a } else { b };
    do_something(both);
}
```

Here, the lifetimes are `a` and `b` are shortened to a common lifetime for `chosen`, allowing them
to subtype into the same type. When compiled, this is completely fine because lifetimes are purely
a compile-time concept, and have no impact on compiled code.

This doesn't quite work for Sea, though, because regions also appear at runtime in the form of
raw pointers to allocated regions. If `a` and `b` were to come from two different regions, what
region would `chosen` have? How would it know which region to read from?

One approach is to retain provenance information: whenever a region is coalesced, store a
discriminant describing which region should be accessed.

Alternative, we can semantically concatenate the regions:
The answer is a combined region: `'[a, b]`. Note that this is an ordered list of regions, not an
unordered set.

```
fn lifetime_subtyping<'a, 'b>(a: &'a str, b: &'[a, b] str) {
    let both: [&str; 2] = [a, b];
    do_something(both);
}
```
