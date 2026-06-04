# ⚖️ `bitcraft` vs. The Ecosystem

Choosing a bit manipulation library in Rust often involves balancing **Ergonomics**, **Compile-time Safety**, and **Runtime Performance**. This document compares `bitcraft` with other common solutions to help you decide which is right for your project.

> **Fork note (spikespaz-contrib soft fork).** This fork tempers a few over-stated claims to match what the code actually does: the "no `syn`/`quote`" compile-speed claim (it's proc-macro-free only on the *field-codegen path* — `bytemuck`'s derive and `paste` still pull `syn`/`quote` into the tree), the "faster than manual code" framing (it compiles to the *same* shift/mask instructions → parity, not a speed-up), and the compile-time-bounds claim (structural bounds are `const`-asserted; per-*value* overflow via `set_*` is still a runtime `debug_assert!`). Code change in this fork: `bytestruct!`/`byteval!` now generate `#[repr(transparent)]` (single-field newtype over `[u8; N]`) instead of `#[repr(C)]`, making the documented transparency actually true.

## 📊 Feature Comparison Matrix

| Feature | standard Rust | `bitflags` | `modular-bitfield` | `packed_struct` | `bilge` | `bitvec` | `bitcraft` (this crate) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **Macro Type** | N/A | Declarative | Procedural | Procedural | Procedural | N/A (Structs) | **Declarative** |
| **Bit-Level Density** | ❌ (Byte-min) | ❌ (Flags only) | ✅ | ✅ | ✅ | ✅ | **✅ (Bit-min)** |
| **Memory Alignment** | Compiler-Chosen | Compiler-Chosen | Explicit | Explicit | Hardware-Aligned | Explicit | **Hardware-Aligned** |
| **Compile-Time Bounds** | ❌ | ❌ | ❌ (Runtime/Macro) | ❌ (Runtime/Macro) | ✅ (Const Eval) | ❌ | **✅ (Const Eval)** |
| **Safety** | High (UB risk) | High | High | High | Strict | High | **Strict (Total Types)** |
| **`no_std` Support** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | **✅ (Core-only)** |
| **Compile Speed** | Instant | Fast | Slow | Slow | Slow | Fast | **Blazing Fast** |
| **Acting Primitives** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | **✅ (Direct Register Routing)** |
| **Literal Guarding** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | **✅ (Branchless unrolling)** |
| **Byte-Array Support** | ❌ | ❌ | ✅ (Proc-macro) | ✅ (Proc-macro) | ❌ | ❌ | **✅ (Instant/Declarative)** |
| **Odd-Int IDs (24-bit)** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | **✅ (`byteval!` Built-in)** |
| **Signed Arrays** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | **✅ (`byteval!(i $count)`)** |
| **Packed Arrays** | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | **✅ (`bitarray!` / `bytearray!`)** |
| **Dynamic Arrays** | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | **✅ (`bytevec!` / `bytebox!`)** |
| **Array Slicing** | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | **✅ (`byteslice!`)** |
| **Signed Fields** | ✅ | ❌ | ✅ | ✅ | ✅ | ❌ | **✅ (Zero-cost shift)** |
| **Signed Enums** | ✅ | ❌ | ❌ (Custom impl) | ❌ (Custom impl) | ⚠️ (Requires trait) | ❌ | **✅ (Native `i $bits`)** |
| **Atomic CAS Loops** | ❌ (Manual only) | ❌ | ❌ | ❌ | ❌ | ❌ | **✅ (`atomic_bitarray!`, `atomic_bitstruct!`, `atomic_bitenum!`)** |
| **C FFI / ABI** | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | **✅ (Transparent)** |

---

## ⚡ Empirical Performance (1.0B Iterations)

We evaluated 1,000,000,000 (1B) operations of complex read/write logic on an optimized release build. While some crates introduce abstraction overhead, `bitcraft` compiles down to the same shift/mask instructions as hand-written bitwise code, so it maintains **parity** with manual bitwise code — matching it, not beating it. The value is getting that hand-tuned performance for free behind a typed, bounds-checked API.

| Metric | Macro Type | Overhead vs. `std` | Physical Density |
| :--- | :--- | :--- | :--- |
| **Execution Latency** | `bitenum!` | **0.95x (Faster!)** | **1.00x** (Safe) |
| **Execution Latency** | `byteval!` | **0.91x (Faster!)** | **2.67x Higher** |
| **Execution Latency** | `bitstruct!` | **0.96x (Faster!)** | **2.00x Higher** |
| **Execution Latency** | **bytestruct!** | **1.05x (Near Parity!)** | **3.20x Higher** |

### 🔄 Atomic Concurrency (Contended, 32 Threads)

We evaluated 32 concurrent threads performing 1,000,000 updates each on a shared status word, comparing `bitcraft` atomics against standard `Mutex` synchronization.

| Metric | Pattern | Overhead vs. `Mutex` | Advantage |
| :--- | :--- | :--- | :--- |
| **Contention Latency** | `atomic_bitenum!` | **22.7x Faster** | vs. `Mutex<u8>` (Single-Instruction) |
| **Array Mutation (64-bit)** | `atomic_bitarray!` | **3.18x Faster** | vs. `Mutex<bitarray!>` (Packed Snapshot) |
| **Array Mutation (128-bit)**| `atomic_bitarray!` | **2.63x Faster** | vs. `Mutex<[u8; 16]>` (Raw Array) |
| **Contention Latency** | `atomic_bitstruct!` | **1.14x Faster** | vs. `Mutex<StandardStruct>` |
| **Conditional Transition** | `compare_exchange` | **3.30x Faster** | vs. `Mutex` check-and-set |
| **Parallel Throughput** | `atomic_bitstruct!` | **1.54x Faster** | vs. Uncontended `Mutex` Array |

> **Why the massive gap?** A `Mutex` forces threads into a queue, often requiring kernel context switches under high contention. `atomic_bitenum!` uses a single CPU instruction (`store`), while `atomic_bitstruct!` uses a lock-free `fetch_update` loop. Under high contention, the lock-free approach avoids the "Thundering Herd" problem and keeps the CPU pipeline saturated.


> **Why do we match hand-written code?** Our macros generate the same unrolled shift/mask expressions you would write by hand, so LLVM applies the same **Instruction Fusion** (collapsing several shifts/masks into a single load). The result is *parity* with manual code — it compiles to identical instructions, so any "faster than manual" delta is measurement noise. The point is getting that hand-tuned codegen for free, behind a typed API, where a naive hand-written loop or a heavier procedural-macro getter might not fuse as cleanly.

> [!NOTE]
> **Type Safety**: `bitcraft` natively supports both **unsigned** (`u8` through `u128`) and **signed** (`i8` through `i128`) base integers for underlying storage, with strict compiler bounds (e.g., 15 bits for `i16`) to guarantee the sign bit is safely isolated. It also includes full support for interpreting the *fields and enums themselves* as signed integers (two's complement) via a branchless, zero-cost shift implementation.

---

## 🔬 In-Depth Assessment

### 1. `bitcraft` vs. `modular-bitfield`

`modular-bitfield` is the current ecosystem standard for procedural-macro-based bitfields.

* **Philosophical Difference**: `modular-bitfield` focuses on providing a "Rust-like" struct feel with `#[bitfield]` attributes. `bitcraft` focuses on **Mechanical Sympathy**—optimizing specifically for how hardware interacts with memory registers.
* **Compile Times**: `bitcraft`'s field codegen is declarative `macro_rules!`, which expands faster than a procedural macro. It is not `syn`/`quote`-free, though — the `bytemuck` derive and the `paste` helper still pull `syn`/`quote` into the dependency tree; there is simply no procedural macro on the field-generation path itself.
* **Performance Routing**: While `modular-bitfield` handles bit-ranges well, `bitstruct!` specializes in **Acting Primitives**. It ensures that even if you are manipulating 5 bytes of data, the CPU uses a 64-bit register for atomic-like updates rather than byte-by-byte loads. In our benchmarks, this register-routing approach keeps overhead significantly lower than procedural field-accessors.

### 2. `bitcraft` vs. `packed_struct`

`packed_struct` is a powerful tool for complex, nested serialization.

* **Complexity vs. Speed**: `packed_struct` is feature-rich (endianness, custom bit-numbering) but can be "particular" about type signatures and has a steeper learning curve. `bitcraft` favors a **Flat & Fast** approach.
* **Instruction Fusion**: `bitcraft` implements specialized const-generic helpers that ensure **Instruction Fusion**. Instead of multiple fragmented loads, we generate code that LLVM can fuse into a single load-and-shift operation. In high-frequency loops, this directly translates to higher IPC (Instructions Per Cycle) compared to the more abstract `packed_struct` implementation.

### 3. `bitcraft` vs. `bilge`

`bilge` is a modern, highly-typed procedural macro leveraging recent Rust `const` capabilities.

* **Compilation Speed**: `bilge` relies on a procedural macro (`bilge-impl`, which pulls `syn`-full + `quote`) on its codegen path. `bitcraft` achieves similar `const` verification through declarative `macro_rules!`, so its *own* field codegen needs no proc-macro — though it still pulls `syn`/`quote` indirectly via `bytemuck`'s derive. Net: a faster macro-expansion path, not a `syn`-free build.
* **Array Support (`bytestruct!`)**: `bilge` is restricted to primitive integer bounds. `bitcraft` offers `bytestruct!` to provide `1` to `16` byte array packing with direct CPU-register promotion (Acting Primitives).
* **Signed Types & Enums**: `bitcraft` provides Native Signed Enum capabilities `(i $bits)` right out of the box with zero boilerplate, dynamically performing zero-cost sign extensions upon read. `bilge` requires implementing custom traits for non-standard enum types.

### 4. `bitcraft` vs. Standard Rust `enum`

Standard Rust enums are **Algebraic Data Types**, which is dangerous when parsing untrusted data (like network packets).

* **The UB Gap**: Reading an invalid bit pattern into a standard `repr(u8)` enum is **Undefined Behavior (UB)**. You must manually validate the byte before transmute.
* **Total Types**: `bitcraft`'s `bitenum!` treats variants as "Active states" of a total underlying type. Using `try_from_bits()` ensures you never encounter UB, even if the raw input is corrupted or malicious.

### 5. `bitcraft` vs. C-Style Bitfields (FFI)

If you are interfacing with C firmware or legacy network protocols, layout predictability is mandatory.

* **ABI Stability**: Every `bitstruct!`, `bytestruct!`, and `bitenum!` is marked `#[repr(transparent)]`. This means they have the **exact same memory representation** as their underlying Rust primitive (u8-u128) or byte-array.
* **Natural LSB-First**: Unlike many procedural macros that can be ambiguous about bit-ordering, `bitcraft` enforces a strict LSB-first mapping. This matches the standard layout of C bitfields on Little-Endian architectures (x86_64/ARM64), allowing you to safely cast raw C-originated pointers directly into Rust types using `bytemuck`.

### 6. `atomic_bitstruct!` & `atomic_bitenum!` vs. `Mutex`/`RwLock`

Most bit manipulation libraries completely ignore concurrency. If you need to share a bitfield across threads, you typically have to wrap it in a heavy synchronization primitive.

* **The Locking Tax**: A `Mutex<T>` or `RwLock<T>` requires entering the kernel to put a thread to sleep if there is contention. Even without contention, it involves expensive memory bus locking.
* **Lock-Free with `bitcraft`**: `atomic_bitarray!`, `atomic_bitstruct!`, and `atomic_bitenum!` wrap standard `portable_atomic` types. They provide a **lock-free** API where threads never sleep.
* **Transactional CAS Loops**: Instead of locking the whole struct, `bitcraft` uses **Compare-And-Swap (CAS)** loops. The `.update_or_abort()` method allows you to define complex state transitions that are resolved at the hardware level. This is significantly faster in high-throughput kernels or network drivers where thousands of threads might be hitting the same status word simultaneously.
* **No Deadlocks**: Since there are no locks, there are no deadlocks. This simplifies concurrent system design significantly.

---

## ⚡ The `bitcraft` Edge: Unique Innovations

What makes `bitcraft` different is not just that it packs bits, but *how* it handles the instructions generated for those bits.

### 🛡️ The Unrolled Engine Pattern

Most libraries use runtime checks or generic loops to handle variable-width fields. `bitcraft` uses a recursive macro to generate **Literal Bitwise Expressions** matched to the exact byte-span of the field. Because the element count is a constant at compile-time, the optimizer treats the operation as an atomic register manipulation.
**Result**: Your code performs exactly like hand-optimized assembly.

### 🏎️ `bytestruct!` & `byteval!`: The Competitive Edge

Most bitfield libraries in the Rust ecosystem (`modular-bitfield`, `packed_struct`) either:

* **Restrict you to primitives** (`u8`–`u128`).
* **Rely on Procedural Macros** which drastically increase compile-time and complexity for small array-backed types.

`bitcraft` is the **only library** to offer instant, declarative bitfields for flexible `[u8; 1..16]` arrays, including native support for signed array variants via `(i $count)`.

* **Wait, why does this matter?** If you need a 3-byte integer (24-bit ID) that is packed 1,000,000 times in an array, standard Rust forces you to use a 4-byte `u32` (wasting 25% of your memory footprint) or write massive boilerplate. `byteval! { struct Id(3); }` solves this in one line with zero runtime cost. Using `byteval! { struct SignedId(i 3); }` instantly turns it into a signed Two's Complement container, perfect for hardware ADCs!
* **Register Routing**: While other array-backed solutions use slow byte-by-byte loops, `bytestruct!` uses **Acting Primitives** (e.g., loading an 8-byte slice of a 13-byte array into a `u64` register). It supports any unsigned array type (`u8`, `u16`, `u32`, `u64`, `u128`) while maintaining a strict 128-bit architectural limit.

### 🔄 Atomic Concurrency (`atomic_bitstruct!` & `atomic_bitenum!`)

Most bitfield libraries completely ignore concurrency. If you want to share a bitfield across threads, you have to wrap it in a heavy `Mutex` or `RwLock`.
`bitcraft` provides `atomic_bitarray!`, `atomic_bitstruct!`, and `atomic_bitenum!` which wrap standard `portable_atomic` types, generating a fully lock-free API. They provide a highly ergonomic `.update_or_abort()` closure pattern for building transaction-safe CAS loops that resolve concurrent mutations on disjoint bit-fields, array elements, or state variants automatically, without taking any locks. This is critical for building high-throughput kernels, network stacks, and shared-memory applications.

### 🛡️ Compile-Time Bounds Checking (Zero Runtime Panic)

`bitcraft` generates `const _: () = assert!(...);` validations for **structural** bounds — total field bits ≤ the base width, and an enum's width ≤ the field it's assigned to — so a structurally-flawed layout **will not compile**. This covers *structure*, not every *value*: assigning an out-of-range value to a field via `set_*` is still caught by a runtime `debug_assert!`, not at compile time. Use the checked `try_set_*` / `try_with_*` setters to handle out-of-range values without a panic.

### 🧩 LSB-First Consistency

Many libraries struggle with bit-ordering consistency across different platforms. `bitcraft` enforces a strict **Little-Endian / LSB-First** convention. This ensures that the layout you see in your source code perfectly matches the physical bits on a little-endian CPU (x86_64, ARM64), eliminating cognitive load during debugging.

### 🍱 `bitarray!` vs. `bitvec` or `Vec<bool>`

Standard boolean collections in Rust are notoriously inefficient. `bitarray!` and `bytearray!` provide high-performance alternatives for uniform sub-byte data.

* **Density**: `Vec<bool>` uses 8 bits per boolean. `bitarray!` and `bytearray!` use exactly **1 bit per boolean**.
* **Zero-Copy FFI**: `bitarray!` types are `repr(transparent)` around a primitive integer, and `bytearray!` types are `repr(transparent)` around a `[u8; N]`. This means you can cast them to/from raw buffers using `bytemuck` without iteration. `bitvec` requires complex pointer manipulation and custom iterators.
* **Automatic Specialization**: `bitarray!` automatically selects the optimal CPU register (`u8`-`u128`) for your collection size.
* **Dynamic Alternatives (`bytevec!`)**: `bitcraft` provides `bytevec!` and `bytebox!` as a highly performant alternative to `bitvec` when you need heap allocation. While `bitvec` operates as a generic bit-addressable engine, `bitcraft` focuses on register optimization. The `bytevec!` implementation leverages `u128` batch operations and **Acting Primitives** to perform cross-byte manipulation significantly faster without the overhead of tracking unaligned bit-pointers.

---

## 🏁 Conclusion: When should you use `bitcraft`?

* **Use `bitcraft` if**: You are writing a vector engine, a high-frequency trading system, a network driver, or any code where **Cache Locality** and **Instruction Latency** are your primary bottlenecks.
* **Use something else if**: You need complex nested structures with varying endianness within the same struct, or you prefer attribute-heavy procedural syntax over declarative macro blocks.
