# Compile fail tests

## No attribute args yes

```rust
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed(what the)]
async fn f() {}
```

yields:

```rust error
error: #[async_fn_boxed]: unexpected token
 --> tests/errors/snippet_01.rs:4:18
  |
4 | #[async_fn_boxed(what the)]
  |                  ^^^^
```
---

## `async` is mandatory

```rs
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
fn demo(s: &str) -> i32 {
    drop(s);
    async {}.await;
    s;
    42
}
```

yields:

```rust error
error: #[async_fn_boxed]: expected an `async fn`
 --> tests/errors/snippet_02.rs:5:1
  |
5 | fn demo(s: &str) -> i32 {
  | ^^
```
---

## Type mismatch

```rs
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
async fn demo() -> i32 {
    "typo"
}
```

yields:

```rust error
error[E0308]: mismatched types
 --> tests/errors/snippet_03.rs:6:5
  |
6 |     "typo"
  |     ^^^^^^ expected `i32`, found `&str`
  |
note: return type inferred to be `i32` here
 --> tests/errors/snippet_03.rs:5:20
  |
5 | async fn demo() -> i32 {
  |                    ^^^
```

which we can compare to:

```rs
async fn no_macro() -> i32 {
    "typo"
}
```

yielding:

```rust error
error[E0308]: mismatched types
 --> tests/errors/snippet_04.rs:3:5
  |
2 | async fn no_macro() -> i32 {
  |                        --- expected `i32` because of return type
3 |     "typo"
  |     ^^^^^^ expected `i32`, found `&str`
```
---

#### With an elided `-> ()`

```rs
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
async fn demo() {
    "typo"
}
```

yields:

```rust error
error[E0308]: mismatched types
 --> tests/errors/snippet_05.rs:6:5
  |
6 |     "typo"
  |     ^^^^^^ expected `()`, found `&str`
  |
note: return type inferred to be `()` here
 --> tests/errors/snippet_05.rs:5:17
  |
5 | async fn demo() {
  |                 ^
```

which we can compare to:

```rs
async fn demo() {
    "typo"
}
```

yielding:

```rust error
error[E0308]: mismatched types
 --> tests/errors/snippet_06.rs:3:5
  |
2 | async fn demo() {
  |                - help: try adding a return type: `-> &'static str`
3 |     "typo"
  |     ^^^^^^ expected `()`, found `&str`
```
---

## Lifetimes are checked

```rs
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
async fn foo(_: &str) {}

fn call_site() {
    let local = String::from("short_lived");
    let _fut = foo(&local);
    drop(local);
}
```

yields:

```rust error
error[E0505]: cannot move out of `local` because it is borrowed
  --> tests/errors/snippet_07.rs:10:10
   |
8  |     let local = String::from("short_lived");
   |         ----- binding `local` declared here
9  |     let _fut = foo(&local);
   |                    ------ borrow of `local` occurs here
10 |     drop(local);
   |          ^^^^^ move out of `local` occurs here
11 | }
   | - borrow might be used here, when `_fut` is dropped and runs the destructor for type `Pin<Box<dyn Future<Output = ()> + Send>>`
   |
help: consider cloning the value if the performance cost is acceptable
   |
9  -     let _fut = foo(&local);
9  +     let _fut = foo(local.clone());
   |
```

which we can compare to:

```rs
async fn foo(_: &str) {}

fn call_site() {
    let local = String::from("short_lived");
    let _fut = foo(&local);
    drop(local);
}
```

yields:

```rust error
error[E0505]: cannot move out of `local` because it is borrowed
 --> tests/errors/snippet_08.rs:7:10
  |
5 |     let local = String::from("short_lived");
  |         ----- binding `local` declared here
6 |     let _fut = foo(&local);
  |                    ------ borrow of `local` occurs here
7 |     drop(local);
  |          ^^^^^ move out of `local` occurs here
8 | }
  | - borrow might be used here, when `_fut` is dropped and runs the destructor for type `impl Future<Output = ()>`
  |
help: consider cloning the value if the performance cost is acceptable
  |
6 -     let _fut = foo(&local);
6 +     let _fut = foo(local.clone());
  |
```
---

## Improper lifetime elision is detected

```rs
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
async fn foo() -> &str {
    ""
}
```

yields:

```rust error
error[E0106]: missing lifetime specifier
 --> tests/errors/snippet_09.rs:5:19
  |
5 | async fn foo() -> &str {
  |                   ^ expected named lifetime parameter
  |
  = help: this function's return type contains a borrowed value, but there is no value for it to be borrowed from
help: consider using the `'__fut` lifetime
  |
5 | async fn foo() -> &'__fut str {
  |                    ++++++
```
---

## Anonymous lifetimes are properly spanned

```rust
use ::async_fn_boxed::async_fn_boxed;

#[async_fn_boxed]
async fn baz(a: &mut &str, b: &mut &'_ str) {
    *a = *b;
}
```

yields:

```rust error
error: lifetime may not live long enough
 --> tests/errors/snippet_10.rs:6:5
  |
5 | async fn baz(a: &mut &str, b: &mut &'_ str) {
  |                      -              -- lifetime `'__2` defined here
  |                      |
  |                      lifetime `'__0` defined here
6 |     *a = *b;
  |     ^^^^^^^ assignment requires that `'__2` must outlive `'__0`
  |
  = help: consider adding the following bound: `'__2: '__0`
```
---
